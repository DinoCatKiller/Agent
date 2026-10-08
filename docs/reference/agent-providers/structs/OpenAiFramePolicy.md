---
id: OpenAiFramePolicy
title: OpenAiFramePolicy
---

# Struct: OpenAiFramePolicy

Defined in: [`crates/kernel/providers/src/openai.rs:410`](../../../../crates/kernel/providers/src/openai.rs#L410)

OpenAI 流式帧判决策略：`SseEvent` → 归一化 `StreamEvent`。

- `[DONE]` 是终止记录；usage 帧（`include_usage` 打开时）即时产出 `Usage`；
- 非法 JSON 帧 → `Frame::Skip`（`Q1` 用例 6）；
- EOF 无 `[DONE]` → `on_truncated` 补 `Usage`(如缺) + `End { Truncated }`（用例 5/11）。

_（存在非公开字段）_

## Implementations

### new()

```rust
pub fn new() -> Self
```

Defined in: [`crates/kernel/providers/src/openai.rs:422`](../../../../crates/kernel/providers/src/openai.rs#L422)

#### Returns

`Self`

## Trait Implementations

- `impl Borrow for OpenAiFramePolicy`
- `impl BorrowMut for OpenAiFramePolicy`
- `impl Into for OpenAiFramePolicy`
- `impl From for OpenAiFramePolicy`
- `impl TryInto for OpenAiFramePolicy`
- `impl TryFrom for OpenAiFramePolicy`
- `impl Any for OpenAiFramePolicy`
- `impl Instrument for OpenAiFramePolicy`
- `impl WithSubscriber for OpenAiFramePolicy`
- `impl PolicyExt for OpenAiFramePolicy`
- `impl Debug for OpenAiFramePolicy`
- `impl Default for OpenAiFramePolicy`
- `impl FramePolicy for OpenAiFramePolicy`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

