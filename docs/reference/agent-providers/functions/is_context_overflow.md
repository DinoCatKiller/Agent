---
id: is_context_overflow
title: is_context_overflow
---

# Function: is_context_overflow()

```rust
pub fn is_context_overflow(raw: &Value) -> bool
```

Defined in: [`crates/kernel/providers/src/openai.rs:640`](../../../../crates/kernel/providers/src/openai.rs#L640)

400 错误体是否暗示上下文超限（`Q1` 用例 10）。OpenAI 用 `context_length_exceeded`，
兼容实现文案不一，按子串兜底。

## Parameters

### raw

`&Value`

## Returns

`bool`

