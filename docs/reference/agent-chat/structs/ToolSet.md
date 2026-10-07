---
id: ToolSet
title: ToolSet
---

# Struct: ToolSet

Defined in: [`crates/features/chat/src/tools.rs:32`](../../../../crates/features/chat/src/tools.rs#L32)

工具注册表：模型看到 `definitions()`，调用走 `get()`。

_（存在非公开字段）_

## Implementations

### new()

```rust
pub fn new() -> Self
```

Defined in: [`crates/features/chat/src/tools.rs:37`](../../../../crates/features/chat/src/tools.rs#L37)

#### Returns

`Self`


***

### with()

```rust
pub fn with(self, tool: Arc<dyn Tool>) -> Self
```

Defined in: [`crates/features/chat/src/tools.rs:42`](../../../../crates/features/chat/src/tools.rs#L42)

追加一个工具（builder 风格）。

#### Parameters

##### tool

`Arc<dyn Tool>`

#### Returns

`Self`


***

### definitions()

```rust
pub fn definitions(&self) -> Vec<ToolDefinition>
```

Defined in: [`crates/features/chat/src/tools.rs:47`](../../../../crates/features/chat/src/tools.rs#L47)

#### Returns

`Vec<ToolDefinition>`


***

### get()

```rust
pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>>
```

Defined in: [`crates/features/chat/src/tools.rs:51`](../../../../crates/features/chat/src/tools.rs#L51)

#### Parameters

##### name

`&str`

#### Returns

`Option<Arc<dyn Tool>>`


***

### is_empty()

```rust
pub fn is_empty(&self) -> bool
```

Defined in: [`crates/features/chat/src/tools.rs:58`](../../../../crates/features/chat/src/tools.rs#L58)

#### Returns

`bool`


***

### len()

```rust
pub fn len(&self) -> usize
```

Defined in: [`crates/features/chat/src/tools.rs:62`](../../../../crates/features/chat/src/tools.rs#L62)

#### Returns

`usize`

## Trait Implementations

- `impl Borrow for ToolSet`
- `impl BorrowMut for ToolSet`
- `impl CloneToUninit for ToolSet`
- `impl Into for ToolSet`
- `impl From for ToolSet`
- `impl TryInto for ToolSet`
- `impl TryFrom for ToolSet`
- `impl Any for ToolSet`
- `impl ToOwned for ToolSet`
- `impl CastableFrom for ToolSet`
- `impl CastableFrom for ToolSet`
- `impl Read for ToolSet`
- `impl Instrument for ToolSet`
- `impl WithSubscriber for ToolSet`
- `impl PolicyExt for ToolSet`
- `impl Default for ToolSet`
- `impl Clone for ToolSet`

## Auto Trait Implementations

`Freeze` `Send` `Sync` `Unpin` `UnsafeUnpin`

