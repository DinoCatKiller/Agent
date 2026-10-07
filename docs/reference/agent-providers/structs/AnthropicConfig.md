---
id: AnthropicConfig
title: AnthropicConfig
---

# Struct: AnthropicConfig

Defined in: [`crates/kernel/providers/src/anthropic.rs:43`](../../../../crates/kernel/providers/src/anthropic.rs#L43)

Anthropic 适配器配置。

`models` 非空时做本地能力协商（模型不在清单或能力不足即 fast-fail，`A2` §1）；
为空时不拦截，交给供应商报错（用于尚未登记清单的兼容实现）。

## Fields

### id

```rust
id: &'static str
```

Defined in: [`crates/kernel/providers/src/anthropic.rs:45`](../../../../crates/kernel/providers/src/anthropic.rs#L45)

供应商 id（如 `"anthropic"`）。


***

### base_url

```rust
base_url: String
```

Defined in: [`crates/kernel/providers/src/anthropic.rs:48`](../../../../crates/kernel/providers/src/anthropic.rs#L48)

如 `https://api.anthropic.com`（`/v1/messages` 由适配器拼接，不带尾部斜杠）。
真实 endpoint 由配置层注入，不入库（`R1`）。


***

### api_key

```rust
api_key: Option<String>
```

Defined in: [`crates/kernel/providers/src/anthropic.rs:50`](../../../../crates/kernel/providers/src/anthropic.rs#L50)

密钥由配置层注入；`None` 表示不带头（部分本地代理）。


***

### http

```rust
http: HttpConfig
```

Defined in: [`crates/kernel/providers/src/anthropic.rs:51`](../../../../crates/kernel/providers/src/anthropic.rs#L51)


***

### models

```rust
models: Vec<ModelSpec>
```

Defined in: [`crates/kernel/providers/src/anthropic.rs:52`](../../../../crates/kernel/providers/src/anthropic.rs#L52)

## Trait Implementations

- `impl Borrow for AnthropicConfig`
- `impl BorrowMut for AnthropicConfig`
- `impl CloneToUninit for AnthropicConfig`
- `impl Into for AnthropicConfig`
- `impl From for AnthropicConfig`
- `impl TryInto for AnthropicConfig`
- `impl TryFrom for AnthropicConfig`
- `impl Any for AnthropicConfig`
- `impl ToOwned for AnthropicConfig`
- `impl Instrument for AnthropicConfig`
- `impl WithSubscriber for AnthropicConfig`
- `impl PolicyExt for AnthropicConfig`
- `impl Debug for AnthropicConfig`
- `impl Clone for AnthropicConfig`
- `impl Default for AnthropicConfig`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

