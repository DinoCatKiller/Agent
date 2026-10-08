---
id: ModelSpec
title: ModelSpec
---

# Struct: ModelSpec

Defined in: [`crates/common/src/model.rs:34`](../../../../crates/common/src/model.rs#L34)

一个模型的静态元信息。由适配器声明，路由层消费。

## Fields

### id

```rust
id: String
```

Defined in: [`crates/common/src/model.rs:35`](../../../../crates/common/src/model.rs#L35)


***

### provider

```rust
provider: String
```

Defined in: [`crates/common/src/model.rs:36`](../../../../crates/common/src/model.rs#L36)


***

### context_window

```rust
context_window: u32
```

Defined in: [`crates/common/src/model.rs:37`](../../../../crates/common/src/model.rs#L37)


***

### max_output

```rust
max_output: u32
```

Defined in: [`crates/common/src/model.rs:38`](../../../../crates/common/src/model.rs#L38)


***

### capabilities

```rust
capabilities: Vec<Capability>
```

Defined in: [`crates/common/src/model.rs:39`](../../../../crates/common/src/model.rs#L39)


***

### pricing

```rust
pricing: Option<Pricing>
```

Defined in: [`crates/common/src/model.rs:41`](../../../../crates/common/src/model.rs#L41)


***

### deprecated

```rust
deprecated: bool
```

Defined in: [`crates/common/src/model.rs:43`](../../../../crates/common/src/model.rs#L43)

## Implementations

### supports()

```rust
pub fn supports(&self, capability: Capability) -> bool
```

Defined in: [`crates/common/src/model.rs:47`](../../../../crates/common/src/model.rs#L47)

#### Parameters

##### capability

[`Capability`](../enums/Capability.md)

#### Returns

`bool`


***

### missing()

```rust
pub fn missing(&self, required: &[Capability]) -> Vec<Capability>
```

Defined in: [`crates/common/src/model.rs:54`](../../../../crates/common/src/model.rs#L54)

能力校验：返回缺失的能力列表，空表示通过。

路由层在发请求**之前**调用它，避免把必然失败的请求打到供应商。

#### Parameters

##### required

`&[Capability]`

#### Returns

`Vec<Capability>`

## Trait Implementations

- `impl Borrow for ModelSpec`
- `impl BorrowMut for ModelSpec`
- `impl CloneToUninit for ModelSpec`
- `impl Into for ModelSpec`
- `impl From for ModelSpec`
- `impl TryInto for ModelSpec`
- `impl TryFrom for ModelSpec`
- `impl Any for ModelSpec`
- `impl ToOwned for ModelSpec`
- `impl DeserializeOwned for ModelSpec`
- `impl Debug for ModelSpec`
- `impl Clone for ModelSpec`
- `impl StructuralPartialEq for ModelSpec`
- `impl PartialEq for ModelSpec`
- `impl Serialize for ModelSpec`
- `impl Deserialize for ModelSpec`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

