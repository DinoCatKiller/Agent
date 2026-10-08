---
id: openai_default_models
title: openai_default_models
---

# Function: openai_default_models()

```rust
pub fn openai_default_models() -> Vec<ModelSpec>
```

Defined in: [`crates/kernel/providers/src/openai.rs:61`](../../../../crates/kernel/providers/src/openai.rs#L61)

OpenAI 官方常用模型静态清单。兼容实现（DeepSeek 等）应由配置层传入自己的 `ModelSpec`。

## Returns

`Vec<ModelSpec>`

