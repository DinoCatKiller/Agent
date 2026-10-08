//! `tui` 子命令：一期终端界面（`D3` 一期 / `S1` M5）。
//!
//! 职责划分（`D5`：界面跟功能走；`D6`：组装归 `app`）：
//!
//! - **终端生命周期**：raw mode / 备用屏（[`TerminalGuard`] 兜底恢复）；
//! - **事件泵**：crossterm 按键 → 动作（焦点、输入编辑、命令）；
//! - **worker**：唯一拥有 `Chat` 与 `SessionRepo`——跑 `run_round`，把 [`LoopEvent`]
//!   转发给 UI，轮末落盘并刷新会话列表；UI 永不直接碰 SQLite；
//! - **绘制装配**：侧栏（`agent-sessions::ui`）+ 转录/输入（`agent_chat::ui`）+ 状态栏。
//!
//! `D3` 的「UI 无关的事件流 + 命令通道」落在两条 channel 上：
//! worker → UI 是 [`UiEvent`]（[`LoopEvent`] 的搬运工），UI → worker 是 [`Cmd`]。

use std::io::{self, stdout};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use agent_chat::ui::{ChatUi, StatusInfo, Theme, render_input, render_status, render_transcript};
use agent_chat::{Chat, ChatService, LoopEvent, SessionMeta, SessionRepo};
use agent_sessions::session::derive_title;
use agent_sessions::ui::SessionPane;
use agent_store::Store;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::{execute, terminal};
use futures::StreamExt;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::setup::{self, ChatArgs};

/// `tui` 子命令参数。
#[derive(Debug, Clone)]
pub struct TuiArgs {
    pub provider: String,
    pub model: String,
    /// 降级候选（`--model a,b` 的 b…，`R2`）。
    pub fallbacks: Vec<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    /// SQLite 路径（缺省 `./agent-sessions.db`；`R1` 落地前先落 cwd）。
    pub db: Option<String>,
}

/// UI → worker 的命令通道。
#[derive(Debug)]
enum Cmd {
    Submit(String),
    Load { id: String, title: String },
    New,
}

