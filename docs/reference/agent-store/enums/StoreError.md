---
id: StoreError
title: StoreError
---

# Enum: StoreError

Defined in: [`crates/kernel/store/src/error.rs:7`](../../../../crates/kernel/store/src/error.rs#L7)

存储层错误。SQL 层错误直接透传（`#[from]`），迁移错误携带版本号便于定位。

## Variants

### Open

```rust
{ .. }
```

Defined in: [`crates/kernel/store/src/error.rs:9`](../../../../crates/kernel/store/src/error.rs#L9)


***

### Sql

```rust
{ .. }
```

Defined in: [`crates/kernel/store/src/error.rs:14`](../../../../crates/kernel/store/src/error.rs#L14)


***

### Migrate

```rust
{ .. }
```

Defined in: [`crates/kernel/store/src/error.rs:19`](../../../../crates/kernel/store/src/error.rs#L19)


***

### Poisoned

Defined in: [`crates/kernel/store/src/error.rs:21`](../../../../crates/kernel/store/src/error.rs#L21)

## Trait Implementations

- `impl Borrow for StoreError`
- `impl BorrowMut for StoreError`
- `impl Into for StoreError`
- `impl From for StoreError`
- `impl TryInto for StoreError`
- `impl TryFrom for StoreError`
- `impl Any for StoreError`
- `impl ToString for StoreError`
- `impl CastableFrom for StoreError`
- `impl CastableFrom for StoreError`
- `impl Read for StoreError`
- `impl Debug for StoreError`
- `impl Error for StoreError`
- `impl Display for StoreError`
- `impl From for StoreError`

## Auto Trait Implementations

`Freeze` `Send` `Sync` `Unpin` `UnsafeUnpin`

