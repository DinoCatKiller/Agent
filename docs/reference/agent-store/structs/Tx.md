---
id: Tx
title: Tx
---

# Struct: Tx

Defined in: [`crates/kernel/store/src/tx.rs:11`](../../../../crates/kernel/store/src/tx.rs#L11)

一个进行中的事务，持锁直到结束。
未显式 `commit` / `rollback` 即被 Drop 时，自动回滚（防悬挂事务）。

_（存在非公开字段）_

## Implementations

### conn()

```rust
pub fn conn(&self) -> &Connection
```

Defined in: [`crates/kernel/store/src/tx.rs:26`](../../../../crates/kernel/store/src/tx.rs#L26)

事务内的连接（可执行任意 SQL）。

#### Returns

`&Connection`


***

### commit()

```rust
pub fn commit(self) -> Result<(), StoreError>
```

Defined in: [`crates/kernel/store/src/tx.rs:30`](../../../../crates/kernel/store/src/tx.rs#L30)

#### Returns

`Result<(), StoreError>`


***

### rollback()

```rust
pub fn rollback(self) -> Result<(), StoreError>
```

Defined in: [`crates/kernel/store/src/tx.rs:35`](../../../../crates/kernel/store/src/tx.rs#L35)

#### Returns

`Result<(), StoreError>`

## Trait Implementations

- `impl Borrow for Tx`
- `impl BorrowMut for Tx`
- `impl Into for Tx`
- `impl From for Tx`
- `impl TryInto for Tx`
- `impl TryFrom for Tx`
- `impl Any for Tx`
- `impl CastableFrom for Tx`
- `impl CastableFrom for Tx`
- `impl Read for Tx`
- `impl Drop for Tx`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Unpin` `UnsafeUnpin` `UnwindSafe`

