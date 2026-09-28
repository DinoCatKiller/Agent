# agent-common · 契约唯一来源

| | |
|---|---|
| 状态 | ✅ 已落地（M1） |
| 边界 | **只依赖** `serde` / `serde_json` / `thiserror`。不依赖 HTTP、UI、任何供应商实现 |
| 上游 | 无（最底层） |
| 下游 | 所有其它 crate 都依赖它 |
| 何时读 | 改契约类型前；新增字段 / 新增能力位前 |

## 这个 crate 是什么

把 `A2`（Provider 接入契约）与 `A3`（消息协议）从文档变成**编译期可校验的类型**。
Core、适配器、UI 三方都只认这里的类型，供应商差异不许越过这道边界。

## 模块

| 模块 | 内容 | 对应文档 |
|------|------|---------|
| `message` | `Role` / `ContentPart` / `Message`（含 `system()`/`user()`/`assistant()`/`tool_result()` 构造器、`text()`） | `A3` |
| `model` | `Capability`（10 位）/ `Pricing` / `ModelSpec`（`supports()` / `missing()`） | `A2` §1 |
| `completion` | `ModelRequest` / `ModelResponse` / `ToolDefinition` / `ToolCall` / `ToolChoice` / `ResponseFormat` / `FinishReason` / `Usage` | `A2` §3 |
| `stream` | `DeltaKind` / `StreamEvent`（Start / Delta / ToolCall / Usage / Error / End） | `A2` §4 |
| `error` | `ErrorCategory`（9 类）/ `ProviderError` | `A2` §5 |

## 三个必须知道的设计决定

1. **`JsonMode` 与 `JsonSchema` 是两个能力位**：OpenAI 两者都有，DeepSeek Chat 只有前者。合并会导致必然失败的请求被打到供应商。
2. **`ProviderError` 不用 `Box<dyn Error>`**，改用 `source_message: String`：错误要进 `StreamEvent`、日志和 fixtures，必须 `Clone + Serialize`。
3. **`FinishReason::Truncated` 是独立变体**：流被提前掐断不能当 `Stop`。

## 用法

```rust
use agent_common::{Message, ModelRequest, ResponseFormat};

let mut req = ModelRequest::new("gpt-4o-mini", vec![
    Message::system("你是一个简洁的助手。"),
    Message::user("用一句话说明 SSE 的好处。"),
]);
req.response_format = Some(ResponseFormat::JsonObject);
```

## 相关

- `A2` Provider 接入契约 ｜ `A3` 消息与多模态协议 ｜ `Q1` 测试策略
- 契约变更流程：先改 `A2` / `A3` → 再改这里的类型 → 再改适配器 → 最后改 Core / UI
