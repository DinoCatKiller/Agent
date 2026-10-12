//! 依赖装配（`chat` / `tui` 两个子命令共用）：供应商构造、密钥来源、demo 工具。
//!
//! 组装是 `app` 的职责（`README` 边界）；业务逻辑零实现——都在 `features/*`。
//!
//! 密钥来源（`R1` 落地前的过渡约定）：`--api-key` 参数或按 provider 推断的环境变量
//! （`OPENAI_API_KEY` / `ANTHROPIC_API_KEY`）。**密钥不入库、不打日志**（硬约束 5）。

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use agent_chat::{Tool, ToolOutput, ToolSet};
use agent_common::ToolDefinition;
use agent_config::{AppConfig, ConfigPaths, ExposeSecret, SecretString};
use agent_providers::{
    AnthropicCompatible, AnthropicConfig, ErasedProvider, OpenAiCompatible, OpenAiConfig,
};
use agent_transport::{HttpConfig, RetryPolicy};
use serde_json::{Value, json};

/// `chat` / `tui` 共用的供应商参数。
#[derive(Debug, Clone)]
pub struct ChatArgs {
    /// `openai-compatible` 或 `anthropic`。
    pub provider: String,
    /// 主模型；`--model a,b` 时 `a` 在前、`b` 起为降级链（`R2`）。
    pub model: String,
    /// 降级候选（不含主模型本身）。
    pub fallbacks: Vec<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    /// 配置文件路径（`--config`）；缺省走平台标准目录（`R1`）。
    pub config: Option<String>,
}

/// `--provider` / `--model` 都可省的 CLI 参数（其余同 [`ChatArgs`]）。
#[derive(Debug, Clone, Default)]
pub struct ChatCli {
    pub provider: Option<String>,
    /// 逗号分隔的主模型 + 降级链；空 = 交给配置文件（`R1`）。
    pub model: Vec<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub config: Option<String>,
}

/// 把 CLI 参数解析成**生效的** [`ChatArgs`]。
///
/// 解析顺序（`R1` §优先级，2026-10-12 定）：
///
/// | 项 | 顺序 |
/// |----|------|
/// | provider | `--provider` > 配置文件 `default_provider` > **报错** |
/// | model | `--model` 首个 > `providers.<provider>.model` > `default_model` > **报错** |
///
/// 为什么缺了要报错而不是随便挑一个：模型选错是**真花钱**的事，不能猜（`A2` §2 的精神）。
/// 这也是「配置文件里的 `default_*` 终于有人读」的地方 —— 在此之前它们只写不读，
/// 因为 CLI 曾把 provider/model 设成必填，默认值永远轮不到。
pub fn resolve(cli: &ChatCli) -> Result<ChatArgs, String> {
    let config = load_config(cli.config.as_deref());

    let provider = match cli
        .provider
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .or_else(|| config.default_provider.clone())
    {
        Some(provider) => provider,
        None => {
            return Err(
                "没给供应商：加 --provider，或在配置文件里设 default_provider（可用：openai-compatible / anthropic）"
                    .to_string(),
            );
        }
    };

    let mut models = cli
        .model
        .iter()
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty());
    let first = models.next();
    let fallbacks: Vec<String> = models.collect();
    let model = match first
        .or_else(|| config.model_for(&provider).map(str::to_string))
        .or_else(|| config.default_model.clone())
    {
        Some(model) => model,
        None => {
            return Err(format!(
                "没给模型：加 --model，或在配置文件里设 default_model 或 providers.{provider}.model"
            ));
        }
    };

    Ok(ChatArgs {
        provider,
        model,
        fallbacks,
        base_url: cli.base_url.clone(),
        api_key: cli.api_key.clone(),
        config: cli.config.clone(),
    })
}

