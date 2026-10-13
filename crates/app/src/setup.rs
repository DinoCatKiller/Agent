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

// 设置页的供应商行（只随 `gui` 拉进来：`agent-settings` 本身是可选依赖）。
#[cfg(feature = "gui")]
use agent_settings::ui::ProviderRow;

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
    // 条目 → 生效配置（适配器 / 端点 / 模型清单要不要拦）全在 `kernel/providers` 的目录里算。
    // `app` 只负责把三层来源（CLI / 配置 / 顶层兜底）摆进去 —— 与 `model_choices` /
    // `provider_rows` 用的是**同一个**摆法（`declared_for`），不会出现"能列出来但连不上"。
    let resolved = match agent_providers::catalog::resolve(
        &args.provider,
        declared_for(&config, &args.provider, args),
    ) {
        Ok(resolved) => resolved,
        Err(error) => {
            eprintln!("错误：{error}");
            return None;
        }
    };

    let models = if resolved.custom_base {
        // 自定义端点（vLLM / 本地服务 / 代理）：**不拦**模型清单，交服务端报错
        Vec::new()
    } else {
        builtin_models(resolved.adapter)
    };
    let base_url = resolved.base_url.clone();

    let provider: Arc<dyn ErasedProvider> = match resolved.adapter {
        "openai-compatible" => Arc::new(
            OpenAiCompatible::new(OpenAiConfig {
                // 注意：这是**适配器**的 id，不是条目 id（`ollama` 也走这一支）。
                // 它只进路由/观测的标签，不参与任何查找。
                id: "openai-compatible",
                base_url,
                api_key: expose(&api_key),
                http: http.clone(),
                models,
            })
            .ok()?,
        ),
        _ => Arc::new(
            AnthropicCompatible::new(AnthropicConfig {
                id: "anthropic",
                base_url,
                api_key: expose(&api_key),
                http,
                models,
            })
            .ok()?,
        ),
    };
    if api_key.is_none() {
        match resolved.api_key_env {
            Some(env) => println!(
                "提示：未提供密钥（--api-key / 环境变量 {env} / 系统钥匙串）。无鉴权端点（如本地服务）可忽略。"
            ),
            None => println!(
                "提示：未提供密钥（--api-key / 系统钥匙串）。条目 {} 没有约定环境变量名，无鉴权端点可忽略。",
                resolved.id
            ),
        }
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
    } else if resolved.custom_base {
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
    // 环境变量名由**目录**给（厂商知识归 `kernel/providers`，`D6`）：
    // `config` 只负责「先环境变量、后钥匙串」这个顺序。
    let env = agent_providers::catalog::api_key_env_for(&args.provider);
    match agent_config::resolve_api_key(&args.provider, env) {
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

/// 把三层来源摆成目录要的 [`agent_providers::catalog::Declared`]。
///
/// **CLI 的 `--base-url` 只作用于生效的那个条目**（`id == args.provider`）：切到别家还带着它，
/// 等于把 A 家的端点发给 B 家 —— 跟 `--api-key` 是同一类串味（见 `worker::ProviderFactory`）。
///
/// `build_provider` / `model_choices` / `provider_rows` 共用这一个摆法，
/// 于是不会出现「界面里列得出来，但连不上」。
fn declared_for<'a>(
    config: &'a AppConfig,
    id: &str,
    args: &'a ChatArgs,
) -> agent_providers::catalog::Declared<'a> {
    let entry = config.providers.get(id);
    agent_providers::catalog::Declared {
        adapter: entry.and_then(|it| it.adapter.as_deref()),
        base_url: entry.and_then(|it| it.base_url.as_deref()),
        fallback_base_url: config.base_url.as_deref(),
        cli_base_url: if id == args.provider {
            args.base_url.as_deref()
        } else {
            None
        },
    }
}

/// 某个适配器的内置模型清单 —— 只在「用的就是它的官方端点」时才有意义（`R2`）。
fn builtin_models(adapter: &str) -> Vec<agent_common::ModelSpec> {
    match adapter {
        "anthropic" => agent_providers::anthropic_default_models(),
        _ => agent_providers::openai_default_models(),
    }
}

/// 可切换的条目 id：两个通用适配器 ∪ 配置里**已连接**的 ∪ 当前生效的。
fn entry_ids(config: &AppConfig, args: &ChatArgs) -> Vec<String> {
    let mut ids: Vec<String> = agent_providers::catalog::ADAPTERS
        .iter()
        .map(|id| (*id).to_string())
        .collect();
    for id in config.providers.keys() {
        if !ids.contains(id) {
            ids.push(id.clone());
        }
    }
    if !ids.contains(&args.provider) {
        ids.push(args.provider.clone());
    }
    ids
}

