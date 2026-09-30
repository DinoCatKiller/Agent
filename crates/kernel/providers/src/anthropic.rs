//! Anthropic 适配器（供应商差异记录：`P2`）。
//!
//! 覆盖 Anthropic Messages API 及其兼容实现（如 Kimi 等 Anthropic 兼容端点）。
//! 只依赖 `A2` 契约与 `agent-transport`；供应商语义（字段名、错误码、收尾）都在本模块归一化，
//! 不得泄漏到编排核心（`A1` 硬约束）。
//!
//! 与 OpenAI 差异最大的几处（详见 `P2`）：
//! - POST `{base_url}/v1/messages`，`x-api-key` + `anthropic-version` 头鉴权；
//! - `max_tokens` **必填**（缺省由本模块兜底）；`system` 是顶层参数；role 只有 user / assistant；
//! - 消息 role **必须交替**：连续同角色消息合并，连续 Tool 结果并入同一条 user 消息；
//! - 流式是 **命名事件** SSE（`event: message_start` …），usage 分两处（`message_start` 给
//!   input、`message_delta` 给累计 output）；工具参数是结构化 `input`（非字符串）。

use std::collections::BTreeMap;
use std::sync::Arc;

use agent_common::{
    Capability, ContentPart, DeltaKind, ErrorCategory, FinishReason, ModelRequest, ModelResponse,
    ModelSpec, ProviderError, Role, StreamEvent, ToolCall, ToolChoice, ToolDefinition, Usage,
};
use agent_transport::{
    Frame, FramePolicy, HttpClient, HttpConfig, SseEvent, TransportError, finalize, guard,
    parse_sse,
};
use async_stream::stream;
use futures::StreamExt;
use reqwest::RequestBuilder;
use serde_json::{Map, Value, json};

use crate::{CallContext, EventStream, Provider};

/// Anthropic API 版本头。日期固定于接入时验证过的版本，升级需重跑 fixtures。
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Anthropic 请求 `max_tokens` **必填**；调用方未给时的兜底值（`P2` 已知坑）。
pub const DEFAULT_MAX_TOKENS: u32 = 4096;

/// Anthropic 适配器配置。
///
/// `models` 非空时做本地能力协商（模型不在清单或能力不足即 fast-fail，`A2` §1）；
/// 为空时不拦截，交给供应商报错（用于尚未登记清单的兼容实现）。
#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    /// 供应商 id（如 `"anthropic"`）。
    pub id: &'static str,
    /// 如 `https://api.anthropic.com`（`/v1/messages` 由适配器拼接，不带尾部斜杠）。
    /// 真实 endpoint 由配置层注入，不入库（`R1`）。
    pub base_url: String,
    /// 密钥由配置层注入；`None` 表示不带头（部分本地代理）。
    pub api_key: Option<String>,
    pub http: HttpConfig,
    pub models: Vec<ModelSpec>,
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            id: "anthropic",
            base_url: "https://api.anthropic.com".to_string(),
            api_key: None,
            http: HttpConfig::default(),
            models: anthropic_default_models(),
        }
    }
}

/// Anthropic 官方常用模型静态清单（快照，随时点过时）。兼容实现应由配置层传入自己的 `ModelSpec`。
pub fn anthropic_default_models() -> Vec<ModelSpec> {
    fn spec(id: &str, ctx: u32, out: u32, caps: &[Capability]) -> ModelSpec {
        ModelSpec {
            id: id.to_string(),
            provider: "anthropic".to_string(),
            context_window: ctx,
            max_output: out,
            capabilities: caps.to_vec(),
            pricing: None,
            deprecated: false,
        }
    }
    // 3.7 起支持扩展思考；无 JsonMode/JsonSchema（协议无 response_format，见 `P2` §3）。
    let thinking = [
        Capability::Text,
        Capability::Vision,
        Capability::Tools,
        Capability::Reasoning,
    ];
    let chat = [Capability::Text, Capability::Vision, Capability::Tools];
    vec![
        spec("claude-opus-4-1-20250805", 200_000, 32_000, &thinking),
        spec("claude-sonnet-4-5-20250929", 200_000, 64_000, &thinking),
        spec("claude-sonnet-4-20250514", 200_000, 64_000, &thinking),
        spec("claude-3-7-sonnet-20250219", 200_000, 64_000, &thinking),
        spec("claude-3-5-sonnet-20241022", 200_000, 8_192, &chat),
        spec("claude-3-5-haiku-20241022", 200_000, 8_192, &chat),
    ]
}

/// Anthropic 适配器。`Arc<HttpClient>` 保证 `Clone` 只是引用计数（`A2` §2 硬要求）。
#[derive(Debug, Clone)]
pub struct AnthropicCompatible {
    id: &'static str,
    client: Arc<HttpClient>,
    base_url: String,
    api_key: Option<String>,
    models: Vec<ModelSpec>,
}

