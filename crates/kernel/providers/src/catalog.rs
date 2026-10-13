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

/// 适配器的**官方端点** —— 用来判断「用户是不是用了自定义端点」。
///
/// 直接从目录里同名的条目取，不另写一份字面量（两个适配器 id 本身就在目录里，有测试钉着）。
pub fn adapter_default_base_url(adapter: &str) -> Option<&'static str> {
    preset(adapter).map(|preset| preset.base_url)
}

/// 是不是本地地址（`localhost` / `127.0.0.1` / `[::1]` / `0.0.0.0`）。
///
/// **只用于界面提示「要不要密钥」**：不进注册表、不进路由，不引入「本地/云端」维度
/// （`D7` 方案 C 不变）。
pub fn is_local(base_url: &str) -> bool {
    let host = base_url.split("://").nth(1).unwrap_or(base_url);
    let host = host.split('/').next().unwrap_or(host);
    // IPv6 是 `[::1]:port`，先整体看一眼再去端口（免得把 `[::1]` 切成 `[`）
    let host = if host.starts_with('[') {
        host.split(']')
            .next()
            .map_or(host, |it| &host[..it.len() + 1])
    } else {
        host.split(':').next().unwrap_or(host)
    };
    matches!(host, "localhost" | "127.0.0.1" | "0.0.0.0" | "[::1]")
}

/// 配置里对某个条目的声明 —— 目录要的输入（`app` 把 CLI / 配置 / 顶层兜底摆进来）。
///
/// 生命周期同一个：调用方持有 `AppConfig` 与 `ChatArgs`，两者都活得比本结构久。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Declared<'a> {
    /// 用哪个适配器。**目录里没有的 id 必填**；目录里有的可以省。
    pub adapter: Option<&'a str>,
    /// 条目自己的端点（覆盖目录默认）。
    pub base_url: Option<&'a str>,
    /// 顶层 `base_url` —— 老配置的兜底写法，对任何条目都生效（`R1` §1）。
    pub fallback_base_url: Option<&'a str>,
    /// CLI 的 `--base-url`。**只对生效的那个条目有意义**，由调用方自己把关：
    /// 切换后的条目不该带着 CLI 的端点（跟 `--api-key` 是同一类串味）。
    pub cli_base_url: Option<&'a str>,
}

/// 生效配置：CLI / 配置 / 目录三层合成之后的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// 条目 id（用户写的那个）。
    pub id: String,
    /// 用哪个适配器实现。
    pub adapter: &'static str,
    /// 生效端点。
    pub base_url: String,
    /// `true` = 用的**不是**该适配器的官方端点 → 模型清单不由我们提供，交服务端报错
    /// （`R2` 的「不拦清单」）。
    pub custom_base: bool,
    /// 约定环境变量名（`None` = 没有约定名，只看钥匙串）。
    pub api_key_env: Option<&'static str>,
    /// 通常需要密钥吗。**只是界面提示**，不参与任何校验或路由。
    pub key_required: bool,
}

/// 解析条目失败的原因。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    /// 显式写了 `adapter`，但那个适配器不存在。
    UnknownAdapter { id: String, adapter: String },
    /// 目录里没有这个 id，配置里也没写 `adapter`。
    MissingAdapter { id: String },
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownAdapter { id, adapter } => write!(
                f,
                "条目 `{id}` 写了未知适配器 `{adapter}`（可用：{}）",
                ADAPTERS.join(" / ")
            ),
            Self::MissingAdapter { id } => write!(
                f,
                "未知供应商 `{id}`：它不在内置目录里，请为它写 `adapter`（可用：{}），\
                 或直接用目录里的名字（如 ollama / openrouter / deepseek）",
                ADAPTERS.join(" / ")
            ),
        }
    }
}

impl std::error::Error for ResolveError {}

