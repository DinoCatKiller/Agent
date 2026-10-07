---
id: Capability
title: Capability
---

# Enum: Capability

Defined in: [`crates/common/src/model.rs:11`](../../../../crates/common/src/model.rs#L11)

能力位。

`JsonMode`（只保证合法 JSON）与 `JsonSchema`（严格 schema 输出）是**两种能力**：
OpenAI 两者都有，DeepSeek Chat 只有前者。请求前必须按能力 fast-fail。

## Variants

### Text

Defined in: [`crates/common/src/model.rs:12`](../../../../crates/common/src/model.rs#L12)


***

### Vision

Defined in: [`crates/common/src/model.rs:13`](../../../../crates/common/src/model.rs#L13)


***

### Audio

Defined in: [`crates/common/src/model.rs:14`](../../../../crates/common/src/model.rs#L14)


***

### File

Defined in: [`crates/common/src/model.rs:15`](../../../../crates/common/src/model.rs#L15)


***

### Tools

Defined in: [`crates/common/src/model.rs:16`](../../../../crates/common/src/model.rs#L16)


***

### JsonMode

Defined in: [`crates/common/src/model.rs:17`](../../../../crates/common/src/model.rs#L17)


***

### JsonSchema

Defined in: [`crates/common/src/model.rs:18`](../../../../crates/common/src/model.rs#L18)


***

### Reasoning

Defined in: [`crates/common/src/model.rs:19`](../../../../crates/common/src/model.rs#L19)


***

### Logprobs

Defined in: [`crates/common/src/model.rs:20`](../../../../crates/common/src/model.rs#L20)


***

### Embedding

Defined in: [`crates/common/src/model.rs:21`](../../../../crates/common/src/model.rs#L21)

## Trait Implementations

- `impl Borrow for Capability`
- `impl BorrowMut for Capability`
- `impl CloneToUninit for Capability`
- `impl Into for Capability`
- `impl From for Capability`
- `impl TryInto for Capability`
- `impl TryFrom for Capability`
- `impl Any for Capability`
- `impl ToOwned for Capability`
- `impl DeserializeOwned for Capability`
- `impl Debug for Capability`
- `impl Clone for Capability`
- `impl Copy for Capability`
- `impl StructuralPartialEq for Capability`
- `impl PartialEq for Capability`
- `impl Eq for Capability`
- `impl Hash for Capability`
- `impl Serialize for Capability`
- `impl Deserialize for Capability`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

