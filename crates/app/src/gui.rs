//! `gui` 子命令：二期图形界面（`D3` 方案 A，需 `--features gui` 构建）。
//!
//! 线程模型（`X2` §5）：
//!
//! ```text
//! 主线程      GPUI 事件循环（`application().run`）—— 绝不 block_on
//! 后台线程池   tokio runtime：worker 跑在这里
//! 桥           tokio mpsc → GPUI 侧 `cx.spawn` 里 `recv().await` → `view.update`
//! ```
//!
//! 与 TUI 的关系：**同一份 worker、同一份 `ChatUi` 状态机、同一份 `Cmd`/`UiEvent`**，
//! 差别只在渲染端（`features/chat/src/gpui.rs` vs `ui.rs`）。

use std::sync::Arc;

use agent_chat::{ChatService, ChatView, Cmd, StatusInfo, UiEvent};
use agent_settings::SettingsService;
use agent_settings::ui::SettingsView;
use agent_ui_kit::UiTheme;
use agent_ui_kit::kit::*;
use tokio::sync::mpsc;

use crate::setup::{self, ChatArgs};
use crate::worker;

/// `gui` 子命令参数。
#[derive(Debug, Clone)]
pub struct GuiArgs {
    pub provider: String,
    pub model: String,
    /// 降级候选（`--model a,b` 的 b…，`R2`）。
    pub fallbacks: Vec<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    /// 配置文件路径（`R1`）。
    pub config: Option<String>,
    /// SQLite 路径（缺省 `./agent-sessions.db`）。
    pub db: Option<String>,
}

/// 起窗口并跑消息循环，返回进程退出码。
///
/// 同步函数：GPUI 的 `application().run` 自己占住主线程，不需要（也不能）`block_on`。
pub fn run(args: GuiArgs) -> i32 {
    let chat_args = ChatArgs {
        provider: args.provider.clone(),
        model: args.model.clone(),
        fallbacks: args.fallbacks.clone(),
        base_url: args.base_url.clone(),
        api_key: args.api_key.clone(),
        config: args.config.clone(),
    };
    let Some(provider) = setup::build_provider(&chat_args) else {
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

    let model = args.model.clone();
    let service = ChatService::new(provider, setup::demo_tools());

    // 后台 runtime：worker 跑在这里，UI 线程只收事件。
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("创建 tokio 运行时失败：{error}");
            return 2;
        }
    };

    let cancel_slot = worker::cancel_slot();
    let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>(32);
    let (ui_tx, mut ui_rx) = mpsc::unbounded_channel::<UiEvent>();
    runtime.spawn(worker::run_worker(
        model,
        service,
        repo,
        cancel_slot.clone(),
        cmd_rx,
        ui_tx,
    ));

    let status = StatusInfo {
        provider: args.provider.clone(),
        model: args.model.clone(),
    };

    // 设置面板的数据入口。定位配置目录失败不该挡启动（`R1`：配置问题不挡「跑起来」），
    // 只是设置页不可用——所以这里降级为一句可读的提示。
    let (settings_service, settings_error) = match SettingsService::discover() {
        Ok(service) => (Some(Arc::new(service)), None),
        Err(error) => (
            None,
            Some(format!("定位配置目录失败，设置页不可用：{error}")),
        ),
    };

    application().run(move |cx| {
        // 组件库 + 设计令牌主题（必须在开窗之前）
        agent_ui_kit::init(cx);
        open_window(window_options(), cx, |window, cx| {
            let chat = cx.new(|cx| {
                ChatView::new(
                    window,
                    cx,
                    status.clone(),
                    cmd_tx.clone(),
                    cancel_slot.clone(),
                )
            });
            // 桥：worker 的事件流 → 视图。`rx.recv().await` 在 GPUI 的执行器里轮询
            // （`X2` §8 已验证过这个形态）。
            chat.update(cx, |_view, cx| {
                cx.spawn(async move |this, cx| {
                    while let Some(event) = ui_rx.recv().await {
                        if this.update(cx, |view, cx| view.apply(event, cx)).is_err() {
                            break; // 视图已销毁
                        }
                    }
                })
                .detach();
            });

            let settings = settings_service
                .as_ref()
                .map(|service| cx.new(|cx| SettingsView::new(window, cx, service.clone())));

            cx.new(|_| Shell {
                chat,
                settings,
                settings_error: settings_error.clone(),
                tab: Tab::Chat,
            })
        })
        .expect("open window");
    });

    // 走到这里说明窗口已关闭：runtime 随本函数返回而 drop，worker 一并收尾。
    let _ = runtime;
    0
}

/// 窗口参数：给个像样的初始尺寸与标题（`WindowBounds` 在屏幕坐标里）。
fn window_options() -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(80.), px(80.)),
            size: size(px(1100.), px(760.)),
        })),
        titlebar: Some(TitlebarOptions {
            title: Some("CodingRocket".into()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

// —— 外壳：标签页（对话 / 设置） ——
//
// 放在 `app` 是因为它**跨功能**组装两个 `features/*` 的视图（`D6`：组装归宿主）。
// 各功能自己不知道对方存在。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Chat,
    Settings,
}

struct Shell {
    chat: Entity<ChatView>,
    /// `None` = 定位配置目录失败（原因在 `settings_error`）。
    settings: Option<Entity<SettingsView>>,
    settings_error: Option<String>,
    tab: Tab,
}

impl Render for Shell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = UiTheme::global(cx);
        let (bg, fg, dim, border, surface, accent) = (
            theme.colors.background,
            theme.colors.foreground,
            theme.colors.muted_foreground,
            theme.colors.border,
            theme.colors.surface,
            theme.colors.accent,
        );

        let body = match self.tab {
            Tab::Chat => div().flex().flex_1().min_h(px(0.)).child(self.chat.clone()),
            Tab::Settings => match &self.settings {
                Some(view) => div().flex().flex_1().min_h(px(0.)).child(view.clone()),
                None => div().p_4().text_color(dim).child(
                    self.settings_error
                        .clone()
                        .unwrap_or_else(|| "设置页不可用".to_string()),
                ),
            },
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(bg)
            .text_color(fg)
            .text_sm()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .bg(surface)
                    .border_b_1()
                    .border_color(border)
                    .child(tab_button(
                        "tab-chat",
                        "对话",
                        self.tab == Tab::Chat,
                        accent,
                        dim,
                        cx.listener(|this, _, _, cx| this.select(Tab::Chat, cx)),
                    ))
                    .child(tab_button(
                        "tab-settings",
                        "设置",
                        self.tab == Tab::Settings,
                        accent,
                        dim,
                        cx.listener(|this, _, window, cx| this.select_settings(window, cx)),
                    )),
            )
            .child(body)
    }
}

impl Shell {
    fn select(&mut self, tab: Tab, cx: &mut Context<Self>) {
        self.tab = tab;
        cx.notify();
    }

    /// 切到设置页时重新读盘：用户可能刚在外部改过（也可能刚把它改坏）。
    fn select_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.tab = Tab::Settings;
        if let Some(view) = self.settings.clone() {
            view.update(cx, |view, cx| view.reload(window, cx));
        }
        cx.notify();
    }
}

fn tab_button(
    id: &'static str,
    label: &'static str,
    active: bool,
    accent: Hsla,
    dim: Hsla,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    div()
        .id(ElementId::Name(id.into()))
        .px_2()
        .py_1()
        .rounded_md()
        .cursor_pointer()
        .text_color(if active { accent } else { dim })
        .hover(|d| d.text_color(accent))
        .on_click(on_click)
        .child(label)
        .into_any_element()
}
