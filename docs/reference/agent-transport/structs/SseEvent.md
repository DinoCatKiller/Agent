---
id: SseEvent
title: SseEvent
---

# Struct: SseEvent

Defined in: [`crates/kernel/transport/src/sse.rs:45`](../../../../crates/kernel/transport/src/sse.rs#L45)

一条已解析的 SSE 事件（通用字段，不含厂商语义）。

字段对应 SSE 规范：`event` / `data` / `id` / `retry`。
多行 `data:` 按规范以 `\n` 连接后放入 `SseEvent::data`。

## Fields

### event

```rust
event: Option<String>
```

Defined in: [`crates/kernel/transport/src/sse.rs:48`](../../../../crates/kernel/transport/src/sse.rs#L48)

命名事件类型（如 Anthropic 的 `message_start`，或 OpenAI Responses 的
`response.completed`）。未出现 `event:` 字段时为 `None`（data-only 流）。


***

### data

```rust
data: String
```

Defined in: [`crates/kernel/transport/src/sse.rs:50`](../../../../crates/kernel/transport/src/sse.rs#L50)

数据负载。多个 `data:` 行以 `\n` 连接。


***

### id

```rust
id: Option<String>
```

Defined in: [`crates/kernel/transport/src/sse.rs:52`](../../../../crates/kernel/transport/src/sse.rs#L52)

最近一次 `id:`（忽略含 `\0` 的非法值）。


***

### retry

```rust
retry: Option<u64>
```

Defined in: [`crates/kernel/transport/src/sse.rs:54`](../../../../crates/kernel/transport/src/sse.rs#L54)

最近一次 `retry:`，单位毫秒。

## Implementations

### is_done()

```rust
pub fn is_done(&self) -> bool
```

Defined in: [`crates/kernel/transport/src/sse.rs:59`](../../../../crates/kernel/transport/src/sse.rs#L59)

OpenAI / DeepSeek Chat 流以 `data: [DONE]` 表示正常结束。

#### Returns

`bool`

## Trait Implementations

- `impl Borrow for SseEvent`
- `impl BorrowMut for SseEvent`
- `impl CloneToUninit for SseEvent`
- `impl Into for SseEvent`
- `impl From for SseEvent`
- `impl TryInto for SseEvent`
- `impl TryFrom for SseEvent`
- `impl Any for SseEvent`
- `impl ToOwned for SseEvent`
- `impl Instrument for SseEvent`
- `impl WithSubscriber for SseEvent`
- `impl PolicyExt for SseEvent`
- `impl Debug for SseEvent`
- `impl Clone for SseEvent`
- `impl StructuralPartialEq for SseEvent`
- `impl PartialEq for SseEvent`
- `impl Eq for SseEvent`
- `impl Default for SseEvent`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

