---
id: ModelRequest
title: ModelRequest
---

# Struct: ModelRequest

Defined in: [`crates/common/src/completion.rs:57`](../../../../crates/common/src/completion.rs#L57)

归一化请求。

## Fields

### model

```rust
model: String
```

Defined in: [`crates/common/src/completion.rs:58`](../../../../crates/common/src/completion.rs#L58)


***

### messages

```rust
messages: Vec<Message>
```

Defined in: [`crates/common/src/completion.rs:59`](../../../../crates/common/src/completion.rs#L59)


***

### temperature

```rust
temperature: Option<f32>
```

Defined in: [`crates/common/src/completion.rs:61`](../../../../crates/common/src/completion.rs#L61)


***

### max_tokens

```rust
max_tokens: Option<u32>
```

Defined in: [`crates/common/src/completion.rs:63`](../../../../crates/common/src/completion.rs#L63)


***

### tools

```rust
tools: Vec<ToolDefinition>
```

Defined in: [`crates/common/src/completion.rs:65`](../../../../crates/common/src/completion.rs#L65)


***

### tool_choice

```rust
tool_choice: Option<ToolChoice>
```

Defined in: [`crates/common/src/completion.rs:67`](../../../../crates/common/src/completion.rs#L67)


***

### response_format

```rust
response_format: Option<ResponseFormat>
```

Defined in: [`crates/common/src/completion.rs:69`](../../../../crates/common/src/completion.rs#L69)


***

### stop

```rust
stop: Vec<String>
```

Defined in: [`crates/common/src/completion.rs:71`](../../../../crates/common/src/completion.rs#L71)


***

### metadata

```rust
metadata: Map<String, Value>
```

Defined in: [`crates/common/src/completion.rs:74`](../../../../crates/common/src/completion.rs#L74)

业务侧附加信息，不发给供应商。


***

### additional_params

```rust
additional_params: Map<String, Value>
```

Defined in: [`crates/common/src/completion.rs:77`](../../../../crates/common/src/completion.rs#L77)

厂商特有参数透传。**唯一**允许出现厂商字段的地方。

## Implementations

### new()

```rust
pub fn new<impl Into<String>: Into>(model: impl ?, messages: Vec<Message>) -> Self
```

Defined in: [`crates/common/src/completion.rs:81`](../../../../crates/common/src/completion.rs#L81)

#### Parameters

##### model

`impl ?`

##### messages

`Vec<Message>`

#### Returns

`Self`

## Trait Implementations

- `impl Borrow for ModelRequest`
- `impl BorrowMut for ModelRequest`
- `impl CloneToUninit for ModelRequest`
- `impl Into for ModelRequest`
- `impl From for ModelRequest`
- `impl TryInto for ModelRequest`
- `impl TryFrom for ModelRequest`
- `impl Any for ModelRequest`
- `impl ToOwned for ModelRequest`
- `impl DeserializeOwned for ModelRequest`
- `impl Debug for ModelRequest`
- `impl Clone for ModelRequest`
- `impl StructuralPartialEq for ModelRequest`
- `impl PartialEq for ModelRequest`
- `impl Serialize for ModelRequest`
- `impl Deserialize for ModelRequest`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

