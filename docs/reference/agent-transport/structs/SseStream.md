---
id: SseStream
title: SseStream
---

# Struct: SseStream

Defined in: [`crates/kernel/transport/src/sse.rs:212`](../../../../crates/kernel/transport/src/sse.rs#L212)

把字节流适配成 [`SseEvent`](SseEvent.md) 流。

输入流若出错，则**原样上抛**该 [`TransportError`](../enums/TransportError.md) 并结束（传输错误语义，`A2` §4）。
输入流正常结束（EOF）时按 [`SseDecoder::finish`](SseDecoder.md) 收尾，不补发不完整事件。

_（存在非公开字段）_

## Implementations

### new()

```rust
pub fn new(inner: S) -> Self
```

Defined in: [`crates/kernel/transport/src/sse.rs:223`](../../../../crates/kernel/transport/src/sse.rs#L223)

#### Parameters

##### inner

`S`

#### Returns

`Self`

## Trait Implementations

- `impl Borrow for SseStream`
- `impl BorrowMut for SseStream`
- `impl Into for SseStream`
- `impl From for SseStream`
- `impl TryInto for SseStream`
- `impl TryFrom for SseStream`
- `impl Any for SseStream`
- `impl TryStream for SseStream`
- `impl StreamExt for SseStream`
- `impl TryStreamExt for SseStream`
- `impl Instrument for SseStream`
- `impl WithSubscriber for SseStream`
- `impl PolicyExt for SseStream`
- `impl Stream for SseStream`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

