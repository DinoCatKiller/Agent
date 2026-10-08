---
id: stream_error_category
title: stream_error_category
---

# Function: stream_error_category()

```rust
pub fn stream_error_category(error_type: &str) -> ErrorCategory
```

Defined in: [`crates/kernel/providers/src/anthropic.rs:715`](../../../../crates/kernel/providers/src/anthropic.rs#L715)

流中 `error` 事件的 `error.type` → `ErrorCategory`（无 HTTP 状态码，按类型映射）。

## Parameters

### error_type

`&str`

## Returns

`ErrorCategory`