pub fn build_provider(args: &ChatArgs) -> Option<Arc<dyn ErasedProvider>> {
    let config = load_config(args.config.as_deref());
    // 密钥优先级（`X1` §4）：CLI 参数 > 环境变量 > 系统钥匙串。
    let api_key = resolve_key(args);

    let mut http = HttpConfig::default();
    if let Some(proxy) = &config.proxy {
        http.proxy = Some(proxy.clone());
    }
    if let Some(secs) = config.stream_idle_timeout_secs {
        http.stream_idle_timeout = Duration::from_secs(secs);
    }
    // 自定义端点（CLI 或配置）时不拦模型清单，交服务端报错（vLLM 等）。
    let custom_base = args.base_url.is_some() || config.base_url_for(&args.provider).is_some();
    // 端点优先级：CLI > 配置（供应商级 → 顶层） > 内置默认。
    let base_url = |default: &str| -> String {
        args.base_url
            .clone()
            .or_else(|| config.base_url_for(&args.provider).map(str::to_string))
            .unwrap_or_else(|| default.to_string())
    };

    let provider: Arc<dyn ErasedProvider> = match args.provider.as_str() {
        "openai-compatible" => Arc::new(
            OpenAiCompatible::new(OpenAiConfig {
                id: "openai-compatible",
                base_url: base_url("https://api.openai.com/v1"),
                api_key: expose(&api_key),
                http: http.clone(),
                models: if custom_base {
                    Vec::new()
                } else {
                    agent_providers::openai_default_models()
                },
            })
            .ok()?,
        ),
        "anthropic" => Arc::new(
            AnthropicCompatible::new(AnthropicConfig {
                id: "anthropic",
                base_url: base_url("https://api.anthropic.com"),
                api_key: expose(&api_key),
                http,
                models: if custom_base {
                    Vec::new()
                } else {
                    agent_providers::anthropic_default_models()
                },
            })
            .ok()?,
        ),
        other => {
            eprintln!("未知 provider：{other}（可用：openai-compatible / anthropic）");
            return None;
        }
    };
    if api_key.is_none() {
        println!(
            "提示：未提供密钥（--api-key / 环境变量 {} / 系统钥匙串）。无鉴权端点（如本地服务）可忽略。",
            agent_config::api_key_env(&args.provider)
        );
    }
    // M6（`R2`）：请求统一经 Router 出——单候选时无感；`--model a,b` 时
    // a 遇可降级失败（429/5xx/超时/断网/Auth）自动落到 b。
    // A6：先在同一候选内退避重试（429/5xx/超时/断网），额度用尽才换下一家。
    let provider_id = provider.id();
    let mut router = agent_routing::Router::new()
        .with_erased(provider)
        .with_retry(RetryPolicy::default());
    if !args.fallbacks.is_empty() {
        let mut chain = vec![(provider_id.to_string(), args.model.clone())];
        chain.extend(
            args.fallbacks
                .iter()
                .map(|model| (provider_id.to_string(), model.clone())),
        );
        router = router.with_route(args.model.clone(), chain);
    } else if custom_base {
        // 自定义端点（vLLM / 本地服务 / 代理）：我们**故意**不拦截模型清单（上面 `models: Vec::new()`），
        // 但路由表也就没有可查的模型 → 必须显式登一条，否则请求死在
        // 「no route for model」（README 说的「不拦清单」在这里才真正成立）。
        // 只登当前这个模型：自定义端点下模型总是显式给出的（CLI `--model` 或配置文件）。
        router = router.with_route(
            args.model.clone(),
            vec![(provider_id.to_string(), args.model.clone())],
        );
    }
    Some(Arc::new(router) as Arc<dyn ErasedProvider>)
}

/// 加载配置文件。`--config` 显式指定则用它，否则走平台标准目录（`R1`）。
///
/// 任何失败都**降级为默认配置并提示**：配置问题不该挡在「跑起来」前面。
fn load_config(explicit: Option<&str>) -> AppConfig {
    let paths = match explicit {
        Some(path) => ConfigPaths::from_file(path),
        None => match ConfigPaths::discover() {
            Ok(paths) => paths,
            Err(error) => {
                eprintln!("定位配置目录失败（{error}），改用默认配置。");
                return AppConfig::default();
            }
        },
    };
    match agent_config::load(&paths) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("读取配置失败（{error}），改用默认配置。");
            AppConfig::default()
        }
    }
}

/// 密钥优先级（`X1` §4）：CLI 参数 > 环境变量 > 系统钥匙串。
fn resolve_key(args: &ChatArgs) -> Option<SecretString> {
    if let Some(raw) = &args.api_key {
        return Some(SecretString::from(raw.clone()));
    }
    match agent_config::resolve_api_key(&args.provider) {
        Ok(secret) => secret,
        Err(error) => {
            eprintln!("读取密钥失败（{error}）。");
            None
        }
    }
}

/// 在**装配边界**把密钥解出明文交给适配器（适配器要用它构造鉴权头）。
///
/// 明文只活在这个局部值与适配器配置中：不进日志、不入库（`X1` §4 / 硬约束 5）。
fn expose(key: &Option<SecretString>) -> Option<String> {
    key.as_ref()
        .map(|secret| secret.expose_secret().to_string())
}

