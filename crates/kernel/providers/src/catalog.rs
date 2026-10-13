//! 内置供应商目录（预设表）。
//!
//! **厂商**与**适配器**是两件事：
//!
//! | | 是什么 | 谁看见 |
//! |---|---|---|
//! | 适配器 | 实现（`OpenAiCompatible` / `AnthropicCompatible`），决定协议怎么讲 | 我们 |
//! | 厂商 | 「适配器 + 默认端点 + 密钥从哪来」的一组预设 | 用户 |
//!
//! 没有这张表的时候，用户被迫在 `--provider` / `default_provider` 里填**适配器名**，
//! 于是"为什么 Ollama 要写 openai-compatible"成了必修课（`S2` Q9）。有了目录，
//! 用户看见的永远是厂商名，适配器由表给出。
//!
//! 表里也收着两个**通用适配器**条目（`openai-compatible` / `anthropic`）：老配置里
//! `providers.openai-compatible` 就是它，`default_provider` 也一直写这两个 id ——
//! 收进同一张表之后，「向后兼容」就只是"id 命中适配器名"这一种情况，不需要额外分支。
//!
//! # 端点值怎么来的
//!
//! 每条都带 `docs`，值取自各家文档的 **OpenAI 兼容**地址。目录写错**不会**编译失败，
//! 只会表现为"连不上"——所以接入时请按 `docs` 核对一次（`X2` 的实测口径：不信记录，信实测）。
//! 本地服务（Ollama / LM Studio / vLLM）的端口也是各自的默认值。

/// 一个供应商预设。
///
/// 全 `&'static str`：这是**编译期常量表**，不需要解析、不需要网络、不存在"文件丢了"。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderPreset {
    /// 条目 id：用户写进 `default_provider` 与 `providers.<id>` 的名字。
    pub id: &'static str,
    /// 展示名（界面上分组的标题、设置页里那一行）。
    pub name: &'static str,
    /// 用哪个适配器实现 —— 对应 `build_provider` 的 `match` 分支。
    pub adapter: &'static str,
    /// 默认端点（`providers.<id>.base_url` 可覆盖）。
    pub base_url: &'static str,
    /// 密钥的约定环境变量名。`None` = 没有约定俗成的名字（本地服务），只用钥匙串。
    pub api_key_env: Option<&'static str>,
    /// 是否**必须**有密钥（本地服务不需要；界面据此决定要不要提示）。
    pub key_required: bool,
    /// 文档来源（核对端点用）。
    pub docs: &'static str,
}

/// 适配器 id —— `build_provider` 能构造的两种实现。
///
/// **单点定义**：目录里每条的 `adapter` 都必须落在这两个里（有测试钉着）。
pub const ADAPTERS: [&str; 2] = ["openai-compatible", "anthropic"];

/// 内置目录。
pub const PRESETS: &[ProviderPreset] = &[
    ProviderPreset {
        id: "openai-compatible",
        name: "OpenAI 兼容（自定义端点）",
        adapter: "openai-compatible",
        base_url: "https://api.openai.com/v1",
        api_key_env: Some("OPENAI_API_KEY"),
        key_required: true,
        docs: "https://platform.openai.com/docs/api-reference",
    },
    ProviderPreset {
        id: "anthropic",
        name: "Anthropic",
        adapter: "anthropic",
        base_url: "https://api.anthropic.com",
        api_key_env: Some("ANTHROPIC_API_KEY"),
        key_required: true,
        docs: "https://docs.anthropic.com/en/api",
    },
    ProviderPreset {
        id: "ollama",
        name: "Ollama（本地）",
        adapter: "openai-compatible",
        base_url: "http://localhost:11434/v1",
        api_key_env: None,
        key_required: false,
        docs: "https://github.com/ollama/ollama/blob/main/docs/openai.md",
    },
    ProviderPreset {
        id: "lmstudio",
        name: "LM Studio（本地）",
        adapter: "openai-compatible",
        base_url: "http://localhost:1234/v1",
        api_key_env: None,
        key_required: false,
        docs: "https://lmstudio.ai/docs/app/api",
    },
    ProviderPreset {
        id: "vllm",
        name: "vLLM（本地）",
        adapter: "openai-compatible",
        base_url: "http://localhost:8000/v1",
        api_key_env: None,
        key_required: false,
        docs: "https://docs.vllm.ai/en/latest/serving/openai_compatible_server.html",
    },
    ProviderPreset {
        id: "openrouter",
        name: "OpenRouter",
        adapter: "openai-compatible",
        base_url: "https://openrouter.ai/api/v1",
        api_key_env: Some("OPENROUTER_API_KEY"),
        key_required: true,
        docs: "https://openrouter.ai/docs/quickstart",
    },
    ProviderPreset {
        id: "deepseek",
        name: "DeepSeek",
        adapter: "openai-compatible",
        base_url: "https://api.deepseek.com/v1",
        api_key_env: Some("DEEPSEEK_API_KEY"),
        key_required: true,
        docs: "https://api-docs.deepseek.com",
    },
    ProviderPreset {
        id: "groq",
        name: "Groq",
        adapter: "openai-compatible",
        base_url: "https://api.groq.com/openai/v1",
        api_key_env: Some("GROQ_API_KEY"),
        key_required: true,
        docs: "https://console.groq.com/docs/openai",
    },
    ProviderPreset {
        id: "xai",
        name: "xAI",
        adapter: "openai-compatible",
        base_url: "https://api.x.ai/v1",
        api_key_env: Some("XAI_API_KEY"),
        key_required: true,
        docs: "https://docs.x.ai/docs/api-reference",
    },
    ProviderPreset {
        id: "moonshot",
        name: "Moonshot（Kimi）",
        adapter: "openai-compatible",
        base_url: "https://api.moonshot.cn/v1",
        api_key_env: Some("MOONSHOT_API_KEY"),
        key_required: true,
        docs: "https://platform.moonshot.cn/docs",
    },
    ProviderPreset {
        id: "zhipu",
        name: "智谱 GLM",
        adapter: "openai-compatible",
        base_url: "https://open.bigmodel.cn/api/paas/v4",
        api_key_env: Some("ZHIPUAI_API_KEY"),
        key_required: true,
        docs: "https://open.bigmodel.cn/dev/api",
    },
    ProviderPreset {
        id: "siliconflow",
        name: "SiliconFlow",
        adapter: "openai-compatible",
        base_url: "https://api.siliconflow.cn/v1",
        api_key_env: Some("SILICONFLOW_API_KEY"),
        key_required: true,
        docs: "https://docs.siliconflow.cn",
    },
];

