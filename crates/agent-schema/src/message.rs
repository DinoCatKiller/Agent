//! 内部消息模型（对应 `A3`，ID `A3`）。
//!
//! 所有适配器都必须把厂商格式转换到这里定义的类型；Core 只认这套。

use serde::{Deserialize, Serialize};

use super::completion::ToolCall;

/// 消息角色。四家的 role 集合并集就是这四个。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    System,
    User,
    Assistant,
    /// 工具结果回填。必须同时带 `tool_call_id`。
    Tool,
}

/// 消息内容块。一期只实现 Text / Image / File，音频留位。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text {
        text: String,
    },
    /// 图片：URL 或 `data:` URI（base64）。
    Image {
        url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
    },
    /// 附件（PDF / 文本等）。一期只透传，不做解析。
    File {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        data: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        url: Option<String>,
    },
}

impl ContentPart {
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }
}

/// 归一化消息。
///
/// - `role = Tool` 时 `tool_call_id` 必填（回填到哪个 `tool_call`）。
/// - `role = Assistant` 且发起工具调用时，`tool_calls` 非空，`content` 可为空。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<ContentPart>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
}

impl Message {
    pub fn system(text: impl Into<String>) -> Self {
        Self::text_only(Role::System, text)
    }

    pub fn user(text: impl Into<String>) -> Self {
        Self::text_only(Role::User, text)
    }

    pub fn assistant(text: impl Into<String>) -> Self {
        Self::text_only(Role::Assistant, text)
    }

    /// 工具结果回填（`role = Tool`，必须带 `tool_call_id`）。
    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: vec![ContentPart::text(content)],
            tool_call_id: Some(tool_call_id.into()),
            tool_calls: Vec::new(),
        }
    }

    fn text_only(role: Role, text: impl Into<String>) -> Self {
        Self {
            role,
            content: vec![ContentPart::text(text)],
            tool_call_id: None,
            tool_calls: Vec::new(),
        }
    }

    /// 拼接全部 `Text` 块。用于日志、UI 预览与 token 估算，**不要**用于重新序列化。
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|part| match part {
                ContentPart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }
}
