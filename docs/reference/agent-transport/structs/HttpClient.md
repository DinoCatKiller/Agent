---
id: HttpClient
title: HttpClient
---

# Struct: HttpClient

Defined in: [`crates/kernel/transport/src/http.rs:90`](../../../../crates/kernel/transport/src/http.rs#L90)

共享的 HTTP 客户端。`reqwest::Client` 内部是 `Arc`，clone 代价极低。

_（存在非公开字段）_

## Implementations

### new()

```rust
pub fn new(config: HttpConfig) -> Result<Self, TransportError>
```

Defined in: [`crates/kernel/transport/src/http.rs:97`](../../../../crates/kernel/transport/src/http.rs#L97)

按配置构建客户端。代理字符串非法或 TLS 后端不可用时在此 fast-fail。

#### Parameters

##### config

[`HttpConfig`](HttpConfig.md)

#### Returns

`Result<Self, TransportError>`


***

### send()

```rust
pub async fn send(&self, request: RequestBuilder, token: &CancellationToken) -> Result<Response, TransportError>
```

Defined in: [`crates/kernel/transport/src/http.rs:124`](../../../../crates/kernel/transport/src/http.rs#L124)

发送请求，并把 `token` 接进这次调用（**请求阶段**取消）。

**不**判定 HTTP 状态码：4xx / 5xx 同样返回 `Ok(Response)`——
「状态 → `ErrorCategory`」是适配器的映射表（`A2` §5）。

#### Parameters

##### request

`RequestBuilder`

##### token

`&CancellationToken`

#### Returns

`Result<Response, TransportError>`


***

### client()

```rust
pub fn client(&self) -> &Client
```

Defined in: [`crates/kernel/transport/src/http.rs:134`](../../../../crates/kernel/transport/src/http.rs#L134)

底层客户端，用于 `get` / `post` 等构造请求。

#### Returns

`&Client`


***

### config()

```rust
pub fn config(&self) -> &HttpConfig
```

Defined in: [`crates/kernel/transport/src/http.rs:138`](../../../../crates/kernel/transport/src/http.rs#L138)

#### Returns

`&HttpConfig`


***

### stream_idle_timeout()

```rust
pub fn stream_idle_timeout(&self) -> Duration
```

Defined in: [`crates/kernel/transport/src/http.rs:142`](../../../../crates/kernel/transport/src/http.rs#L142)

#### Returns

`Duration`


***

### guard_idle()

```rust
pub fn guard_idle<S, T>(&self, stream: S) -> IdleTimeout<S>
where
    S: Stream + Unpin
```

Defined in: [`crates/kernel/transport/src/http.rs:156`](../../../../crates/kernel/transport/src/http.rs#L156)

给**已归一化**的字节流套上空闲超时（SSE 解析前调用，见 `A4`）。

输入必须是 `Item = Result<T, TransportError>`——即先做错误归一化：

```text
client.guard_idle(response.bytes_stream().map(TransportError::from))
```

这样输出仍是 `Result<T, TransportError>`，可以直接接 `parse_sse` /
`finalize` / `guard`，而不会出现 `Result<Result<_, _>, _>` 的嵌套。

#### Parameters

##### stream

`S`

#### Returns

[`IdleTimeout<S>`](IdleTimeout.md)

## Trait Implementations

- `impl Borrow for HttpClient`
- `impl BorrowMut for HttpClient`
- `impl CloneToUninit for HttpClient`
- `impl Into for HttpClient`
- `impl From for HttpClient`
- `impl TryInto for HttpClient`
- `impl TryFrom for HttpClient`
- `impl Any for HttpClient`
- `impl ToOwned for HttpClient`
- `impl Instrument for HttpClient`
- `impl WithSubscriber for HttpClient`
- `impl PolicyExt for HttpClient`
- `impl Debug for HttpClient`
- `impl Clone for HttpClient`

## Auto Trait Implementations

`Freeze` `Send` `Sync` `Unpin` `UnsafeUnpin`