/// 按 id 查预设。
pub fn preset(id: &str) -> Option<&'static ProviderPreset> {
    PRESETS.iter().find(|preset| preset.id == id)
}

/// 某条目用哪个适配器实现。
///
/// `None` = 既不在目录里、也不是适配器名 —— 这种条目必须自己在配置里写 `adapter`
/// （自定义厂商，见 `R1` 的条目形状）。
pub fn adapter_for(id: &str) -> Option<&'static str> {
    if let Some(preset) = preset(id) {
        return Some(preset.adapter);
    }
    ADAPTERS.iter().copied().find(|adapter| *adapter == id)
}

/// 某条目的密钥环境变量名（`None` = 没有约定名，只走钥匙串）。
pub fn api_key_env_for(id: &str) -> Option<&'static str> {
    preset(id).and_then(|preset| preset.api_key_env)
}

/// 展示名：目录里有就用目录的，否则回退成 id 本身。
///
/// 自定义条目（用户自己起的名字）没有像样的展示名，用 id 比"未命名"更可读。
pub fn display_name(id: &str) -> &str {
    preset(id).map_or(id, |preset| preset.name)
}

/// 目录里所有 id（含两个通用适配器条目）。
pub fn ids() -> impl Iterator<Item = &'static str> {
    PRESETS.iter().map(|preset| preset.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_non_empty() {
        for (index, preset) in PRESETS.iter().enumerate() {
            assert!(!preset.id.is_empty(), "空 id 会变成一条谁也选不中的条目");
            assert!(!preset.name.is_empty());
            assert!(!preset.docs.is_empty(), "{} 缺文档来源", preset.id);
            assert!(
                !PRESETS[..index].iter().any(|other| other.id == preset.id),
                "{} 重复",
                preset.id
            );
        }
    }

    #[test]
    fn every_adapter_is_constructible() {
        // 目录里写了目录里没有的适配器 = 运行时才炸（`build_provider` 的 `other` 分支）
        for preset in PRESETS {
            assert!(
                ADAPTERS.contains(&preset.adapter),
                "{} 指向未知适配器 {}",
                preset.id,
                preset.adapter
            );
        }
    }

    #[test]
    fn https_unless_local() {
        for preset in PRESETS {
            let local =
                preset.base_url.contains("localhost") || preset.base_url.contains("127.0.0.1");
            assert!(
                preset.base_url.starts_with("https://") || local,
                "{} 的端点既不是 https 也不是本地地址：{}",
                preset.id,
                preset.base_url
            );
            // 本地服务不该要密钥（要了会挡住零配置直连）
            if local {
                assert!(!preset.key_required, "{} 是本地的却要求密钥", preset.id);
            }
        }
    }

    #[test]
    fn adapter_ids_are_themselves_presets() {
        // 老配置写的就是这两个 id：它们必须在目录里，否则分组名只能退回 id
        for adapter in ADAPTERS {
            let preset = preset(adapter).expect("适配器 id 必须在目录里");
            assert_eq!(preset.adapter, adapter, "通用条目的适配器应当是它自己");
        }
    }

    #[test]
    fn lookup_and_fallbacks() {
        assert_eq!(
            preset("ollama").map(|it| it.adapter),
            Some("openai-compatible")
        );
        assert_eq!(adapter_for("ollama"), Some("openai-compatible"));
        assert_eq!(adapter_for("anthropic"), Some("anthropic"));
        // 适配器名本身（老配置的写法）也能推断出适配器
        assert_eq!(adapter_for("openai-compatible"), Some("openai-compatible"));

        // 未知条目：必须显式写 `adapter`，这里推断不出来是有意的
        assert_eq!(adapter_for("my-private-gateway"), None);
        assert_eq!(api_key_env_for("openrouter"), Some("OPENROUTER_API_KEY"));
        assert_eq!(api_key_env_for("ollama"), None);
        assert_eq!(api_key_env_for("my-private-gateway"), None);

        // 展示名：目录里有就用目录的，没有就回退 id
        assert_eq!(display_name("ollama"), "Ollama（本地）");
        assert_eq!(display_name("my-private-gateway"), "my-private-gateway");
    }

    #[test]
    fn ids_iterates_the_whole_catalog() {
        assert_eq!(ids().count(), PRESETS.len());
        assert!(ids().any(|id| id == "ollama"));
    }
}
