//! 二进制入口：组装依赖、承载 CLI 命令。**不含**业务逻辑（都在 `features/*` 里）。
//!
//! - `self-check`：契约层冒烟（M1）。
//! - `chat`：对话 REPL（M4）——`agent-chat` 的事件消费参考实现。
//! - `tui`：一期终端界面（M5，`D3`）——侧栏会话 + 流式转录 + 轮末落盘。
//! - `gui`：二期图形界面（`D3` 方案 A）——需 `--features gui` 构建。
//!
//! [`worker`] 是 TUI 与 GPUI **共用**的会话宿主（`Cmd` / `UiEvent` 通道那一侧）。

mod repl;
mod setup;
mod tui;
mod worker;

#[cfg(feature = "gui")]
mod gui;

use clap::{Args, Parser, Subcommand};
use setup::ChatArgs;

/// 命令行入口（`X6` A1：clap 声明式解析，取代原来的手写循环）。
#[derive(Parser)]
#[command(
    // 显示名（Usage / help 里出现）：面向用户用产品名；crate 包名仍为 `agent-app`（`S2` Q7）。
    name = "codingrocket",
    version,
    about = "CodingRocket —— 统一接入多家模型的桌面客户端（命名见 `S2` Q7）",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 契约层冒烟（M1）。
    SelfCheck,
    /// 对话 REPL（M4）。
    Chat(SessionArgs),
    /// 一期终端界面（M5，`D3`）。
    Tui(SessionCliArgs),
    /// 二期图形界面（`D3` 方案 A；需以 `--features gui` 构建）。
    #[cfg(feature = "gui")]
    Gui(SessionCliArgs),
}

/// `chat` / `tui` / `gui` 共用参数。
#[derive(Args, Debug)]
struct SessionArgs {
    /// 供应商标识：openai-compatible | anthropic。
    /// 省略时取配置文件 `default_provider`（`R1`）。
    #[arg(long)]
    provider: Option<String>,
    /// 主模型；逗号分隔（或重复传入）时，`b…` 为降级链（`R2`）。
    /// 省略时取 `providers.<provider>.model`，再回退到 `default_model`（`R1`）。
    #[arg(long, value_delimiter = ',')]
    model: Vec<String>,
    /// 自定义端点（vLLM / 代理等）。
    #[arg(long)]
    base_url: Option<String>,
    /// 密钥；缺省按「环境变量 → 系统钥匙串」解析（`R1`）。
    #[arg(long)]
    api_key: Option<String>,
    /// 配置文件路径；缺省走平台标准目录（`R1`）。
    #[arg(long)]
    config: Option<String>,
}

/// `tui` / `gui` 共用参数（都是「带会话库的完整界面」）。
#[derive(Args, Debug)]
struct SessionCliArgs {
    #[command(flatten)]
    session: SessionArgs,
    /// SQLite 路径（缺省 `./agent-sessions.db`）。
    #[arg(long)]
    db: Option<String>,
}

fn main() {
    // `R3`：日志初始化（级别读 `RUST_LOG`，输出 stderr；幂等）
    agent_telemetry::init_from_env();
    match Cli::parse().command {
        Command::SelfCheck => self_check(),
        Command::Chat(args) => {
            let Some(chat_args) = session_args_into_chat(args) else {
                std::process::exit(2);
            };
            let code = tokio::runtime::Runtime::new()
                .expect("tokio runtime")
                .block_on(repl::run(chat_args));
            std::process::exit(code);
        }
        Command::Tui(args) => {
            let Some(chat_args) = session_args_into_chat(args.session) else {
                std::process::exit(2);
            };
            let code = tokio::runtime::Runtime::new()
                .expect("tokio runtime")
                .block_on(tui::run(tui::TuiArgs {
                    provider: chat_args.provider,
                    model: chat_args.model,
                    fallbacks: chat_args.fallbacks,
                    base_url: chat_args.base_url,
                    api_key: chat_args.api_key,
                    config: chat_args.config,
                    db: args.db,
                }));
            std::process::exit(code);
        }
        #[cfg(feature = "gui")]
        Command::Gui(args) => {
            let Some(chat_args) = session_args_into_chat(args.session) else {
                std::process::exit(2);
            };
            // `gui::run` 是同步的：GPUI 的 `application().run` 自己占住主线程，不能 block_on
            let code = gui::run(gui::GuiArgs {
                provider: chat_args.provider,
                model: chat_args.model,
                fallbacks: chat_args.fallbacks,
                base_url: chat_args.base_url,
                api_key: chat_args.api_key,
                config: chat_args.config,
                db: args.db,
            });
            std::process::exit(code);
        }
    }
}

/// CLI 参数 → **生效的** `ChatArgs`。
///
/// 解析策略只有一处（[`setup::resolve`]）：`--provider` / `--model` 都可省，
/// 省了就从配置文件的 `default_provider` / `providers.<p>.model` / `default_model` 取（`R1`）；
/// 两边都没有 → 报错退出（模型选错是真花钱的事，不能猜）。
fn session_args_into_chat(args: SessionArgs) -> Option<ChatArgs> {
    let cli = setup::ChatCli {
        provider: args.provider,
        model: args.model,
        base_url: args.base_url,
        api_key: args.api_key,
        config: args.config,
    };
    match setup::resolve(&cli) {
        Ok(chat_args) => Some(chat_args),
        Err(message) => {
            eprintln!("错误：{message}");
            None
        }
    }
}

