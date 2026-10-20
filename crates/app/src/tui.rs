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

use agent_chat::channel::{Cmd, UiEvent};
use agent_chat::ui::{ChatUi, StatusInfo, Theme, render_input, render_status, render_transcript};
use agent_sessions::ui::SessionPane;
use crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use crossterm::{execute, terminal};
use futures::StreamExt;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Rect},
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::worker;

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
    /// 配置文件路径（`R1`）。
    pub config: Option<String>,
    /// SQLite 路径（缺省 `./agent-sessions.db`；会话库路径不在 `R1` 范围内）。
    pub db: Option<String>,
}

// `Cmd` / `UiEvent` 已上移到 `agent_chat::channel`：二期 GPUI 渲染端消费的是同一套契约，
// 放在切片里两边才不会各写一份（`D3`「UI 无关的事件流 + 命令通道」）。

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
        config: args.config.clone(),
    };
    let Some(built) = setup::build_provider(&chat_args) else {
        return 2;
    };
    let db_path = args
        .db
        .clone()
        .unwrap_or_else(|| "agent-sessions.db".into());
    let repo = match worker::open_repo(&db_path) {
        Ok(repo) => repo,
        Err(error) => {
            eprintln!("打开会话库失败（{db_path}）：{error}");
            return 2;
        }
    };

    // 生效组合（provider + model）与「重建 provider 的上下文」——切换模型时 worker 用它。
    // `chat_args` 已由 `setup::resolve` 定好（CLI > 配置文件默认，`R1`）。
    let mut selection = worker::Selection::new(chat_args.provider.clone(), chat_args.model.clone());
    let factory = worker::ProviderFactory::new(&chat_args, setup::demo_tools());
    // 取消令牌的共享槽位：worker 轮内不接收命令，所以取消不能走命令通道——
    // UI 直接把令牌塞进槽位并 cancel（worker 轮末清空）。
    let cancel_slot: Arc<std::sync::Mutex<Option<CancellationToken>>> = worker::cancel_slot();
    let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>(32);
    let (ui_tx, mut ui_rx) = mpsc::unbounded_channel::<UiEvent>();
    tokio::spawn(worker::run_worker(
        selection.clone(),
        built,
        factory,
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
    // 上一帧的区域快照：ratatui 是即时模式，鼠标命中测试要用它（见 `HitAreas`）。
    let mut areas = HitAreas::default();
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
            apply(
                &mut chat_ui,
                &mut pane,
                &mut current_title,
                &mut selection,
                event,
            );
        }
        if quit_wait && !chat_ui.generating() && !pending_save {
            break;
        }
        let status = StatusInfo {
            provider: &selection.provider,
            model: &selection.model,
            session: current_title.as_deref(),
        };
        if let Err(error) =
            terminal.draw(|f| draw(f, &chat_ui, &mut pane, focus, &theme, &status, &mut areas))
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
                    if process_event(
                        event,
                        &mut focus,
                        &mut chat_ui,
                        &mut pane,
                        &cmd_tx,
                        &areas,
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
                    apply(
                        &mut chat_ui,
                        &mut pane,
                        &mut current_title,
                        &mut selection,
                        event,
                    );
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

fn apply(
    chat_ui: &mut ChatUi,
    pane: &mut SessionPane,
    current_title: &mut Option<String>,
    selection: &mut worker::Selection,
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
        // 生效组合以 worker 为准（`channel.rs` 的约定：宿主不自己猜）
        UiEvent::ModelChanged { provider, model } => {
            *selection = worker::Selection::new(provider, model);
        }
        UiEvent::Rejected(message) => chat_ui.rejected(message),
        UiEvent::Notice(message) => chat_ui.set_notice(message),
        // 联网刷新（C2）是设置页的功能；TUI 不发起，收到就报个提示。
        UiEvent::ModelsRefreshed { provider, models } => {
            chat_ui.set_notice(format!("{provider} 模型清单已刷新（{} 个）", models.len()))
        }
        UiEvent::ModelsRefreshFailed { provider, message } => {
            chat_ui.set_notice(format!("刷新 {provider} 模型失败：{message}"))
        }
    }
}

/// 事件分发：按键抬起（Windows 会同时上报 Press/Release）忽略；鼠标走 `handle_mouse`。
fn process_event(
    event: Event,
    focus: &mut Focus,
    chat_ui: &mut ChatUi,
    pane: &mut SessionPane,
    cmd_tx: &mpsc::Sender<Cmd>,
    areas: &HitAreas,
) -> Intent {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => {
            handle_key(key, focus, chat_ui, pane, cmd_tx)
        }
        Event::Mouse(mouse) => handle_mouse(mouse, focus, chat_ui, pane, cmd_tx, areas),
        _ => Intent::None,
    }
}

