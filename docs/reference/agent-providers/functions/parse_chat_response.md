---
id: parse_chat_response
title: parse_chat_response
---

# Function: parse_chat_response()

```rust
pub fn parse_chat_response(_provider: &str, body: Value) -> Result<ModelResponse, ProviderError>
```

Defined in: [`crates/kernel/providers/src/openai.rs:679`](../../../../crates/kernel/providers/src/openai.rs#L679)

非流式响应体 → `ModelResponse`（`Q1` 用例 1）。`arguments` 字符串在此解析为结构化 JSON。

## Parameters

### _provider

`&str`

### body

`Value`

## Returns

`Result<ModelResponse, ProviderError>`

