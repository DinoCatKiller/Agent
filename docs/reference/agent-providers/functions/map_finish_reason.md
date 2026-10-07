---
id: map_finish_reason
title: map_finish_reason
---

# Function: map_finish_reason()

```rust
pub fn map_finish_reason(value: &str) -> FinishReason
```

Defined in: [`crates/kernel/providers/src/openai.rs:589`](../../../../crates/kernel/providers/src/openai.rs#L589)

OpenAI 结束原因 → 归一化 `FinishReason`。未知值归一为 `Other` 并保留原文到日志。

## Parameters

### value

`&str`

## Returns

`FinishReason`

