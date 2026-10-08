---
id: Router
title: Router
---

# Struct: Router

Defined in: [`crates/kernel/routing/src/lib.rs:72`](../../../../crates/kernel/routing/src/lib.rs#L72)

模型路由器。

- **隐式路由**：未登记显式路由的模型，按注册顺序找「清单里有该模型」的供应商；
- **显式兜底链**（[`Router::with_route`](Router.md)）：请求 `target` 时按候选依次尝试——
  主模型挂了换备模型、跨供应商别名都靠它。

_（存在非公开字段）_

## Implementations

### new()

```rust
pub fn new() -> Self
```

Defined in: [`crates/kernel/routing/src/lib.rs:78`](../../../../crates/kernel/routing/src/lib.rs#L78)

#### Returns

`Self`


***

### with_provider()

```rust
pub fn with_provider<P: Provider>(self, provider: P) -> Self
```

Defined in: [`crates/kernel/routing/src/lib.rs:83`](../../../../crates/kernel/routing/src/lib.rs#L83)

登记一个供应商；注册顺序即隐式路由的优先级。

#### Parameters

##### provider

`P`

#### Returns

`Self`


***

### with_erased()

```rust
pub fn with_erased(self, provider: Arc<dyn ErasedProvider>) -> Self
```

Defined in: [`crates/kernel/routing/src/lib.rs:89`](../../../../crates/kernel/routing/src/lib.rs#L89)

登记一个已擦除的供应商（调用方手里是 `Arc<dyn ErasedProvider>` 时用）。

#### Parameters

##### provider

`Arc<dyn ErasedProvider>`

#### Returns

`Self`


***

### with_route()

```rust
pub fn with_route<impl Into<String>: Into, impl Into<String>: Into, impl Into<String>: Into, impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>: IntoIterator>(self, target: impl ?, candidates: impl ?) -> Self
```

Defined in: [`crates/kernel/routing/src/lib.rs:96`](../../../../crates/kernel/routing/src/lib.rs#L96)

显式兜底链：请求 `target` 时按 `candidates`（`(provider_id, model_id)`）依次尝试。
`provider_id` 未登记的候选被忽略（由测试与 `resolve` 的长度约束此行为）。

#### Parameters

##### target

`impl ?`

##### candidates

`impl ?`

#### Returns

`Self`

## Trait Implementations

- `impl Borrow for Router`
- `impl BorrowMut for Router`
- `impl CloneToUninit for Router`
- `impl Into for Router`
- `impl From for Router`
- `impl TryInto for Router`
- `impl TryFrom for Router`
- `impl Any for Router`
- `impl ToOwned for Router`
- `impl ErasedProvider for Router`
- `impl Instrument for Router`
- `impl WithSubscriber for Router`
- `impl PolicyExt for Router`
- `impl Clone for Router`
- `impl Default for Router`
- `impl Provider for Router`

## Auto Trait Implementations

`Freeze` `Send` `Sync` `Unpin` `UnsafeUnpin`

