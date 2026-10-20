//! OpenAI 兼容适配器（供应商差异记录：`P1`）。
//!
//! 覆盖 OpenAI Chat Completions 及其兼容实现（DeepSeek / vLLM / LiteLLM / Ollama OpenAI 端口等）。
//! 只依赖 `A2` 契约与 `agent-transport`；供应商语义（字段名、错误码、收尾）都在本模块归一化，
//! 不得泄漏到编排核心（`A1` 硬约束）。
//!
//! 协议要点：
//! - POST `{base_url}/chat/completions`，`Authorization: Bearer <api_key>`（无 key 则不带头）。
//! - 流式：SSE data-only，`data: [DONE]` 结束；`stream_options.include_usage = true` 强制回 usage。
//! - 工具参数 `arguments` 是 JSON **字符串**（可能分片、可能非法），在本模块拼装并解析为结构化
//!   `Value`（`A2` §3：Core 只看到结构化结果）。

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use agent_common::{
    Capability, ContentPart, DeltaKind, ErrorCategory, FinishReason, ModelRequest, ModelResponse,
    ModelSpec, ProviderError, ResponseFormat, Role, StreamEvent, ToolCall, ToolChoice,
    ToolDefinition, Usage,
};
use agent_transport::{
    Frame, FramePolicy, HttpClient, HttpConfig, SseEvent, TransportError, finalize, guard,
    parse_sse,
};
use async_stream::stream;
use futures::StreamExt;
use futures::future::BoxFuture;
use reqwest::RequestBuilder;
use serde_json::{Map, Value, json};

use crate::{CallContext, EventStream, Provider};

/// 单次 `GET /models` 拉取的超时（C2）：传输层的 `HttpConfig` 默认不带请求超时，
/// 若不设，一个不响应的端点会让 worker 的刷新循环永久挂起。
const MODELS_FETCH_TIMEOUT: Duration = Duration::from_secs(30);

/// OpenAI 兼容适配器配置。
///
/// `models` 非空时做本地能力协商（模型不在清单或能力不足即 fast-fail，`A2` §1）；
/// 为空时不拦截，交给供应商报错（用于尚未登记清单的兼容实现）。
#[derive(Clone)]
pub struct OpenAiConfig {
    /// 供应商 id（如 `"openai"` / `"deepseek"`）。
    pub id: &'static str,
    /// 如 `https://api.openai.com/v1`（不带尾部斜杠）。真实 endpoint 由配置层注入，不入库（`R1`）。
    pub base_url: String,
    /// 密钥由配置层注入；`None` 表示无鉴权（Ollama 等本地服务）。
    pub api_key: Option<String>,
    pub http: HttpConfig,
    pub models: Vec<ModelSpec>,
}

/// 手写 `Debug`：`api_key` **绝不**进日志 / 错误 / 快照（`Q3` / 硬约束 5），只显示是否设置。
impl fmt::Debug for OpenAiConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenAiConfig")
            .field("id", &self.id)
            .field("base_url", &self.base_url)
            .field("api_key", &self.api_key.as_deref().map(|_| "***"))
            .field("http", &self.http)
            .field("models", &self.models)
            .finish()
    }
}

impl Default for OpenAiConfig {
    fn default() -> Self {
        Self {
            id: "openai",
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: None,
            http: HttpConfig::default(),
            models: openai_default_models(),
        }
    }
}

/// OpenAI 官方常用模型静态清单。兼容实现（DeepSeek 等）应由配置层传入自己的 `ModelSpec`。
pub fn openai_default_models() -> Vec<ModelSpec> {
    fn spec(id: &str, ctx: u32, out: u32, caps: &[Capability]) -> ModelSpec {
        ModelSpec {
            id: id.to_string(),
            provider: "openai".to_string(),
            context_window: ctx,
            max_output: out,
            capabilities: caps.to_vec(),
            pricing: None,
            deprecated: false,
            capabilities_known: true,
        }
    }
    let chat = [
        Capability::Text,
        Capability::Vision,
        Capability::Tools,
        Capability::JsonMode,
        Capability::JsonSchema,
    ];
    let reasoning = [Capability::Text, Capability::Tools, Capability::Reasoning];
    vec![
        spec("gpt-4o", 128_000, 16_384, &chat),
        spec("gpt-4o-mini", 128_000, 16_384, &chat),
        spec("gpt-4.1", 1_048_576, 32_768, &chat),
        spec("gpt-4.1-mini", 1_048_576, 32_768, &chat),
        spec("gpt-4.1-nano", 1_048_576, 32_768, &chat),
        spec("o3", 200_000, 100_000, &reasoning),
        spec(
            "o4-mini",
            200_000,
            100_000,
            &[
                Capability::Text,
                Capability::Vision,
                Capability::Tools,
                Capability::Reasoning,
            ],
        ),
    ]
}