/// worker → UI 的事件通道（`D3`：UI 无关——二期 GPUI 消费同一套）。
enum UiEvent {
    /// 会话列表整体刷新（启动、每次落盘后）。
    Sessions(Vec<SessionMeta>),
    /// 会话已加载（worker 侧 `Chat` 已替换）。
    SessionLoaded { title: String, chat: Chat },
    /// 新会话（worker 侧已重置）。
    Cleared,
    /// 编排事件原样转发。
    Loop(LoopEvent),
    /// `run_round` 同步 fast-fail。
    Rejected(String),
    /// 杂项提示（生成中不可切换、落盘失败…）。
    Notice(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    Input,
    Sidebar,
}

pub async fn run(args: TuiArgs) -> i32 {
    let chat_args = ChatArgs {
        provider: args.provider.clone(),
        model: args.model.clone(),
        fallbacks: args.fallbacks.clone(),
        base_url: args.base_url,
        api_key: args.api_key,
    };
    let Some(provider) = setup::build_provider(&chat_args) else {
        return 2;
    };
    let db_path = args
        .db
        .clone()
        .unwrap_or_else(|| "agent-sessions.db".into());
    let repo = match open_repo(&db_path) {
        Ok(repo) => repo,
        Err(error) => {
            eprintln!("打开会话库失败（{db_path}）：{error}");
            return 2;
        }
    };

    let model = args.model.clone();
    let service = ChatService::new(provider, setup::demo_tools());
    // 取消令牌的共享槽位：worker 轮内不接收命令，所以取消不能走命令通道——
    // UI 直接把令牌塞进槽位并 cancel（worker 轮末清空）。
    let cancel_slot: Arc<std::sync::Mutex<Option<CancellationToken>>> =
        Arc::new(std::sync::Mutex::new(None));
    let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>(32);
    let (ui_tx, mut ui_rx) = mpsc::unbounded_channel::<UiEvent>();
    tokio::spawn(worker(
        model,
        service,
        repo,
        cancel_slot.clone(),
        cmd_rx,
        ui_tx,
    ));

    let mut terminal = match init_terminal() {
        Ok(terminal) => terminal,
        Err(error) => {
            eprintln!("终端初始化失败：{error}");
            eprintln!("当前环境多半不是 Windows 控制台（如 Git Bash 的 MinTTY / 管道重定向）。");
            eprintln!("改用：Windows Terminal / PowerShell / cmd 直接运行；");
            eprintln!("或在 Git Bash 里加 winpty 前缀：winpty cargo run -p agent-app -- tui …");
            return 2;
        }
    };
    let _guard = TerminalGuard;
    let theme = Theme::load();
    let mut chat_ui = ChatUi::new();
    let mut pane = SessionPane::new();
    let mut focus = Focus::Input;
    let mut current_title: Option<String> = None;
    // 退出时序：生成中退出要先取消，并等 worker 把最后一轮落盘（Sessions 刷新事件即落盘完成
    // 的标记），再恢复终端——否则 process::exit 会把保存任务一起杀掉。
    let mut quit_wait = false;
    let mut pending_save = false;
    let mut events = EventStream::new();

    loop {
        while let Ok(event) = ui_rx.try_recv() {
            if matches!(event, UiEvent::Sessions(_)) {
                pending_save = false;
            }
            apply(&mut chat_ui, &mut pane, &mut current_title, event);
        }
        if quit_wait && !chat_ui.generating() && !pending_save {
            break;
        }
        let status = StatusInfo {
            provider: &args.provider,
            model: &args.model,
            session: current_title.as_deref(),
        };
        if let Err(error) = terminal.draw(|f| draw(f, &chat_ui, &mut pane, focus, &theme, &status))
        {
            eprintln!("绘制失败：{error}");
            break;
        }
        if quit_wait && !chat_ui.generating() && !pending_save {
            break;
        }

        tokio::select! {
            biased;
            ev = events.next() => match ev {
                None => break, // 事件源关闭（stdin 关闭等）
                Some(Err(_)) => break,
                Some(Ok(event)) => {
                    if process_key(
                        event,
                        &mut focus,
                        &mut chat_ui,
                        &mut pane,
                        &cmd_tx,
                    ) == Intent::Quit
                    {
                        if chat_ui.generating() {
                            if let Some(token) =
                                cancel_slot.lock().expect("cancel slot").as_ref()
                            {
                                token.cancel();
                            }
                            pending_save = true;
                        }
                        quit_wait = true;
                    }
                }
            },
            ui = ui_rx.recv() => match ui {
                None => break, // worker 已退出
                Some(event) => {
                    if matches!(event, UiEvent::Sessions(_)) {
                        pending_save = false;
                    }
                    apply(&mut chat_ui, &mut pane, &mut current_title, event);
                }
            }
        }
        if quit_wait && !chat_ui.generating() && !pending_save {
            break;
        }
    }
    0
}

/// 按键处理的全局意图（退出在 run 循环里统一收口，便于挂起取消与等待落盘）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Intent {
    None,
    Quit,
}

fn open_repo(path: &str) -> Result<SessionRepo, Box<dyn std::error::Error>> {
    let store = Store::open(path)?;
    Ok(SessionRepo::open(store)?)
}

fn apply(
    chat_ui: &mut ChatUi,
    pane: &mut SessionPane,
    current_title: &mut Option<String>,
    event: UiEvent,
) {
    match event {
        UiEvent::Sessions(items) => pane.set_sessions(items),
        UiEvent::SessionLoaded { title, chat } => {
            chat_ui.load_chat(&chat);
            *current_title = Some(title);
        }
        UiEvent::Cleared => {
            chat_ui.reset();
            *current_title = None;
        }
        UiEvent::Loop(event) => chat_ui.on_event(event),
        UiEvent::Rejected(message) => chat_ui.rejected(message),
        UiEvent::Notice(message) => chat_ui.set_notice(message),
    }
}

/// 非按键事件忽略；按键抬起（Windows 会同时上报 Press/Release）忽略。
fn process_key(
    event: Event,
    focus: &mut Focus,
    chat_ui: &mut ChatUi,
    pane: &mut SessionPane,
    cmd_tx: &mpsc::Sender<Cmd>,
) -> Intent {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => {
            handle_key(key, focus, chat_ui, pane, cmd_tx)
        }
        _ => Intent::None,
    }
}