/// M4 的两个演示工具：证明工具循环端到端可用。真实工具集由配置层/宿主提供（M6）。
pub fn demo_tools() -> ToolSet {
    ToolSet::new()
        .with(Arc::new(CurrentTime))
        .with(Arc::new(Echo))
}

struct CurrentTime;

impl Tool for CurrentTime {
    fn def(&self) -> ToolDefinition {
        ToolDefinition {
            name: "current_time".into(),
            description: "获取当前 Unix 时间戳（秒）".into(),
            parameters: json!({"type": "object", "properties": {}}),
        }
    }

    fn run(&self, _arguments: Value) -> futures::future::BoxFuture<'static, ToolOutput> {
        Box::pin(async {
            let secs = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            Ok(json!({ "unix_seconds": secs }))
        })
    }
}

struct Echo;

impl Tool for Echo {
    fn def(&self) -> ToolDefinition {
        ToolDefinition {
            name: "echo".into(),
            description: "原样返回传入的参数，用于验证工具链路".into(),
            parameters: json!({"type": "object"}),
        }
    }

    fn run(&self, arguments: Value) -> futures::future::BoxFuture<'static, ToolOutput> {
        Box::pin(async move { Ok(json!({ "echo": arguments })) })
    }
}

#[cfg(test)]
mod tests {
    //! `resolve` 的解析顺序（`R1`）：只读临时配置文件，**不碰钥匙串、不联网**（`Q1`）。

    use super::*;

    fn cli_at(config: &std::path::Path) -> ChatCli {
        ChatCli {
            config: Some(config.display().to_string()),
            ..ChatCli::default()
        }
    }

    fn write_config(name: &str, body: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "agent-app-setup-{}-{name}.toml",
            std::process::id()
        ));
        std::fs::write(&path, body).expect("写临时配置");
        path
    }

    #[test]
    fn cli_beats_config() {
        let path = write_config(
            "cli-wins",
            "default_provider = \"anthropic\"\ndefault_model = \"c1\"\n",
        );
        let mut cli = cli_at(&path);
        cli.provider = Some("openai-compatible".into());
        cli.model = vec!["m1".into(), "m2".into()];

        let args = resolve(&cli).expect("解析");
        assert_eq!(args.provider, "openai-compatible");
        assert_eq!(args.model, "m1");
        assert_eq!(args.fallbacks, vec!["m2".to_string()]);
    }

    #[test]
    fn config_defaults_fill_in_when_cli_silent() {
        let path = write_config(
            "defaults",
            "default_provider = \"anthropic\"\ndefault_model = \"claude-x\"\n",
        );
        let args = resolve(&cli_at(&path)).expect("解析");
        assert_eq!(args.provider, "anthropic");
        assert_eq!(args.model, "claude-x");
        // 降级链只由 CLI 的 `--model a,b` 表达（`R2`），配置默认不带
        assert!(args.fallbacks.is_empty());
    }

    #[test]
    fn provider_level_model_beats_top_level() {
        let path = write_config(
            "provider-model",
            "default_provider = \"openai-compatible\"\ndefault_model = \"top\"\n\n[providers.openai-compatible]\nmodel = \"per-provider\"\n",
        );
        let args = resolve(&cli_at(&path)).expect("解析");
        assert_eq!(args.model, "per-provider");
    }

    #[test]
    fn missing_provider_and_model_are_errors() {
        let path = write_config("empty", "");
        let provider_error = resolve(&cli_at(&path)).expect_err("没有 provider 应当报错");
        assert!(
            provider_error.contains("default_provider"),
            "错误提示应指路：{provider_error}"
        );

        let mut cli = cli_at(&path);
        cli.provider = Some("anthropic".into());
        let model_error = resolve(&cli).expect_err("没有 model 应当报错");
        assert!(
            model_error.contains("default_model"),
            "错误提示应指路：{model_error}"
        );
    }

    #[test]
    fn blank_cli_values_fall_through_to_config() {
        let path = write_config(
            "blank",
            "default_provider = \"anthropic\"\ndefault_model = \"c1\"\n",
        );
        let mut cli = cli_at(&path);
        cli.provider = Some("   ".into());
        cli.model = vec!["  ".into()];

        let args = resolve(&cli).expect("空白等于没给");
        assert_eq!(args.provider, "anthropic");
        assert_eq!(args.model, "c1");
    }
}
