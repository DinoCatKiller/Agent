---
id: Role
title: Role
---

# Enum: Role

Defined in: [`crates/common/src/message.rs:12`](../../../../crates/common/src/message.rs#L12)

消息角色。四家的 role 集合并集就是这四个。

## Variants

### System

Defined in: [`crates/common/src/message.rs:13`](../../../../crates/common/src/message.rs#L13)


***

### User

Defined in: [`crates/common/src/message.rs:14`](../../../../crates/common/src/message.rs#L14)


***

### Assistant

Defined in: [`crates/common/src/message.rs:15`](../../../../crates/common/src/message.rs#L15)


***

### Tool

Defined in: [`crates/common/src/message.rs:17`](../../../../crates/common/src/message.rs#L17)

工具结果回填。必须同时带 `tool_call_id`。

## Trait Implementations

- `impl Borrow for Role`
- `impl BorrowMut for Role`
- `impl CloneToUninit for Role`
- `impl Into for Role`
- `impl From for Role`
- `impl TryInto for Role`
- `impl TryFrom for Role`
- `impl Any for Role`
- `impl ToOwned for Role`
- `impl DeserializeOwned for Role`
- `impl Debug for Role`
- `impl Clone for Role`
- `impl Copy for Role`
- `impl StructuralPartialEq for Role`
- `impl PartialEq for Role`
- `impl Eq for Role`
- `impl Serialize for Role`
- `impl Deserialize for Role`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

