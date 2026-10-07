//! 对话状态机（`A3` 消息协议的运行时侧）。
//!
//! [`Chat`] 是**纯数据**：一次会话的全部历史与累计用量，可序列化（M5 持久化时原样落盘）。
//! [`TurnDraft`] 是流式累积器：把一轮 `StreamEvent` 攒成一条 assistant 消息，
//! 纯 reducer、零 IO，单测不需要网络（`Q1`）。

use serde::{Deserialize, Serialize};

use agent_common::{
    ContentPart, DeltaKind, ErrorCategory, FinishReason, Message, ModelRequest, ProviderError,
    Role, StreamEvent, ToolCall, ToolDefinition, Usage,
};

/// 一轮请求的模型侧参数。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stop: Vec<String>,
}

/// 一次会话的完整状态。
///
/// - `messages` 以 `System`（可选）开头，之后是 User / Assistant / Tool 的完整历史；
/// - 取消 / 出错的轮次**不入史**（增量只存在于 [`TurnDraft`]，丢弃即回滚）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Chat {
    pub model: String,
    #[serde(default)]
    pub settings: ChatSettings,
    #[serde(default)]
    pub messages: Vec<Message>,
    /// 会话累计用量（各轮 `Usage` 事件之和）。
    #[serde(default)]
    pub total_usage: Usage,
}

impl Chat {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            ..Self::default()
        }
    }

    /// 带 system 提示的会话（system 必须是最前一条，`A3` §1）。
    pub fn with_system(model: impl Into<String>, system: impl Into<String>) -> Self {
        let mut chat = Self::new(model);
        chat.messages.push(Message::system(system));
        chat
    }

    pub fn push_user(&mut self, text: impl Into<String>) {
        self.messages.push(Message::user(text));
    }

    pub fn push_tool_result(
        &mut self,
        tool_call_id: impl Into<String>,
        content: impl Into<String>,
    ) {
        self.messages
            .push(Message::tool_result(tool_call_id, content));
    }

    /// 提交一轮已完结（`End` 之后）的模型输出，并累计用量。
    pub fn commit_turn(&mut self, draft: TurnDraft) {
        let usage = draft.usage();
        self.messages.push(draft.into_message());
        self.total_usage.input_tokens += usage.input_tokens;
        self.total_usage.output_tokens += usage.output_tokens;
    }

    /// 构造下一次模型请求。
    ///
    /// 含 `A3` 落地清单的运行期校验：`role = Tool` 而缺 `tool_call_id`
    /// 直接 `InvalidRequest`（本阶段在 Core 加，M4）。
    pub fn request(&self, tools: &[ToolDefinition]) -> Result<ModelRequest, ProviderError> {
        for msg in &self.messages {
            if msg.role == Role::Tool && msg.tool_call_id.as_deref().is_none_or(str::is_empty) {
                return Err(ProviderError::new("chat", ErrorCategory::InvalidRequest)
                    .with_message(
                        "tool message is missing tool_call_id (A3: 一条结果必须对应一次调用)",
                    ));
            }
        }
        let mut req = ModelRequest::new(self.model.clone(), self.messages.clone());
        req.temperature = self.settings.temperature;
        req.max_tokens = self.settings.max_tokens;
        req.stop = self.settings.stop.clone();
        req.tools = tools.to_vec();
        Ok(req)
    }
}

/// 流式累积器：吃一轮 `StreamEvent`，攒出一条 assistant 消息。
///
/// - `Delta(Text)` 追加文本；`Delta(Thinking)` 只给 UI 显示，**不进历史**
///   （`ContentPart` 无 thinking 变体，`A3` 不为此加）；
/// - 工具调用按适配器给出的完整 [`ToolCall`] 收集（分片拼装已在适配器完成，`A2` §4）；
/// - 只在 `commit_turn` 时入史——中途丢弃即回滚。
#[derive(Debug, Clone, Default)]
pub struct TurnDraft {
    text: String,
    tool_calls: Vec<ToolCall>,
    usage: Option<Usage>,
    finish_reason: Option<FinishReason>,
}

