---
id: AnthropicCompatible
title: AnthropicCompatible
---

# Struct: AnthropicCompatible

Defined in: [`crates/kernel/providers/src/anthropic.rs:100`](../../../../crates/kernel/providers/src/anthropic.rs#L100)

Anthropic 适配器。`Arc<HttpClient>` 保证 `Clone` 只是引用计数（`A2` §2 硬要求）。

_（存在非公开字段）_

## Implementations

### new()

```rust
pub fn new(config: AnthropicConfig) -> Result<Self, ProviderError>
```

Defined in: [`crates/kernel/providers/src/anthropic.rs:109`](../../../../crates/kernel/providers/src/anthropic.rs#L109)

#### Parameters

##### config

[`AnthropicConfig`](AnthropicConfig.md)

#### Returns

`Result<Self, ProviderError>`


***

### check_capabilities()

```rust
pub fn check_capabilities(&self, req: &ModelRequest) -> Result<(), ProviderError>
```

Defined in: [`crates/kernel/providers/src/anthropic.rs:135`](../../../../crates/kernel/providers/src/anthropic.rs#L135)

能力协商 fast-fail（`A2` §1 / `Q1` 用例 12）：发请求**之前**拒绝不支持的组合。

#### Parameters

##### req

`&ModelRequest`

#### Returns

`Result<(), ProviderError>`


***

### build_request_body()

```rust
pub fn build_request_body(&self, req: &ModelRequest, stream: bool) -> Result<Value, ProviderError>
```

Defined in: [`crates/kernel/providers/src/anthropic.rs:143`](../../../../crates/kernel/providers/src/anthropic.rs#L143)

归一化 `ModelRequest` → Anthropic Messages 请求体。`pub` 供契约测试断言（`Q1` 用例 4）。

`response_format` 无原生对应：清单模型缺 `JsonMode`/`JsonSchema` 能力，由
[`Self::check_capabilities`](AnthropicCompatible.md) 在发请求前拦截，本函数不做映射（`P2` §3）。

#### Parameters

##### req

`&ModelRequest`

##### stream

`bool`

#### Returns

`Result<Value, ProviderError>`

## Trait Implementations

- `impl ErasedProvider for AnthropicCompatible`
- `impl Borrow for AnthropicCompatible`
- `impl BorrowMut for AnthropicCompatible`
- `impl CloneToUninit for AnthropicCompatible`
- `impl Into for AnthropicCompatible`
- `impl From for AnthropicCompatible`
- `impl TryInto for AnthropicCompatible`
- `impl TryFrom for AnthropicCompatible`
- `impl Any for AnthropicCompatible`
- `impl ToOwned for AnthropicCompatible`
- `impl Instrument for AnthropicCompatible`
- `impl WithSubscriber for AnthropicCompatible`
- `impl PolicyExt for AnthropicCompatible`
- `impl Debug for AnthropicCompatible`
- `impl Clone for AnthropicCompatible`
- `impl Provider for AnthropicCompatible`

## Auto Trait Implementations

`Freeze` `Send` `Sync` `Unpin` `UnsafeUnpin`

