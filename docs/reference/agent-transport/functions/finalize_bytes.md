---
id: finalize_bytes
title: finalize_bytes
---

# Function: finalize_bytes()

```rust
pub fn finalize_bytes<S, P>(bytes: S, policy: P) -> FinalizedStream<SseStream<S>, SseEvent, P>
where
    S: Stream + Unpin,
    P: [FramePolicy](traits/FramePolicy.md) + Unpin
```

Defined in: [`crates/kernel/transport/src/finish.rs:209`](../../../../crates/kernel/transport/src/finish.rs#L209)

**本模块唯一提到 SSE 的地方**：SSE 装配便捷入口（`parse_sse` + [`finalize`](finalize.md)）。

只做「解析 + 收尾」；空闲超时由 [`crate::http::HttpClient::guard_idle`](../structs/HttpClient.md) 在更上游套好。
不使用本函数时，直接写 `finalize(parse_sse(bytes), policy)` 完全等价。

## Type Parameters

### S

`S`

### P

`P`

## Parameters

### bytes

`S`

### policy

`P`

## Returns

[`FinalizedStream<SseStream<S>, SseEvent, P>`](../structs/FinalizedStream.md)