impl AnthropicCompatible {
    pub fn new(config: AnthropicConfig) -> Result<Self, ProviderError> {
        let client = HttpClient::new(config.http).map_err(|e| {
            ProviderError::new(config.id, ErrorCategory::Unknown).with_message(e.to_string())
        })?;
        Ok(Self {
            id: config.id,
            client: Arc::new(client),
            base_url: config.base_url,
            api_key: config.api_key,
            models: config.models,
        })
    }

    fn messages_url(&self) -> String {
        format!("{}/v1/messages", self.base_url.trim_end_matches('/'))
    }

    fn with_auth(&self, builder: RequestBuilder) -> RequestBuilder {
        let builder = builder.header("anthropic-version", ANTHROPIC_VERSION);
        match &self.api_key {
            Some(key) => builder.header("x-api-key", key),
            None => builder,
        }
    }

    /// 能力协商 fast-fail（`A2` §1 / `Q1` 用例 12）：发请求**之前**拒绝不支持的组合。
    pub fn check_capabilities(&self, req: &ModelRequest) -> Result<(), ProviderError> {
        crate::check_model_capabilities(self.id, &self.models, req)
    }

    /// 归一化 `ModelRequest` → Anthropic Messages 请求体。`pub` 供契约测试断言（`Q1` 用例 4）。
    ///
    /// `response_format` 无原生对应：清单模型缺 `JsonMode`/`JsonSchema` 能力，由
    /// [`Self::check_capabilities`] 在发请求前拦截，本函数不做映射（`P2` §3）。
    pub fn build_request_body(
        &self,
        req: &ModelRequest,
        stream: bool,
    ) -> Result<Value, ProviderError> {
        let mut body = Map::new();
        body.insert("model".into(), Value::String(req.model.clone()));
        // 协议必填：调用方未给时兜底，避免 400（`P2` 已知坑）。
        body.insert(
            "max_tokens".into(),
            Value::from(req.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS)),
        );
        let (system, messages) = self.map_messages(req);
        body.insert("messages".into(), Value::Array(messages));
        if let Some(system) = system {
            body.insert("system".into(), Value::String(system));
        }
        body.insert("stream".into(), Value::Bool(stream));
        if let Some(temperature) = req.temperature {
            body.insert("temperature".into(), Value::from(temperature));
        }
        if !req.stop.is_empty() {
            body.insert(
                "stop_sequences".into(),
                Value::Array(req.stop.iter().map(|s| Value::String(s.clone())).collect()),
            );
        }
        if !req.tools.is_empty() {
            body.insert(
                "tools".into(),
                Value::Array(req.tools.iter().map(map_tool).collect()),
            );
            if let Some(choice) = &req.tool_choice {
                body.insert("tool_choice".into(), map_tool_choice(choice));
            }
        }
        // 厂商特有参数透传（`A2` §2：唯一允许出现厂商字段的地方；如 thinking / top_k / top_p）。
        for (key, value) in &req.additional_params {
            body.insert(key.clone(), value.clone());
        }
        Ok(Value::Object(body))
    }

    /// 消息映射。与 OpenAI 最大的差异：
    /// - `System` 提升为顶层 `system`（多条按序以空行连接）；
    /// - role 只有 user / assistant 且**必须交替**——连续同角色消息合并为一条，
    ///   连续 Tool 结果并入同一条 user 消息的多个 `tool_result` 块（`P2` 已知坑）。
    fn map_messages(&self, req: &ModelRequest) -> (Option<String>, Vec<Value>) {
        let mut system_parts: Vec<String> = Vec::new();
        let mut out: Vec<Value> = Vec::new();
        // 正在累积的「同侧」消息：user 侧 = User + Tool（Tool 结果是 user 消息里的块）。
        let mut side: Option<Role> = None;
        let mut blocks: Vec<Value> = Vec::new();

        for msg in &req.messages {
            match msg.role {
                Role::System => system_parts.push(msg.text()),
                Role::User | Role::Tool => {
                    if side != Some(Role::User) {
                        flush_side_message(&mut out, &mut side, &mut blocks);
                        side = Some(Role::User);
                    }
                    if msg.role == Role::Tool {
                        // 工具结果回填（`Q1` 用例 4）：user 消息里的 tool_result 块。
                        blocks.push(json!({
                            "type": "tool_result",
                            "tool_use_id": msg.tool_call_id.clone().unwrap_or_default(),
                            "content": msg.text(),
                        }));
                    } else {
                        blocks.extend(self.map_content(&msg.content));
                    }
                }
                Role::Assistant => {
                    if side != Some(Role::Assistant) {
                        flush_side_message(&mut out, &mut side, &mut blocks);
                        side = Some(Role::Assistant);
                    }
                    blocks.extend(self.map_content(&msg.content));
                    for call in &msg.tool_calls {
                        // `input` 直接用结构化 JSON（协议如此，与入方向归一化对称）。
                        blocks.push(json!({
                            "type": "tool_use",
                            "id": call.id,
                            "name": call.name,
                            "input": call.arguments,
                        }));
                    }
                }
            }
        }
        flush_side_message(&mut out, &mut side, &mut blocks);

        let system = if system_parts.is_empty() {
            None
        } else {
            Some(system_parts.join("\n\n"))
        };
        (system, out)
    }

    /// 内容块映射：Text → `text` 块（空文本跳过，空块会被 400）；Image → `image` 块；
    /// 图片类 File 同 Image；非图片 File 丢弃并告警（`P2` 已知坑，document 块属二期）。
    fn map_content(&self, parts: &[ContentPart]) -> Vec<Value> {
        let mut blocks = Vec::new();
        for part in parts {
            match part {
                ContentPart::Text { text } => {
                    if text.is_empty() {
                        continue;
                    }
                    blocks.push(json!({ "type": "text", "text": text }));
                }
                ContentPart::Image { url, .. } => {
                    blocks.push(map_image_source(url));
                }
                ContentPart::File {
                    name,
                    mime_type,
                    data,
                    url,
                } => {
                    let is_image = mime_type
                        .as_deref()
                        .map(|m| m.starts_with("image/"))
                        .unwrap_or(false);
                    if is_image {
                        if let Some(data) = data {
                            let mime = mime_type.clone().unwrap_or_else(|| "image/png".to_string());
                            blocks.push(json!({
                                "type": "image",
                                "source": {"type": "base64", "media_type": mime, "data": data}
                            }));
                        } else if let Some(url) = url {
                            blocks.push(map_image_source(url));
                        }
                    } else {
                        tracing::warn!(
                            file = name,
                            "dropping non-image file attachment (Messages API 一期不支持，见 P2 §6)"
                        );
                    }
                }
            }
        }
        blocks
    }
}