/// 上一帧各区域的屏幕矩形（鼠标命中测试用）。
///
/// ratatui 是即时模式：区域只在 `draw` 那一帧存在，因此在这里留存一份副本
/// （`run` 循环每帧刷新，事件处理读它）。
#[derive(Debug, Default, Clone, Copy)]
struct HitAreas {
    sidebar: Rect,
    transcript: Rect,
    input: Rect,
    /// 会话列表实际渲染的首项偏移（列表长于可视区时非零）。
    sidebar_offset: usize,
}

impl HitAreas {
    /// 点是否落在区域内（左闭右开，与 ratatui 的 `Rect` 语义一致）。
    fn contains(area: Rect, column: u16, row: u16) -> bool {
        column >= area.x
            && column < area.x.saturating_add(area.width)
            && row >= area.y
            && row < area.y.saturating_add(area.height)
    }

    /// 侧栏内鼠标行 → 会话索引；落在标题 / 下边框、空白或越界时为 `None`。
    fn sidebar_index(&self, row: u16, len: usize) -> Option<usize> {
        // `Block::bordered` 上下各占 1 行，列表项从 `sidebar.y + 1` 开始。
        let visible = self.sidebar.height.saturating_sub(2) as usize;
        if visible == 0 || row <= self.sidebar.y {
            return None;
        }
        let row_in_list = usize::from(row - self.sidebar.y - 1);
        if row_in_list >= visible {
            return None; // 下边框
        }
        let index = self.sidebar_offset + row_in_list;
        (index < len).then_some(index)
    }
}

/// 鼠标：左键点击切焦点 / 载入会话；滚轮在侧栏移动选择、在其它区域滚动转录。
fn handle_mouse(
    mouse: MouseEvent,
    focus: &mut Focus,
    chat_ui: &mut ChatUi,
    pane: &mut SessionPane,
    cmd_tx: &mpsc::Sender<Cmd>,
    areas: &HitAreas,
) -> Intent {
    let (column, row) = (mouse.column, mouse.row);
    let on_sidebar = HitAreas::contains(areas.sidebar, column, row);
    match mouse.kind {
        MouseEventKind::ScrollUp => {
            if on_sidebar {
                pane.previous();
            } else {
                chat_ui.scroll_up(3);
            }
        }
        MouseEventKind::ScrollDown => {
            if on_sidebar {
                pane.next();
            } else {
                chat_ui.scroll_down(3);
            }
        }
        MouseEventKind::Down(MouseButton::Left) => {
            if on_sidebar {
                *focus = Focus::Sidebar;
                if let Some(index) = areas.sidebar_index(row, pane.len()) {
                    pane.select(index);
                    load_selected(pane, chat_ui, cmd_tx);
                }
            } else if HitAreas::contains(areas.transcript, column, row)
                || HitAreas::contains(areas.input, column, row)
            {
                *focus = Focus::Input;
            }
        }
        _ => {}
    }
    Intent::None
}

/// 载入侧栏当前选中项（侧栏 Enter 与鼠标点击共用）。
fn load_selected(pane: &SessionPane, chat_ui: &mut ChatUi, cmd_tx: &mpsc::Sender<Cmd>) {
    if chat_ui.generating() {
        chat_ui.set_notice("生成中，先取消再切换".into());
    } else if let Some(meta) = pane.selected().cloned() {
        let _ = cmd_tx.try_send(Cmd::Load {
            id: meta.id,
            title: meta.title,
        });
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
                KeyCode::Enter => load_selected(pane, chat_ui, cmd_tx),
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
    areas: &mut HitAreas,
) {
    pane.set_focused(focus == Focus::Sidebar);
    let outer = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(f.area());
    let body = Layout::horizontal([Constraint::Length(26), Constraint::Min(20)]).split(outer[0]);
    let chat = Layout::vertical([Constraint::Min(3), Constraint::Length(3)]).split(body[1]);
    // 留存本帧区域，供鼠标命中（列表偏移由侧栏渲染回传）。
    areas.sidebar = body[0];
    areas.transcript = chat[0];
    areas.input = chat[1];
    areas.sidebar_offset = pane.render(f, body[0], theme);
    render_transcript(f, chat_ui, chat[0], theme);
    render_input(f, chat_ui, chat[1], focus == Focus::Input, theme);
    render_status(f, chat_ui, status, outer[1], theme);
}

// worker 已上移到 `crate::worker`：命令通道与事件语义只能有一份，
// 否则 TUI 与 GPUI 的行为会慢慢漂开（见 `worker.rs` 顶部注释）。

// —— 终端生命周期 ——

fn init_terminal() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    terminal::enable_raw_mode()?;
    // 启用鼠标捕获（点击 / 滚轮）。必须与 `TerminalGuard` 的 `DisableMouseCapture`
    // 成对：crossterm 靠「Enable 时记录的初始模式」来恢复，直接 Disable 在 Windows 上
    // 会报「Initial console modes not set」。
    execute!(stdout(), terminal::EnterAlternateScreen, EnableMouseCapture)?;
    Terminal::new(CrosstermBackend::new(stdout()))
}

