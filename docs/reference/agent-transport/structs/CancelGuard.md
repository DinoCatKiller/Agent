---
id: CancelGuard
title: CancelGuard
---

# Struct: CancelGuard

Defined in: [`crates/kernel/transport/src/cancel.rs:55`](../../../../crates/kernel/transport/src/cancel.rs#L55)

取消感知的流适配器，由 [`guard`](../functions/guard.md) 构造（或 [`CancelGuard::new`](CancelGuard.md)）。

语义：取消后产出**一次** [`TransportError::Cancelled`](../enums/TransportError.md)，随后结束；
上游的错误同样会被转发并结束本流（与 `sse` / `finish` 的收尾一致）。

_（存在非公开字段）_

## Implementations

### new()

```rust
pub fn new(inner: S, token: CancellationToken) -> Self
```

Defined in: [`crates/kernel/transport/src/cancel.rs:66`](../../../../crates/kernel/transport/src/cancel.rs#L66)

用 `token` 守卫 `inner`。

#### Parameters

##### inner

`S`

##### token

`CancellationToken`

#### Returns

`Self`


***

### token()

```rust
pub fn token(&self) -> &CancellationToken
```

Defined in: [`crates/kernel/transport/src/cancel.rs:77`](../../../../crates/kernel/transport/src/cancel.rs#L77)

本守卫监听的取消令牌。

#### Returns

`&CancellationToken`


***

### is_cancelled()

```rust
pub fn is_cancelled(&self) -> bool
```

Defined in: [`crates/kernel/transport/src/cancel.rs:82`](../../../../crates/kernel/transport/src/cancel.rs#L82)

令牌是否已取消（收尾后可区分「被取消结束」与「正常结束」）。

#### Returns

`bool`

## Trait Implementations

- `impl Borrow for CancelGuard`
- `impl BorrowMut for CancelGuard`
- `impl Into for CancelGuard`
- `impl From for CancelGuard`
- `impl TryInto for CancelGuard`
- `impl TryFrom for CancelGuard`
- `impl Any for CancelGuard`
- `impl TryStream for CancelGuard`
- `impl StreamExt for CancelGuard`
- `impl TryStreamExt for CancelGuard`
- `impl Instrument for CancelGuard`
- `impl WithSubscriber for CancelGuard`
- `impl PolicyExt for CancelGuard`
- `impl Stream for CancelGuard`

## Auto Trait Implementations

`Freeze` `Send` `Sync` `Unpin` `UnsafeUnpin`

