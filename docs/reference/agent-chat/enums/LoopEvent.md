---
id: LoopEvent
title: LoopEvent
---

# Enum: LoopEvent

Defined in: [`crates/features/chat/src/service.rs:38`](../../../../crates/features/chat/src/service.rs#L38)

编排层事件：UI 与测试消费的单位。

## Variants

### Delta

```rust
{ .. }
```

Defined in: [`crates/features/chat/src/service.rs:40`](../../../../crates/features/chat/src/service.rs#L40)

模型增量（Text / Thinking 都透传；thinking 只显示不进历史）。


***

### ModelTurnEnded

```rust
{ .. }
```

Defined in: [`crates/features/chat/src/service.rs:45`](../../../../crates/features/chat/src/service.rs#L45)

一次模型调用结束（收到 `End` 并已入史）。


***

### ToolStarted

```rust
{ .. }
```

Defined in: [`crates/features/chat/src/service.rs:49`](../../../../crates/features/chat/src/service.rs#L49)


***

### ToolFinished

```rust
{ .. }
```

Defined in: [`crates/features/chat/src/service.rs:54`](../../../../crates/features/chat/src/service.rs#L54)

`ok = false` 表示执行失败（错误文案已喂回模型）。


***

### RoundEnded

```rust
{ .. }
```

Defined in: [`crates/features/chat/src/service.rs:59`](../../../../crates/features/chat/src/service.rs#L59)

整轮结束：`usage` 为本轮各次调用之和（会话累计在 `Chat::total_usage`）。


***

### Cancelled

Defined in: [`crates/features/chat/src/service.rs:65`](../../../../crates/features/chat/src/service.rs#L65)

用户取消（Ctrl-C）。未提交的增量已回滚，之后不再有任何事件。


***

### Error

```rust
{ .. }
```

Defined in: [`crates/features/chat/src/service.rs:67`](../../../../crates/features/chat/src/service.rs#L67)

终止性错误。之后不再有任何事件（`A2` §4）。

## Trait Implementations

- `impl Borrow for LoopEvent`
- `impl BorrowMut for LoopEvent`
- `impl CloneToUninit for LoopEvent`
- `impl Into for LoopEvent`
- `impl From for LoopEvent`
- `impl TryInto for LoopEvent`
- `impl TryFrom for LoopEvent`
- `impl Any for LoopEvent`
- `impl ToOwned for LoopEvent`
- `impl CastableFrom for LoopEvent`
- `impl CastableFrom for LoopEvent`
- `impl Read for LoopEvent`
- `impl Instrument for LoopEvent`
- `impl WithSubscriber for LoopEvent`
- `impl PolicyExt for LoopEvent`
- `impl IntoEither for LoopEvent`
- `impl Debug for LoopEvent`
- `impl Clone for LoopEvent`
- `impl StructuralPartialEq for LoopEvent`
- `impl PartialEq for LoopEvent`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

