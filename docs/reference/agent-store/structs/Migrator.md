---
id: Migrator
title: Migrator
---

# Struct: Migrator

Defined in: [`crates/kernel/store/src/migrate.rs:10`](../../../../crates/kernel/store/src/migrate.rs#L10)

迁移列表。每个条目 = 目标版本号 + 该版本的 SQL 脚本（按序执行）。
版本号从 1 起严格递增；`apply` 从 `user_version + 1` 执行到最新版。

_（存在非公开字段）_

## Implementations

### new()

```rust
pub fn new() -> Self
```

Defined in: [`crates/kernel/store/src/migrate.rs:15`](../../../../crates/kernel/store/src/migrate.rs#L15)

#### Returns

`Self`


***

### add()

```rust
pub fn add(&self, version: i32, sql: &[&'static str])
```

Defined in: [`crates/kernel/store/src/migrate.rs:20`](../../../../crates/kernel/store/src/migrate.rs#L20)

追加一个版本的迁移脚本。

#### Parameters

##### version

`i32`

##### sql

`&[&'static str]`


***

### target()

```rust
pub fn target(&self) -> i32
```

Defined in: [`crates/kernel/store/src/migrate.rs:25`](../../../../crates/kernel/store/src/migrate.rs#L25)

#### Returns

`i32`


***

### apply()

```rust
pub fn apply(&self, conn: &Connection) -> Result<(), StoreError>
```

Defined in: [`crates/kernel/store/src/migrate.rs:30`](../../../../crates/kernel/store/src/migrate.rs#L30)

把连接迁移到最新版本。每版本一个事务：失败回滚，`user_version` 保持原值。

#### Parameters

##### conn

`&Connection`

#### Returns

`Result<(), StoreError>`

## Trait Implementations

- `impl Borrow for Migrator`
- `impl BorrowMut for Migrator`
- `impl Into for Migrator`
- `impl From for Migrator`
- `impl TryInto for Migrator`
- `impl TryFrom for Migrator`
- `impl Any for Migrator`
- `impl CastableFrom for Migrator`
- `impl CastableFrom for Migrator`
- `impl Read for Migrator`
- `impl Debug for Migrator`
- `impl Default for Migrator`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