/// 由一个模型 id 得出 `ModelSpec`（C2 的**唯一**字典合并口径，装配层与拉取共用）。
///
/// - 命中 [`openai_default_models`] → 沿用官方规格（窗口 / 能力），只把 `provider` 改成
///   `provider_id`（官方条目默认写 `openai`，而 `ollama` 等也走本适配器）；
///   官方规格 `capabilities_known = true`，可据此 fast-fail。
/// - 未命中（未知 / 本地模型）→ **保守默认**（[`crate::conservative_spec`]）：
///   仅 `Text`、128k 上下文、4096 最大输出，且 `capabilities_known = false` ——
///   保守不等于拦截：路由层只确认「id 在清单里」，不据此拒绝能力请求，避免误杀本地模型。
pub fn model_spec_for(provider_id: &str, id: &str) -> ModelSpec {
    match openai_default_models()
        .into_iter()
        .find(|spec| spec.id == id)
    {
        Some(mut spec) => {
            spec.provider = provider_id.to_string();
            spec
        }
        None => crate::conservative_spec(provider_id, id),
    }
}

/// OpenAI 兼容适配器。`Arc<HttpClient>` 保证 `Clone` 只是引用计数（`A2` §2 硬要求）。
#[derive(Clone)]
pub struct OpenAiCompatible {
    id: &'static str,
    client: Arc<HttpClient>,
    base_url: String,
    api_key: Option<String>,
    /// 模型缓存（C2a）：`Arc<RwLock<..>>` 让刷新句柄与主实例**共享同一份**，
    /// 刷新成功后 `list_models()` 自动读到最新值；失败时缓存保持原状。
    models: Arc<RwLock<Vec<ModelSpec>>>,
}

/// 手写 `Debug`：`api_key` 脱敏（同 [`OpenAiConfig`]）。
impl fmt::Debug for OpenAiCompatible {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenAiCompatible")
            .field("id", &self.id)
            .field("base_url", &self.base_url)
            .field("api_key", &self.api_key.as_deref().map(|_| "***"))
            .field("models", &self.models)
            .finish()
    }
}

impl OpenAiCompatible {
    pub fn new(config: OpenAiConfig) -> Result<Self, ProviderError> {
        let client = HttpClient::new(config.http).map_err(|e| {
            ProviderError::new(config.id, ErrorCategory::Unknown).with_message(e.to_string())
        })?;
        Ok(Self {
            id: config.id,
            client: Arc::new(client),
            base_url: config.base_url,
            api_key: config.api_key,
            models: Arc::new(RwLock::new(config.models)),
        })
    }

    fn chat_url(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }

    fn with_auth(&self, builder: RequestBuilder) -> RequestBuilder {
        match &self.api_key {
            Some(key) => builder.header("Authorization", format!("Bearer {key}")),
            None => builder,
        }
    }

    /// 能力协商 fast-fail（`A2` §1 / `Q1` 用例 12）：发请求**之前**拒绝不支持的组合。
    pub fn check_capabilities(&self, req: &ModelRequest) -> Result<(), ProviderError> {
        let models = self.models.read().unwrap_or_else(|e| e.into_inner());
        crate::check_model_capabilities(self.id, &models, req)
    }

    /// `GET {base_url}/models`（C2a 联网拉取）。
    fn models_url(&self) -> String {
        format!("{}/models", self.base_url.trim_end_matches('/'))
    }