/// Drop 时恢复终端（raw mode / 鼠标捕获 / 备用屏），panic 路径也不把终端留成花屏。
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(
            stdout(),
            DisableMouseCapture,
            terminal::LeaveAlternateScreen
        );
    }
}

#[cfg(test)]
mod tests {
    //! worker 协议的端到端测试（`Q1` §1）：真实适配器 + HTTP 栈 → wiremock，
    //! 断网可跑。终端初始化与按键泵需要真控制台，不在自动测试范围（见 README）。

    use std::sync::Mutex;
    use std::time::Duration;

    use agent_chat::{Chat, LoopEvent, RoundStop, SessionRepo, ToolSet};
    use agent_common::{DeltaKind, FinishReason, Role};
    // provider 构造统一走 `worker::ProviderFactory`（下面 `spawn_worker`），这里不再直接用适配器
    use agent_store::Store;
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

    /// 测试用的**空配置**：`config: None` 会走平台标准目录，读到开发者本机的真实配置，
    /// 于是本机配的代理 / 默认模型就会让测试结果随机器而变（曾因此挂掉两条用例）。
    fn isolated_config(name: &str) -> String {
        let path =
            std::env::temp_dir().join(format!("agent-app-test-{}-{name}.toml", std::process::id()));
        std::fs::write(&path, "").expect("写空配置");
        path.display().to_string()
    }

    fn spawn_worker(
        repo: &SessionRepo,
        base_url: String,
    ) -> (mpsc::Sender<Cmd>, mpsc::UnboundedReceiver<UiEvent>, Slot) {
        // 测试也走 `ProviderFactory`：顺带把「切换 provider 时重建 provider」这条路跑一遍
        let chat_args = ChatArgs {
            provider: "openai-compatible".into(),
            model: "fake-model".into(),
            fallbacks: Vec::new(),
            base_url: Some(base_url),
            api_key: Some("test-key".into()),
            config: Some(isolated_config("worker")),
        };
        let factory = worker::ProviderFactory::new(&chat_args, ToolSet::new());
        let selection = worker::Selection::new(&chat_args.provider, &chat_args.model);
        let built = factory.build(&selection).expect("provider builds");
        let slot: Slot = Arc::new(Mutex::new(None));
        let (cmd_tx, cmd_rx) = mpsc::channel(32);
        let (ui_tx, ui_rx) = mpsc::unbounded_channel();
        tokio::spawn(worker::run_worker(
            selection,
            built,
            factory,
            repo.clone(),
            slot.clone(),
            cmd_rx,
            ui_tx,
        ));
        (cmd_tx, ui_rx, slot)
    }

