---
id: parse_sse
title: parse_sse
---

# Function: parse_sse()

```rust
pub fn parse_sse<S>(stream: S) -> SseStream<S>
where
    S: Stream + Unpin
```

Defined in: [`crates/kernel/transport/src/sse.rs:275`](../../../../crates/kernel/transport/src/sse.rs#L275)

便捷入口：`bytes_stream` → [`SseEvent`](../structs/SseEvent.md) 流。

典型管线：
`resp.bytes_stream().map(TransportError::from)` →（可选）`HttpClient::guard_idle`
→ `parse_sse`。

## Type Parameters

### S

`S`

## Parameters

### stream

`S`

## Returns

[`SseStream<S>`](../structs/SseStream.md)

