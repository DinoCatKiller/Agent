---
id: SseDecoder
title: SseDecoder
---

# Struct: SseDecoder

Defined in: [`crates/kernel/transport/src/sse.rs:69`](../../../../crates/kernel/transport/src/sse.rs#L69)

增量 SSE 解码器：喂入原始字节，吐出**完整**事件。

在**字节层**切行（先找 `\n` / `\r` 终止符，再整行按 UTF-8 解码），
因此天然免疫「多字节字符被切在 chunk 边界」的问题。

_（存在非公开字段）_

## Implementations

### new()

```rust
pub fn new() -> Self
```

Defined in: [`crates/kernel/transport/src/sse.rs:85`](../../../../crates/kernel/transport/src/sse.rs#L85)

#### Returns

`Self`


***

### feed()

```rust
pub fn feed(&self, chunk: &[u8], out: &Vec<SseEvent>)
```

Defined in: [`crates/kernel/transport/src/sse.rs:90`](../../../../crates/kernel/transport/src/sse.rs#L90)

喂入一块字节，把新解析出的完整事件追加到 `out`。

#### Parameters

##### chunk

`&[u8]`

##### out

`&Vec<SseEvent>`


***

### finish()

```rust
pub fn finish(&self, out: &Vec<SseEvent>)
```

Defined in: [`crates/kernel/transport/src/sse.rs:100`](../../../../crates/kernel/transport/src/sse.rs#L100)

流结束。处理末尾残行，但**不派发**未以空行收尾的不完整事件。

依据 SSE 规范：文件在事件中间结束时，该不完整事件应被丢弃
（这正是 `A2` §4 的「截断不得当成功」在本层的落点）。

#### Parameters

##### out

`&Vec<SseEvent>`

## Trait Implementations

- `impl Borrow for SseDecoder`
- `impl BorrowMut for SseDecoder`
- `impl Into for SseDecoder`
- `impl From for SseDecoder`
- `impl TryInto for SseDecoder`
- `impl TryFrom for SseDecoder`
- `impl Any for SseDecoder`
- `impl Instrument for SseDecoder`
- `impl WithSubscriber for SseDecoder`
- `impl PolicyExt for SseDecoder`
- `impl Debug for SseDecoder`
- `impl Default for SseDecoder`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

