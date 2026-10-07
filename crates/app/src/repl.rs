//! `chat` 子命令：CLI 上的对话 REPL（M4）。
//!
//! 组装（本 crate 的全部职责）：按命令行参数构造适配器 → [`ChatService`] + demo 工具 →
//! 消费 [`LoopEvent`] 流做流式打印。业务逻辑零实现——都在 `agent-chat`。
//!
//! 密钥来源（`R1` 落地前的过渡约定）：`--api-key` 参数或按 provider 推断的环境变量
//! （`OPENAI_API_KEY` / `ANTHROPIC_API_KEY`）。**密钥不入库、不打日志**（硬约束 5）。

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use agent_chat::{Chat, ChatService, LoopEvent, RoundStop, Tool, ToolOutput, ToolSet};
use agent_common::{DeltaKind, ToolDefinition};
use agent_providers::{AnthropicCompatible, AnthropicConfig, OpenAiCompatible, OpenAiConfig};
use agent_transport::HttpConfig;
use futures::StreamExt;
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// `chat` 子命令参数。
#[derive(Debug, Clone)]
pub struct ChatArgs {
    /// `openai-compatible` 或 `anthropic`。
    pub provider: String,
    pub model: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
}

pub async fn run(args: ChatArgs) -> i32 {
    let Some(provider) = build_provider(&args) else {
        return 2;
    };
    let tools = demo_tools();
    let service = ChatService::new(provider, tools);
    let mut chat = Chat::new(args.model.clone());

    println!(
        "agent-chat REPL ｜ provider={} model={} ｜ 输入 /quit 退出，Ctrl-C 取消当前生成",
        args.provider, args.model
    );

    // stdin 是阻塞 IO：放 blocking 线程逐行读，经 channel 进异步世界。
    let (line_tx, mut line_rx) = mpsc::unbounded_channel::<String>();
    tokio::task::spawn_blocking(move || {
        let mut buf = String::new();
        let stdin = std::io::stdin();
        loop {
            buf.clear();
            match stdin.read_line(&mut buf) {
                Ok(0) | Err(_) => break, // EOF / 读失败
                Ok(_) => {
                    let line = buf.trim().to_string();
                    let stop = line == "/quit" || line == "/exit";
                    if line_tx.send(line).is_err() || stop {
                        break;
                    }
                }
            }
        }
    });

    let mut cancel = CancellationToken::new();
    loop {
        tokio::select! {
            biased;
            _ = tokio::signal::ctrl_c() => {
                // 生成中：取消当前轮；空闲中：提示（不退出，退出用 /quit）。
                cancel.cancel();
                continue;
            }
            line = line_rx.recv() => {
                let Some(line) = line else { break }; // stdin 关闭
                if line == "/quit" || line == "/exit" {
                    break;
                }
                if line.is_empty() {
                    continue;
                }
                if line.starts_with('/') {
                    println!("未知命令 {line}（可用：/quit）");
                    continue;
                }
                cancel = CancellationToken::new();
                consume_round(&service, &mut chat, &line, &cancel).await;
            }
        }
    }
    println!("再见。");
    0
}

/// 消费一轮事件流并打印。这是未来 UI 消费 [`LoopEvent`] 的参考实现。
async fn consume_round(
    service: &ChatService,
    chat: &mut Chat,
    input: &str,
    cancel: &CancellationToken,
) {
    let round = match service.run_round(chat, input, cancel.clone()) {
        Ok(round) => round,
        // fast-fail（能力协商 / 校验）：用户输入已入史，提示后可继续。
        Err(error) => {
            println!("[拒绝] {error}");
            return;
        }
    };
    tokio::pin!(round);
    let mut thinking = false;
    while let Some(event) = round.next().await {
        match event {
            LoopEvent::Delta {
                kind: DeltaKind::Text,
                text,
            } => {
                if thinking {
                    println!();
                    thinking = false;
                }
                use std::io::Write;
                print!("{text}");
                let _ = std::io::stdout().flush();
            }
            LoopEvent::Delta {
                kind: DeltaKind::Thinking,
                text,
            } => {
                use std::io::Write;
                if !thinking {
                    print!("\n⟪thinking⟫ ");
                    thinking = true;
                }
                print!("{text}");
                let _ = std::io::stderr().flush();
            }
            LoopEvent::Delta { .. } => {}
            LoopEvent::ModelTurnEnded {
                finish_reason,
                usage,
            } => {
                println!();
                println!(
                    "（模型轮结束：{finish_reason:?}，tokens {}/{}）",
                    usage.input_tokens, usage.output_tokens
                );
            }
            LoopEvent::ToolStarted { id, name } => {
                println!("[tool] {name}({id}) 执行中…");
            }
            LoopEvent::ToolFinished { id, ok } => {
                println!(
                    "[tool] {id} {}",
                    if ok {
                        "完成"
                    } else {
                        "失败（已回传模型）"
                    }
                );
            }
            LoopEvent::RoundEnded {
                stop,
                turns,
                round_usage,
            } => match stop {
                RoundStop::Finished(reason) => println!(
                    "—— 本轮 {turns} 次模型调用，{reason:?}，tokens {}/{} ——",
                    round_usage.input_tokens, round_usage.output_tokens
                ),
                RoundStop::MaxTurns(n) => {
                    println!("—— 达到最大模型轮次（{n}），停止工具循环 ——")
                }
            },
            LoopEvent::Cancelled => println!("\n[已取消：本轮输出已丢弃，可继续输入]"),
            LoopEvent::Error { error } => println!("\n[错误] {error}"),
        }
    }
}

fn build_provider(args: &ChatArgs) -> Option<Arc<dyn agent_providers::ErasedProvider>> {
    let api_key = args
        .api_key
        .clone()
        .or_else(|| api_key_from_env(&args.provider));
    let http = HttpConfig::default();
    let provider: Arc<dyn agent_providers::ErasedProvider> = match args.provider.as_str() {
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
    Some(provider)
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
fn demo_tools() -> ToolSet {
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