fn handle_key(
    key: KeyEvent,
    focus: &mut Focus,
    chat_ui: &mut ChatUi,
    pane: &mut SessionPane,
    cmd_tx: &mpsc::Sender<Cmd>,
) -> Intent {
    // 全局键（无论焦点）
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('c') | KeyCode::Char('q') => return Intent::Quit,
            _ => {}
        }
    }
    match key.code {
        KeyCode::Tab => {
            *focus = match focus {
                Focus::Input => Focus::Sidebar,
                Focus::Sidebar => Focus::Input,
            };
        }
        KeyCode::Esc => *focus = Focus::Input,
        _ => match focus {
            Focus::Input => match key.code {
                KeyCode::Enter => {
                    if chat_ui.generating() {
                        chat_ui.set_notice("生成中，等本轮结束或 Ctrl-C 取消".into());
                    } else {
                        let text = chat_ui.take_input();
                        if !text.is_empty() {
                            chat_ui.push_user(&text);
                            let _ = cmd_tx.try_send(Cmd::Submit(text));
                        }
                    }
                }
                KeyCode::Backspace => chat_ui.backspace(),
                KeyCode::Delete => chat_ui.delete(),
                KeyCode::Left => chat_ui.left(),
                KeyCode::Right => chat_ui.right(),
                KeyCode::Home => chat_ui.home(),
                KeyCode::End => chat_ui.end(),
                KeyCode::PageUp => chat_ui.scroll_up(10),
                KeyCode::PageDown => chat_ui.scroll_down(10),
                KeyCode::Up => chat_ui.scroll_up(1),
                KeyCode::Down => chat_ui.scroll_down(1),
                KeyCode::Char(c)
                    if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
                {
                    chat_ui.insert(c);
                }
                _ => {}
            },
            Focus::Sidebar => match key.code {
                KeyCode::Up => pane.previous(),
                KeyCode::Down => pane.next(),
                KeyCode::Enter => {
                    if chat_ui.generating() {
                        chat_ui.set_notice("生成中，先取消再切换".into());
                    } else if let Some(meta) = pane.selected().cloned() {
                        let _ = cmd_tx.try_send(Cmd::Load {
                            id: meta.id,
                            title: meta.title,
                        });
                    }
                }
                KeyCode::Char('n') => {
                    if !chat_ui.generating() {
                        let _ = cmd_tx.try_send(Cmd::New);
                    } else {
                        chat_ui.set_notice("生成中，先取消再新建".into());
                    }
                }
                KeyCode::PageUp => chat_ui.scroll_up(10),
                KeyCode::PageDown => chat_ui.scroll_down(10),
                _ => {}
            },
        },
    }
    Intent::None
}

fn draw(
    f: &mut ratatui::Frame,
    chat_ui: &ChatUi,
    pane: &mut SessionPane,
    focus: Focus,
    theme: &Theme,
    status: &StatusInfo,
) {
    pane.set_focused(focus == Focus::Sidebar);
    let outer = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(f.area());
    let body = Layout::horizontal([Constraint::Length(26), Constraint::Min(20)]).split(outer[0]);
    let chat = Layout::vertical([Constraint::Min(3), Constraint::Length(3)]).split(body[1]);
    pane.render(f, body[0], theme);
    render_transcript(f, chat_ui, chat[0], theme);
    render_input(f, chat_ui, chat[1], focus == Focus::Input, theme);
    render_status(f, chat_ui, status, outer[1], theme);
}

// —— worker：唯一拥有 `Chat` 与 `SessionRepo` ——

