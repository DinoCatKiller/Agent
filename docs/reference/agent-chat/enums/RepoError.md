---
id: RepoError
title: RepoError
---

# Enum: RepoError

Defined in: [`crates/features/chat/src/repo.rs:36`](../../../../crates/features/chat/src/repo.rs#L36)

持久化错误：存储层错误 + 会话 JSON 的（反）序列化错误。

JSON 是**业务数据**的编码，所以它在这一层，而不是 `kernel/store`（`D6` §1）。

## Variants

### Store

```rust
(StoreError)
```

Defined in: [`crates/features/chat/src/repo.rs:38`](../../../../crates/features/chat/src/repo.rs#L38)


***

### Json

```rust
(Error)
```

Defined in: [`crates/features/chat/src/repo.rs:40`](../../../../crates/features/chat/src/repo.rs#L40)

## Trait Implementations

- `impl Borrow for RepoError`
- `impl BorrowMut for RepoError`
- `impl Into for RepoError`
- `impl From for RepoError`
- `impl TryInto for RepoError`
- `impl TryFrom for RepoError`
- `impl Any for RepoError`
- `impl ToString for RepoError`
- `impl CastableFrom for RepoError`
- `impl CastableFrom for RepoError`
- `impl Read for RepoError`
- `impl Instrument for RepoError`
- `impl WithSubscriber for RepoError`
- `impl PolicyExt for RepoError`
- `impl ToLine for RepoError`
- `impl ToSpan for RepoError`
- `impl ToText for RepoError`
- `impl ToCompactString for RepoError`
- `impl IntoEither for RepoError`
- `impl Debug for RepoError`
- `impl Error for RepoError`
- `impl Display for RepoError`
- `impl From for RepoError`
- `impl From for RepoError`

## Auto Trait Implementations

`Freeze` `Send` `Sync` `Unpin` `UnsafeUnpin`

