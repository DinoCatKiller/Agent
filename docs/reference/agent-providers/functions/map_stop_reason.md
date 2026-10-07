---
id: map_stop_reason
title: map_stop_reason
---

# Function: map_stop_reason()

```rust
pub fn map_stop_reason(value: &str) -> FinishReason
```

Defined in: [`crates/kernel/providers/src/anthropic.rs:701`](../../../../crates/kernel/providers/src/anthropic.rs#L701)

Anthropic `stop_reason` → 归一化 `FinishReason`。未知值归一为 `Other` 并保留原文到日志。

## Parameters

### value

`&str`

## Returns

`FinishReason`