    /// 联网拉取模型清单（C2a）：`GET /models` → 与内置字典合并后返回。
    ///
    /// 合并策略：拉到的 id 命中内置字典沿用官方规格；未命中的未知模型回落保守默认
    /// （仅 Text、128k 上下文、4096 最大输出，`capabilities_known = false`），放行不拦截
    /// （`P1`：别误杀本地模型）。
    /// **只拉取、不写缓存**：写缓存由 [`crate::RefreshableModels::refresh_models`] 负责，
    /// 由它保证「失败不清空」的契约。
    ///
    /// **健壮性（C2）**：单次请求自带 [`MODELS_FETCH_TIMEOUT`] 超时（传输层默认无请求超时，
    /// 否则一个不响应的端点会永久占住 worker 的刷新循环）；响应体**必须**含非空 `data`
    /// 数组，否则按失败处理 —— 宁可保留旧缓存，也不要用空清单把候选与协商一起清掉。
    pub async fn fetch_models(&self) -> Result<Vec<ModelSpec>, ProviderError> {
        use tokio_util::sync::CancellationToken;
        let request = self
            .with_auth(self.client.client().get(self.models_url()))
            .timeout(MODELS_FETCH_TIMEOUT);
        let response = self
            .client
            .send(request, &CancellationToken::new())
            .await
            .map_err(|e| self.map_transport_error(&e))?;
        let status = response.status();
        if !status.is_success() {
            let raw = response.json::<Value>().await.unwrap_or(Value::Null);
            return Err(map_http_error(self.id, status.as_u16(), raw));
        }
        let raw: Value = response.json().await.map_err(|e| {
            ProviderError::new(self.id, ErrorCategory::Unknown).with_message(e.to_string())
        })?;
        let ids: Vec<String> = match raw.get("data").and_then(Value::as_array) {
            Some(arr) => arr
                .iter()
                .filter_map(|m| m.get("id").and_then(Value::as_str).map(str::to_string))
                .collect(),
            None => {
                return Err(ProviderError::new(self.id, ErrorCategory::Unknown)
                    .with_message("GET /models 响应缺少 `data` 数组（保留已有模型清单）"));
            }
        };
        if ids.is_empty() {
            return Err(ProviderError::new(self.id, ErrorCategory::Unknown)
                .with_message("GET /models 返回空清单（保留已有模型清单）"));
        }
        Ok(self.merge_with_dictionary(ids))
    }

    /// 拉到的 id 与内置字典合并（C2a）：命中沿用官方规格（`capabilities_known = true`），
    /// 未知回落保守默认（`capabilities_known = false`，不据此拦截能力）。
    fn merge_with_dictionary(&self, ids: Vec<String>) -> Vec<ModelSpec> {
        let mut merged: Vec<ModelSpec> = ids
            .into_iter()
            .map(|id| model_spec_for(self.id, &id))
            .collect();
        merged.sort_by(|a, b| a.id.cmp(&b.id));
        merged
    }