/// 把累积中的同侧消息收口为一条消息。空内容段直接丢弃
/// （如仅含空文本占位的 assistant 段，发出去会被 400）。
fn flush_side_message(out: &mut Vec<Value>, side: &mut Option<Role>, blocks: &mut Vec<Value>) {
    let Some(role) = side.take() else {
        return;
    };
    if blocks.is_empty() {
        return;
    }
    let role = if role == Role::Assistant {
        "assistant"
    } else {
        "user"
    };
    out.push(json!({ "role": role, "content": std::mem::take(blocks) }));
}

/// 图片 URL → Anthropic `image` 块。`data:` URI 拆出 media_type 与 base64，
/// 其余按 URL source 透传（要求模型支持 URL 输入）。
fn map_image_source(url: &str) -> Value {
    if let Some(rest) = url.strip_prefix("data:")
        && let Some((mime, data)) = rest.split_once(";base64,")
    {
        return json!({
            "type": "image",
            "source": {"type": "base64", "media_type": mime, "data": data}
        });
    }
    json!({ "type": "image", "source": {"type": "url", "url": url} })
}

fn map_tool(tool: &ToolDefinition) -> Value {
    json!({
        "name": tool.name,
        "description": tool.description,
        "input_schema": tool.parameters,
    })
}

/// `tool_choice` 映射。我们的 `Required` 对应 Anthropic 的 `any`（`A2` §3）。
fn map_tool_choice(choice: &ToolChoice) -> Value {
    match choice {
        ToolChoice::Auto => json!({"type": "auto"}),
        ToolChoice::None => json!({"type": "none"}),
        ToolChoice::Required => json!({"type": "any"}),
        // 只支持指定**一个**工具；多取第一个。
        ToolChoice::Specific(names) => match names.first() {
            Some(name) => json!({"type": "tool", "name": name}),
            None => json!({"type": "auto"}),
        },
    }
}

