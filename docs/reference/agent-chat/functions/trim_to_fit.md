---
id: trim_to_fit
title: trim_to_fit
---

# Function: trim_to_fit()

```rust
pub fn trim_to_fit<impl Fn(&[Message]) -> u32: Fn(&[Message]) -> u32>(messages: &Vec<Message>, window: u32, keep_ratio: f64, estimate: impl ?)
```

Defined in: [`crates/features/chat/src/context.rs:21`](../../../../crates/features/chat/src/context.rs#L21)

把历史裁到窗口的 `keep_ratio` 以内（默认 0.8，给输出留余量）。

`estimate` 返回全量消息的近似 token 数。逐块从最老处删，直到达标或无块可删（尽力而为）。

## Type Parameters

### impl Fn(&[Message]) -> u32

`impl Fn(&[Message]) -> u32` *extends* `Fn(&[Message]) -> u32`

## Parameters

### messages

`&Vec<Message>`

### window

`u32`

### keep_ratio

`f64`

### estimate

`impl ?`