/// 模型选择器的候选清单：`(条目 id, model)`。
///
/// **来源 = 每个可切换条目的内置清单 ∪ 配置里为它声明的模型 ∪ 当前生效的那一对**：
///
/// - 内置清单：只有「用的是该适配器官方端点」时才列 —— `gpt-4o` 对 Ollama / vLLM 没有意义，
///   与服务端那边的口径一致（`build_provider` 的 `custom_base`）；
/// - 配置声明：`providers.<条目>.model` 归该条目；顶层 `default_model` **只归生效的那个**
///   （否则同一个模型名会被挂到每个条目上）；
/// - 当前生效：解析出的那一对永远在列（否则切走之后选不回来）。
///
/// 放在 `app` 是因为「哪个条目用哪个适配器、哪份清单」是**装配层 + 目录**的知识 —
/// `features/*` 不认识供应商名单（`D6`）。
///
/// 只有 `gui` 用它（TUI 的模型由 `--model` / 配置决定），故随 feature 走。
#[cfg(feature = "gui")]
pub fn model_choices(args: &ChatArgs) -> Vec<(String, String)> {
    /// 追加（跳过空名与重复）。
    fn push(out: &mut Vec<(String, String)>, provider: &str, model: &str) {
        let entry = (provider.to_string(), model.to_string());
        if !model.is_empty() && !out.contains(&entry) {
            out.push(entry);
        }
    }

    let config = load_config(args.config.as_deref());
    let mut out: Vec<(String, String)> = Vec::new();

    for id in entry_ids(&config, args) {
        // 解析不了的条目不该挡界面：跳过（真正的原因在启动时那句错误里）
        let Ok(resolved) = agent_providers::catalog::resolve(&id, declared_for(&config, &id, args))
        else {
            continue;
        };
        if !resolved.custom_base {
            for spec in builtin_models(resolved.adapter) {
                push(&mut out, &id, &spec.id);
            }
        }
        if let Some(model) = config.providers.get(&id).and_then(|it| it.model.as_deref()) {
            push(&mut out, &id, model);
        }
        if id == args.provider
            && let Some(model) = config.default_model.as_deref()
        {
            push(&mut out, &id, model);
        }
    }
    push(&mut out, &args.provider, &args.model);

    out
}

/// 设置页要列出的供应商条目（id / 展示名 / 约定环境变量名 / 是否必须密钥）。
///
/// 来源与 [`entry_ids`] 一致。不列整张目录：那是「连接提供商」页的活（切片 C1c），
/// 设置页只展示"已经在用的"。
///
/// 环境变量名与「要不要密钥」都由目录算（`catalog::resolve`）——
/// 自定义条目写 `adapter = "anthropic"` 时，`ANTHROPIC_API_KEY` 仍然生效。
#[cfg(feature = "gui")]
pub fn provider_rows(args: &ChatArgs) -> Vec<ProviderRow> {
    let config = load_config(args.config.as_deref());

    entry_ids(&config, args)
        .into_iter()
        .filter_map(|id| {
            let resolved =
                agent_providers::catalog::resolve(&id, declared_for(&config, &id, args)).ok()?;
            Some(ProviderRow {
                name: agent_providers::catalog::display_name(&id).to_string(),
                env: resolved.api_key_env.map(str::to_string),
                key_required: resolved.key_required,
                id,
            })
        })
        .collect()
}

/// 读 UI 偏好（`models.toml`）。与 [`load_config`] 同样的容错：定位/读取失败都退回默认值
/// （偏好不该挡启动，理由见 `agent_config::prefs`）。
#[cfg(feature = "gui")]
pub fn load_preferences(explicit: Option<&str>) -> agent_config::Preferences {
    let paths = match explicit {
        Some(path) => ConfigPaths::from_file(path),
        None => match ConfigPaths::discover() {
            Ok(paths) => paths,
            Err(_) => return agent_config::Preferences::default(),
        },
    };
    agent_config::Preferences::load(&paths)
}

/// 选择器里**可见**的候选 = 全量 − 用户在「管理模型」里关掉的。
///
/// **当前生效的那个永远可见**：否则它切走之后就再也选不回来了 ——
/// 那比"菜单里多一项"糟得多。
#[cfg(feature = "gui")]
pub fn visible_choices(
    all: &[(String, String)],
    prefs: &agent_config::Preferences,
    current_provider: &str,
    current_model: &str,
) -> Vec<(String, String)> {
    all.iter()
        .filter(|(provider, model)| {
            (provider == current_provider && model == current_model)
                || !prefs.is_hidden(provider, model)
        })
        .cloned()
        .collect()
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
