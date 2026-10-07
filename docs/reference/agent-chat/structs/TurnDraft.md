---
id: TurnDraft
title: TurnDraft
---

# Struct: TurnDraft

Defined in: [`crates/features/chat/src/chat.rs:106`](../../../../crates/features/chat/src/chat.rs#L106)

流式累积器：吃一轮 `StreamEvent`，攒出一条 assistant 消息。

- `Delta(Text)` 追加文本；`Delta(Thinking)` 只给 UI 显示，**不进历史**
  （`ContentPart` 无 thinking 变体，`A3` 不为此加）；
- 工具调用按适配器给出的完整 `ToolCall` 收集（分片拼装已在适配器完成，`A2` §4）；
- 只在 `commit_turn` 时入史——中途丢弃即回滚。

_（存在非公开字段）_

## Implementations

### apply()

```rust
pub fn apply(&self, event: &StreamEvent)
```

Defined in: [`crates/features/chat/src/chat.rs:114`](../../../../crates/features/chat/src/chat.rs#L114)

#### Parameters

##### event

`&StreamEvent`


***

### finish_reason()

```rust
pub fn finish_reason(&self) -> Option<FinishReason>
```

Defined in: [`crates/features/chat/src/chat.rs:127`](../../../../crates/features/chat/src/chat.rs#L127)

#### Returns

`Option<FinishReason>`


***

### tool_calls()

```rust
pub fn tool_calls(&self) -> &[ToolCall]
```

Defined in: [`crates/features/chat/src/chat.rs:131`](../../../../crates/features/chat/src/chat.rs#L131)

#### Returns

`&[ToolCall]`


***

### usage()

```rust
pub fn usage(&self) -> Usage
```

Defined in: [`crates/features/chat/src/chat.rs:135`](../../../../crates/features/chat/src/chat.rs#L135)

#### Returns

`Usage`


***

### into_message()

```rust
pub fn into_message(self) -> Message
```

Defined in: [`crates/features/chat/src/chat.rs:140`](../../../../crates/features/chat/src/chat.rs#L140)

攒出的 assistant 消息：文本与 `tool_calls` 并存（`A3` §1）。

#### Returns

`Message`

## Trait Implementations

- `impl Borrow for TurnDraft`
- `impl BorrowMut for TurnDraft`
- `impl CloneToUninit for TurnDraft`
- `impl Into for TurnDraft`
- `impl From for TurnDraft`
- `impl TryInto for TurnDraft`
- `impl TryFrom for TurnDraft`
- `impl Any for TurnDraft`
- `impl ToOwned for TurnDraft`
- `impl CastableFrom for TurnDraft`
- `impl CastableFrom for TurnDraft`
- `impl Read for TurnDraft`
- `impl Instrument for TurnDraft`
- `impl WithSubscriber for TurnDraft`
- `impl PolicyExt for TurnDraft`
- `impl Debug for TurnDraft`
- `impl Clone for TurnDraft`
- `impl Default for TurnDraft`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

