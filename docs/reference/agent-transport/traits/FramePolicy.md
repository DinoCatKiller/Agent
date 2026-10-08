---
id: FramePolicy
title: FramePolicy
---

# Trait: FramePolicy

Defined in: [`crates/kernel/transport/src/finish.rs:85`](../../../../crates/kernel/transport/src/finish.rs#L85)

收尾判决：由适配器实现，把「一帧」翻译成「归一化事件 + 终局判定」。

`F` 是**输入帧类型**（典型为 [`SseEvent`](../structs/SseEvent.md)，但机制不限于 SSE——见模块文档）。
实现者**不得**在此读网络或做 IO：它只是纯函数式的翻译 + 判定。

