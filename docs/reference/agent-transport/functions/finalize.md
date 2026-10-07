---
id: finalize
title: finalize
---

# Function: finalize()

```rust
pub fn finalize<S, F, P>(stream: S, policy: P) -> FinalizedStream<S, F, P>
where
    S: Stream + Unpin,
    P: [FramePolicy](traits/FramePolicy.md) + Unpin
```

Defined in: [`crates/kernel/transport/src/finish.rs:197`](../../../../crates/kernel/transport/src/finish.rs#L197)

把**任意帧流**交给 `policy` 做三分收尾（见模块文档）。

帧类型 `F` 由调用方决定：机制不假设它是 SSE。

## Type Parameters

### S

`S`

### F

`F`

### P

`P`

## Parameters

### stream

`S`

### policy

`P`

## Returns

[`FinalizedStream<S, F, P>`](../structs/FinalizedStream.md)

