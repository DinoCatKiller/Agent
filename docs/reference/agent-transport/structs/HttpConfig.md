---
id: HttpConfig
title: HttpConfig
---

# Struct: HttpConfig

Defined in: [`crates/kernel/transport/src/http.rs:59`](../../../../crates/kernel/transport/src/http.rs#L59)

传输层配置。字段全部来自配置层（`R1`），不在此处读环境变量或文件。

## Fields

### connect_timeout

```rust
connect_timeout: Duration
```

Defined in: [`crates/kernel/transport/src/http.rs:61`](../../../../crates/kernel/transport/src/http.rs#L61)

建立连接（含 TLS 握手）超时。


***

### request_timeout

```rust
request_timeout: Option<Duration>
```

Defined in: [`crates/kernel/transport/src/http.rs:63`](../../../../crates/kernel/transport/src/http.rs#L63)

整体请求超时。**流式请求应为 `None`**，改用 `Self::stream_idle_timeout` 兜底。


***

### stream_idle_timeout

```rust
stream_idle_timeout: Duration
```

Defined in: [`crates/kernel/transport/src/http.rs:65`](../../../../crates/kernel/transport/src/http.rs#L65)

流式响应两次数据之间的最大间隔。


***

### proxy

```rust
proxy: Option<String>
```

Defined in: [`crates/kernel/transport/src/http.rs:67`](../../../../crates/kernel/transport/src/http.rs#L67)

代理地址（如 `http://127.0.0.1:7890`）。`None` 表示直连。


***

### user_agent

```rust
user_agent: Option<String>
```

Defined in: [`crates/kernel/transport/src/http.rs:69`](../../../../crates/kernel/transport/src/http.rs#L69)

User-Agent。`None` 表示用 reqwest 默认值。


***

### pool_idle_timeout

```rust
pool_idle_timeout: Duration
```

Defined in: [`crates/kernel/transport/src/http.rs:71`](../../../../crates/kernel/transport/src/http.rs#L71)

连接池中空闲连接的存活时间。

## Trait Implementations

- `impl Borrow for HttpConfig`
- `impl BorrowMut for HttpConfig`
- `impl CloneToUninit for HttpConfig`
- `impl Into for HttpConfig`
- `impl From for HttpConfig`
- `impl TryInto for HttpConfig`
- `impl TryFrom for HttpConfig`
- `impl Any for HttpConfig`
- `impl ToOwned for HttpConfig`
- `impl Instrument for HttpConfig`
- `impl WithSubscriber for HttpConfig`
- `impl PolicyExt for HttpConfig`
- `impl Debug for HttpConfig`
- `impl Clone for HttpConfig`
- `impl Default for HttpConfig`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

