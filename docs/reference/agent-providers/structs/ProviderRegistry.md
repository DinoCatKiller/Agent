---
id: ProviderRegistry
title: ProviderRegistry
---

# Struct: ProviderRegistry

Defined in: [`crates/kernel/providers/src/lib.rs:123`](../../../../crates/kernel/providers/src/lib.rs#L123)

运行时注册表。Core 只与它打交道，不认识任何具体供应商。

_（存在非公开字段）_

## Implementations

### new()

```rust
pub fn new() -> Self
```

Defined in: [`crates/kernel/providers/src/lib.rs:128`](../../../../crates/kernel/providers/src/lib.rs#L128)

#### Returns

`Self`


***

### register()

```rust
pub fn register<P: [Provider](traits/Provider.md)>(&self, provider: P) -> &Self
```

Defined in: [`crates/kernel/providers/src/lib.rs:132`](../../../../crates/kernel/providers/src/lib.rs#L132)

#### Parameters

##### provider

`P`

#### Returns

`&Self`


***

### get()

```rust
pub fn get(&self, provider_id: &str) -> Option<Arc<dyn ErasedProvider>>
```

Defined in: [`crates/kernel/providers/src/lib.rs:137`](../../../../crates/kernel/providers/src/lib.rs#L137)

#### Parameters

##### provider_id

`&str`

#### Returns

`Option<Arc<dyn ErasedProvider>>`


***

### ids()

```rust
pub fn ids(&self) -> Vec<&'static str>
```

Defined in: [`crates/kernel/providers/src/lib.rs:141`](../../../../crates/kernel/providers/src/lib.rs#L141)

#### Returns

`Vec<&'static str>`


***

### models()

```rust
pub fn models(&self) -> Vec<ModelSpec>
```

Defined in: [`crates/kernel/providers/src/lib.rs:148`](../../../../crates/kernel/providers/src/lib.rs#L148)

所有已注册供应商的模型清单（路由层的输入），按 provider + id 排序。

#### Returns

`Vec<ModelSpec>`


***

### find_model()

```rust
pub fn find_model(&self, model_id: &str) -> Option<ModelSpec>
```

Defined in: [`crates/kernel/providers/src/lib.rs:160`](../../../../crates/kernel/providers/src/lib.rs#L160)

#### Parameters

##### model_id

`&str`

#### Returns

`Option<ModelSpec>`

## Trait Implementations

- `impl Borrow for ProviderRegistry`
- `impl BorrowMut for ProviderRegistry`
- `impl Into for ProviderRegistry`
- `impl From for ProviderRegistry`
- `impl TryInto for ProviderRegistry`
- `impl TryFrom for ProviderRegistry`
- `impl Any for ProviderRegistry`
- `impl Instrument for ProviderRegistry`
- `impl WithSubscriber for ProviderRegistry`
- `impl PolicyExt for ProviderRegistry`
- `impl Default for ProviderRegistry`

## Auto Trait Implementations

`Freeze` `Send` `Sync` `Unpin` `UnsafeUnpin`

