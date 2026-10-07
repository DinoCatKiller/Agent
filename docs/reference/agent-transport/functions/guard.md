---
id: guard
title: guard
---

# Function: guard()

```rust
pub fn guard<S>(stream: S, token: CancellationToken) -> CancelGuard<S>
```

Defined in: [`crates/kernel/transport/src/cancel.rs:122`](../../../../crates/kernel/transport/src/cancel.rs#L122)

给流套上取消守卫。典型位置是**流水线最外层**：
`guard(finalize(parse_sse(bytes), policy), ctx.cancel)`。

## Type Parameters

### S

`S`

## Parameters

### stream

`S`

### token

`CancellationToken`

## Returns

[`CancelGuard<S>`](../structs/CancelGuard.md)

