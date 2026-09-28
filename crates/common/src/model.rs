//! 模型元信息与能力协商（对应 A2 §1）。

use serde::{Deserialize, Serialize};

/// 能力位。
///
/// `JsonMode`（只保证合法 JSON）与 `JsonSchema`（严格 schema 输出）是**两种能力**：
/// OpenAI 两者都有，DeepSeek Chat 只有前者。请求前必须按能力 fast-fail。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Text,
    Vision,
    Audio,
    File,
    Tools,
    JsonMode,
    JsonSchema,
    Reasoning,
    Logprobs,
    Embedding,
}

/// 价格（每百万 token）。货币用 ISO 代码，便于多币种混算。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pricing {
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
    pub currency: String,
}

/// 一个模型的静态元信息。由适配器声明，路由层消费。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelSpec {
    pub id: String,
    pub provider: String,
    pub context_window: u32,
    pub max_output: u32,
    pub capabilities: Vec<Capability>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pricing: Option<Pricing>,
    #[serde(default)]
    pub deprecated: bool,
}

impl ModelSpec {
    pub fn supports(&self, capability: Capability) -> bool {
        self.capabilities.contains(&capability)
    }

    /// 能力校验：返回缺失的能力列表，空表示通过。
    ///
    /// 路由层在发请求**之前**调用它，避免把必然失败的请求打到供应商。
    pub fn missing(&self, required: &[Capability]) -> Vec<Capability> {
        required
            .iter()
            .copied()
            .filter(|c| !self.supports(*c))
            .collect()
    }
}
