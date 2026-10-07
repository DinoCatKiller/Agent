---
id: AnthropicFramePolicy
title: AnthropicFramePolicy
---

# Struct: AnthropicFramePolicy

Defined in: [`crates/kernel/providers/src/anthropic.rs:451`](../../../../crates/kernel/providers/src/anthropic.rs#L451)

Anthropic 流式帧判决策略：命名事件 `SseEvent` → 归一化 `StreamEvent`。

- `message_stop` 是终止记录；usage 分两处（`message_start` 的 input + `message_delta`
  的累计 output），在终止处合并发出（`A2` §4：`End` 前必须给一次 `Usage`）；
- 流中 `error` 事件（如 overloaded）→ `Error` 终止，之后不再发任何事件（`A2` §4）；
- 非法 JSON 帧 → `Frame::Skip`（`Q1` 用例 6）；
- EOF 无 `message_stop` → `on_truncated` 补 `Usage` + `End { Truncated }`（用例 5/11）。

与 [`OpenAiFramePolicy`](OpenAiFramePolicy.md) 不同，本策略需要 provider 名：
Anthropic 的错误可以以**流中事件**形式出现，`ProviderError::provider` 必须有归属。

_（存在非公开字段）_

## Implementations

### new()

```rust
pub fn new(provider: &str) -> Self
```

Defined in: [`crates/kernel/providers/src/anthropic.rs:462`](../../../../crates/kernel/providers/src/anthropic.rs#L462)

#### Parameters

##### provider

`&str`

#### Returns

`Self`

## Trait Implementations

- `impl Borrow for AnthropicFramePolicy`
- `impl BorrowMut for AnthropicFramePolicy`
- `impl Into for AnthropicFramePolicy`
- `impl From for AnthropicFramePolicy`
- `impl TryInto for AnthropicFramePolicy`
- `impl TryFrom for AnthropicFramePolicy`
- `impl Any for AnthropicFramePolicy`
- `impl Instrument for AnthropicFramePolicy`
- `impl WithSubscriber for AnthropicFramePolicy`
- `impl PolicyExt for AnthropicFramePolicy`
- `impl Debug for AnthropicFramePolicy`
- `impl Default for AnthropicFramePolicy`
- `impl FramePolicy for AnthropicFramePolicy`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`

