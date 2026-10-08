---
id: CallContext
title: CallContext
---

# Struct: CallContext

Defined in: [`crates/kernel/providers/src/lib.rs:37`](../../../../crates/kernel/providers/src/lib.rs#L37)

一次调用的上下文。用 struct 而非裸 token，后续加字段不破坏适配器签名。

## Fields

### request_id

```rust
request_id: Option<String>
```

Defined in: [`crates/kernel/providers/src/lib.rs:39`](../../../../crates/kernel/providers/src/lib.rs#L39)

观测用；最终会落到 `ProviderError::request_id` 与 trace。


***

### cancel

```rust
cancel: CancellationToken
```

Defined in: [`crates/kernel/providers/src/lib.rs:41`](../../../../crates/kernel/providers/src/lib.rs#L41)

取消令牌：UI 点「停止」时触发。

## Implementations

### new()

```rust
pub fn new() -> Self
```

Defined in: [`crates/kernel/providers/src/lib.rs:45`](../../../../crates/kernel/providers/src/lib.rs#L45)

#### Returns

`Self`


***

### with_request_id()

```rust
pub fn with_request_id<impl Into<String>: Into>(self, request_id: impl ?) -> Self
```

Defined in: [`crates/kernel/providers/src/lib.rs:49`](../../../../crates/kernel/providers/src/lib.rs#L49)

#### Parameters

##### request_id

`impl ?`

#### Returns

`Self`

## Trait Implementations

- `impl Borrow for CallContext`
- `impl BorrowMut for CallContext`
- `impl CloneToUninit for CallContext`
- `impl Into for CallContext`
- `impl From for CallContext`
- `impl TryInto for CallContext`
- `impl TryFrom for CallContext`
- `impl Any for CallContext`
- `impl ToOwned for CallContext`
- `impl Instrument for CallContext`
- `impl WithSubscriber for CallContext`
- `impl PolicyExt for CallContext`
- `impl Debug for CallContext`
- `impl Clone for CallContext`
- `impl Default for CallContext`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

