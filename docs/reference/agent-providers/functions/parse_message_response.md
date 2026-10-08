---
id: parse_message_response
title: parse_message_response
---

# Function: parse_message_response()

```rust
pub fn parse_message_response(_provider: &str, body: Value) -> Result<ModelResponse, ProviderError>
```

Defined in: [`crates/kernel/providers/src/anthropic.rs:784`](../../../../crates/kernel/providers/src/anthropic.rs#L784)

非流式响应体 → `ModelResponse`（`Q1` 用例 1）。`tool_use` 的 `input` 已是结构化 JSON，直接取。
`thinking` / `redacted_thinking` 块不进 `text`（raw 已保留）。

## Parameters

### _provider

`&str`

### body

`Value`

## Returns

`Result<ModelResponse, ProviderError>`