impl Provider for AnthropicCompatible {
    fn id(&self) -> &'static str {
        self.id
    }

    fn list_models(&self) -> Vec<ModelSpec> {
        self.models.clone()
    }

    async fn chat(
        &self,
        req: ModelRequest,
        ctx: CallContext,
    ) -> Result<ModelResponse, ProviderError> {
        self.check_capabilities(&req)?;
        let body = self.build_request_body(&req, false)?;
        let request = self.with_auth(self.client.client().post(self.messages_url()).json(&body));
        let response = self
            .client
            .send(request, &ctx.cancel)
            .await
            .map_err(|e| crate::map_transport_error(self.id, &e))?;
        let status = response.status();
        if !status.is_success() {
            let raw = response.json::<Value>().await.unwrap_or(Value::Null);
            return Err(map_http_error(self.id, status.as_u16(), raw));
        }
        let raw: Value = response.json().await.map_err(|e| {
            ProviderError::new(self.id, ErrorCategory::Unknown).with_message(e.to_string())
        })?;
        parse_message_response(self.id, raw)
    }

    fn stream(&self, req: ModelRequest, ctx: CallContext) -> Result<EventStream, ProviderError> {
        // 同步 fast-fail：能力协商 / 参数错误在发请求前暴露（`A2` §2）。
        self.check_capabilities(&req)?;
        let this = self.clone();
        let out = stream! {
            let body = match this.build_request_body(&req, true) {
                Ok(body) => body,
                Err(err) => { yield StreamEvent::Error { error: err }; return; }
            };
            let request = this.with_auth(this.client.client().post(this.messages_url()).json(&body));
            let response = match this.client.send(request, &ctx.cancel).await {
                Ok(response) => response,
                Err(err) => { yield StreamEvent::Error { error: crate::map_transport_error(this.id, &err) }; return; }
            };
            let status = response.status();
            if !status.is_success() {
                let raw = response.json::<Value>().await.unwrap_or(Value::Null);
                yield StreamEvent::Error { error: map_http_error(this.id, status.as_u16(), raw) };
                return;
            }
            // 管线：字节流 → 空闲超时 → SSE → 三分收尾（FramePolicy）→ 取消守卫（`A4`）。
            let bytes = response.bytes_stream().map(|r| r.map_err(TransportError::from));
            let mut finalized = guard(
                finalize(
                    parse_sse(this.client.guard_idle(bytes)),
                    AnthropicFramePolicy::new(this.id),
                ),
                ctx.cancel,
            );
            while let Some(item) = finalized.next().await {
                match item {
                    Ok(event) => yield event,
                    Err(err) => {
                        yield StreamEvent::Error { error: crate::map_transport_error(this.id, &err) };
                        return;
                    }
                }
            }
        };
        Ok(Box::pin(out))
    }

    /// 本地粗略估算（≈4 字符/token）。Anthropic 有真实计数端点
    /// （POST `/v1/messages/count_tokens`），但 `Provider::count_tokens` 是同步签名，
    /// 一期不做 HTTP（`P2` §6）。
    fn count_tokens(&self, req: &ModelRequest) -> Option<u32> {
        let chars: usize = req.messages.iter().map(|m| m.text().chars().count()).sum();
        Some((chars / 4) as u32)
    }
}

/// `tool_use` 块的拼装状态：`content_block_start` 给 id / name（可能带完整 `input`），
/// `input_json_delta` 分片给参数文本，`content_block_stop` 收口（`P2` §3）。
#[derive(Debug, Default)]
struct PendingToolBlock {
    id: String,
    name: String,
    arguments: String,
    /// start 时自带的 input（无分片时直接用它，如缓存命中或实现差异）。
    initial: Value,
}

/// Anthropic 流式帧判决策略：命名事件 `SseEvent` → 归一化 `StreamEvent`。
///
/// - `message_stop` 是终止记录；usage 分两处（`message_start` 的 input + `message_delta`
///   的累计 output），在终止处合并发出（`A2` §4：`End` 前必须给一次 `Usage`）；
/// - 流中 `error` 事件（如 overloaded）→ `Error` 终止，之后不再发任何事件（`A2` §4）；
/// - 非法 JSON 帧 → `Frame::Skip`（`Q1` 用例 6）；
/// - EOF 无 `message_stop` → `on_truncated` 补 `Usage` + `End { Truncated }`（用例 5/11）。
///
/// 与 [`OpenAiFramePolicy`](crate::openai::OpenAiFramePolicy) 不同，本策略需要 provider 名：
/// Anthropic 的错误可以以**流中事件**形式出现，`ProviderError::provider` 必须有归属。
#[derive(Debug, Default)]
pub struct AnthropicFramePolicy {
    provider: String,
    response_id: Option<String>,
    model: Option<String>,
    started: bool,
    usage: Option<Usage>,
    finish_reason: Option<FinishReason>,
    pending: BTreeMap<usize, PendingToolBlock>,
}

impl AnthropicFramePolicy {
    pub fn new(provider: &str) -> Self {
        Self {
            provider: provider.to_string(),
            ..Self::default()
        }
    }

    fn ensure_start(&mut self, events: &mut Vec<StreamEvent>) {
        if self.started {
            return;
        }
        self.started = true;
        events.push(StreamEvent::Start {
            response_id: self.response_id.clone().unwrap_or_default(),
            model: self.model.clone().unwrap_or_default(),
        });
    }

    /// 终止/截断时的末帧序列：`ToolCall*`（未收口分片的兜底）→ `Usage` → `End`
    /// （`A2` §4：`End` 前必须给一次 `Usage`；Anthropic 无独立 usage 帧，必在此补发）。
    fn finish(&mut self, fallback: FinishReason) -> Vec<StreamEvent> {
        let mut events = Vec::new();
        // BTreeMap 按 index 有序，产出顺序与块出现顺序一致。
        for pending in self.pending.values() {
            // 拼装完成的 arguments 必须是可解析的完整 JSON（`Q1` 用例 3）；非法时容错为 Null。
            let arguments = if pending.arguments.trim().is_empty() {
                pending.initial.clone()
            } else {
                serde_json::from_str(&pending.arguments).unwrap_or(Value::Null)
            };
            events.push(StreamEvent::ToolCall {
                call: ToolCall {
                    id: pending.id.clone(),
                    name: pending.name.clone(),
                    arguments,
                    provider_id: None,
                },
            });
        }
        self.pending.clear();
        events.push(StreamEvent::Usage {
            usage: self.usage.unwrap_or_default(),
        });
        events.push(StreamEvent::End {
            finish_reason: self.finish_reason.unwrap_or(fallback),
        });
        events
    }

