---
id: ToolDefinition
title: ToolDefinition
---

# Struct: ToolDefinition

Defined in: [`crates/common/src/completion.rs:11`](../../../../crates/common/src/completion.rs#L11)

工具定义。`parameters` 是 JSON Schema，三家的字段名不同，
由适配器在边界处翻译（OpenAI `function.parameters` / Anthropic `input_schema`）。

## Fields

### name

```rust
name: String
```

Defined in: [`crates/common/src/completion.rs:12`](../../../../crates/common/src/completion.rs#L12)


***

### description

```rust
description: String
```

Defined in: [`crates/common/src/completion.rs:13`](../../../../crates/common/src/completion.rs#L13)


***

### parameters

```rust
parameters: Value
```

Defined in: [`crates/common/src/completion.rs:14`](../../../../crates/common/src/completion.rs#L14)

## Trait Implementations

- `impl Borrow for ToolDefinition`
- `impl BorrowMut for ToolDefinition`
- `impl CloneToUninit for ToolDefinition`
- `impl Into for ToolDefinition`
- `impl From for ToolDefinition`
- `impl TryInto for ToolDefinition`
- `impl TryFrom for ToolDefinition`
- `impl Any for ToolDefinition`
- `impl ToOwned for ToolDefinition`
- `impl DeserializeOwned for ToolDefinition`
- `impl Debug for ToolDefinition`
- `impl Clone for ToolDefinition`
- `impl StructuralPartialEq for ToolDefinition`
- `impl PartialEq for ToolDefinition`
- `impl Serialize for ToolDefinition`
- `impl Deserialize for ToolDefinition`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

