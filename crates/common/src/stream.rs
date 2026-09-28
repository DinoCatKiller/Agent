//! 归一化流式事件（对应 A2 §4）。

use serde::{Deserialize, Serialize};

use super::completion::{FinishReason, ToolCall, Usage};
use super::error::ProviderError;

/// 增量类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeltaKind {
    Text,
    /// 推理内容（如 reasoning 模型）。
    Thinking,
    /// 工具参数的**分片**。适配器负责按 index/id 拼接，
    /// 拼装完成后才发 `StreamEvent::ToolCall`。
    ToolArgs,
}

/// 归一化流式事件。
///
/// 契约（适配器必须保证）：
/// 1. `Start` 是首个事件，`End` 是末个事件。
/// 2. `Delta` 必须可**无状态拼接**（消费者只做 push_str）。
/// 3. `End` 之前必须给一次 `Usage`。
/// 4. `Error` 之后不得再发任何事件。
/// 5. 截断（EOF 无终止记录）不得静默当成功：补 `End { Truncated }` 或 `Error`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum StreamEvent {
    Start { response_id: String, model: String },
    Delta { kind: DeltaKind, text: String },
    ToolCall { call: ToolCall },
    Usage { usage: Usage },
    Error { error: ProviderError },
    End { finish_reason: FinishReason },
}
