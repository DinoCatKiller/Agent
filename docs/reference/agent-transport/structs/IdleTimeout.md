---
id: IdleTimeout
title: IdleTimeout
---

# Struct: IdleTimeout

Defined in: [`crates/kernel/transport/src/http.rs:172`](../../../../crates/kernel/transport/src/http.rs#L172)

空闲超时流包装：两次数据之间超过 `idle` 即产出
[`TransportError::IdleTimeout`](../enums/TransportError.md)，收到数据则重置计时。

用「相邻数据间隔」而非「整体耗时」作为判据，长回答不会误杀。

**不做错误归一化**：要求上游已经是 `Result<T, TransportError>`
（由 `bytes_stream().map(TransportError::from)` 完成），输出类型不变，
因此可以嵌进管线的任意位置。

_（存在非公开字段）_

## Implementations

### new()

```rust
pub fn new(stream: S, idle: Duration) -> Self
```

Defined in: [`crates/kernel/transport/src/http.rs:179`](../../../../crates/kernel/transport/src/http.rs#L179)

#### Parameters

##### stream

`S`

##### idle

`Duration`

#### Returns

`Self`

## Trait Implementations

- `impl Borrow for IdleTimeout`
- `impl BorrowMut for IdleTimeout`
- `impl Into for IdleTimeout`
- `impl From for IdleTimeout`
- `impl TryInto for IdleTimeout`
- `impl TryFrom for IdleTimeout`
- `impl Any for IdleTimeout`
- `impl TryStream for IdleTimeout`
- `impl StreamExt for IdleTimeout`
- `impl TryStreamExt for IdleTimeout`
- `impl Instrument for IdleTimeout`
- `impl WithSubscriber for IdleTimeout`
- `impl PolicyExt for IdleTimeout`
- `impl Stream for IdleTimeout`

## Auto Trait Implementations

`Freeze` `Send` `Sync` `Unpin` `UnsafeUnpin`

