---
id: ToolChoice
title: ToolChoice
---

# Enum: ToolChoice

Defined in: [`crates/common/src/completion.rs:34`](../../../../crates/common/src/completion.rs#L34)

工具选择。OpenAI 的 `required` 与 Anthropic 的 `any` 都归一为 `Required`。

## Variants

### Auto

Defined in: [`crates/common/src/completion.rs:35`](../../../../crates/common/src/completion.rs#L35)


***

### None

Defined in: [`crates/common/src/completion.rs:36`](../../../../crates/common/src/completion.rs#L36)


***

### Required

Defined in: [`crates/common/src/completion.rs:37`](../../../../crates/common/src/completion.rs#L37)


***

### Specific

```rust
(Vec<String>)
```

Defined in: [`crates/common/src/completion.rs:38`](../../../../crates/common/src/completion.rs#L38)

## Trait Implementations

- `impl Borrow for ToolChoice`
- `impl BorrowMut for ToolChoice`
- `impl CloneToUninit for ToolChoice`
- `impl Into for ToolChoice`
- `impl From for ToolChoice`
- `impl TryInto for ToolChoice`
- `impl TryFrom for ToolChoice`
- `impl Any for ToolChoice`
- `impl ToOwned for ToolChoice`
- `impl DeserializeOwned for ToolChoice`
- `impl Debug for ToolChoice`
- `impl Clone for ToolChoice`
- `impl StructuralPartialEq for ToolChoice`
- `impl PartialEq for ToolChoice`
- `impl Serialize for ToolChoice`
- `impl Deserialize for ToolChoice`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

