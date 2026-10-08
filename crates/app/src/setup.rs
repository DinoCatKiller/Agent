//! 依赖装配（`chat` / `tui` 两个子命令共用）：供应商构造、密钥来源、demo 工具。
//!
//! 组装是 `app` 的职责（`README` 边界）；业务逻辑零实现——都在 `features/*`。
//!
//! 密钥来源（`R1` 落地前的过渡约定）：`--api-key` 参数或按 provider 推断的环境变量
//! （`OPENAI_API_KEY` / `ANTHROPIC_API_KEY`）。**密钥不入库、不打日志**（硬约束 5）。

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use agent_chat::{Tool, ToolOutput, ToolSet};
use agent_common::ToolDefinition;
use agent_providers::{
    AnthropicCompatible, AnthropicConfig, ErasedProvider, OpenAiCompatible, OpenAiConfig,
};
use agent_transport::HttpConfig;
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
}

pub fn build_provider(args: &ChatArgs) -> Option<Arc<dyn ErasedProvider>> {
    let api_key = args
        .api_key
        .clone()
        .or_else(|| api_key_from_env(&args.provider));
    let http = HttpConfig::default();
    let provider: Arc<dyn ErasedProvider> = match args.provider.as_str() {
        "openai-compatible" => {
            let base_url = args
                .base_url
                .clone()
                .unwrap_or_else(|| "https://api.openai.com/v1".into());
            Arc::new(
                OpenAiCompatible::new(OpenAiConfig {
                    id: "openai-compatible",
                    base_url,
                    api_key,
                    http,
                    // 未显式传清单时用官方默认；自定义 base_url（vLLM 等）应自行登记模型。
                    models: if args.base_url.is_some() {
                        Vec::new()
                    } else {
                        agent_providers::openai_default_models()
                    },
                })
                .ok()?,
            )
        }
        "anthropic" => {
            let base_url = args
                .base_url
                .clone()
                .unwrap_or_else(|| "https://api.anthropic.com".into());
            Arc::new(
                AnthropicCompatible::new(AnthropicConfig {
                    id: "anthropic",
                    base_url,
                    api_key,
                    http,
                    models: if args.base_url.is_some() {
                        Vec::new()
                    } else {
                        agent_providers::anthropic_default_models()
                    },
                })
                .ok()?,
            )
        }
        other => {
            eprintln!("未知 provider：{other}（可用：openai-compatible / anthropic）");
            return None;
        }
    };
    if args.api_key.is_none() && std::env::var(api_key_env(&args.provider)).is_err() {
        println!("提示：未提供密钥（--api-key 或对应环境变量）。无鉴权端点（如本地服务）可忽略。");
    }
    // M6（`R2`）：请求统一经 Router 出——单候选时无感；`--model a,b` 时
    // a 遇可降级失败（429/5xx/超时/断网/Auth）自动落到 b。
    let provider_id = provider.id();
    let mut router = agent_routing::Router::new().with_erased(provider);
    if !args.fallbacks.is_empty() {
        let mut chain = vec![(provider_id.to_string(), args.model.clone())];
        chain.extend(
            args.fallbacks
                .iter()
                .map(|model| (provider_id.to_string(), model.clone())),
        );
        router = router.with_route(args.model.clone(), chain);
    }
    Some(Arc::new(router) as Arc<dyn ErasedProvider>)
}

/// 密钥环境变量约定（`R1` 落地前的过渡）：按 provider id 推断变量名。
fn api_key_env(provider: &str) -> &'static str {
    match provider {
        "anthropic" => "ANTHROPIC_API_KEY",
        _ => "OPENAI_API_KEY",
    }
}

fn api_key_from_env(provider: &str) -> Option<String> {
    std::env::var(api_key_env(provider))
        .ok()
        .filter(|k| !k.is_empty())
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
