# 11 · Provider 接入契约

> 索引 ID：`A2` ｜ 状态：🚧 草案（Rust 表达，尚未落码） ｜ 最后更新：2026-09-27

## 目的

定义所有供应商适配器必须实现的统一接口。新增供应商 = 实现本契约 + 注册表登记，核心代码零改动。

技术栈已定（`adr/0001-tech-stack.md`）：本契约用 **Rust** 表达，落地位置为 `agent-schema`（类型）+ `agent-providers`（trait）。抽象路线的取舍见 `adr/0002-llm-abstraction.md`。

## 适用范围

- 包含：能力协商、请求/响应、流式事件、错误分类的归一化约定。
- 不包含：厂商字段差异（`providers/*.md`）、路由与兜底策略（`21-model-registry.md`）。

## 1. 能力协商

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    Text, Vision, Audio, File, Tools, JsonMode, Reasoning, Logprobs, Embedding,
}

pub struct ModelSpec {
    pub id: String,
    pub provider: String,
    pub context_window: u32,
    pub max_output: u32,
    pub capabilities: EnumSet<Capability>,
    pub pricing: Option<Pricing>,   // input/output 每百万 token
    pub deprecated: bool,
}
```

规则：请求所需能力必须是模型能力的子集，否则在**路由阶段 fast-fail**，不要发到供应商再报错。
注意 `JsonMode`（只保证合法 JSON）与严格 schema 输出是**两种能力**，见 §3。

## 2. 统一接口

```rust
pub trait Provider: Send + Sync + 'static {
    fn id(&self) -> &str;
    fn list_models(&self) -> Vec<ModelSpec>;

    fn chat(
        &self,
        req: ModelRequest,
        ctx: &CallContext,
    ) -> impl Future<Output = Result<ModelResponse, ProviderError>> + Send;

    fn stream(
        &self,
        req: ModelRequest,
        ctx: &CallContext,
    ) -> Result<EventStream, ProviderError>;   // 同步返回流句柄，便于 fast-fail

    fn count_tokens(&self, req: &ModelRequest) -> Option<u32> { None }
}

pub type EventStream = Pin<Box<dyn Stream<Item = StreamEvent> + Send + 'static>>;
```

- Core 只依赖上面这些方法（G-1/G-2）。
- `stream` 故意是**同步返回**：鉴权/参数错误要在发出请求前就暴露，而不是等第一次 poll。
- 厂商特有参数一律走 `req.additional_params`，**不允许污染公共字段**。

## 3. 请求 / 响应

```rust
pub struct ModelRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub tools: Vec<ToolDefinition>,
    pub tool_choice: Option<ToolChoice>,
    pub response_format: Option<ResponseFormat>,
    pub stop: Vec<String>,
    pub metadata: serde_json::Map<String, serde_json::Value>,
    pub additional_params: serde_json::Map<String, serde_json::Value>, // provider 透传
}

pub enum ResponseFormat {
    Text,
    JsonObject,                          // 只保证合法 JSON（DeepSeek Chat 仅有此档）
    JsonSchema { name: String, schema: serde_json::Value },
}

pub struct ModelResponse {
    pub id: String,
    pub model: String,
    pub text: String,
    pub tool_calls: Vec<ToolCall>,
    pub finish_reason: FinishReason,
    pub usage: Usage,
    pub raw: Option<serde_json::Value>,
}

pub enum FinishReason { Stop, Length, ToolCalls, ContentFilter, Other(String) }
```

工具类型（**在 provider 边界完成归一化**）：

```rust
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,   // JSON Schema
}

pub struct ToolCall {
    pub id: String,                      // 归一化后的句柄，用于回传 tool_result
    pub name: String,
    pub arguments: serde_json::Value,    // 统一为结构化 JSON
    pub provider_id: Option<String>,     // 供应商原发 id（OpenAI Responses 有双 id）
}
```

规则：OpenAI / DeepSeek 返回的 `arguments` 是 **JSON 字符串**（可能非法），Anthropic 的 `input` 已是结构化 JSON —— 反序列化与容错必须在 adapter 内完成，Core 只看到结构化结果。

## 4. 流式事件（归一化）

| 事件 | 载荷 | 说明 |
|------|------|------|
| `Start` | `{ response_id, model }` | 首个事件 |
| `Delta` | `{ kind: Text \| Thinking \| ToolArgs, text }` | 增量，**必须可无状态拼接** |
| `ToolCall` | `{ call }` | 完整的工具调用（分片拼装完成后） |
| `Usage` | `{ usage }` | 结束前必须给一次 |
| `Error` | `{ error }` | 之后不再发任何事件 |
| `End` | `{ finish_reason }` | 末个事件 |

```rust
pub enum StreamEvent {
    Start { response_id: String, model: String },
    Delta { kind: DeltaKind, text: String },
    ToolCall(ToolCall),
    Usage(Usage),
    Error(ProviderError),
    End { finish_reason: FinishReason },
}
```

适配器必须处理的收尾语义（三类要区分）：

1. **传输错误** → `Error` 后停止。
2. **可恢复坏帧** → 跳过该帧继续，不中断整个流。
3. **EOF 无终止记录（截断）** → 补一个 `End { finish_reason: Length }` 或映射为 `Error`，禁止静默当成功。

## 5. 错误分类

| 类别 | 触发 | 默认策略 |
|------|------|---------|
| `Auth` | 401 / 403 / 密钥无效 | 不重试，告警 |
| `RateLimit` | 429 | 指数退避 + 抖动，必要时换模型 |
| `Timeout` | 超时（含流式空闲超时） | 短请求重试，长请求改流式 |
| `InvalidRequest` | 400（参数 / 模型不支持） | 不重试，降级到兼容模型 |
| `Server` | 5xx | 退避重试 |
| `ContextOverflow` | 超上下文窗口 | 裁剪 / 摘要后重试一次 |
| `ContentFilter` | 命中审核 | 不重试，上抛业务 |
| `Truncated` | 流提前结束 | 视业务决定是否重发 |
| `Unknown` | 其它 | 保留 raw 后上抛 |

```rust
#[derive(Debug, thiserror::Error)]
#[error("{provider}: {category:?} (status={status:?})")]
pub struct ProviderError {
    pub provider: String,
    pub category: ErrorCategory,
    pub retryable: bool,
    pub status: Option<u16>,
    pub request_id: Option<String>,
    pub raw: Option<serde_json::Value>,
    #[source]
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}
```

每个 adapter 必须提供「HTTP 状态 / 错误体 → `ErrorCategory`」的映射表。

## 6. 新增供应商落地清单

- [ ] 建 `docs/providers/<name>.md`：endpoint、鉴权、字段差异、配额、已知坑
- [ ] 实现 `Provider`（`chat` 与 `stream` 都要）
- [ ] 声明 `ModelSpec` 清单：上下文窗口、能力、价格
- [ ] 错误映射覆盖第 5 节全部类别
- [ ] 契约测试：用同一份 fixtures 跑通（见 `30-testing.md`）
- [ ] 密钥走配置层，禁止写死（见 `20-config-and-secrets.md`）
- [ ] 在 `AGENTS.md` 索引中回填状态为 ✅

## 7. 待定问题

- embedding / rerank 是否纳入同一契约
- `count_tokens` 是否强制实现（本地 tiktoken 近似 vs 供应商接口）
- 首批 adapter 范围 → `adr/0002-llm-abstraction.md` §待确认
