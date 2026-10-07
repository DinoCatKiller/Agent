---
id: RoundStop
title: RoundStop
---

# Enum: RoundStop

Defined in: [`crates/features/chat/src/service.rs:26`](../../../../crates/features/chat/src/service.rs#L26)

一轮对话怎么收场。

## Variants

### Finished

```rust
(FinishReason)
```

Defined in: [`crates/features/chat/src/service.rs:28`](../../../../crates/features/chat/src/service.rs#L28)

模型自然结束（`Stop` / `Length` / `Truncated`…，`A2` §4）。


***

### MaxTurns

```rust
(u32)
```

Defined in: [`crates/features/chat/src/service.rs:30`](../../../../crates/features/chat/src/service.rs#L30)

达到最大模型轮次（防工具乒乓死循环）。

## Trait Implementations

- `impl Borrow for RoundStop`
- `impl BorrowMut for RoundStop`
- `impl CloneToUninit for RoundStop`
- `impl Into for RoundStop`
- `impl From for RoundStop`
- `impl TryInto for RoundStop`
- `impl TryFrom for RoundStop`
- `impl Any for RoundStop`
- `impl ToOwned for RoundStop`
- `impl Equivalent for RoundStop`
- `impl CastableFrom for RoundStop`
- `impl CastableFrom for RoundStop`
- `impl Read for RoundStop`
- `impl Instrument for RoundStop`
- `impl WithSubscriber for RoundStop`
- `impl PolicyExt for RoundStop`
- `impl Debug for RoundStop`
- `impl Clone for RoundStop`
- `impl Copy for RoundStop`
- `impl StructuralPartialEq for RoundStop`
- `impl PartialEq for RoundStop`
- `impl Eq for RoundStop`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