async fn worker(
    model: String,
    service: ChatService,
    repo: SessionRepo,
    cancel_slot: Arc<std::sync::Mutex<Option<CancellationToken>>>,
    mut cmd_rx: mpsc::Receiver<Cmd>,
    ui_tx: mpsc::UnboundedSender<UiEvent>,
) {
    let mut chat = Chat::new(model);
    // (session_id, title)；首次落盘时创建。
    let mut session: Option<(String, String)> = None;
    // 是否有一轮进行中（New / Load / Submit 互斥的依据）——以 cancel_slot 是否持有令牌为准。
    let busy = || cancel_slot.lock().expect("cancel slot").is_some();
    let _ = ui_tx.send(UiEvent::Sessions(repo_list(&repo).await));

    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            Cmd::New => {
                if busy() {
                    let _ = ui_tx.send(UiEvent::Notice("生成中，先取消再新建".into()));
                    continue;
                }
                chat = Chat::new(chat.model.clone());
                session = None;
                let _ = ui_tx.send(UiEvent::Cleared);
            }
            Cmd::Load { id, title } => {
                if busy() {
                    let _ = ui_tx.send(UiEvent::Notice("生成中，先取消再切换".into()));
                    continue;
                }
                let repo2 = repo.clone();
                let id2 = id.clone();
                let loaded = match tokio::task::spawn_blocking(move || repo2.load(&id2)).await {
                    Ok(Ok(Some(chat))) => chat,
                    Ok(Ok(None)) => {
                        let _ = ui_tx.send(UiEvent::Notice("会话不存在或已被删除".into()));
                        continue;
                    }
                    Ok(Err(error)) => {
                        let _ = ui_tx.send(UiEvent::Notice(format!("读取会话失败：{error}")));
                        continue;
                    }
                    Err(_) => {
                        let _ = ui_tx.send(UiEvent::Notice("读取会话任务失败".into()));
                        continue;
                    }
                };
                chat = loaded;
                session = Some((id, title.clone()));
                let _ = ui_tx.send(UiEvent::SessionLoaded {
                    title,
                    chat: chat.clone(),
                });
            }
            Cmd::Submit(text) => {
                if busy() {
                    let _ = ui_tx.send(UiEvent::Notice("生成中…".into()));
                    continue;
                }
                let existing = session
                    .as_ref()
                    .map(|(id, title)| (id.clone(), title.clone()));
                let (id, title) = match existing {
                    Some(pair) => pair,
                    None => {
                        let id = new_session_id();
                        let title = derive_title(&text);
                        session = Some((id.clone(), title.clone()));
                        (id, title)
                    }
                };

                let token = CancellationToken::new();
                *cancel_slot.lock().expect("cancel slot") = Some(token.clone());

                match service.run_round(&mut chat, text, token) {
                    Err(error) => {
                        // fast-fail：用户输入已入史（见 run_round），提示后可继续。
                        let _ = ui_tx.send(UiEvent::Rejected(error.to_string()));
                    }
                    Ok(round) => {
                        tokio::pin!(round);
                        while let Some(event) = round.next().await {
                            if ui_tx.send(UiEvent::Loop(event)).is_err() {
                                break; // UI 已退出
                            }
                        }
                    }
                }
                *cancel_slot.lock().expect("cancel slot") = None;

                // 轮末落盘（取消 / 错误也存：用户输入已在史中）。
                let (repo2, chat2, id2, title2) = (repo.clone(), chat.clone(), id, title);
                let now = now_ms();
                let saved =
                    tokio::task::spawn_blocking(move || repo2.save(&id2, &title2, &chat2, now))
                        .await;
                match saved {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        let _ = ui_tx.send(UiEvent::Notice(format!("保存会话失败：{error}")));
                    }
                    Err(_) => {
                        let _ = ui_tx.send(UiEvent::Notice("保存会话任务失败".into()));
                    }
                }
                let _ = ui_tx.send(UiEvent::Sessions(repo_list(&repo).await));
            }
        }
    }
}

async fn repo_list(repo: &SessionRepo) -> Vec<SessionMeta> {
    let repo2 = repo.clone();
    tokio::task::spawn_blocking(move || repo2.list().unwrap_or_default())
        .await
        .unwrap_or_default()
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 会话 id：纳秒时间戳的十六进制（单用户本地场景足够唯一；正式方案随 `R1`）。
fn new_session_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("s{nanos:024x}")
}

// —— 终端生命周期 ——

fn init_terminal() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    terminal::enable_raw_mode()?;
    // 不碰鼠标捕获：crossterm 的 DisableMouseCapture 需要 Enable 先行（它靠 enable
    // 捕获初始模式来恢复），直接 Disable 在 Windows 上必报
    // 「Initial console modes not set」。本界面只用键盘。
    execute!(stdout(), terminal::EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(stdout()))
}

/// Drop 时恢复终端（raw mode / 备用屏），panic 路径也不把终端留成花屏。
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(stdout(), terminal::LeaveAlternateScreen);
    }
}