    fn block_index(value: &Value) -> usize {
        value.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize
    }
}

impl FramePolicy<SseEvent> for AnthropicFramePolicy {
    type Output = StreamEvent;

    fn classify(&mut self, frame: SseEvent) -> Frame<StreamEvent> {
        let value: Value = match serde_json::from_str(frame.data.trim()) {
            Ok(value) => value,
            // 可恢复坏帧：跳过、流继续（`A2` §4-2 / `Q1` 用例 6）。
            Err(_) => return Frame::Skip,
        };
        // 命名事件为准；`event:` 缺失时兜底到 data 里的 `type`（协议中两者一致）。
        let kind = frame
            .event
            .or_else(|| {
                value
                    .get("type")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            })
            .unwrap_or_default();
        match kind.as_str() {
            "ping" => Frame::Skip,
            "message_start" => {
                let message = value.get("message").cloned().unwrap_or(Value::Null);
                if let Some(id) = message.get("id").and_then(|v| v.as_str()) {
                    self.response_id = Some(id.to_string());
                }
                if let Some(model) = message.get("model").and_then(|v| v.as_str()) {
                    self.model = Some(model.to_string());
                }
                // input 计数在这里；output 累计值在 message_delta（`P2` §3）。
                if let Some(usage) = message.get("usage") {
                    self.usage = Some(Usage {
                        input_tokens: usage
                            .get("input_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as u32,
                        output_tokens: usage
                            .get("output_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as u32,
                    });
                }
                let mut events = Vec::new();
                self.ensure_start(&mut events);
                Frame::Emit(events)
            }
            "content_block_start" => {
                let index = Self::block_index(&value);
                let block = value.get("content_block").cloned().unwrap_or(Value::Null);
                if block.get("type").and_then(|v| v.as_str()) == Some("tool_use") {
                    self.pending.insert(
                        index,
                        PendingToolBlock {
                            id: block
                                .get("id")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            name: block
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            arguments: String::new(),
                            initial: block.get("input").cloned().unwrap_or(Value::Null),
                        },
                    );
                }
                Frame::Emit(Vec::new())
            }
            "content_block_delta" => {
                let index = Self::block_index(&value);
                let delta = value.get("delta").cloned().unwrap_or(Value::Null);
                let mut events = Vec::new();
                match delta
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                {
                    "text_delta" => {
                        if let Some(text) = delta.get("text").and_then(|v| v.as_str()) {
                            self.ensure_start(&mut events);
                            if !text.is_empty() {
                                events.push(StreamEvent::Delta {
                                    kind: DeltaKind::Text,
                                    text: text.to_string(),
                                });
                            }
                        }
                    }
                    // 扩展思考（3.7+）；signature_delta 是校验签名，不外发。
                    "thinking_delta" => {
                        if let Some(text) = delta.get("thinking").and_then(|v| v.as_str()) {
                            self.ensure_start(&mut events);
                            if !text.is_empty() {
                                events.push(StreamEvent::Delta {
                                    kind: DeltaKind::Thinking,
                                    text: text.to_string(),
                                });
                            }
                        }
                    }
                    "input_json_delta" => {
                        // 防御：未见 content_block_start 也容忍（视为空块拼装）。
                        let entry = self.pending.entry(index).or_default();
                        if let Some(fragment) = delta.get("partial_json").and_then(|v| v.as_str()) {
                            entry.arguments.push_str(fragment);
                        }
                    }
                    _ => {}
                }
                Frame::Emit(events)
            }
            "content_block_stop" => {
                let index = Self::block_index(&value);
                if let Some(pending) = self.pending.remove(&index) {
                    let arguments = assemble_arguments(&pending);
                    Frame::Emit(vec![StreamEvent::ToolCall {
                        call: ToolCall {
                            id: pending.id,
                            name: pending.name,
                            arguments,
                            provider_id: None,
                        },
                    }])
                } else {
                    Frame::Emit(Vec::new())
                }
            }
            "message_delta" => {
                if let Some(reason) = value.pointer("/delta/stop_reason").and_then(|v| v.as_str()) {
                    self.finish_reason = Some(map_stop_reason(reason));
                }
                if let Some(usage) = value.get("usage") {
                    let entry = self.usage.get_or_insert_with(Usage::default);
                    if let Some(output) = usage.get("output_tokens").and_then(|v| v.as_u64()) {
                        entry.output_tokens = output as u32;
                    }
                    if let Some(input) = usage.get("input_tokens").and_then(|v| v.as_u64()) {
                        entry.input_tokens = input as u32;
                    }
                }
                Frame::Emit(Vec::new())
            }
            "message_stop" => Frame::Terminal(self.finish(FinishReason::Stop)),
            "error" => {
                // 流中错误（如 overloaded）：`Error` 之后不再发任何事件（`A2` §4）。
                let error_type = value
                    .pointer("/error/type")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let message = value
                    .pointer("/error/message")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let error =
                    ProviderError::new(self.provider.as_str(), stream_error_category(&error_type))
                        .with_message(message)
                        .with_raw(value);
                Frame::Terminal(vec![StreamEvent::Error { error }])
            }
            other => {
                tracing::debug!(event = other, "unknown Anthropic SSE event, skipped");
                Frame::Skip
            }
        }
    }

    fn on_truncated(&mut self) -> Vec<StreamEvent> {
        self.finish(FinishReason::Truncated)
    }
}

/// 拼装 `tool_use` 参数：有分片则解析分片文本，否则用 start 自带的 `input`。
fn assemble_arguments(pending: &PendingToolBlock) -> Value {
    if pending.arguments.trim().is_empty() {
        pending.initial.clone()
    } else {
        serde_json::from_str(&pending.arguments).unwrap_or(Value::Null)
    }
}

/// Anthropic `stop_reason` → 归一化 `FinishReason`。未知值归一为 `Other` 并保留原文到日志。
pub fn map_stop_reason(value: &str) -> FinishReason {
    match value {
        "end_turn" | "stop_sequence" => FinishReason::Stop,
        "max_tokens" | "model_context_window_exceeded" => FinishReason::Length,
        "tool_use" => FinishReason::ToolCalls,
        "refusal" => FinishReason::ContentFilter,
        other => {
            tracing::warn!(stop_reason = other, "unknown stop_reason, mapped to Other");
            FinishReason::Other
        }
    }
}

/// 流中 `error` 事件的 `error.type` → `ErrorCategory`（无 HTTP 状态码，按类型映射）。
pub fn stream_error_category(error_type: &str) -> ErrorCategory {
    match error_type {
        "rate_limit_error" => ErrorCategory::RateLimit,
        "authentication_error" | "permission_error" => ErrorCategory::Auth,
        "timeout_error" => ErrorCategory::Timeout,
        "api_error" | "overloaded_error" => ErrorCategory::Server,
        "invalid_request_error" | "not_found_error" | "request_too_large" => {
            ErrorCategory::InvalidRequest
        }
        _ => ErrorCategory::Unknown,
    }
}

/// 400 错误体是否暗示上下文超限（`Q1` 用例 10）。Anthropic 文案为
/// "prompt is too long: N tokens > M maximum"；兼容实现按子串兜底。
pub fn is_context_overflow(raw: &Value) -> bool {
    let Some(message) = raw.pointer("/error/message").and_then(|v| v.as_str()) else {
        return false;
    };
    let lower = message.to_lowercase();
    [
        "prompt is too long",
        "maximum context",
        "context length",
        "too many tokens",
        "token limit",
    ]
    .iter()
    .any(|key| lower.contains(key))
}

/// HTTP 状态 + 错误体 → `ErrorCategory`（覆盖 `A2` §5 全部类别）。
pub fn error_category(status: u16, raw: &Value) -> ErrorCategory {
    match status {
        400 => {
            if is_context_overflow(raw) {
                ErrorCategory::ContextOverflow
            } else {
                ErrorCategory::InvalidRequest
            }
        }
        401 | 403 => ErrorCategory::Auth,
        404 | 413 | 422 => ErrorCategory::InvalidRequest,
        408 => ErrorCategory::Timeout,
        429 => ErrorCategory::RateLimit,
        // 529 是 Anthropic 的 overloaded（非标准状态码，不在 5xx 段内）。
        529 => ErrorCategory::Server,
        500..=599 => ErrorCategory::Server,
        _ => ErrorCategory::Unknown,
    }
}

/// HTTP 状态 + 错误体 → `ProviderError`。错误体形如
/// `{"type":"error","error":{"type":…,"message":…},"request_id":…}`（request_id 在顶层）。
pub fn map_http_error(provider: &str, status: u16, raw: Value) -> ProviderError {
    let mut err = ProviderError::new(provider, error_category(status, &raw))
        .with_status(status)
        .with_raw(raw.clone());
    if let Some(message) = raw.pointer("/error/message").and_then(|v| v.as_str()) {
        err = err.with_message(message);
    }
    if let Some(request_id) = raw.get("request_id").and_then(|v| v.as_str()) {
        err = err.with_request_id(request_id);
    }
    err
}

/// 非流式响应体 → `ModelResponse`（`Q1` 用例 1）。`tool_use` 的 `input` 已是结构化 JSON，直接取。
/// `thinking` / `redacted_thinking` 块不进 `text`（raw 已保留）。
pub fn parse_message_response(
    _provider: &str,
    body: Value,
) -> Result<ModelResponse, ProviderError> {
    let id = body
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let model = body
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let mut text = String::new();
    let mut tool_calls = Vec::new();
    if let Some(blocks) = body.get("content").and_then(|v| v.as_array()) {
        for block in blocks {
            match block
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
            {
                "text" => {
                    if let Some(t) = block.get("text").and_then(|v| v.as_str()) {
                        text.push_str(t);
                    }
                }
                "tool_use" => {
                    tool_calls.push(ToolCall {
                        id: block
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        name: block
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        arguments: block.get("input").cloned().unwrap_or(Value::Null),
                        provider_id: None,
                    });
                }
                _ => {}
            }
        }
    }
    let finish_reason = body
        .get("stop_reason")
        .and_then(|v| v.as_str())
        .map(map_stop_reason)
        .unwrap_or(FinishReason::Stop);
    let usage = body
        .get("usage")
        .map(|u| Usage {
            input_tokens: u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            output_tokens: u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        })
        .unwrap_or_default();
    Ok(ModelResponse {
        id,
        model,
        text,
        tool_calls,
        finish_reason,
        usage,
        raw: Some(body),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_common::{Message, ToolDefinition};

    fn frame(event: &str, data: &str) -> SseEvent {
        SseEvent {
            event: Some(event.to_string()),
            data: data.to_string(),
            ..SseEvent::default()
        }
    }

    /// 依次判决一组帧，把产出的事件拍平。
    fn run_frames(policy: &mut AnthropicFramePolicy, frames: Vec<SseEvent>) -> Vec<StreamEvent> {
        let mut events = Vec::new();
        for frame in frames {
            match policy.classify(frame) {
                Frame::Emit(mut emitted) => events.append(&mut emitted),
                Frame::Skip => {}
                Frame::Terminal(mut tail) => events.append(&mut tail),
            }
        }
        events
    }

    #[test]
    fn stop_reason_mapping_covers_known_values() {
        assert_eq!(map_stop_reason("end_turn"), FinishReason::Stop);
        assert_eq!(map_stop_reason("stop_sequence"), FinishReason::Stop);
        assert_eq!(map_stop_reason("max_tokens"), FinishReason::Length);
        assert_eq!(map_stop_reason("tool_use"), FinishReason::ToolCalls);
        assert_eq!(map_stop_reason("refusal"), FinishReason::ContentFilter);
        assert_eq!(map_stop_reason("weird"), FinishReason::Other);
    }

    #[test]
    fn error_category_mapping_covers_a2_section5() {
        let raw = || Value::Null;
        assert_eq!(error_category(401, &raw()), ErrorCategory::Auth);
        assert_eq!(error_category(403, &raw()), ErrorCategory::Auth);
        assert_eq!(error_category(429, &raw()), ErrorCategory::RateLimit);
        assert_eq!(error_category(500, &raw()), ErrorCategory::Server);
        assert_eq!(
            error_category(529, &raw()),
            ErrorCategory::Server,
            "overloaded"
        );
        assert_eq!(error_category(400, &raw()), ErrorCategory::InvalidRequest);
        assert_eq!(error_category(413, &raw()), ErrorCategory::InvalidRequest);
        assert_eq!(error_category(408, &raw()), ErrorCategory::Timeout);
        let overflow = json!({"type": "error", "error": {"type": "invalid_request_error",
            "message": "prompt is too long: 200032 tokens > 199563 maximum"}});
        assert_eq!(
            error_category(400, &overflow),
            ErrorCategory::ContextOverflow
        );
    }

    #[test]
    fn tool_call_arguments_are_reassembled_and_parsed() {
        let mut policy = AnthropicFramePolicy::new("anthropic");
        let events = run_frames(
            &mut policy,
            vec![
                frame(
                    "message_start",
                    r#"{"type":"message_start","message":{"id":"msg_1","type":"message","role":"assistant","model":"claude-sonnet-4-5-20250929","content":[],"stop_reason":null,"usage":{"input_tokens":11,"output_tokens":1}}}"#,
                ),
                frame(
                    "content_block_start",
                    r#"{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_1","name":"weather","input":{}}}"#,
                ),
                frame(
                    "content_block_delta",
                    r#"{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"city\":"}}"#,
                ),
                frame(
                    "content_block_delta",
                    r#"{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"\"Shanghai\"}"}}"#,
                ),
                frame(
                    "content_block_stop",
                    r#"{"type":"content_block_stop","index":0}"#,
                ),
                frame(
                    "message_delta",
                    r#"{"type":"message_delta","delta":{"stop_reason":"tool_use","stop_sequence":null},"usage":{"output_tokens":40}}"#,
                ),
                frame("message_stop", r#"{"type":"message_stop"}"#),
            ],
        );

        assert!(matches!(
            events.first(),
            Some(StreamEvent::Start { response_id, model })
                if response_id == "msg_1" && model == "claude-sonnet-4-5-20250929"
        ));
        let calls: Vec<&ToolCall> = events
            .iter()
            .filter_map(|e| match e {
                StreamEvent::ToolCall { call } => Some(call),
                _ => None,
            })
            .collect();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "toolu_1");
        assert_eq!(calls[0].name, "weather");
        assert_eq!(
            calls[0].arguments,
            json!({"city": "Shanghai"}),
            "跨分片拼装后必须是可解析的完整 JSON（Q1 用例 3）"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, StreamEvent::Usage { usage }
                if usage.input_tokens == 11 && usage.output_tokens == 40)),
            "message_start 的 input 与 message_delta 的 output 必须合并"
        );
        assert!(events.iter().any(|e| matches!(
            e,
            StreamEvent::End {
                finish_reason: FinishReason::ToolCalls
            }
        )));
    }

    #[test]
    fn chat_body_maps_system_tool_results_and_choice() {
        let provider = AnthropicCompatible::new(AnthropicConfig {
            id: "test",
            base_url: "https://example.com".into(),
            ..AnthropicConfig::default()
        })
        .unwrap();
        let mut req = ModelRequest::new(
            "claude-sonnet-4-5-20250929",
            vec![Message::system("be brief"), Message::user("hi")],
        );
        let mut assistant = Message::assistant("");
        assistant.tool_calls = vec![ToolCall {
            id: "toolu_1".into(),
            name: "weather".into(),
            arguments: json!({"city": "Shanghai"}),
            provider_id: None,
        }];
        req.messages.push(assistant);
        req.messages.push(Message::tool_result("toolu_1", "sunny"));
        req.tools = vec![ToolDefinition {
            name: "weather".into(),
            description: "get weather".into(),
            parameters: json!({"type": "object"}),
        }];
        req.tool_choice = Some(ToolChoice::Required);

        let body = provider.build_request_body(&req, false).unwrap();
        assert_eq!(body["system"], "be brief", "system 提升为顶层参数");
        assert_eq!(body["max_tokens"], DEFAULT_MAX_TOKENS, "协议必填，缺省兜底");
        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 3, "system 提升后剩 user / assistant / user");
        assert_eq!(messages[1]["role"], "assistant");
        assert_eq!(messages[1]["content"][0]["type"], "tool_use");
        assert_eq!(messages[1]["content"][0]["input"]["city"], "Shanghai");
        assert_eq!(messages[2]["role"], "user", "tool_result 装在 user 消息里");
        assert_eq!(messages[2]["content"][0]["type"], "tool_result");
        assert_eq!(messages[2]["content"][0]["tool_use_id"], "toolu_1");
        assert_eq!(messages[2]["content"][0]["content"], "sunny");
        assert_eq!(body["tools"][0]["input_schema"]["type"], "object");
        assert_eq!(body["tool_choice"], json!({"type": "any"}));
        assert_eq!(body["stream"], false);
    }

    #[test]
    fn consecutive_messages_merge_to_keep_role_alternation() {
        let provider = AnthropicCompatible::new(AnthropicConfig {
            id: "test",
            base_url: "https://example.com".into(),
            ..AnthropicConfig::default()
        })
        .unwrap();
        let mut req = ModelRequest::new("claude-sonnet-4-5-20250929", vec![Message::user("q")]);
        for (id, name) in [("toolu_1", "w1"), ("toolu_2", "w2")] {
            let mut assistant = Message::assistant("");
            assistant.tool_calls = vec![ToolCall {
                id: id.into(),
                name: name.into(),
                arguments: json!({}),
                provider_id: None,
            }];
            req.messages.push(assistant);
        }
        req.messages.push(Message::tool_result("toolu_1", "r1"));
        req.messages.push(Message::tool_result("toolu_2", "r2"));

        let body = provider.build_request_body(&req, false).unwrap();
        let messages = body["messages"].as_array().unwrap();
        assert_eq!(
            messages.len(),
            3,
            "连续 assistant 合并、连续 tool 结果并入一条 user"
        );
        assert_eq!(messages[1]["role"], "assistant");
        assert_eq!(
            messages[1]["content"].as_array().unwrap().len(),
            2,
            "两个 tool_use 块在同一条 assistant 消息里"
        );
        assert_eq!(messages[2]["role"], "user");
        let content = messages[2]["content"].as_array().unwrap();
        assert_eq!(
            content.len(),
            2,
            "两个 tool_result 块在同一条 user 消息里（role 必须交替，P2 已知坑）"
        );
        assert_eq!(content[0]["tool_use_id"], "toolu_1");
        assert_eq!(content[1]["tool_use_id"], "toolu_2");
    }
}
