---
id: ErrorCategory
title: ErrorCategory
---

# Enum: ErrorCategory

Defined in: [`crates/common/src/error.rs:13`](../../../../crates/common/src/error.rs#L13)

错误分类。每个适配器都要提供「HTTP 状态 / 错误体 → 本枚举」的映射表。

## Variants

### Auth

Defined in: [`crates/common/src/error.rs:15`](../../../../crates/common/src/error.rs#L15)

401 / 403 / 密钥无效：不重试，告警。


***

### RateLimit

Defined in: [`crates/common/src/error.rs:17`](../../../../crates/common/src/error.rs#L17)

429：指数退避 + 抖动，必要时换模型。


***

### Timeout

Defined in: [`crates/common/src/error.rs:19`](../../../../crates/common/src/error.rs#L19)

超时（含流式空闲超时）。


***

### InvalidRequest

Defined in: [`crates/common/src/error.rs:21`](../../../../crates/common/src/error.rs#L21)

400：参数或模型不支持。不重试，可降级到兼容模型。


***

### Server

Defined in: [`crates/common/src/error.rs:23`](../../../../crates/common/src/error.rs#L23)

5xx：退避重试。


***

### ContextOverflow

Defined in: [`crates/common/src/error.rs:25`](../../../../crates/common/src/error.rs#L25)

超上下文窗口：裁剪 / 摘要后重试一次。


***

### ContentFilter

Defined in: [`crates/common/src/error.rs:27`](../../../../crates/common/src/error.rs#L27)

命中内容审核：不重试，上抛业务。


***

### Truncated

Defined in: [`crates/common/src/error.rs:29`](../../../../crates/common/src/error.rs#L29)

流提前结束。


***

### Unknown

Defined in: [`crates/common/src/error.rs:30`](../../../../crates/common/src/error.rs#L30)

## Implementations

### retryable()

```rust
pub fn retryable(self) -> bool
```

Defined in: [`crates/common/src/error.rs:35`](../../../../crates/common/src/error.rs#L35)

默认是否可重试。策略层可在 `ProviderError::retryable` 上覆盖。

#### Returns

`bool`

## Trait Implementations

- `impl Borrow for ErrorCategory`
- `impl BorrowMut for ErrorCategory`
- `impl CloneToUninit for ErrorCategory`
- `impl Into for ErrorCategory`
- `impl From for ErrorCategory`
- `impl TryInto for ErrorCategory`
- `impl TryFrom for ErrorCategory`
- `impl Any for ErrorCategory`
- `impl ToOwned for ErrorCategory`
- `impl DeserializeOwned for ErrorCategory`
- `impl Debug for ErrorCategory`
- `impl Clone for ErrorCategory`
- `impl Copy for ErrorCategory`
- `impl StructuralPartialEq for ErrorCategory`
- `impl PartialEq for ErrorCategory`
- `impl Eq for ErrorCategory`
- `impl Serialize for ErrorCategory`
- `impl Deserialize for ErrorCategory`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

