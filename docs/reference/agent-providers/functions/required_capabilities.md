---
id: required_capabilities
title: required_capabilities
---

# Function: required_capabilities()

```rust
pub fn required_capabilities(req: &ModelRequest) -> Vec<Capability>
```

Defined in: [`crates/kernel/providers/src/lib.rs:172`](../../../../crates/kernel/providers/src/lib.rs#L172)

从请求推导出所需的模型能力。

路由层在发请求**之前**用它做 fast-fail：凡是模型声明不支持的能力，
都不要把请求打到供应商那边去（见 A2 §1）。

## Parameters

### req

`&ModelRequest`

## Returns

`Vec<Capability>`

