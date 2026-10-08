---
id: Message
title: Message
---

# Struct: Message

Defined in: [`crates/common/src/message.rs:56`](../../../../crates/common/src/message.rs#L56)

归一化消息。

- `role = Tool` 时 `tool_call_id` 必填（回填到哪个 `tool_call`）。
- `role = Assistant` 且发起工具调用时，`tool_calls` 非空，`content` 可为空。

## Fields

### role

```rust
role: Role
```

Defined in: [`crates/common/src/message.rs:57`](../../../../crates/common/src/message.rs#L57)


***

### content

```rust
content: Vec<ContentPart>
```

Defined in: [`crates/common/src/message.rs:59`](../../../../crates/common/src/message.rs#L59)


***

### tool_call_id

```rust
tool_call_id: Option<String>
```

Defined in: [`crates/common/src/message.rs:61`](../../../../crates/common/src/message.rs#L61)


***

### tool_calls

```rust
tool_calls: Vec<ToolCall>
```

Defined in: [`crates/common/src/message.rs:63`](../../../../crates/common/src/message.rs#L63)

## Implementations

### system()

```rust
pub fn system<impl Into<String>: Into>(text: impl ?) -> Self
```

Defined in: [`crates/common/src/message.rs:67`](../../../../crates/common/src/message.rs#L67)

#### Parameters

##### text

`impl ?`

#### Returns

`Self`


***

### user()

```rust
pub fn user<impl Into<String>: Into>(text: impl ?) -> Self
```

Defined in: [`crates/common/src/message.rs:71`](../../../../crates/common/src/message.rs#L71)

#### Parameters

##### text

`impl ?`

#### Returns

`Self`


***

### assistant()

```rust
pub fn assistant<impl Into<String>: Into>(text: impl ?) -> Self
```

Defined in: [`crates/common/src/message.rs:75`](../../../../crates/common/src/message.rs#L75)

#### Parameters

##### text

`impl ?`

#### Returns

`Self`


***

### tool_result()

```rust
pub fn tool_result<impl Into<String>: Into, impl Into<String>: Into>(tool_call_id: impl ?, content: impl ?) -> Self
```

Defined in: [`crates/common/src/message.rs:80`](../../../../crates/common/src/message.rs#L80)

工具结果回填（`role = Tool`，必须带 `tool_call_id`）。

#### Parameters

##### tool_call_id

`impl ?`

##### content

`impl ?`

#### Returns

`Self`


***

### text()

```rust
pub fn text(&self) -> String
```

Defined in: [`crates/common/src/message.rs:99`](../../../../crates/common/src/message.rs#L99)

拼接全部 `Text` 块。用于日志、UI 预览与 token 估算，**不要**用于重新序列化。

#### Returns

`String`

## Trait Implementations

- `impl Borrow for Message`
- `impl BorrowMut for Message`
- `impl CloneToUninit for Message`
- `impl Into for Message`
- `impl From for Message`
- `impl TryInto for Message`
- `impl TryFrom for Message`
- `impl Any for Message`
- `impl ToOwned for Message`
- `impl DeserializeOwned for Message`
- `impl Debug for Message`
- `impl Clone for Message`
- `impl StructuralPartialEq for Message`
- `impl PartialEq for Message`
- `impl Serialize for Message`
- `impl Deserialize for Message`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

