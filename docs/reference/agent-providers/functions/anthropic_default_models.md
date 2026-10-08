---
id: anthropic_default_models
title: anthropic_default_models
---

# Function: anthropic_default_models()

```rust
pub fn anthropic_default_models() -> Vec<ModelSpec>
```

Defined in: [`crates/kernel/providers/src/anthropic.rs:68`](../../../../crates/kernel/providers/src/anthropic.rs#L68)

Anthropic 官方常用模型静态清单（快照，随时点过时）。兼容实现应由配置层传入自己的 `ModelSpec`。

## Returns

`Vec<ModelSpec>`

