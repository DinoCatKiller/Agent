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
    /// `capabilities` 是否**权威可信**（C2）。
    ///
    /// - `true`：来自官方字典或用户显式声明 —— 路由层可据此做能力 fast-fail；
    /// - `false`：只是「未知模型」的保守占位（适配器的 `conservative_spec`）——
    ///   路由层**只**用它确认「id 在清单里」，**不得**据此拦截能力
    ///   （否则会误杀本地 / 兼容端点，见 `P1`「别误杀本地模型」）。
    ///
    /// 这是**运行时来源标记**，不参与序列化（从配置反序列化时按 `false` 保守处理）。
    #[serde(skip)]
    pub capabilities_known: bool,
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
