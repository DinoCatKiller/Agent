---
id: ToolCall
title: ToolCall
---

# Struct: ToolCall

Defined in: [`crates/common/src/completion.rs:23`](../../../../crates/common/src/completion.rs#L23)

归一化后的工具调用。

- `arguments` 统一为**结构化 JSON**。OpenAI 侧返回的是 JSON 字符串（可能非法），
  反序列化与容错由适配器负责，Core 只看结构化结果。
- `provider_id` 保留供应商原发 id（如 OpenAI Responses 的 `item_id`），回传时可能用到。

## Fields

### id

```rust
id: String
```

Defined in: [`crates/common/src/completion.rs:24`](../../../../crates/common/src/completion.rs#L24)


***

### name

```rust
name: String
```

Defined in: [`crates/common/src/completion.rs:25`](../../../../crates/common/src/completion.rs#L25)


***

### arguments

```rust
arguments: Value
```

Defined in: [`crates/common/src/completion.rs:26`](../../../../crates/common/src/completion.rs#L26)


***

### provider_id

```rust
provider_id: Option<String>
```

Defined in: [`crates/common/src/completion.rs:28`](../../../../crates/common/src/completion.rs#L28)

## Trait Implementations

- `impl Borrow for ToolCall`
- `impl BorrowMut for ToolCall`
- `impl CloneToUninit for ToolCall`
- `impl Into for ToolCall`
- `impl From for ToolCall`
- `impl TryInto for ToolCall`
- `impl TryFrom for ToolCall`
- `impl Any for ToolCall`
- `impl ToOwned for ToolCall`
- `impl DeserializeOwned for ToolCall`
- `impl Debug for ToolCall`
- `impl Clone for ToolCall`
- `impl StructuralPartialEq for ToolCall`
- `impl PartialEq for ToolCall`
- `impl Serialize for ToolCall`
- `impl Deserialize for ToolCall`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

