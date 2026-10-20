//! `chat` 子命令：CLI 上的对话 REPL（M4）。
//!
//! 组装在 [`crate::setup`]（与 `tui` 共用）；本文件只做两件事：消费 [`LoopEvent`] 流做
//! 流式打印（未来 UI 消费事件流的参考实现）、处理输入与 Ctrl-C。
//! 业务逻辑零实现——都在 `agent-chat`。
//!
//! 输入用 `rustyline`（`X6` A2）：历史 / 行内编辑 / Ctrl-R。非 TTY（管道输入）时
//! 回退到裸 `read_line`，保持脚本可用。

use std::io::IsTerminal;
use std::sync::{Arc, Mutex};

use agent_chat::{Chat, ChatService, LoopEvent, RoundStop};
use agent_common::DeltaKind;
use futures::StreamExt;
use rustyline::error::ReadlineError;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::setup::{self, ChatArgs};

/// REPL 提示符。
const PROMPT: &str = "> ";

pub async fn run(args: ChatArgs) -> i32 {
    // REPL 不做联网刷新（C2 走界面），只用装配好的 provider。
    let Some(built) = setup::build_provider(&args) else {
        return 2;
    };
    let service = ChatService::new(built.provider, setup::demo_tools());
    let mut chat = Chat::new(args.model.clone());

    println!(
        "CodingRocket REPL ｜ provider={} model={} ｜ 输入 /quit 退出，Ctrl-C 取消当前生成",
        args.provider, args.model
    );

    // 生成中的取消令牌共享槽：输入线程在 Ctrl-C 时取走并取消当前轮
    // （与 `tui` 的 cancel_slot 同一套思路）。
    let cancel_slot: Arc<Mutex<Option<CancellationToken>>> = Arc::new(Mutex::new(None));

    // 输入是阻塞 IO：放 blocking 线程逐行读，经 channel 进异步世界。
    let (line_tx, mut line_rx) = mpsc::unbounded_channel::<String>();
    {
        let slot = cancel_slot.clone();
        tokio::task::spawn_blocking(move || read_lines(line_tx, slot));
    }

    loop {
        let Some(line) = line_rx.recv().await else {
            break; // 输入线程已退出（EOF / Ctrl-D）
        };
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
        let cancel = CancellationToken::new();
        *cancel_slot.lock().expect("cancel slot") = Some(cancel.clone());
        consume_round(&service, &mut chat, &line, &cancel).await;
        *cancel_slot.lock().expect("cancel slot") = None;
    }
    println!("再见。");
    0
}

/// 阻塞读入行。TTY 用 `rustyline`（历史与行内编辑），非 TTY（管道）回退裸 `read_line`。
fn read_lines(
    line_tx: mpsc::UnboundedSender<String>,
    cancel_slot: Arc<Mutex<Option<CancellationToken>>>,
) {
    if std::io::stdin().is_terminal() {
        read_lines_rustyline(line_tx, cancel_slot);
    } else {
        read_lines_plain(line_tx);
    }
}

fn read_lines_rustyline(
    line_tx: mpsc::UnboundedSender<String>,
    cancel_slot: Arc<Mutex<Option<CancellationToken>>>,
) {
    let mut editor = match rustyline::DefaultEditor::new() {
        Ok(editor) => editor,
        Err(error) => {
            eprintln!("初始化行编辑器失败（{error}），回退到简单输入。");
            read_lines_plain(line_tx);
            return;
        }
    };
    loop {
        match editor.readline(PROMPT) {
            Ok(line) => {
                let line = line.trim().to_string();
                let stop = line == "/quit" || line == "/exit";
                let _ = editor.add_history_entry(line.clone());
                if line_tx.send(line).is_err() || stop {
                    break;
                }
            }
            // Ctrl-C：生成中取消当前轮；空闲时无动作（不退出，退出用 /quit）。
            Err(ReadlineError::Interrupted) => {
                if let Some(token) = cancel_slot.lock().expect("cancel slot").take() {
                    token.cancel();
                }
            }
            // Ctrl-D / EOF / 其它错误：结束输入。
            Err(_) => break,
        }
    }
}

fn read_lines_plain(line_tx: mpsc::UnboundedSender<String>) {
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
}

/// 消费一轮事件流并打印。这是未来 UI 消费 [`LoopEvent`] 的参考实现
/// （TUI 版见 `agent_chat::ui::ChatUi`，同一套事件、另一种呈现）。
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