/// 解析一个条目的生效配置。
///
/// 三条规则：
///
/// | 项 | 顺序 |
/// |----|------|
/// | adapter | 配置里显式写的 > 目录推断（目录 id / 适配器 id）> **报错**（不猜） |
/// | base_url | CLI > 条目 > 顶层兜底 > 目录 > 适配器官方端点 |
/// | api_key_env | 条目自己的 > 它所用适配器的 |
pub fn resolve(id: &str, declared: Declared<'_>) -> Result<Resolved, ResolveError> {
    let adapter = match declared.adapter.map(str::trim).filter(|it| !it.is_empty()) {
        Some(explicit) => ADAPTERS
            .iter()
            .copied()
            .find(|it| *it == explicit)
            .ok_or_else(|| ResolveError::UnknownAdapter {
                id: id.to_string(),
                adapter: explicit.to_string(),
            })?,
        // 目录里有的（含两个适配器 id 本身）才推断得出来；推断不出就报错，不自作主张
        None => {
            adapter_for(id).ok_or_else(|| ResolveError::MissingAdapter { id: id.to_string() })?
        }
    };

    let adapter_default = adapter_default_base_url(adapter).unwrap_or_default();
    let base_url = declared
        .cli_base_url
        .or(declared.base_url)
        .or(declared.fallback_base_url)
        .map(str::trim)
        .filter(|it| !it.is_empty())
        .or_else(|| preset(id).map(|preset| preset.base_url))
        .unwrap_or(adapter_default)
        .to_string();

    Ok(Resolved {
        id: id.to_string(),
        adapter,
        custom_base: base_url != adapter_default,
        // 环境变量名：**目录里有的条目以目录为准** —— `api_key_env: None` 就是"没有约定名"
        // （本地服务），这时**不能**拿适配器的名字兜底：Ollama 的密钥会变成 `OPENAI_API_KEY`，
        // 等于把你的 OpenAI 密钥发给 ollama.com。回退只给"目录里没有的自定义条目"。
        api_key_env: match preset(id) {
            Some(preset) => preset.api_key_env,
            None => api_key_env_for(adapter),
        },
        key_required: preset(id)
            .map(|preset| preset.key_required)
            // 自定义条目：按「是不是本地地址」猜 —— 本地通常不需要密钥
            .unwrap_or(!is_local(&base_url)),
        base_url,
    })
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

    #[test]
    fn is_local_recognises_the_usual_forms() {
        assert!(is_local("http://localhost:11434/v1"));
        assert!(is_local("http://127.0.0.1:1234/v1"));
        assert!(is_local("http://0.0.0.0:8000/v1"));
        assert!(is_local("http://[::1]:8000/v1"));
        assert!(!is_local("https://api.openai.com/v1"));
        // 主机名里带 localhost 的假阳性不值得防（提示位而已，不进路由）
        assert!(!is_local("https://api.example.com/localhost/v1"));
    }

    #[test]
    fn catalog_entry_resolves_to_its_adapter_and_endpoint() {
        let resolved = resolve("ollama", Declared::default()).expect("目录里的条目");
        assert_eq!(resolved.adapter, "openai-compatible");
        assert_eq!(resolved.base_url, "http://localhost:11434/v1");
        assert!(resolved.custom_base, "本地端点不是适配器的官方端点");
        assert_eq!(resolved.api_key_env, None);
        assert!(!resolved.key_required);
    }

    #[test]
    fn adapter_entries_are_not_custom_base() {
        // 两个通用适配器：端点就是各自的官方端点 → 不是自定义端点 → 用内置模型清单
        for adapter in ADAPTERS {
            let resolved = resolve(adapter, Declared::default()).expect("适配器条目");
            assert_eq!(resolved.adapter, adapter);
            assert!(!resolved.custom_base, "{adapter} 不该被当成自定义端点");
            assert!(resolved.key_required);
        }
    }

    #[test]
    fn base_url_priority_is_cli_then_entry_then_fallback_then_catalog() {
        let entry_only = resolve(
            "deepseek",
            Declared {
                base_url: Some("https://entry.example/v1"),
                ..Declared::default()
            },
        )
        .expect("解析");
        assert_eq!(entry_only.base_url, "https://entry.example/v1");

        let with_fallback = resolve(
            "deepseek",
            Declared {
                fallback_base_url: Some("https://fallback.example/v1"),
                ..Declared::default()
            },
        )
        .expect("解析");
        assert_eq!(with_fallback.base_url, "https://fallback.example/v1");

        let with_cli = resolve(
            "deepseek",
            Declared {
                cli_base_url: Some("https://cli.example/v1"),
                base_url: Some("https://entry.example/v1"),
                fallback_base_url: Some("https://fallback.example/v1"),
                ..Declared::default()
            },
        )
        .expect("解析");
        assert_eq!(with_cli.base_url, "https://cli.example/v1");

        let catalog_only = resolve("deepseek", Declared::default()).expect("解析");
        assert_eq!(catalog_only.base_url, preset("deepseek").unwrap().base_url);
    }

    /// 目录里**明确写了** `api_key_env: None` 的条目，不许回退到适配器的变量名 ——
    /// 否则 Ollama 的密钥会读 `OPENAI_API_KEY`，把你的 OpenAI 密钥发给 ollama.com。
    /// （这个 bug 是裸跑 `codingrocket gui` 时那条提示暴露出来的。）
    #[test]
    fn catalog_entry_does_not_borrow_the_adapter_env_name() {
        for id in ["ollama", "lmstudio", "vllm"] {
            let resolved = resolve(id, Declared::default()).expect("目录里的条目");
            assert_eq!(resolved.adapter, "openai-compatible");
            assert_eq!(resolved.api_key_env, None, "{id} 不该借 OPENAI_API_KEY");
        }
    }

    #[test]
    fn custom_entry_needs_an_adapter_and_borrows_its_env_name() {
        // 没写 adapter 的自定义条目：报错，且文案要指路（说清可用的适配器）
        let error = resolve("my-gateway", Declared::default()).expect_err("推断不出来");
        let text = error.to_string();
        assert!(text.contains("my-gateway"), "报错要带上 id：{text}");
        assert!(text.contains("ollama"), "报错要指路到目录：{text}");

        // 写了 adapter：照它走，环境变量名回退到**适配器**的约定名
        let resolved = resolve(
            "my-gateway",
            Declared {
                adapter: Some("anthropic"),
                base_url: Some("https://my-gateway.example"),
                ..Declared::default()
            },
        )
        .expect("显式 adapter");
        assert_eq!(resolved.adapter, "anthropic");
        assert_eq!(resolved.api_key_env, Some("ANTHROPIC_API_KEY"));
        assert!(resolved.key_required, "远程端点默认要密钥");

        // 本地地址的自定义条目：提示位说"不需要密钥"
        let local = resolve(
            "my-box",
            Declared {
                adapter: Some("openai-compatible"),
                base_url: Some("http://localhost:9000/v1"),
                ..Declared::default()
            },
        )
        .expect("显式 adapter");
        assert!(!local.key_required);
    }

    #[test]
    fn unknown_explicit_adapter_is_rejected() {
        let error = resolve(
            "my-gateway",
            Declared {
                adapter: Some("grpc"),
                ..Declared::default()
            },
        )
        .expect_err("未知适配器");
        assert!(matches!(error, ResolveError::UnknownAdapter { .. }));
        assert!(error.to_string().contains("grpc"));
    }

    #[test]
    fn blank_declarations_are_ignored() {
        // 空字符串等同于没写（设置界面清空输入框会写出空串）
        let resolved = resolve(
            "ollama",
            Declared {
                adapter: Some("   "),
                base_url: Some(""),
                ..Declared::default()
            },
        )
        .expect("空值应当被忽略");
        assert_eq!(resolved.adapter, "openai-compatible");
        assert_eq!(resolved.base_url, preset("ollama").unwrap().base_url);
    }
}
