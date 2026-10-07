---
id: OpenAiConfig
title: OpenAiConfig
---

# Struct: OpenAiConfig

Defined in: [`crates/kernel/providers/src/openai.rs:37`](../../../../crates/kernel/providers/src/openai.rs#L37)

OpenAI 兼容适配器配置。

`models` 非空时做本地能力协商（模型不在清单或能力不足即 fast-fail，`A2` §1）；
为空时不拦截，交给供应商报错（用于尚未登记清单的兼容实现）。

## Fields

### id

```rust
id: &'static str
```

Defined in: [`crates/kernel/providers/src/openai.rs:39`](../../../../crates/kernel/providers/src/openai.rs#L39)

供应商 id（如 `"openai"` / `"deepseek"`）。


***

### base_url

```rust
base_url: String
```

Defined in: [`crates/kernel/providers/src/openai.rs:41`](../../../../crates/kernel/providers/src/openai.rs#L41)

如 `https://api.openai.com/v1`（不带尾部斜杠）。真实 endpoint 由配置层注入，不入库（`R1`）。


***

### api_key

```rust
api_key: Option<String>
```

Defined in: [`crates/kernel/providers/src/openai.rs:43`](../../../../crates/kernel/providers/src/openai.rs#L43)

密钥由配置层注入；`None` 表示无鉴权（Ollama 等本地服务）。


***

### http

```rust
http: HttpConfig
```

Defined in: [`crates/kernel/providers/src/openai.rs:44`](../../../../crates/kernel/providers/src/openai.rs#L44)


***

### models

```rust
models: Vec<ModelSpec>
```

Defined in: [`crates/kernel/providers/src/openai.rs:45`](../../../../crates/kernel/providers/src/openai.rs#L45)

## Trait Implementations

- `impl Borrow for OpenAiConfig`
- `impl BorrowMut for OpenAiConfig`
- `impl CloneToUninit for OpenAiConfig`
- `impl Into for OpenAiConfig`
- `impl From for OpenAiConfig`
- `impl TryInto for OpenAiConfig`
- `impl TryFrom for OpenAiConfig`
- `impl Any for OpenAiConfig`
- `impl ToOwned for OpenAiConfig`
- `impl Instrument for OpenAiConfig`
- `impl WithSubscriber for OpenAiConfig`
- `impl PolicyExt for OpenAiConfig`
- `impl Debug for OpenAiConfig`
- `impl Clone for OpenAiConfig`
- `impl Default for OpenAiConfig`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

