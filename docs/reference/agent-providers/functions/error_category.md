---
id: error_category
title: error_category
---

# Function: error_category()

```rust
pub fn error_category(status: u16, raw: &Value) -> ErrorCategory
```

Defined in: [`crates/kernel/providers/src/openai.rs:620`](../../../../crates/kernel/providers/src/openai.rs#L620)

HTTP 状态 + 错误体 → `ErrorCategory`（覆盖 `A2` §5 全部类别）。

## Parameters

### status

`u16`

### raw

`&Value`

## Returns

`ErrorCategory`