impl TurnDraft {
    pub fn apply(&mut self, event: &StreamEvent) {
        match event {
            StreamEvent::Delta {
                kind: DeltaKind::Text,
                text,
            } => self.text.push_str(text),
            StreamEvent::ToolCall { call } => self.tool_calls.push(call.clone()),
            StreamEvent::Usage { usage } => self.usage = Some(*usage),
            StreamEvent::End { finish_reason } => self.finish_reason = Some(*finish_reason),
            StreamEvent::Delta { .. } | StreamEvent::Start { .. } | StreamEvent::Error { .. } => {}
        }
    }

    pub fn finish_reason(&self) -> Option<FinishReason> {
        self.finish_reason
    }

    pub fn tool_calls(&self) -> &[ToolCall] {
        &self.tool_calls
    }

    pub fn usage(&self) -> Usage {
        self.usage.unwrap_or_default()
    }

    /// 攒出的 assistant 消息：文本与 `tool_calls` 并存（`A3` §1）。
    pub fn into_message(self) -> Message {
        Message {
            role: Role::Assistant,
            content: if self.text.is_empty() {
                Vec::new()
            } else {
                vec![ContentPart::text(self.text)]
            },
            tool_call_id: None,
            tool_calls: self.tool_calls,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn draft_accumulates_deltas_tools_and_usage() {
        let mut draft = TurnDraft::default();
        draft.apply(&StreamEvent::Start {
            response_id: "r1".into(),
            model: "m".into(),
        });
        draft.apply(&StreamEvent::Delta {
            kind: DeltaKind::Text,
            text: "你好".into(),
        });
        draft.apply(&StreamEvent::Delta {
            kind: DeltaKind::Thinking,
            text: "内心戏".into(),
        });
        draft.apply(&StreamEvent::ToolCall {
            call: ToolCall {
                id: "c1".into(),
                name: "weather".into(),
                arguments: json!({"city": "上海"}),
                provider_id: None,
            },
        });
        draft.apply(&StreamEvent::Usage {
            usage: Usage {
                input_tokens: 3,
                output_tokens: 7,
            },
        });
        draft.apply(&StreamEvent::End {
            finish_reason: FinishReason::ToolCalls,
        });

        assert_eq!(draft.finish_reason(), Some(FinishReason::ToolCalls));
        assert_eq!(draft.usage().total(), 10);
        let msg = draft.into_message();
        assert_eq!(msg.role, Role::Assistant);
        assert_eq!(msg.text(), "你好", "thinking 不进历史");
        assert_eq!(msg.tool_calls.len(), 1);
        assert_eq!(msg.tool_calls[0].id, "c1");
    }

    #[test]
    fn chat_rejects_tool_message_without_id() {
        let mut chat = Chat::new("m");
        chat.messages.push(Message {
            role: Role::Tool,
            content: vec![ContentPart::text("结果")],
            tool_call_id: None,
            tool_calls: Vec::new(),
        });
        let err = chat.request(&[]).expect_err("缺 tool_call_id 必须被拒");
        assert_eq!(err.category, ErrorCategory::InvalidRequest);
    }

    #[test]
    fn request_carries_settings_and_tools() {
        let mut chat = Chat::with_system("m", "简洁");
        chat.settings.temperature = Some(0.2);
        chat.settings.max_tokens = Some(512);
        chat.push_user("hi");
        let tools = vec![ToolDefinition {
            name: "t".into(),
            description: "d".into(),
            parameters: json!({"type": "object"}),
        }];
        let req = chat.request(&tools).unwrap();
        assert_eq!(req.messages.len(), 2);
        assert_eq!(req.messages[0].role, Role::System);
        assert_eq!(req.temperature, Some(0.2));
        assert_eq!(req.max_tokens, Some(512));
        assert_eq!(req.tools.len(), 1);
    }
}
