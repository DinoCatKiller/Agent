---
id: Tool
title: Tool
---

# Trait: Tool

Defined in: [`crates/features/chat/src/tools.rs:22`](../../../../crates/features/chat/src/tools.rs#L22)

一个可被模型调用的工具。

`run` 返回 owned future（手写 `BoxFuture`，与 `Provider` trait 的风格一致，不引 `async_trait`）；
取消由调用方（service）在 future 外层用 `select` 处理，工具自身不必感知。

