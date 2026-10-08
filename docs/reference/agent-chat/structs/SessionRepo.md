---
id: SessionRepo
title: SessionRepo
---

# Struct: SessionRepo

Defined in: [`crates/features/chat/src/repo.rs:65`](../../../../crates/features/chat/src/repo.rs#L65)

会话仓储：连接来自 `agent-store`，表结构与 SQL 归本模块。

_（存在非公开字段）_

## Implementations

### open()

```rust
pub fn open(store: Store) -> Result<Self, RepoError>
```

Defined in: [`crates/features/chat/src/repo.rs:71`](../../../../crates/features/chat/src/repo.rs#L71)

迁移到最新表结构并返回仓储（重复调用幂等）。

#### Parameters

##### store

`Store`

#### Returns

`Result<Self, RepoError>`


***

### store()

```rust
pub fn store(&self) -> &Store
```

Defined in: [`crates/features/chat/src/repo.rs:76`](../../../../crates/features/chat/src/repo.rs#L76)

#### Returns

`&Store`


***

### save()

```rust
pub fn save(&self, id: &str, title: &str, chat: &Chat, now_ms: i64) -> Result<(), RepoError>
```

Defined in: [`crates/features/chat/src/repo.rs:81`](../../../../crates/features/chat/src/repo.rs#L81)

新建或覆盖一个会话（按 `id` upsert）；`created_at` 首写后不变。

#### Parameters

##### id

`&str`

##### title

`&str`

##### chat

`&Chat`

##### now_ms

`i64`

#### Returns

`Result<(), RepoError>`


***

### load()

```rust
pub fn load(&self, id: &str) -> Result<Option<Chat>, RepoError>
```

Defined in: [`crates/features/chat/src/repo.rs:100`](../../../../crates/features/chat/src/repo.rs#L100)

取回一个会话的完整状态；不存在返回 `None`。

#### Parameters

##### id

`&str`

#### Returns

`Result<Option<Chat>, RepoError>`


***

### list()

```rust
pub fn list(&self) -> Result<Vec<SessionMeta>, RepoError>
```

Defined in: [`crates/features/chat/src/repo.rs:117`](../../../../crates/features/chat/src/repo.rs#L117)

会话列表，按最近更新倒序。

#### Returns

`Result<Vec<SessionMeta>, RepoError>`


***

### rename()

```rust
pub fn rename(&self, id: &str, title: &str, now_ms: i64) -> Result<bool, RepoError>
```

Defined in: [`crates/features/chat/src/repo.rs:141`](../../../../crates/features/chat/src/repo.rs#L141)

改名；返回是否命中了一个已存在的会话。

#### Parameters

##### id

`&str`

##### title

`&str`

##### now_ms

`i64`

#### Returns

`Result<bool, RepoError>`


***

### delete()

```rust
pub fn delete(&self, id: &str) -> Result<bool, RepoError>
```

Defined in: [`crates/features/chat/src/repo.rs:152`](../../../../crates/features/chat/src/repo.rs#L152)

删除；返回是否命中了一个已存在的会话。

#### Parameters

##### id

`&str`

#### Returns

`Result<bool, RepoError>`

## Trait Implementations

- `impl Borrow for SessionRepo`
- `impl BorrowMut for SessionRepo`
- `impl CloneToUninit for SessionRepo`
- `impl Into for SessionRepo`
- `impl From for SessionRepo`
- `impl TryInto for SessionRepo`
- `impl TryFrom for SessionRepo`
- `impl Any for SessionRepo`
- `impl ToOwned for SessionRepo`
- `impl CastableFrom for SessionRepo`
- `impl CastableFrom for SessionRepo`
- `impl Read for SessionRepo`
- `impl Instrument for SessionRepo`
- `impl WithSubscriber for SessionRepo`
- `impl PolicyExt for SessionRepo`
- `impl IntoEither for SessionRepo`
- `impl Clone for SessionRepo`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

