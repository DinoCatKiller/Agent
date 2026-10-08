---
id: Entry
title: Entry
---

# Enum: Entry

Defined in: [`crates/features/chat/src/ui.rs:148`](../../../../crates/features/chat/src/ui.rs#L148)

转录条目：显示单位。直播时由 [`LoopEvent`](LoopEvent.md) 攒出，历史回放时由 [`Chat`](../structs/Chat.md) 消息映射而来。

## Variants

### User

```rust
(String)
```

Defined in: [`crates/features/chat/src/ui.rs:149`](../../../../crates/features/chat/src/ui.rs#L149)


***

### Assistant

```rust
(String)
```

Defined in: [`crates/features/chat/src/ui.rs:150`](../../../../crates/features/chat/src/ui.rs#L150)


***

### Thinking

```rust
(String)
```

Defined in: [`crates/features/chat/src/ui.rs:152`](../../../../crates/features/chat/src/ui.rs#L152)

思考增量（只显示不进历史，`A3`）——重启回放后消失是预期行为。


***

### Tool

```rust
{ .. }
```

Defined in: [`crates/features/chat/src/ui.rs:153`](../../../../crates/features/chat/src/ui.rs#L153)


***

### ToolResult

```rust
(String)
```

Defined in: [`crates/features/chat/src/ui.rs:158`](../../../../crates/features/chat/src/ui.rs#L158)

工具结果（仅历史回放；直播时结果不展示，只更新 [`Entry::Tool`](Entry.md) 状态）。


***

### Note

```rust
(String)
```

Defined in: [`crates/features/chat/src/ui.rs:160`](../../../../crates/features/chat/src/ui.rs#L160)

系统注记：轮次统计、取消、错误、拒绝。

## Trait Implementations

- `impl Borrow for Entry`
- `impl BorrowMut for Entry`
- `impl CloneToUninit for Entry`
- `impl Into for Entry`
- `impl From for Entry`
- `impl TryInto for Entry`
- `impl TryFrom for Entry`
- `impl Any for Entry`
- `impl ToOwned for Entry`
- `impl Equivalent for Entry`
- `impl CastableFrom for Entry`
- `impl CastableFrom for Entry`
- `impl Read for Entry`
- `impl Instrument for Entry`
- `impl WithSubscriber for Entry`
- `impl PolicyExt for Entry`
- `impl IntoEither for Entry`
- `impl Equivalent for Entry`
- `impl Debug for Entry`
- `impl Clone for Entry`
- `impl StructuralPartialEq for Entry`
- `impl PartialEq for Entry`
- `impl Eq for Entry`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

