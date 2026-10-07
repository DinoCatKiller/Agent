---
id: map_http_error
title: map_http_error
---

# Function: map_http_error()

```rust
pub fn map_http_error(provider: &str, status: u16, raw: Value) -> ProviderError
```

Defined in: [`crates/kernel/providers/src/openai.rs:665`](../../../../crates/kernel/providers/src/openai.rs#L665)

HTTP 状态 + 错误体（含 `request_id`）→ `ProviderError`。`provider` 名由调用方传入。

## Parameters

### provider

`&str`

### status

`u16`

### raw

`Value`

## Returns

`ProviderError`