    /// 收事件直到 `cond` 命中（整体 5 秒超时），返回含终止事件在内的全部事件。
    ///
    /// 超时会把**已收到的事件打出来**：失败时最想知道的就是「到底收到了什么」。
    async fn collect_until(
        ui_rx: &mut mpsc::UnboundedReceiver<UiEvent>,
        mut cond: impl FnMut(&UiEvent) -> bool,
    ) -> Vec<UiEvent> {
        let mut events: Vec<UiEvent> = Vec::new();
        loop {
            let Ok(next) = tokio::time::timeout(Duration::from_secs(5), ui_rx.recv()).await else {
                panic!(
                    "collect_until 超时（5s）：已收到 {} 个事件 —— {events:#?}",
                    events.len()
                );
            };
            let event = next.expect("worker alive");
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
        assert_eq!(texts, "你好", "事件：{events:#?}");
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
    async fn select_model_switches_and_next_round_uses_it() {
        let server = mount_stream(plain_stream()).await;
        let repo = SessionRepo::open(Store::open_in_memory().unwrap()).unwrap();
        let (cmd_tx, mut ui_rx, _slot) = spawn_worker(&repo, format!("{}/v1", server.uri()));

        // 启动时播报一次生效组合（界面以 worker 为准）
        let startup =
            collect_until(&mut ui_rx, |e| matches!(e, UiEvent::ModelChanged { .. })).await;
        assert!(
            matches!(startup.last(), Some(UiEvent::ModelChanged { model, .. }) if model == "fake-model"),
            "启动应当播报生效组合：{startup:?}"
        );

        // 切模型：worker 播报新组合
        cmd_tx
            .send(Cmd::SelectModel {
                provider: "openai-compatible".into(),
                model: "other-model".into(),
            })
            .await
            .unwrap();
        let switched = collect_until(
            &mut ui_rx,
            |e| matches!(e, UiEvent::ModelChanged { model, .. } if model == "other-model"),
        )
        .await;
        assert!(
            switched.iter().any(|e| matches!(
                e,
                UiEvent::ModelChanged { model, .. } if model == "other-model"
            )),
            "切换后应当播报新组合：{switched:?}"
        );

        // 下一轮真的用新模型 —— 看真实请求体（不是看 UI 显示）
        cmd_tx.send(Cmd::Submit("你好".into())).await.unwrap();
        let mut saw_loop = false;
        let _ = collect_until(&mut ui_rx, |e| {
            if matches!(e, UiEvent::Loop(_)) {
                saw_loop = true;
            }
            saw_loop && matches!(e, UiEvent::Sessions(_))
        })
        .await;

        let requests = server.received_requests().await.expect("wiremock 记录请求");
        let bodies: Vec<String> = requests
            .iter()
            .map(|request| String::from_utf8_lossy(&request.body).to_string())
            .collect();
        assert!(
            bodies.iter().any(|body| body.contains("other-model")),
            "切换后的请求应当带新模型：{bodies:?}"
        );
        assert!(
            !bodies.iter().any(|body| body.contains("fake-model")),
            "切换后不该再出现旧模型：{bodies:?}"
        );
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

    // —— 鼠标命中（纯函数，无需真终端）——

    fn rect(x: u16, y: u16, width: u16, height: u16) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn contains_uses_half_open_bounds() {
        let area = rect(2, 3, 4, 5); // 覆盖 x∈[2,6), y∈[3,8)
        assert!(HitAreas::contains(area, 2, 3), "左上角含");
        assert!(HitAreas::contains(area, 5, 7), "右下角前一格含");
        assert!(!HitAreas::contains(area, 6, 7), "右边界外");
        assert!(!HitAreas::contains(area, 2, 8), "下边界外");
        assert!(!HitAreas::contains(area, 1, 3), "左边界外");
    }

    #[test]
    fn sidebar_index_skips_border_rows() {
        let areas = HitAreas {
            sidebar: rect(0, 0, 26, 12), // 可视 10 行
            sidebar_offset: 0,
            ..HitAreas::default()
        };
        assert_eq!(areas.sidebar_index(0, 5), None, "标题行不可点");
        assert_eq!(areas.sidebar_index(1, 5), Some(0), "首项");
        assert_eq!(areas.sidebar_index(3, 5), Some(2));
        assert_eq!(areas.sidebar_index(10, 5), None, "索引越界（len=5）");
        assert_eq!(areas.sidebar_index(10, 20), Some(9), "最后一行仍是列表项");
        assert_eq!(areas.sidebar_index(11, 20), None, "下边框不可点");
    }

    #[test]
    fn sidebar_index_respects_scroll_offset() {
        let areas = HitAreas {
            sidebar: rect(0, 0, 26, 6), // 可视 4 行
            sidebar_offset: 7,
            ..HitAreas::default()
        };
        assert_eq!(areas.sidebar_index(1, 20), Some(7), "首行 = 渲染偏移");
        assert_eq!(areas.sidebar_index(4, 20), Some(10));
        assert_eq!(areas.sidebar_index(4, 9), None, "偏移 + 行号越界");
    }

    #[test]
    fn sidebar_index_is_none_when_no_visible_rows() {
        let areas = HitAreas {
            sidebar: rect(0, 0, 26, 2), // 只剩上下边框
            sidebar_offset: 0,
            ..HitAreas::default()
        };
        assert_eq!(areas.sidebar_index(1, 5), None);
    }
}
