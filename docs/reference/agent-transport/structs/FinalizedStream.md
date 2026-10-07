---
id: FinalizedStream
title: FinalizedStream
---

# Struct: FinalizedStream

Defined in: [`crates/kernel/transport/src/finish.rs:103`](../../../../crates/kernel/transport/src/finish.rs#L103)

给帧流套上三分收尾语义的适配器，由 [`finalize`](../functions/finalize.md) / [`finalize_bytes`](../functions/finalize_bytes.md) 构造。

- `S`：上游帧流 ｜ `F`：帧类型（与 SSE 无关，见模块文档） ｜ `P`：判决策略

_（存在非公开字段）_

## Implementations

### new()

```rust
pub fn new(inner: S, policy: P) -> Self
```

Defined in: [`crates/kernel/transport/src/finish.rs:116`](../../../../crates/kernel/transport/src/finish.rs#L116)

用 `policy` 包装帧流 `inner`。

#### Parameters

##### inner

`S`

##### policy

`P`

#### Returns

`Self`


***

### policy()

```rust
pub fn policy(&self) -> &P
```

Defined in: [`crates/kernel/transport/src/finish.rs:127`](../../../../crates/kernel/transport/src/finish.rs#L127)

已判决的判决对象（供适配器在收尾后读取累积状态，如拼接中的工具参数）。

#### Returns

`&P`


***

### policy_mut()

```rust
pub fn policy_mut(&self) -> &P
```

Defined in: [`crates/kernel/transport/src/finish.rs:132`](../../../../crates/kernel/transport/src/finish.rs#L132)

已判决的判决对象（可变）。

#### Returns

`&P`


***

### terminal_seen()

```rust
pub fn terminal_seen(&self) -> bool
```

Defined in: [`crates/kernel/transport/src/finish.rs:137`](../../../../crates/kernel/transport/src/finish.rs#L137)

是否已见过终止记录（收尾后可用于区分「正常结束」与「截断」）。

#### Returns

`bool`

## Trait Implementations

- `impl Borrow for FinalizedStream`
- `impl BorrowMut for FinalizedStream`
- `impl Into for FinalizedStream`
- `impl From for FinalizedStream`
- `impl TryInto for FinalizedStream`
- `impl TryFrom for FinalizedStream`
- `impl Any for FinalizedStream`
- `impl TryStream for FinalizedStream`
- `impl StreamExt for FinalizedStream`
- `impl TryStreamExt for FinalizedStream`
- `impl Instrument for FinalizedStream`
- `impl WithSubscriber for FinalizedStream`
- `impl PolicyExt for FinalizedStream`
- `impl Stream for FinalizedStream`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

