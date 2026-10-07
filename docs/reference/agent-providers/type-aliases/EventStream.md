---
id: EventStream
title: EventStream
---

# Type Alias: EventStream

Defined in: [`crates/kernel/providers/src/lib.rs:33`](../../../../crates/kernel/providers/src/lib.rs#L33)

归一化流句柄。

## Definition

```rust
pub type EventStream = Pin<Box<dyn Stream + Send>>
```