    /// 归一化 `ModelRequest` → OpenAI `chat/completions` 请求体。`pub` 供契约测试断言（`Q1` 用例 4）。
    pub fn build_request_body(
        &self,
        req: &ModelRequest,
        stream: bool,
    ) -> Result<Value, ProviderError> {
        let mut body = Map::new();
        body.insert("model".into(), Value::String(req.model.clone()));
        body.insert("messages".into(), Value::Array(self.map_messages(req)?));
        body.insert("stream".into(), Value::Bool(stream));
        if let Some(temperature) = req.temperature {
            body.insert("temperature".into(), Value::from(temperature));
        }
        if let Some(max_tokens) = req.max_tokens {
            body.insert("max_tokens".into(), Value::from(max_tokens));
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
        match &req.response_format {
            Some(ResponseFormat::Text) | None => {}
            Some(ResponseFormat::JsonObject) => {
                body.insert("response_format".into(), json!({"type": "json_object"}));
            }
            Some(ResponseFormat::JsonSchema { name, schema }) => {
                body.insert(
                    "response_format".into(),
                    json!({
                        "type": "json_schema",
                        "json_schema": {"name": name, "strict": true, "schema": schema}
                    }),
                );
            }
        }
        if !req.stop.is_empty() {
            body.insert(
                "stop".into(),
                Value::Array(req.stop.iter().map(|s| Value::String(s.clone())).collect()),
            );
        }
        if stream {
            // 不打开时 usage 帧不会回，`End` 前就只能补 default（`P1` 已知坑）。
            body.insert("stream_options".into(), json!({"include_usage": true}));
        }
        // 厂商特有参数透传（`A2` §2：唯一允许出现厂商字段的地方）。
        for (key, value) in &req.additional_params {
            body.insert(key.clone(), value.clone());
        }
        Ok(Value::Object(body))
    }

    fn map_messages(&self, req: &ModelRequest) -> Result<Vec<Value>, ProviderError> {
        req.messages
            .iter()
            .map(|msg| self.map_message(msg))
            .collect()
    }

    fn map_message(&self, msg: &agent_common::Message) -> Result<Value, ProviderError> {
        let role = match msg.role {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::Tool => "tool",
        };
        let mut obj = Map::new();
        obj.insert("role".into(), Value::String(role.to_string()));
        match msg.role {
            // 工具结果回填（`Q1` 用例 4）：role=tool + tool_call_id + 纯文本 content。
            Role::Tool => {
                obj.insert(
                    "tool_call_id".into(),
                    Value::String(msg.tool_call_id.clone().unwrap_or_default()),
                );
                obj.insert("content".into(), Value::String(msg.text()));
            }
            Role::Assistant if !msg.tool_calls.is_empty() => {
                if msg.content.is_empty() {
                    obj.insert("content".into(), Value::Null);
                } else {
                    obj.insert("content".into(), self.map_content(&msg.content)?);
                }
                let calls: Vec<Value> = msg
                    .tool_calls
                    .iter()
                    .map(|call| {
                        // arguments 回传为 JSON **字符串**（OpenAI 形态），与入方向归一化对称。
                        json!({
                            "id": call.id,
                            "type": "function",
                            "function": {
                                "name": call.name,
                                "arguments": call.arguments.to_string()
                            }
                        })
                    })
                    .collect();
                obj.insert("tool_calls".into(), Value::Array(calls));
            }
            _ => {
                obj.insert("content".into(), self.map_content(&msg.content)?);
            }
        }
        Ok(Value::Object(obj))
    }

    /// 内容块映射：纯文本合并为单字符串；多模态展开为 content 数组。
    /// `Image` → `image_url`；图片类 `File` → data URI；非图片 `File` 丢弃并告警（`P1` 已知坑）。
    fn map_content(&self, parts: &[ContentPart]) -> Result<Value, ProviderError> {
        if parts.iter().all(|p| matches!(p, ContentPart::Text { .. })) {
            let text: String = parts
                .iter()
                .filter_map(|p| match p {
                    ContentPart::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect();
            return Ok(Value::String(text));
        }
        let mut array = Vec::new();
        for part in parts {
            match part {
                ContentPart::Text { text } => {
                    array.push(json!({"type": "text", "text": text}));
                }
                ContentPart::Image { url, .. } => {
                    array.push(json!({"type": "image_url", "image_url": {"url": url}}));
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
                            let mime = mime_type.clone().unwrap_or_default();
                            array.push(json!({
                                "type": "image_url",
                                "image_url": {"url": format!("data:{mime};base64,{data}")}
                            }));
                        } else if let Some(url) = url {
                            array.push(json!({"type": "image_url", "image_url": {"url": url}}));
                        }
                    } else {
                        tracing::warn!(
                            file = name,
                            "dropping non-image file attachment (chat/completions 不支持，见 P1 §6)"
                        );
                    }
                }
            }
        }
        Ok(Value::Array(array))
    }

    /// `TransportError` → `ProviderError`（网络层没有供应商错误体，`status` 置空）。
    fn map_transport_error(&self, err: &TransportError) -> ProviderError {
        crate::map_transport_error(self.id, err)
    }
}

impl Provider for OpenAiCompatible {
    fn id(&self) -> &'static str {
        self.id
    }

