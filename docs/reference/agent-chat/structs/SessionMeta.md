---
id: SessionMeta
title: SessionMeta
---

# Struct: SessionMeta

Defined in: [`crates/features/chat/src/repo.rs:22`](../../../../crates/features/chat/src/repo.rs#L22)

会话元数据（列表用，不含消息体）。

## Fields

### id

```rust
id: String
```

Defined in: [`crates/features/chat/src/repo.rs:23`](../../../../crates/features/chat/src/repo.rs#L23)


***

### title

```rust
title: String
```

Defined in: [`crates/features/chat/src/repo.rs:24`](../../../../crates/features/chat/src/repo.rs#L24)


***

### model

```rust
model: String
```

Defined in: [`crates/features/chat/src/repo.rs:25`](../../../../crates/features/chat/src/repo.rs#L25)


***

### created_at

```rust
created_at: i64
```

Defined in: [`crates/features/chat/src/repo.rs:27`](../../../../crates/features/chat/src/repo.rs#L27)

Unix 毫秒。


***

### updated_at

```rust
updated_at: i64
```

Defined in: [`crates/features/chat/src/repo.rs:29`](../../../../crates/features/chat/src/repo.rs#L29)

Unix 毫秒；列表按它倒序。

## Trait Implementations

- `impl Borrow for SessionMeta`
- `impl BorrowMut for SessionMeta`
- `impl CloneToUninit for SessionMeta`
- `impl Into for SessionMeta`
- `impl From for SessionMeta`
- `impl TryInto for SessionMeta`
- `impl TryFrom for SessionMeta`
- `impl Any for SessionMeta`
- `impl ToOwned for SessionMeta`
- `impl DeserializeOwned for SessionMeta`
- `impl Equivalent for SessionMeta`
- `impl CastableFrom for SessionMeta`
- `impl CastableFrom for SessionMeta`
- `impl Read for SessionMeta`
- `impl Instrument for SessionMeta`
- `impl WithSubscriber for SessionMeta`
- `impl PolicyExt for SessionMeta`
- `impl Debug for SessionMeta`
- `impl Clone for SessionMeta`
- `impl StructuralPartialEq for SessionMeta`
- `impl PartialEq for SessionMeta`
- `impl Eq for SessionMeta`
- `impl Serialize for SessionMeta`
- `impl Deserialize for SessionMeta`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