use agent_common::{
    Capability, DeltaKind, ErrorCategory, FinishReason, Message, ModelRequest, ModelSpec, Pricing,
    ProviderError, ResponseFormat, StreamEvent, ToolCall, ToolChoice, ToolDefinition, Usage,
};
use agent_providers::{ProviderRegistry, required_capabilities};
use serde_json::json;

fn self_check() {
    println!("== 1. 消息模型 ==");
    let messages = vec![
        Message::system("你是一个简洁的助手。"),
        Message::user("用一句话说明 SSE 流式的好处。"),
    ];
    println!("用户消息文本: {}", messages[1].text());

    println!("\n== 2. 请求（工具 + 结构化输出）==");
    let mut req = ModelRequest::new("gpt-4o-mini", messages);
    req.temperature = Some(0.3);
    req.max_tokens = Some(256);
    req.response_format = Some(ResponseFormat::JsonObject);
    req.tools = vec![ToolDefinition {
        name: "get_weather".into(),
        description: "查询某城市当前天气".into(),
        parameters: json!({
            "type": "object",
            "properties": { "city": { "type": "string" } },
            "required": ["city"],
            "additionalProperties": false
        }),
    }];
    req.tool_choice = Some(ToolChoice::Auto);
    println!("{}", serde_json::to_string_pretty(&req).unwrap());

    println!("\n== 3. 能力协商（路由阶段 fast-fail）==");
    let spec = ModelSpec {
        id: "gpt-4o-mini".into(),
        provider: "openai-compatible".into(),
        context_window: 128_000,
        max_output: 16_384,
        capabilities: vec![Capability::Text, Capability::Tools, Capability::JsonMode],
        pricing: Some(Pricing {
            input_per_mtok: 0.15,
            output_per_mtok: 0.60,
            currency: "USD".into(),
        }),
        deprecated: false,
        capabilities_known: true,
    };
    let required = required_capabilities(&req);
    let missing = spec.missing(&required);
    println!("请求需要: {required:?}");
    println!("模型支持: {:?}", spec.capabilities);
    println!(
        "缺失能力: {missing:?} -> {}",
        if missing.is_empty() {
            "通过"
        } else {
            "拒绝"
        }
    );

    // 该模型只有 JsonMode（合法 JSON），不支持严格 schema 输出 → 必须被拒绝
    let mut strict = req.clone();
    strict.response_format = Some(ResponseFormat::JsonSchema {
        name: "weather".into(),
        schema: json!({ "type": "object" }),
    });
    let missing_strict = spec.missing(&required_capabilities(&strict));
    assert_eq!(
        missing_strict,
        vec![Capability::JsonSchema],
        "只有 JsonMode 的模型不应被允许做 JsonSchema 输出"
    );
    println!("严格 schema 输出: 已拒绝（缺失 {missing_strict:?}）");

    println!("\n== 4. 流式事件序列（UI 将来消费的就是这个）==");
    let events = vec![
        StreamEvent::Start {
            response_id: "resp_1".into(),
            model: spec.id.clone(),
        },
        StreamEvent::Delta {
            kind: DeltaKind::Text,
            text: "SSE 让".into(),
        },
        StreamEvent::Delta {
            kind: DeltaKind::Text,
            text: "首字更早到达".into(),
        },
        StreamEvent::ToolCall {
            call: ToolCall {
                id: "call_1".into(),
                name: "get_weather".into(),
                arguments: json!({ "city": "北京" }),
                provider_id: None,
            },
        },
        StreamEvent::Usage {
            usage: Usage {
                input_tokens: 42,
                output_tokens: 17,
            },
        },
        StreamEvent::End {
            finish_reason: FinishReason::ToolCalls,
        },
    ];
    for event in &events {
        println!("{}", serde_json::to_string(event).unwrap());
    }

    let mut text = String::new();
    let mut usage = Usage::default();
    for event in events {
        match event {
            StreamEvent::Delta {
                kind: DeltaKind::Text,
                text: chunk,
            } => text.push_str(&chunk),
            StreamEvent::Usage { usage: u } => usage = u,
            _ => {}
        }
    }
    println!("无状态拼接结果: {text}（{} token）", usage.total());

    println!("\n== 5. 工具结果回填 ==");
    let tool_message = Message::tool_result("call_1", r#"{"temp_c":21}"#);
    println!("{}", serde_json::to_string(&tool_message).unwrap());

    println!("\n== 6. 错误分类与重试判定 ==");
    for (category, status) in [
        (ErrorCategory::RateLimit, 429_u16),
        (ErrorCategory::Auth, 401),
        (ErrorCategory::ContextOverflow, 400),
    ] {
        let error = ProviderError::new("openai-compatible", category)
            .with_status(status)
            .with_message("示例错误");
        println!("{error} | retryable={}", error.retryable);
    }

    println!("\n== 7. 供应商注册表 ==");
    let registry = ProviderRegistry::new();
    println!("已注册供应商: {:?}（M3 起登记首个适配器）", registry.ids());

    println!("\n[ok] 契约层自检通过");
}
