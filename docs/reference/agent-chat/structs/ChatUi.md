---
id: ChatUi
title: ChatUi
---

# Struct: ChatUi

Defined in: [`crates/features/chat/src/ui.rs:171`](../../../../crates/features/chat/src/ui.rs#L171)

对话 UI 状态机。

生命周期由宿主驱动：submit 前 [`ChatUi::push_user`](ChatUi.md) → worker 转发 [`LoopEvent`](../enums/LoopEvent.md) 给
[`ChatUi::on_event`](ChatUi.md)；切会话时 [`ChatUi::load_chat`](ChatUi.md) 整体替换。

回滚语义与编排层一致（`README` 铁律 1）：[`LoopEvent::ModelTurnEnded`](../enums/LoopEvent.md) 是提交边界，
[`LoopEvent::Cancelled`](../enums/LoopEvent.md) / [`LoopEvent::Error`](../enums/LoopEvent.md) 把条目截回边界（未提交增量即丢弃）。

_（存在非公开字段）_

## Implementations

### new()

```rust
pub fn new() -> Self
```

Defined in: [`crates/features/chat/src/ui.rs:191`](../../../../crates/features/chat/src/ui.rs#L191)

#### Returns

`Self`


***

### generating()

```rust
pub fn generating(&self) -> bool
```

Defined in: [`crates/features/chat/src/ui.rs:195`](../../../../crates/features/chat/src/ui.rs#L195)

#### Returns

`bool`


***

### notice()

```rust
pub fn notice(&self) -> Option<&str>
```

Defined in: [`crates/features/chat/src/ui.rs:199`](../../../../crates/features/chat/src/ui.rs#L199)

#### Returns

`Option<&str>`


***

### total_usage()

```rust
pub fn total_usage(&self) -> Usage
```

Defined in: [`crates/features/chat/src/ui.rs:203`](../../../../crates/features/chat/src/ui.rs#L203)

#### Returns

`Usage`


***

### push_user()

```rust
pub fn push_user(&self, text: &str)
```

Defined in: [`crates/features/chat/src/ui.rs:209`](../../../../crates/features/chat/src/ui.rs#L209)

用户提交一行（宿主在 submit 时调用；编排层随后也会把它写进历史）。
同时推进提交边界：取消只丢弃模型输出，不丢用户输入（与 REPL 语义一致）。

#### Parameters

##### text

`&str`


***

### on_event()

```rust
pub fn on_event(&self, event: LoopEvent)
```

Defined in: [`crates/features/chat/src/ui.rs:215`](../../../../crates/features/chat/src/ui.rs#L215)

消费一个编排事件（直播）。

#### Parameters

##### event

[`LoopEvent`](../enums/LoopEvent.md)


***

### rejected()

```rust
pub fn rejected(&self, message: String)
```

Defined in: [`crates/features/chat/src/ui.rs:298`](../../../../crates/features/chat/src/ui.rs#L298)

`run_round` 同步 fast-fail（能力协商 / 校验拒绝）。用户输入已入史，提示后可继续。

#### Parameters

##### message

`String`


***

### load_chat()

```rust
pub fn load_chat(&self, chat: &Chat)
```

Defined in: [`crates/features/chat/src/ui.rs:304`](../../../../crates/features/chat/src/ui.rs#L304)

用已落盘的会话整体替换显示（切会话 / 启动恢复）。

#### Parameters

##### chat

`&Chat`


***

### reset()

```rust
pub fn reset(&self)
```

Defined in: [`crates/features/chat/src/ui.rs:334`](../../../../crates/features/chat/src/ui.rs#L334)

新会话：清空显示（worker 侧同步重置 `Chat`）。


***

### set_notice()

```rust
pub fn set_notice(&self, message: String)
```

Defined in: [`crates/features/chat/src/ui.rs:338`](../../../../crates/features/chat/src/ui.rs#L338)

#### Parameters

##### message

`String`


***

### input()

```rust
pub fn input(&self) -> &str
```

Defined in: [`crates/features/chat/src/ui.rs:359`](../../../../crates/features/chat/src/ui.rs#L359)

#### Returns

`&str`


***

### take_input()

```rust
pub fn take_input(&self) -> String
```

Defined in: [`crates/features/chat/src/ui.rs:364`](../../../../crates/features/chat/src/ui.rs#L364)

取走输入（发送时调用）。

#### Returns

`String`


***

### insert()

```rust
pub fn insert(&self, c: char)
```

Defined in: [`crates/features/chat/src/ui.rs:369`](../../../../crates/features/chat/src/ui.rs#L369)

#### Parameters

##### c

`char`


***

### backspace()

```rust
pub fn backspace(&self)
```

Defined in: [`crates/features/chat/src/ui.rs:375`](../../../../crates/features/chat/src/ui.rs#L375)


***

### delete()

```rust
pub fn delete(&self)
```

Defined in: [`crates/features/chat/src/ui.rs:384`](../../../../crates/features/chat/src/ui.rs#L384)


***

### left()

```rust
pub fn left(&self)
```

Defined in: [`crates/features/chat/src/ui.rs:391`](../../../../crates/features/chat/src/ui.rs#L391)


***

### right()

```rust
pub fn right(&self)
```

Defined in: [`crates/features/chat/src/ui.rs:395`](../../../../crates/features/chat/src/ui.rs#L395)


***

### home()

```rust
pub fn home(&self)
```

Defined in: [`crates/features/chat/src/ui.rs:400`](../../../../crates/features/chat/src/ui.rs#L400)


***

### end()

```rust
pub fn end(&self)
```

Defined in: [`crates/features/chat/src/ui.rs:404`](../../../../crates/features/chat/src/ui.rs#L404)


***

### scroll_up()

```rust
pub fn scroll_up(&self, lines: u16)
```

Defined in: [`crates/features/chat/src/ui.rs:418`](../../../../crates/features/chat/src/ui.rs#L418)

#### Parameters

##### lines

`u16`


***

### scroll_down()

```rust
pub fn scroll_down(&self, lines: u16)
```

Defined in: [`crates/features/chat/src/ui.rs:423`](../../../../crates/features/chat/src/ui.rs#L423)

下滚到底后回到跟随模式。

#### Parameters

##### lines

`u16`

## Trait Implementations

- `impl Borrow for ChatUi`
- `impl BorrowMut for ChatUi`
- `impl Into for ChatUi`
- `impl From for ChatUi`
- `impl TryInto for ChatUi`
- `impl TryFrom for ChatUi`
- `impl Any for ChatUi`
- `impl CastableFrom for ChatUi`
- `impl CastableFrom for ChatUi`
- `impl Read for ChatUi`
- `impl Instrument for ChatUi`
- `impl WithSubscriber for ChatUi`
- `impl PolicyExt for ChatUi`
- `impl IntoEither for ChatUi`
- `impl Debug for ChatUi`
- `impl Default for ChatUi`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

