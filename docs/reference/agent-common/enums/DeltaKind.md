---
id: DeltaKind
title: DeltaKind
---

# Enum: DeltaKind

Defined in: [`crates/common/src/stream.rs:11`](../../../../crates/common/src/stream.rs#L11)

增量类型。

## Variants

### Text

Defined in: [`crates/common/src/stream.rs:12`](../../../../crates/common/src/stream.rs#L12)


***

### Thinking

Defined in: [`crates/common/src/stream.rs:14`](../../../../crates/common/src/stream.rs#L14)

推理内容（如 reasoning 模型）。


***

### ToolArgs

Defined in: [`crates/common/src/stream.rs:17`](../../../../crates/common/src/stream.rs#L17)

工具参数的**分片**。适配器负责按 index/id 拼接，
拼装完成后才发 `StreamEvent::ToolCall`。

## Trait Implementations

- `impl Borrow for DeltaKind`
- `impl BorrowMut for DeltaKind`
- `impl CloneToUninit for DeltaKind`
- `impl Into for DeltaKind`
- `impl From for DeltaKind`
- `impl TryInto for DeltaKind`
- `impl TryFrom for DeltaKind`
- `impl Any for DeltaKind`
- `impl ToOwned for DeltaKind`
- `impl DeserializeOwned for DeltaKind`
- `impl Debug for DeltaKind`
- `impl Clone for DeltaKind`
- `impl Copy for DeltaKind`
- `impl StructuralPartialEq for DeltaKind`
- `impl PartialEq for DeltaKind`
- `impl Eq for DeltaKind`
- `impl Serialize for DeltaKind`
- `impl Deserialize for DeltaKind`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

