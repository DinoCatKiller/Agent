---
id: Chat
title: Chat
---

# Struct: Chat

Defined in: [`crates/features/chat/src/chat.rs:30`](../../../../crates/features/chat/src/chat.rs#L30)

一次会话的完整状态。

- `messages` 以 `System`（可选）开头，之后是 User / Assistant / Tool 的完整历史；
- 取消 / 出错的轮次**不入史**（增量只存在于 [`TurnDraft`](TurnDraft.md)，丢弃即回滚）。

## Fields

### model

```rust
model: String
```

Defined in: [`crates/features/chat/src/chat.rs:31`](../../../../crates/features/chat/src/chat.rs#L31)


***

### settings

```rust
settings: ChatSettings
```

Defined in: [`crates/features/chat/src/chat.rs:33`](../../../../crates/features/chat/src/chat.rs#L33)


***

### messages

```rust
messages: Vec<Message>
```

Defined in: [`crates/features/chat/src/chat.rs:35`](../../../../crates/features/chat/src/chat.rs#L35)


***

### total_usage

```rust
total_usage: Usage
```

Defined in: [`crates/features/chat/src/chat.rs:38`](../../../../crates/features/chat/src/chat.rs#L38)

会话累计用量（各轮 `Usage` 事件之和）。

## Implementations

### new()

```rust
pub fn new<impl Into<String>: Into>(model: impl ?) -> Self
```

Defined in: [`crates/features/chat/src/chat.rs:42`](../../../../crates/features/chat/src/chat.rs#L42)

#### Parameters

##### model

`impl ?`

#### Returns

`Self`


***

### with_system()

```rust
pub fn with_system<impl Into<String>: Into, impl Into<String>: Into>(model: impl ?, system: impl ?) -> Self
```

Defined in: [`crates/features/chat/src/chat.rs:50`](../../../../crates/features/chat/src/chat.rs#L50)

带 system 提示的会话（system 必须是最前一条，`A3` §1）。

#### Parameters

##### model

`impl ?`

##### system

`impl ?`

#### Returns

`Self`


***

### push_user()

```rust
pub fn push_user<impl Into<String>: Into>(&self, text: impl ?)
```

Defined in: [`crates/features/chat/src/chat.rs:56`](../../../../crates/features/chat/src/chat.rs#L56)

#### Parameters

##### text

`impl ?`


***

### push_tool_result()

```rust
pub fn push_tool_result<impl Into<String>: Into, impl Into<String>: Into>(&self, tool_call_id: impl ?, content: impl ?)
```

Defined in: [`crates/features/chat/src/chat.rs:60`](../../../../crates/features/chat/src/chat.rs#L60)

#### Parameters

##### tool_call_id

`impl ?`

##### content

`impl ?`


***

### commit_turn()

```rust
pub fn commit_turn(&self, draft: TurnDraft)
```

Defined in: [`crates/features/chat/src/chat.rs:70`](../../../../crates/features/chat/src/chat.rs#L70)

提交一轮已完结（`End` 之后）的模型输出，并累计用量。

#### Parameters

##### draft

[`TurnDraft`](TurnDraft.md)


***

### request()

```rust
pub fn request(&self, tools: &[ToolDefinition]) -> Result<ModelRequest, ProviderError>
```

Defined in: [`crates/features/chat/src/chat.rs:81`](../../../../crates/features/chat/src/chat.rs#L81)

构造下一次模型请求。

含 `A3` 落地清单的运行期校验：`role = Tool` 而缺 `tool_call_id`
直接 `InvalidRequest`（本阶段在 Core 加，M4）。

#### Parameters

##### tools

`&[ToolDefinition]`

#### Returns

`Result<ModelRequest, ProviderError>`

## Trait Implementations

- `impl Borrow for Chat`
- `impl BorrowMut for Chat`
- `impl CloneToUninit for Chat`
- `impl Into for Chat`
- `impl From for Chat`
- `impl TryInto for Chat`
- `impl TryFrom for Chat`
- `impl Any for Chat`
- `impl ToOwned for Chat`
- `impl DeserializeOwned for Chat`
- `impl CastableFrom for Chat`
- `impl CastableFrom for Chat`
- `impl Read for Chat`
- `impl Instrument for Chat`
- `impl WithSubscriber for Chat`
- `impl PolicyExt for Chat`
- `impl IntoEither for Chat`
- `impl Debug for Chat`
- `impl Clone for Chat`
- `impl Default for Chat`
- `impl Serialize for Chat`
- `impl Deserialize for Chat`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

