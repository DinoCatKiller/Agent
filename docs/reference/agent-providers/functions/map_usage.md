---
id: map_usage
title: map_usage
---

# Function: map_usage()

```rust
pub fn map_usage(usage: &Value) -> Usage
```

Defined in: [`crates/kernel/providers/src/openai.rs:606`](../../../../crates/kernel/providers/src/openai.rs#L606)

`prompt_tokens` / `completion_tokens` → `Usage`。缺失字段按 0 计。

## Parameters

### usage

`&Value`

## Returns

`Usage`

