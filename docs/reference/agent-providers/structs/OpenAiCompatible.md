---
id: OpenAiCompatible
title: OpenAiCompatible
---

# Struct: OpenAiCompatible

Defined in: [`crates/kernel/providers/src/openai.rs:104`](../../../../crates/kernel/providers/src/openai.rs#L104)

OpenAI 兼容适配器。`Arc<HttpClient>` 保证 `Clone` 只是引用计数（`A2` §2 硬要求）。

_（存在非公开字段）_

## Implementations

### new()

```rust
pub fn new(config: OpenAiConfig) -> Result<Self, ProviderError>
```

Defined in: [`crates/kernel/providers/src/openai.rs:113`](../../../../crates/kernel/providers/src/openai.rs#L113)

#### Parameters

##### config

[`OpenAiConfig`](OpenAiConfig.md)

#### Returns

`Result<Self, ProviderError>`


***

### check_capabilities()

```rust
pub fn check_capabilities(&self, req: &ModelRequest) -> Result<(), ProviderError>
```

Defined in: [`crates/kernel/providers/src/openai.rs:138`](../../../../crates/kernel/providers/src/openai.rs#L138)

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

Defined in: [`crates/kernel/providers/src/openai.rs:143`](../../../../crates/kernel/providers/src/openai.rs#L143)

归一化 `ModelRequest` → OpenAI `chat/completions` 请求体。`pub` 供契约测试断言（`Q1` 用例 4）。

#### Parameters

##### req

`&ModelRequest`

##### stream

`bool`

#### Returns

`Result<Value, ProviderError>`

## Trait Implementations

- `impl ErasedProvider for OpenAiCompatible`
- `impl Borrow for OpenAiCompatible`
- `impl BorrowMut for OpenAiCompatible`
- `impl CloneToUninit for OpenAiCompatible`
- `impl Into for OpenAiCompatible`
- `impl From for OpenAiCompatible`
- `impl TryInto for OpenAiCompatible`
- `impl TryFrom for OpenAiCompatible`
- `impl Any for OpenAiCompatible`
- `impl ToOwned for OpenAiCompatible`
- `impl Instrument for OpenAiCompatible`
- `impl WithSubscriber for OpenAiCompatible`
- `impl PolicyExt for OpenAiCompatible`
- `impl Debug for OpenAiCompatible`
- `impl Clone for OpenAiCompatible`
- `impl Provider for OpenAiCompatible`

## Auto Trait Implementations

`Freeze` `Send` `Sync` `Unpin` `UnsafeUnpin`