    fn list_models(&self) -> Vec<ModelSpec> {
        self.models
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    async fn chat(
        &self,
        req: ModelRequest,
        ctx: CallContext,
    ) -> Result<ModelResponse, ProviderError> {
        self.check_capabilities(&req)?;
        let body = self.build_request_body(&req, false)?;
        let request = self.with_auth(self.client.client().post(self.chat_url()).json(&body));
        let response = self
            .client
            .send(request, &ctx.cancel)
            .await
            .map_err(|e| self.map_transport_error(&e))?;
        let status = response.status();
        if !status.is_success() {
            let raw = response.json::<Value>().await.unwrap_or(Value::Null);
            return Err(map_http_error(self.id, status.as_u16(), raw));
        }
        let raw: Value = response.json().await.map_err(|e| {
            ProviderError::new(self.id, ErrorCategory::Unknown).with_message(e.to_string())
        })?;
        parse_chat_response(self.id, raw)
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
            let request = this.with_auth(this.client.client().post(this.chat_url()).json(&body));
            let response = match this.client.send(request, &ctx.cancel).await {
                Ok(response) => response,
                Err(err) => { yield StreamEvent::Error { error: this.map_transport_error(&err) }; return; }
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
                    OpenAiFramePolicy::new(),
                ),
                ctx.cancel,
            );
            while let Some(item) = finalized.next().await {
                match item {
                    Ok(event) => yield event,
                    Err(err) => {
                        yield StreamEvent::Error { error: this.map_transport_error(&err) };
                        return;
                    }
                }
            }
        };
        Ok(Box::pin(out))
    }

    /// 本地粗略估算（≈4 字符/token）；OpenAI 无独立 count 端点。
    fn count_tokens(&self, req: &ModelRequest) -> Option<u32> {
        let chars: usize = req.messages.iter().map(|m| m.text().chars().count()).sum();
        Some((chars / 4) as u32)
    }
}

/// C2a：联网刷新模型清单。刷新句柄与主实例共享同一缓存 `Arc<RwLock<..>>`，
/// 成功时与已有缓存**取并集**、失败不动 —— 保证拉取失败不清空现有可用清单。
impl crate::RefreshableModels for OpenAiCompatible {
    fn refresh_models(&self) -> BoxFuture<'static, Result<Vec<ModelSpec>, ProviderError>> {
        let this = self.clone();
        Box::pin(async move {
            let fetched = this.fetch_models().await?;
            // 走到这里说明拉取成功：与已有缓存取并集再写回。
            // **取并集而非覆盖**（C2）：部分端点 `/models` 只回自己认识的一批，覆盖会让
            // 用户手写 / 当前选中的模型「突然不在清单里」而被路由层拦下；拉到的同 id 覆盖旧的。
            let mut cache = this.models.write().unwrap_or_else(|e| e.into_inner());
            let mut merged: BTreeMap<String, ModelSpec> = cache
                .iter()
                .map(|spec| (spec.id.clone(), spec.clone()))
                .collect();
            for spec in fetched {
                merged.insert(spec.id.clone(), spec);
            }
            let merged: Vec<ModelSpec> = merged.into_values().collect();
            *cache = merged.clone();
            Ok(merged)
        })
    }
}

/// 流式工具调用分片状态：按 `index` 拼装 id / name 与 arguments（字符串分片）。
#[derive(Debug, Default)]
struct PendingToolCall {
    id: Option<String>,
    name: Option<String>,
    arguments: String,
}

/// OpenAI 流式帧判决策略：`SseEvent` → 归一化 `StreamEvent`。
///
/// - `[DONE]` 是终止记录；usage 帧（`include_usage` 打开时）即时产出 `Usage`；
/// - 非法 JSON 帧 → `Frame::Skip`（`Q1` 用例 6）；
/// - EOF 无 `[DONE]` → `on_truncated` 补 `Usage`(如缺) + `End { Truncated }`（用例 5/11）。
#[derive(Debug, Default)]
pub struct OpenAiFramePolicy {
    response_id: Option<String>,
    model: Option<String>,
    started: bool,
    usage: Option<Usage>,
    usage_emitted: bool,
    finish_reason: Option<FinishReason>,
    pending: BTreeMap<usize, PendingToolCall>,
    order: Vec<usize>,
}

