---
id: TransportError
title: TransportError
---

# Enum: TransportError

Defined in: [`crates/kernel/transport/src/http.rs:28`](../../../../crates/kernel/transport/src/http.rs#L28)

传输层错误。

刻意不直接产出 `agent_common::ProviderError`：那需要 provider 名称与
HTTP 状态 → 错误类别的映射表，属于适配器职责（`A2` §5）。

## Variants

### Build

```rust
(String)
```

Defined in: [`crates/kernel/transport/src/http.rs:30`](../../../../crates/kernel/transport/src/http.rs#L30)


***

### Timeout

```rust
(String)
```

Defined in: [`crates/kernel/transport/src/http.rs:32`](../../../../crates/kernel/transport/src/http.rs#L32)


***

### Network

```rust
(String)
```

Defined in: [`crates/kernel/transport/src/http.rs:34`](../../../../crates/kernel/transport/src/http.rs#L34)


***

### IdleTimeout

```rust
(Duration)
```

Defined in: [`crates/kernel/transport/src/http.rs:38`](../../../../crates/kernel/transport/src/http.rs#L38)

流式响应两次数据之间超过空闲上限。**必须**上抛，不得静默当成功
（`A2` §4 的截断语义之一）。


***

### Cancelled

Defined in: [`crates/kernel/transport/src/http.rs:40`](../../../../crates/kernel/transport/src/http.rs#L40)

## Trait Implementations

- `impl Borrow for TransportError`
- `impl BorrowMut for TransportError`
- `impl Into for TransportError`
- `impl From for TransportError`
- `impl TryInto for TransportError`
- `impl TryFrom for TransportError`
- `impl Any for TransportError`
- `impl ToString for TransportError`
- `impl Instrument for TransportError`
- `impl WithSubscriber for TransportError`
- `impl PolicyExt for TransportError`
- `impl Debug for TransportError`
- `impl Error for TransportError`
- `impl Display for TransportError`
- `impl From for TransportError`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

