# agent-providers · Provider 契约与适配器

| | |
|---|---|
| 状态 | ✅ 两个适配器已落地（M3：OpenAI 兼容族 `P1`、Anthropic `P2`，同一套 12 条用例各自全绿） |
| 边界 | 定义 `Provider` trait 与注册表；持有各家的协议实现。**不含** Agent 循环、路由策略、UI |
| 上游 | `agent-common` |
| 下游 | `features/*`（如 `chat`，编排核心）/ `kernel/routing`（通过 `ProviderRegistry`） |
| 何时读 | 加/改一家供应商、改 trait、讨论抽象是否合适 |

## 这个 crate 是什么

「插座」。Core 只认识 `ProviderRegistry`，不认识任何具体供应商；
每接一家模型 = 新增一个实现了 `Provider` 的适配器 + 在注册表登记一行。

## 核心内容

| 项 | 说明 |
|----|------|
| `Provider` | `id` / `list_models` / `chat` / `stream` / `count_tokens`。`stream` **同步返回**流句柄，鉴权与参数错误在发请求前就暴露 |
| `ErasedProvider` | 类型擦除视图。`Provider` 因 `impl Future` 返回值不是 object-safe，路由层要 `dyn` 就得靠它 |
| `ProviderRegistry` | `register` / `get` / `ids` / `models` / `find_model` |
| `required_capabilities()` | 从 `ModelRequest` 推导所需能力，供路由阶段 fast-fail（4 个单测覆盖） |
| `CallContext` | `request_id` + `cancel: CancellationToken` |
| [`openai`](src/openai.rs) | OpenAI 兼容适配器：`OpenAiCompatible` + `OpenAiFramePolicy`（SSE→归一化、工具分片拼装、错误映射覆盖 `A2` §5），契约测试 12 用例全绿 |
| [`anthropic`](src/anthropic.rs) | Anthropic Messages 适配器：`AnthropicCompatible` + `AnthropicFramePolicy`（命名事件 SSE→归一化、`tool_use` 拼装、role 交替合并、错误映射覆盖 `A2` §5），契约测试 12 用例全绿（`P2`） |
| `check_model_capabilities()` / `map_transport_error()` | 各适配器共用的能力协商 fast-fail 与传输错误映射（`lib.rs`，crate 私有） |

## 两个必须知道的设计决定

1. **`Provider: Clone` 是硬要求**。类型擦除需要 owned future：
   `let this = self.clone(); async move { this.chat(req, ctx).await }`。
   适配器内部用 `Arc<Client>` 持有连接，clone 只是引用计数。
2. **工具参数在适配器边界归一化**：OpenAI/DeepSeek 返回的是 JSON **字符串**（可能非法），
   Anthropic 返回结构化 `input` → Core 只看到结构化 `Value`。

## 加一家供应商的步骤

1. 复制模板 [`crates/kernel/providers/docs/providers/_template.md`](docs/providers/_template.md) → `docs/providers/<name>.md`，填协议 / 鉴权 / 字段映射 / 错误映射。
2. 写适配器：实现 `Provider`（`chat` 与 `stream` 都要）+ 用 `Arc<Client>` 包住连接。
3. 声明 `ModelSpec` 清单（上下文窗口、能力、价格）。
4. 错误映射表覆盖 `A2` §5 的全部类别。
5. 按 `Q1` 的 12 条用例建 fixtures 契约测试。
6. 在 [`AGENTS.md`](../../../AGENTS.md) §2 地图登记该供应商文档并把状态改成 ✅。

## 相关

- `A2` 接入契约 ｜ `A3` 消息协议 ｜ `X3` 抽象路线调研 ｜ `D2` 抽象决策 ｜ `PT` 供应商模板 ｜ `Q1` 测试策略
