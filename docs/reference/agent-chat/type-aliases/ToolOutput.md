---
id: ToolOutput
title: ToolOutput
---

# Type Alias: ToolOutput

Defined in: [`crates/features/chat/src/tools.rs:16`](../../../../crates/features/chat/src/tools.rs#L16)

工具执行结果：`Ok` = 成功值；`Err` = 错误文案。

错误**不中断循环**：作为 `tool_result` 喂回模型，让它自救或改道（`A5` 错误语义）；
模型反复失败由 `max_model_turns` 兜底（见 `crate::service`)。

## Definition

```rust
pub type ToolOutput = Result<Value, String>
```

