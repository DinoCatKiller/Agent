---
id: cancellable
title: cancellable
---

# Function: cancellable()

```rust
pub async fn cancellable<F, T>(token: &CancellationToken, future: F) -> Result<T, TransportError>
where
    F: Future
```

Defined in: [`crates/kernel/transport/src/cancel.rs:40`](../../../../crates/kernel/transport/src/cancel.rs#L40)

在 `token` 取消前完成 `future`；取消则返回 [`TransportError::Cancelled`](../enums/TransportError.md)。

用于**请求阶段**：适配器应当用它代替裸 `await`，例如
`cancellable(&ctx.cancel, request.send().map_err(TransportError::from))`。

`biased` 保证先看取消——已经取消时**绝不**发起（或继续等待）真实请求。

## Type Parameters

### F

`F`

### T

`T`

## Parameters

### token

`&CancellationToken`

### future

`F`

## Returns

`Result<T, TransportError>`