impl OpenAiFramePolicy {
    pub fn new() -> Self {
        Self::default()
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

    /// 终止/截断时的末帧序列：`ToolCall*` → `Usage`（如未发）→ `End`（`A2` §4：`End` 前必须给一次 `Usage`）。
    fn finish(&mut self, fallback: FinishReason) -> Vec<StreamEvent> {
        let mut events = Vec::new();
        for index in std::mem::take(&mut self.order) {
            if let Some(pending) = self.pending.remove(&index) {
                // 拼装完成的 arguments 必须是可解析的完整 JSON（`Q1` 用例 3）；非法时容错为 Null。
                let arguments = serde_json::from_str(&pending.arguments).unwrap_or(Value::Null);
                events.push(StreamEvent::ToolCall {
                    call: ToolCall {
                        id: pending.id.unwrap_or_default(),
                        name: pending.name.unwrap_or_default(),
                        arguments,
                        provider_id: None,
                    },
                });
            }
        }
        if !self.usage_emitted {
            self.usage_emitted = true;
            events.push(StreamEvent::Usage {
                usage: self.usage.unwrap_or_default(),
            });
        }
        let reason = self.finish_reason.unwrap_or(fallback);
        events.push(StreamEvent::End {
            finish_reason: reason,
        });
        events
    }

    fn accumulate_tool_calls(&mut self, calls: &[Value]) {
        for call in calls {
            let index = call.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            if !self.pending.contains_key(&index) {
                self.order.push(index);
                self.pending.insert(index, PendingToolCall::default());
            }
            let entry = self
                .pending
                .get_mut(&index)
                .expect("pending entry inserted above");
            if let Some(id) = call.get("id").and_then(|v| v.as_str()) {
                entry.id = Some(id.to_string());
            }
            if let Some(function) = call.get("function") {
                if let Some(name) = function.get("name").and_then(|v| v.as_str()) {
                    entry.name = Some(name.to_string());
                }
                if let Some(args) = function.get("arguments").and_then(|v| v.as_str()) {
                    entry.arguments.push_str(args);
                }
            }
        }
    }
}

impl FramePolicy<SseEvent> for OpenAiFramePolicy {
    type Output = StreamEvent;

    fn classify(&mut self, frame: SseEvent) -> Frame<StreamEvent> {
        if frame.is_done() {
            return Frame::Terminal(self.finish(FinishReason::Stop));
        }
        let value: Value = match serde_json::from_str(frame.data.trim()) {
            Ok(value) => value,
            // 可恢复坏帧：跳过、流继续（`A2` §4-2 / `Q1` 用例 6）。
            Err(_) => return Frame::Skip,
        };
        if value.get("object").and_then(|v| v.as_str()) != Some("chat.completion.chunk") {
            return Frame::Skip;
        }
        let mut events = Vec::new();
        if let Some(id) = value.get("id").and_then(|v| v.as_str()) {
            self.response_id = Some(id.to_string());
        }
        if let Some(model) = value.get("model").and_then(|v| v.as_str()) {
            self.model = Some(model.to_string());
        }
        if let Some(usage) = value.get("usage") {
            self.usage = Some(map_usage(usage));
        }
        if let Some(choices) = value.get("choices").and_then(|v| v.as_array()) {
            for choice in choices {
                if let Some(reason) = choice.get("finish_reason").and_then(|v| v.as_str()) {
                    self.finish_reason = Some(map_finish_reason(reason));
                }
                let Some(delta) = choice.get("delta") else {
                    continue;
                };
                if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                    self.ensure_start(&mut events);
                    if !content.is_empty() {
                        events.push(StreamEvent::Delta {
                            kind: DeltaKind::Text,
                            text: content.to_string(),
                        });
                    }
                }
                // 兼容实现（DeepSeek 等）的推理内容字段。
                if let Some(reasoning) = delta.get("reasoning_content").and_then(|v| v.as_str()) {
                    self.ensure_start(&mut events);
                    if !reasoning.is_empty() {
                        events.push(StreamEvent::Delta {
                            kind: DeltaKind::Thinking,
                            text: reasoning.to_string(),
                        });
                    }
                }
                if let Some(calls) = delta.get("tool_calls").and_then(|v| v.as_array()) {
                    self.ensure_start(&mut events);
                    self.accumulate_tool_calls(calls);
                }
            }
        }
        // usage 帧即时产出（通常在末帧、`[DONE]` 之前）。
        if let Some(usage) = self.usage.filter(|_| !self.usage_emitted) {
            self.usage_emitted = true;
            events.push(StreamEvent::Usage { usage });
        }
        Frame::Emit(events)
    }

    fn on_truncated(&mut self) -> Vec<StreamEvent> {
        self.finish(FinishReason::Truncated)
    }
}

