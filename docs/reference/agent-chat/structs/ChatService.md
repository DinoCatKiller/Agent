---
id: ChatService
title: ChatService
---

# Struct: ChatService

Defined in: [`crates/features/chat/src/service.rs:91`](../../../../crates/features/chat/src/service.rs#L91)

会话编排服务。持有一个供应商与一组工具；状态由 [`Chat`](Chat.md) 外置携带。

_（存在非公开字段）_

## Implementations

### new()

```rust
pub fn new(provider: Arc<dyn ErasedProvider>, tools: ToolSet) -> Self
```

Defined in: [`crates/features/chat/src/service.rs:98`](../../../../crates/features/chat/src/service.rs#L98)

#### Parameters

##### provider

`Arc<dyn ErasedProvider>`

##### tools

[`ToolSet`](ToolSet.md)

#### Returns

`Self`


***

### with_config()

```rust
pub fn with_config(self, config: ServiceConfig) -> Self
```

Defined in: [`crates/features/chat/src/service.rs:106`](../../../../crates/features/chat/src/service.rs#L106)

#### Parameters

##### config

[`ServiceConfig`](ServiceConfig.md)

#### Returns

`Self`


***

### provider()

```rust
pub fn provider(&self) -> &Arc<dyn ErasedProvider>
```

Defined in: [`crates/features/chat/src/service.rs:111`](../../../../crates/features/chat/src/service.rs#L111)

#### Returns

`&Arc<dyn ErasedProvider>`


***

### tools()

```rust
pub fn tools(&self) -> &ToolSet
```

Defined in: [`crates/features/chat/src/service.rs:115`](../../../../crates/features/chat/src/service.rs#L115)

#### Returns

`&ToolSet`


***

### config()

```rust
pub fn config(&self) -> &ServiceConfig
```

Defined in: [`crates/features/chat/src/service.rs:119`](../../../../crates/features/chat/src/service.rs#L119)

#### Returns

`&ServiceConfig`


***

### run_round()

```rust
pub fn run_round<'a, impl Into<String>: Into>(&self, chat: &'a Chat, input: impl ?, cancel: CancellationToken) -> Result<impl ? + ?, ProviderError>
```

Defined in: [`crates/features/chat/src/service.rs:127`](../../../../crates/features/chat/src/service.rs#L127)

发起一轮对话。用户输入先入史，然后循环「模型 →（工具 → 回填 →）模型」直到自然结束。

同步阶段（返回 `Err` 即 fast-fail，`A2` §2 的精神）：入史、请求校验、裁剪、
首次 `provider.stream`（能力协商失败在这里就暴露，而不是等第一次 poll）。

#### Parameters

##### chat

`&'a Chat`

##### input

`impl ?`

##### cancel

`CancellationToken`

#### Returns

`Result<impl ? + ?, ProviderError>`

## Trait Implementations

- `impl Borrow for ChatService`
- `impl BorrowMut for ChatService`
- `impl Into for ChatService`
- `impl From for ChatService`
- `impl TryInto for ChatService`
- `impl TryFrom for ChatService`
- `impl Any for ChatService`
- `impl CastableFrom for ChatService`
- `impl CastableFrom for ChatService`
- `impl Read for ChatService`
- `impl Instrument for ChatService`
- `impl WithSubscriber for ChatService`
- `impl PolicyExt for ChatService`
- `impl IntoEither for ChatService`

## Auto Trait Implementations

`Freeze` `Send` `Sync` `Unpin` `UnsafeUnpin`