#[cfg(test)]
mod tests {
    //! worker 协议的端到端测试（`Q1` §1）：真实适配器 + HTTP 栈 → wiremock，
    //! 断网可跑。终端初始化与按键泵需要真控制台，不在自动测试范围（见 README）。

    use std::sync::Mutex;
    use std::time::Duration;

    use agent_chat::{RoundStop, ToolSet};
    use agent_common::{DeltaKind, FinishReason, Role};
    use agent_providers::{OpenAiCompatible, OpenAiConfig};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    fn chunk(delta: &str, finish: Option<&str>) -> String {
        format!(
            "data: {}\n\n",
            serde_json::json!({
                "id": "c1",
                "object": "chat.completion.chunk",
                "model": "fake-model",
                "choices": [{
                    "index": 0,
                    "delta": serde_json::from_str::<serde_json::Value>(delta).unwrap(),
                    "finish_reason": finish
                }]
            })
        )
    }

    fn sse(parts: &[&str]) -> String {
        let mut body = parts.concat();
        body.push_str("data: [DONE]\n\n");
        body
    }

    fn plain_stream() -> String {
        sse(&[
            &chunk(r#"{"role":"assistant","content":""}"#, None),
            &chunk(r#"{"content":"你好"}"#, None),
            &chunk(r#"{}"#, Some("stop")),
        ])
    }

    async fn mount_stream(body: String) -> MockServer {
        mount_stream_delayed(body, None).await
    }

    /// `delay` 非 None 时整个响应延迟后才开始（模拟慢流，给取消留窗口）。
    async fn mount_stream_delayed(body: String, delay: Option<Duration>) -> MockServer {
        let server = MockServer::start().await;
        let mut template = ResponseTemplate::new(200).set_body_raw(body, "text/event-stream");
        if let Some(delay) = delay {
            template = template.set_delay(delay);
        }
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(template)
            .mount(&server)
            .await;
        server
    }

    type Slot = Arc<Mutex<Option<CancellationToken>>>;

    fn spawn_worker(
        repo: &SessionRepo,
        base_url: String,
    ) -> (mpsc::Sender<Cmd>, mpsc::UnboundedReceiver<UiEvent>, Slot) {
        let provider = OpenAiCompatible::new(OpenAiConfig {
            id: "openai-compatible",
            base_url,
            api_key: Some("test-key".into()),
            http: Default::default(),
            models: Vec::new(),
        })
        .expect("provider builds");
        let service = ChatService::new(Arc::new(provider), ToolSet::new());
        let slot: Slot = Arc::new(Mutex::new(None));
        let (cmd_tx, cmd_rx) = mpsc::channel(32);
        let (ui_tx, ui_rx) = mpsc::unbounded_channel();
        tokio::spawn(worker(
            "fake-model".into(),
            service,
            repo.clone(),
            slot.clone(),
            cmd_rx,
            ui_tx,
        ));
        (cmd_tx, ui_rx, slot)
    }

    /// 收事件直到 `cond` 命中（整体 5 秒超时），返回含终止事件在内的全部事件。
    async fn collect_until(
        ui_rx: &mut mpsc::UnboundedReceiver<UiEvent>,
        mut cond: impl FnMut(&UiEvent) -> bool,
    ) -> Vec<UiEvent> {
        let mut events = Vec::new();
        loop {
            let event = tokio::time::timeout(Duration::from_secs(5), ui_rx.recv())
                .await
                .expect("collect timeout")
                .expect("worker alive");
            let hit = cond(&event);
            events.push(event);
            if hit {
                return events;
            }
        }
    }

    #[tokio::test]
    async fn submit_streams_saves_and_refreshes_sidebar() {
        let server = mount_stream(plain_stream()).await;
        let repo = SessionRepo::open(Store::open_in_memory().unwrap()).unwrap();
        let (cmd_tx, mut ui_rx, _slot) = spawn_worker(&repo, format!("{}/v1", server.uri()));

        cmd_tx.send(Cmd::Submit("你好".into())).await.unwrap();
        // worker 启动时会先发一次 Sessions（空列表），要等「见过 Loop 事件之后」的
        // 那次刷新（即轮末落盘后的刷新）才算收尾。
        let mut saw_loop = false;
        let events = collect_until(&mut ui_rx, |e| {
            if matches!(e, UiEvent::Loop(_)) {
                saw_loop = true;
            }
            saw_loop && matches!(e, UiEvent::Sessions(_))
        })
        .await;

        // 流式事件都到了
        let texts: String = events
            .iter()
            .filter_map(|e| match e {
                UiEvent::Loop(LoopEvent::Delta {
                    kind: DeltaKind::Text,
                    text,
                }) => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(texts, "你好");
        assert!(events.iter().any(|e| matches!(
            e,
            UiEvent::Loop(LoopEvent::RoundEnded {
                stop: RoundStop::Finished(FinishReason::Stop),
                ..
            })
        )));
        // 落盘 + 侧栏刷新：标题来自 derive_title
        let UiEvent::Sessions(metas) = events.last().unwrap() else {
            panic!("终事件应是 Sessions");
        };
        assert_eq!(metas.len(), 1);
        assert_eq!(metas[0].title, "你好");
        // 历史真的在库里
        let loaded = repo.load(&metas[0].id).unwrap().expect("saved");
        assert_eq!(loaded.messages.len(), 2);
        assert_eq!(loaded.messages[0].role, Role::User);
        assert_eq!(loaded.messages[1].text(), "你好");
    }

    #[tokio::test]
    async fn load_restores_history_for_display() {
        let server = mount_stream(plain_stream()).await;
        let repo = SessionRepo::open(Store::open_in_memory().unwrap()).unwrap();
        let mut past = Chat::new("fake-model");
        past.push_user("旧问题");
        repo.save("s-old", "旧会话", &past, 1).unwrap();

        let (cmd_tx, mut ui_rx, _slot) = spawn_worker(&repo, format!("{}/v1", server.uri()));
        // 启动时的会话列表已含 s-old
        let events = collect_until(&mut ui_rx, |e| matches!(e, UiEvent::Sessions(_))).await;
        let UiEvent::Sessions(metas) = events.last().unwrap() else {
            panic!("启动应刷新列表");
        };
        assert_eq!(metas[0].id, "s-old");

        cmd_tx
            .send(Cmd::Load {
                id: "s-old".into(),
                title: "旧会话".into(),
            })
            .await
            .unwrap();
        let events =
            collect_until(&mut ui_rx, |e| matches!(e, UiEvent::SessionLoaded { .. })).await;
        let Some(UiEvent::SessionLoaded { title, chat }) = events.into_iter().last() else {
            panic!("应收到 SessionLoaded");
        };
        assert_eq!(title, "旧会话");
        assert_eq!(chat.messages.len(), 1);
        assert_eq!(chat.messages[0].text(), "旧问题");
    }

    #[tokio::test]
    async fn cancel_via_slot_rolls_back_and_still_saves() {
        // 慢流：响应延迟 2 秒才开始，留给取消窗口
        let server = mount_stream_delayed(plain_stream(), Some(Duration::from_secs(2))).await;
        let repo = SessionRepo::open(Store::open_in_memory().unwrap()).unwrap();
        let (cmd_tx, mut ui_rx, slot) = spawn_worker(&repo, format!("{}/v1", server.uri()));

        cmd_tx.send(Cmd::Submit("取消我".into())).await.unwrap();
        // 等 worker 把令牌放进槽位，然后直接取消（模拟 Ctrl-C 路径）
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            if slot.lock().unwrap().is_some() {
                break;
            }
            assert!(tokio::time::Instant::now() < deadline, "等令牌超时");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        slot.lock().unwrap().as_ref().unwrap().cancel();

        let mut saw_cancelled = false;
        let events = collect_until(&mut ui_rx, |e| {
            if matches!(e, UiEvent::Loop(LoopEvent::Cancelled)) {
                saw_cancelled = true;
            }
            saw_cancelled && matches!(e, UiEvent::Sessions(_))
        })
        .await;
        assert!(
            events
                .iter()
                .any(|e| matches!(e, UiEvent::Loop(LoopEvent::Cancelled))),
            "应收到 Cancelled"
        );
        // 取消后仍落盘：历史保留用户输入（与 REPL 语义一致）
        let metas = repo.list().unwrap();
        assert_eq!(metas.len(), 1);
        let loaded = repo.load(&metas[0].id).unwrap().unwrap();
        assert_eq!(loaded.messages.len(), 1, "只有用户消息入史");
        assert_eq!(loaded.messages[0].text(), "取消我");
    }
}
