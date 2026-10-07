---
id: SessionPane
title: SessionPane
---

# Struct: SessionPane

Defined in: [`crates/features/sessions/src/ui.rs:20`](../../../../crates/features/sessions/src/ui.rs#L20)

会话侧栏状态：列表 + 选中下标（纯数据，无 ratatui 状态）。

_（存在非公开字段）_

## Implementations

### new()

```rust
pub fn new() -> Self
```

Defined in: [`crates/features/sessions/src/ui.rs:27`](../../../../crates/features/sessions/src/ui.rs#L27)

#### Returns

`Self`


***

### set_focused()

```rust
pub fn set_focused(&self, focused: bool)
```

Defined in: [`crates/features/sessions/src/ui.rs:31`](../../../../crates/features/sessions/src/ui.rs#L31)

#### Parameters

##### focused

`bool`


***

### set_sessions()

```rust
pub fn set_sessions(&self, items: Vec<SessionMeta>)
```

Defined in: [`crates/features/sessions/src/ui.rs:36`](../../../../crates/features/sessions/src/ui.rs#L36)

整体替换列表（worker 每次落盘后刷新）；保持选中项尽量不动。

#### Parameters

##### items

`Vec<SessionMeta>`


***

### next()

```rust
pub fn next(&self)
```

Defined in: [`crates/features/sessions/src/ui.rs:41`](../../../../crates/features/sessions/src/ui.rs#L41)


***

### previous()

```rust
pub fn previous(&self)
```

Defined in: [`crates/features/sessions/src/ui.rs:47`](../../../../crates/features/sessions/src/ui.rs#L47)


***

### selected()

```rust
pub fn selected(&self) -> Option<&SessionMeta>
```

Defined in: [`crates/features/sessions/src/ui.rs:51`](../../../../crates/features/sessions/src/ui.rs#L51)

#### Returns

`Option<&SessionMeta>`


***

### len()

```rust
pub fn len(&self) -> usize
```

Defined in: [`crates/features/sessions/src/ui.rs:55`](../../../../crates/features/sessions/src/ui.rs#L55)

#### Returns

`usize`


***

### is_empty()

```rust
pub fn is_empty(&self) -> bool
```

Defined in: [`crates/features/sessions/src/ui.rs:59`](../../../../crates/features/sessions/src/ui.rs#L59)

#### Returns

`bool`


***

### render()

```rust
pub fn render(&self, f: &Frame<'_>, area: Rect, theme: &Theme)
```

Defined in: [`crates/features/sessions/src/ui.rs:63`](../../../../crates/features/sessions/src/ui.rs#L63)

#### Parameters

##### f

`&Frame<'_>`

##### area

`Rect`

##### theme

`&Theme`

## Trait Implementations

- `impl Borrow for SessionPane`
- `impl BorrowMut for SessionPane`
- `impl Into for SessionPane`
- `impl From for SessionPane`
- `impl TryInto for SessionPane`
- `impl TryFrom for SessionPane`
- `impl Any for SessionPane`
- `impl IntoEither for SessionPane`
- `impl CastableFrom for SessionPane`
- `impl CastableFrom for SessionPane`
- `impl Read for SessionPane`
- `impl Instrument for SessionPane`
- `impl WithSubscriber for SessionPane`
- `impl PolicyExt for SessionPane`
- `impl Debug for SessionPane`
- `impl Default for SessionPane`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

