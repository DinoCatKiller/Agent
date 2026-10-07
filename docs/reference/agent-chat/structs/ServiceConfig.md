---
id: ServiceConfig
title: ServiceConfig
---

# Struct: ServiceConfig

Defined in: [`crates/features/chat/src/service.rs:71`](../../../../crates/features/chat/src/service.rs#L71)

循环与裁剪的运行参数。

## Fields

### max_model_turns

```rust
max_model_turns: u32
```

Defined in: [`crates/features/chat/src/service.rs:73`](../../../../crates/features/chat/src/service.rs#L73)

一轮用户输入内的最大模型调用次数（默认 8）。


***

### context_keep_ratio

```rust
context_keep_ratio: f64
```

Defined in: [`crates/features/chat/src/service.rs:75`](../../../../crates/features/chat/src/service.rs#L75)

裁剪目标：历史估算 ≤ `context_window × keep_ratio`（默认 0.8，给输出留余量）。

## Trait Implementations

- `impl Borrow for ServiceConfig`
- `impl BorrowMut for ServiceConfig`
- `impl CloneToUninit for ServiceConfig`
- `impl Into for ServiceConfig`
- `impl From for ServiceConfig`
- `impl TryInto for ServiceConfig`
- `impl TryFrom for ServiceConfig`
- `impl Any for ServiceConfig`
- `impl ToOwned for ServiceConfig`
- `impl CastableFrom for ServiceConfig`
- `impl CastableFrom for ServiceConfig`
- `impl Read for ServiceConfig`
- `impl Instrument for ServiceConfig`
- `impl WithSubscriber for ServiceConfig`
- `impl PolicyExt for ServiceConfig`
- `impl IntoEither for ServiceConfig`
- `impl Debug for ServiceConfig`
- `impl Clone for ServiceConfig`
- `impl Default for ServiceConfig`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