fn map_tool(tool: &ToolDefinition) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": tool.name,
            "description": tool.description,
            "parameters": tool.parameters,
        }
    })
}

fn map_tool_choice(choice: &ToolChoice) -> Value {
    match choice {
        ToolChoice::Auto => json!("auto"),
        ToolChoice::None => json!("none"),
        ToolChoice::Required => json!("required"),
        // Chat Completions 只支持指定**一个**函数；多取第一个。
        ToolChoice::Specific(names) => match names.first() {
            Some(name) => json!({"type": "function", "function": {"name": name}}),
            None => json!("auto"),
        },
    }
}

/// OpenAI 结束原因 → 归一化 `FinishReason`。未知值归一为 `Other` 并保留原文到日志。
pub fn map_finish_reason(value: &str) -> FinishReason {
    match value {
        "stop" => FinishReason::Stop,
        "length" => FinishReason::Length,
        "tool_calls" => FinishReason::ToolCalls,
        "content_filter" => FinishReason::ContentFilter,
        other => {
            tracing::warn!(
                finish_reason = other,
                "unknown finish_reason, mapped to Other"
            );
            FinishReason::Other
        }
    }
}

/// `prompt_tokens` / `completion_tokens` → `Usage`。缺失字段按 0 计。
pub fn map_usage(usage: &Value) -> Usage {
    Usage {
        input_tokens: usage
            .get("prompt_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        output_tokens: usage
            .get("completion_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
    }
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
        500..=599 => ErrorCategory::Server,
        _ => ErrorCategory::Unknown,
    }
}

/// 400 错误体是否暗示上下文超限（`Q1` 用例 10）。OpenAI 用 `context_length_exceeded`，
/// 兼容实现文案不一，按子串兜底。
pub fn is_context_overflow(raw: &Value) -> bool {
    let code = raw
        .pointer("/error/code")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if code == "context_length_exceeded" {
        return true;
    }
    let Some(message) = raw.pointer("/error/message").and_then(|v| v.as_str()) else {
        return false;
    };
    let lower = message.to_lowercase();
    [
        "context length",
        "context_length",
        "maximum context",
        "too many tokens",
        "prompt is too long",
        "token limit",
    ]
    .iter()
    .any(|key| lower.contains(key))
}

/// HTTP 状态 + 错误体（含 `request_id`）→ `ProviderError`。`provider` 名由调用方传入。
pub fn map_http_error(provider: &str, status: u16, raw: Value) -> ProviderError {
    let mut err = ProviderError::new(provider, error_category(status, &raw))
        .with_status(status)
        .with_raw(raw.clone());
    if let Some(message) = raw.pointer("/error/message").and_then(|v| v.as_str()) {
        err = err.with_message(message);
    }
    if let Some(request_id) = raw.pointer("/error/request_id").and_then(|v| v.as_str()) {
        err = err.with_request_id(request_id);
    }
    err
}

/// 非流式响应体 → `ModelResponse`（`Q1` 用例 1）。`arguments` 字符串在此解析为结构化 JSON。
pub fn parse_chat_response(_provider: &str, body: Value) -> Result<ModelResponse, ProviderError> {
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
    let choices = body
        .get("choices")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut text = String::new();
    let mut tool_calls = Vec::new();
    let mut finish_reason = FinishReason::Stop;
    for choice in choices {
        if let Some(reason) = choice.get("finish_reason").and_then(|v| v.as_str()) {
            finish_reason = map_finish_reason(reason);
        }
        let Some(message) = choice.get("message") else {
            continue;
        };
        if let Some(content) = message.get("content").and_then(|v| v.as_str()) {
            text.push_str(content);
        }
        if let Some(calls) = message.get("tool_calls").and_then(|v| v.as_array()) {
            for call in calls {
                let call_id = call
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let function = call.get("function").cloned().unwrap_or(Value::Null);
                let name = function
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let arguments_raw = function
                    .get("arguments")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let arguments = serde_json::from_str(arguments_raw).unwrap_or(Value::Null);
                tool_calls.push(ToolCall {
                    id: call_id,
                    name,
                    arguments,
                    provider_id: None,
                });
            }
        }
    }
    let usage = map_usage(body.get("usage").unwrap_or(&Value::Null));
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
    use agent_common::Message;

    #[test]
    fn finish_reason_mapping_covers_known_values() {
        assert_eq!(map_finish_reason("stop"), FinishReason::Stop);
        assert_eq!(map_finish_reason("length"), FinishReason::Length);
        assert_eq!(map_finish_reason("tool_calls"), FinishReason::ToolCalls);
        assert_eq!(
            map_finish_reason("content_filter"),
            FinishReason::ContentFilter
        );
        assert_eq!(map_finish_reason("weird"), FinishReason::Other);
    }

    #[test]
    fn error_category_mapping_covers_a2_section5() {
        let raw = || Value::Null;
        assert_eq!(error_category(401, &raw()), ErrorCategory::Auth);
        assert_eq!(error_category(403, &raw()), ErrorCategory::Auth);
        assert_eq!(error_category(429, &raw()), ErrorCategory::RateLimit);
        assert_eq!(error_category(500, &raw()), ErrorCategory::Server);
        assert_eq!(error_category(400, &raw()), ErrorCategory::InvalidRequest);
        assert_eq!(error_category(422, &raw()), ErrorCategory::InvalidRequest);
        let overflow = json!({"error": {"code": "context_length_exceeded",
            "message": "This model's maximum context length is 128000 tokens"}});
        assert_eq!(
            error_category(400, &overflow),
            ErrorCategory::ContextOverflow
        );
    }

    #[test]
    fn tool_call_arguments_are_reassembled_and_parsed() {
        let mut policy = OpenAiFramePolicy::new();
        let frame = |data: &str| SseEvent {
            data: data.to_string(),
            ..SseEvent::default()
        };
        let first = frame(
            r#"{"object":"chat.completion.chunk","id":"1","model":"m","choices":[{"index":0,"delta":{"role":"assistant","tool_calls":[{"index":0,"id":"call_1","type":"function","function":{"name":"weather","arguments":"{\"city\":\"Shang"}}]},"finish_reason":null}]}"#,
        );
        let second = frame(
            r#"{"object":"chat.completion.chunk","id":"1","model":"m","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"hai\"}"}}]},"finish_reason":"tool_calls"}]}"#,
        );
        let done = frame("[DONE]");

        let mut events = Vec::new();
        if let Frame::Emit(mut ev) = policy.classify(first) {
            events.append(&mut ev);
        }
        if let Frame::Emit(mut ev) = policy.classify(second) {
            events.append(&mut ev);
        }
        let Frame::Terminal(mut ev) = policy.classify(done) else {
            panic!("[DONE] 必须是终止记录");
        };
        events.append(&mut ev);

        let calls: Vec<&ToolCall> = events
            .iter()
            .filter_map(|e| match e {
                StreamEvent::ToolCall { call } => Some(call),
                _ => None,
            })
            .collect();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "weather");
        assert_eq!(
            calls[0].arguments,
            json!({"city": "Shanghai"}),
            "跨分片拼装后必须是可解析的完整 JSON（Q1 用例 3）"
        );
        assert!(events.iter().any(|e| matches!(
            e,
            StreamEvent::End {
                finish_reason: FinishReason::ToolCalls
            }
        )));
    }

    #[test]
    fn chat_body_maps_tool_results_and_choice() {
        let provider = OpenAiCompatible::new(OpenAiConfig {
            id: "test",
            base_url: "https://example.com/v1".into(),
            ..OpenAiConfig::default()
        })
        .unwrap();
        let mut req = ModelRequest::new("gpt-4o", vec![]);
        req.messages.push(Message::assistant(""));
        req.messages.push(Message::tool_result("call_1", "sunny"));
        req.tools = vec![agent_common::ToolDefinition {
            name: "weather".into(),
            description: "get weather".into(),
            parameters: json!({"type": "object"}),
        }];
        req.tool_choice = Some(ToolChoice::Required);
        let body = provider.build_request_body(&req, false).unwrap();
        assert_eq!(body["messages"][1]["role"], "tool");
        assert_eq!(body["messages"][1]["tool_call_id"], "call_1");
        assert_eq!(body["tool_choice"], "required");
        assert_eq!(body["tools"][0]["function"]["name"], "weather");
        assert_eq!(body["stream"], false);
    }
}
