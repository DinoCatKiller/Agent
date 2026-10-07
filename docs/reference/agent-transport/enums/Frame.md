---
id: Frame
title: Frame
---

# Enum: Frame

Defined in: [`crates/kernel/transport/src/finish.rs:47`](../../../../crates/kernel/transport/src/finish.rs#L47)

单帧的判决结果。由适配器在 [`FramePolicy::classify`](../traits/FramePolicy.md) 里给出。

## Variants

### Emit

```rust
(Vec<T>)
```

Defined in: [`crates/kernel/transport/src/finish.rs:49`](../../../../crates/kernel/transport/src/finish.rs#L49)

正常帧：产出 0..n 个下游事件，流继续。


***

### Skip

Defined in: [`crates/kernel/transport/src/finish.rs:53`](../../../../crates/kernel/transport/src/finish.rs#L53)

可恢复坏帧：跳过该帧，**不打断**整条流（`A2` §4-2）。

行为等价于 `Emit(vec![])`，但语义显式——便于观测与排查。


***

### Terminal

```rust
(Vec<T>)
```

Defined in: [`crates/kernel/transport/src/finish.rs:61`](../../../../crates/kernel/transport/src/finish.rs#L61)

终止记录：产出 0..n 个末帧后，流**正常结束**，不再向上游取数据。

为什么是**多个**：契约要求 `End` 之前必须有一次 `Usage`（`A2` §4）。供应商
可能把 usage 与终止原因放在同一条终止记录里（如 OpenAI Responses 的
`response.completed`）；供应商不给 usage 时，适配器也要在此补一次
`Usage::default()`（`Q1` 用例 11）。一帧只能产出一个事件的话，这两件事
无法同时满足。

## Implementations

### one()

```rust
pub fn one(event: T) -> Self
```

Defined in: [`crates/kernel/transport/src/finish.rs:66`](../../../../crates/kernel/transport/src/finish.rs#L66)

最常见情形：一帧 → 一个事件。

#### Parameters

##### event

`T`

#### Returns

`Self`


***

### many()

```rust
pub fn many<impl IntoIterator<Item = T>: IntoIterator>(events: impl ?) -> Self
```

Defined in: [`crates/kernel/transport/src/finish.rs:71`](../../../../crates/kernel/transport/src/finish.rs#L71)

一帧 → 多个事件（如 Anthropic `message_start` 一次给出 `Start` 与 `Usage`）。

#### Parameters

##### events

`impl ?`

#### Returns

`Self`


***

### end()

```rust
pub fn end() -> Self
```

Defined in: [`crates/kernel/transport/src/finish.rs:76`](../../../../crates/kernel/transport/src/finish.rs#L76)

不需要额外末帧的终止记录。

#### Returns

`Self`

## Trait Implementations

- `impl Borrow for Frame`
- `impl BorrowMut for Frame`
- `impl CloneToUninit for Frame`
- `impl Into for Frame`
- `impl From for Frame`
- `impl TryInto for Frame`
- `impl TryFrom for Frame`
- `impl Any for Frame`
- `impl ToOwned for Frame`
- `impl Instrument for Frame`
- `impl WithSubscriber for Frame`
- `impl PolicyExt for Frame`
- `impl Debug for Frame`
- `impl Clone for Frame`
- `impl StructuralPartialEq for Frame`
- `impl PartialEq for Frame`
- `impl Eq for Frame`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

