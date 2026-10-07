---
id: ModelResponse
title: ModelResponse
---

# Struct: ModelResponse

Defined in: [`crates/common/src/completion.rs:124`](../../../../crates/common/src/completion.rs#L124)

归一化响应（非流式）。

## Fields

### id

```rust
id: String
```

Defined in: [`crates/common/src/completion.rs:125`](../../../../crates/common/src/completion.rs#L125)


***

### model

```rust
model: String
```

Defined in: [`crates/common/src/completion.rs:126`](../../../../crates/common/src/completion.rs#L126)


***

### text

```rust
text: String
```

Defined in: [`crates/common/src/completion.rs:127`](../../../../crates/common/src/completion.rs#L127)


***

### tool_calls

```rust
tool_calls: Vec<ToolCall>
```

Defined in: [`crates/common/src/completion.rs:129`](../../../../crates/common/src/completion.rs#L129)


***

### finish_reason

```rust
finish_reason: FinishReason
```

Defined in: [`crates/common/src/completion.rs:130`](../../../../crates/common/src/completion.rs#L130)


***

### usage

```rust
usage: Usage
```

Defined in: [`crates/common/src/completion.rs:131`](../../../../crates/common/src/completion.rs#L131)


***

### raw

```rust
raw: Option<Value>
```

Defined in: [`crates/common/src/completion.rs:134`](../../../../crates/common/src/completion.rs#L134)

原始响应体，仅用于排错与 fixtures 回放。

## Trait Implementations

- `impl Borrow for ModelResponse`
- `impl BorrowMut for ModelResponse`
- `impl CloneToUninit for ModelResponse`
- `impl Into for ModelResponse`
- `impl From for ModelResponse`
- `impl TryInto for ModelResponse`
- `impl TryFrom for ModelResponse`
- `impl Any for ModelResponse`
- `impl ToOwned for ModelResponse`
- `impl DeserializeOwned for ModelResponse`
- `impl Debug for ModelResponse`
- `impl Clone for ModelResponse`
- `impl StructuralPartialEq for ModelResponse`
- `impl PartialEq for ModelResponse`
- `impl Serialize for ModelResponse`
- `impl Deserialize for ModelResponse`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

