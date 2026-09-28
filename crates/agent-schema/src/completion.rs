//! 请求 / 响应 / 工具类型（对应 A2 §3）。

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::message::Message;

/// 工具定义。`parameters` 是 JSON Schema，三家的字段名不同，
/// 由适配器在边界处翻译（OpenAI `function.parameters` / Anthropic `input_schema`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

/// 归一化后的工具调用。
///
/// - `arguments` 统一为**结构化 JSON**。OpenAI 侧返回的是 JSON 字符串（可能非法），
///   反序列化与容错由适配器负责，Core 只看结构化结果。
/// - `provider_id` 保留供应商原发 id（如 OpenAI Responses 的 `item_id`），回传时可能用到。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
}

/// 工具选择。OpenAI 的 `required` 与 Anthropic 的 `any` 都归一为 `Required`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolChoice {
    Auto,
    None,
    Required,
    Specific(Vec<String>),
}

/// 结构化输出请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseFormat {
    Text,
    /// 只保证合法 JSON，不保证 schema 一致（DeepSeek Chat 仅有此档）。
    JsonObject,
    /// 严格 schema 输出；适配器负责把 schema 清洗成该供应商支持的子集。
    JsonSchema {
        name: String,
        schema: Value,
    },
}

/// 归一化请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolDefinition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stop: Vec<String>,
    /// 业务侧附加信息，不发给供应商。
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub metadata: Map<String, Value>,
    /// 厂商特有参数透传。**唯一**允许出现厂商字段的地方。
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub additional_params: Map<String, Value>,
}

impl ModelRequest {
    pub fn new(model: impl Into<String>, messages: Vec<Message>) -> Self {
        Self {
            model: model.into(),
            messages,
            temperature: None,
            max_tokens: None,
            tools: Vec::new(),
            tool_choice: None,
            response_format: None,
            stop: Vec::new(),
            metadata: Map::new(),
            additional_params: Map::new(),
        }
    }
}

/// 结束原因。未知值归一为 `Other`，不要丢弃（日志需要）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
    /// 流被截断（EOF 无终止记录）。
    Truncated,
    Other,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

impl Usage {
    pub fn total(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }
}

/// 归一化响应（非流式）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelResponse {
    pub id: String,
    pub model: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    pub finish_reason: FinishReason,
    pub usage: Usage,
    /// 原始响应体，仅用于排错与 fixtures 回放。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<Value>,
}
