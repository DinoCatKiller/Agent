---
id: FinishReason
title: FinishReason
---

# Enum: FinishReason

Defined in: [`crates/common/src/completion.rs:100`](../../../../crates/common/src/completion.rs#L100)

结束原因。未知值归一为 `Other`，不要丢弃（日志需要）。

## Variants

### Stop

Defined in: [`crates/common/src/completion.rs:101`](../../../../crates/common/src/completion.rs#L101)


***

### Length

Defined in: [`crates/common/src/completion.rs:102`](../../../../crates/common/src/completion.rs#L102)


***

### ToolCalls

Defined in: [`crates/common/src/completion.rs:103`](../../../../crates/common/src/completion.rs#L103)


***

### ContentFilter

Defined in: [`crates/common/src/completion.rs:104`](../../../../crates/common/src/completion.rs#L104)


***

### Truncated

Defined in: [`crates/common/src/completion.rs:106`](../../../../crates/common/src/completion.rs#L106)

流被截断（EOF 无终止记录）。


***

### Other

Defined in: [`crates/common/src/completion.rs:107`](../../../../crates/common/src/completion.rs#L107)

## Trait Implementations

- `impl Borrow for FinishReason`
- `impl BorrowMut for FinishReason`
- `impl CloneToUninit for FinishReason`
- `impl Into for FinishReason`
- `impl From for FinishReason`
- `impl TryInto for FinishReason`
- `impl TryFrom for FinishReason`
- `impl Any for FinishReason`
- `impl ToOwned for FinishReason`
- `impl DeserializeOwned for FinishReason`
- `impl Debug for FinishReason`
- `impl Clone for FinishReason`
- `impl Copy for FinishReason`
- `impl StructuralPartialEq for FinishReason`
- `impl PartialEq for FinishReason`
- `impl Eq for FinishReason`
- `impl Serialize for FinishReason`
- `impl Deserialize for FinishReason`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

