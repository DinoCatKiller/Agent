---
id: Store
title: Store
---

# Struct: Store

Defined in: [`crates/kernel/store/src/store.rs:16`](../../../../crates/kernel/store/src/store.rs#L16)

本地存储：单连接（`Arc<Mutex<Connection>>`），同步 API，无连接池（`X1` §1）。
`Clone` 只增引用计数，不复制连接。

_（存在非公开字段）_

## Implementations

### open()

```rust
pub fn open<impl Into<PathBuf>: Into>(path: impl ?) -> Result<Self, StoreError>
```

Defined in: [`crates/kernel/store/src/store.rs:23`](../../../../crates/kernel/store/src/store.rs#L23)

打开（或创建）指定路径的数据库，并应用基础 PRAGMA。

#### Parameters

##### path

`impl ?`

#### Returns

`Result<Self, StoreError>`


***

### open_in_memory()

```rust
pub fn open_in_memory() -> Result<Self, StoreError>
```

Defined in: [`crates/kernel/store/src/store.rs:33`](../../../../crates/kernel/store/src/store.rs#L33)

打开内存库（测试用，连接关闭即销毁）。

#### Returns

`Result<Self, StoreError>`


***

### path()

```rust
pub fn path(&self) -> &Path
```

Defined in: [`crates/kernel/store/src/store.rs:48`](../../../../crates/kernel/store/src/store.rs#L48)

#### Returns

`&Path`


***

### migrate()

```rust
pub fn migrate(&self, migrator: &Migrator) -> Result<(), StoreError>
```

Defined in: [`crates/kernel/store/src/store.rs:53`](../../../../crates/kernel/store/src/store.rs#L53)

执行版本化迁移到 `Migrator` 的最新版本（重复调用是幂等的）。

#### Parameters

##### migrator

`&Migrator`

#### Returns

`Result<(), StoreError>`


***

### tx()

```rust
pub fn tx(&self) -> Result<Tx<'_>, StoreError>
```

Defined in: [`crates/kernel/store/src/store.rs:59`](../../../../crates/kernel/store/src/store.rs#L59)

开启一个事务（持锁直到 commit / rollback / Drop）。

#### Returns

`Result<Tx<'_>, StoreError>`


***

### execute()

```rust
pub fn execute(&self, sql: &str) -> Result<usize, StoreError>
```

Defined in: [`crates/kernel/store/src/store.rs:64`](../../../../crates/kernel/store/src/store.rs#L64)

便捷写语句（业务查询走 `features/chat` 的 `repo.rs`）。

#### Parameters

##### sql

`&str`

#### Returns

`Result<usize, StoreError>`


***

### query_single()

```rust
pub fn query_single<T>(&self, sql: &str) -> Result<T, StoreError>
where
    T: FromSql
```

Defined in: [`crates/kernel/store/src/store.rs:69`](../../../../crates/kernel/store/src/store.rs#L69)

便捷查单值。

#### Parameters

##### sql

`&str`

#### Returns

`Result<T, StoreError>`


***

### with_conn()

```rust
pub fn with_conn<T, impl FnOnce(&Connection) -> Result<T, StoreError>: FnOnce(&Connection) -> Result<T, StoreError>>(&self, f: impl ?) -> Result<T, StoreError>
```

Defined in: [`crates/kernel/store/src/store.rs:80`](../../../../crates/kernel/store/src/store.rs#L80)

在锁内拿到连接，执行任意读写（含参数化 SQL）。

这是给上层写业务查询的**逃生舱**（机制提供连接，语义自己写 SQL，`D6` §1）；
`&Connection` 仅在本闭包内有效，闭包结束即释放锁。

#### Parameters

##### f

`impl ?`

#### Returns

`Result<T, StoreError>`

## Trait Implementations

- `impl Borrow for Store`
- `impl BorrowMut for Store`
- `impl CloneToUninit for Store`
- `impl Into for Store`
- `impl From for Store`
- `impl TryInto for Store`
- `impl TryFrom for Store`
- `impl Any for Store`
- `impl ToOwned for Store`
- `impl CastableFrom for Store`
- `impl CastableFrom for Store`
- `impl Read for Store`
- `impl Clone for Store`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

