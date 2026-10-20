//! GPUI 渲染端（二期，`D3` 方案 A / `D4`）。
//!
//! 与 [`crate::ui`] 的 ratatui 渲染端**并列**：消费同一个 [`ChatUi`] 状态机，
//! 一个业务判断都不重写（`chat/README.md`：「二期换 GPUI 只换渲染端」）。
//! 数据流与 TUI 完全一致（`D3`「事件流 + 命令通道」，见 [`crate::channel`]）：
//!
//! ```text
//! 宿主 worker ──UiEvent──▶ ChatView::apply ──▶ ChatUi::on_event
//! 用户操作 ──▶ ChatView ──Cmd──▶ 宿主 worker
//! ```
//!
//! 本模块只在 `gui` feature 下编译：GPUI 的依赖树与体积不进一期 TUI 产物。

use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use agent_ui_kit::UiTheme;
use agent_ui_kit::component::button::{Button, ButtonVariants as _};
use agent_ui_kit::component::input::{Input, InputEvent, InputState};
use agent_ui_kit::component::list::ListItem;
use agent_ui_kit::component::popover::{Popover, PopoverState};
use agent_ui_kit::component::spinner::Spinner;
use agent_ui_kit::component::{Icon, IconName, Sizable as _};
use agent_ui_kit::kit::prelude::FluentBuilder as _;
use agent_ui_kit::kit::*;
use tokio_util::sync::CancellationToken;

use crate::channel::{Cmd, UiEvent};
use crate::repo::SessionMeta;
use crate::ui::{ChatUi, Entry, ToolStatus};

/// 状态栏数据：`ChatUi` 不知道 provider / model，这部分由宿主喂。
#[derive(Debug, Clone)]
pub struct StatusInfo {
    pub provider: String,
    pub model: String,
}

/// 一帧用的配色拷贝。
///
/// [`UiTheme`] 是 `Global`，`UiTheme::global(cx)` 借 `cx`；而构建 element 树时还要
/// `&mut Context`（点击回调）。`Hsla` 是 `Copy`，拷一份出来借用就结束了。
#[derive(Debug, Clone, Copy)]
struct Palette {
    bg: Hsla,
    fg: Hsla,
    dim: Hsla,
    surface: Hsla,
    /// 比 surface 亮一档的台面色（选中 / 悬浮 / 用户气泡）。
    secondary: Hsla,
    accent: Hsla,
    thinking: Hsla,
    tool: Hsla,
    ok: Hsla,
    err: Hsla,
}

impl Palette {
    fn of(cx: &App) -> Self {
        let t = UiTheme::global(cx);
        Self {
            bg: t.colors.background,
            fg: t.colors.foreground,
            dim: t.colors.muted_foreground,
            surface: t.colors.surface,
            secondary: t.colors.secondary,
            accent: t.colors.accent,
            thinking: t.chat.thinking,
            tool: t.chat.tool,
            ok: t.chat.ok,
            err: t.chat.err,
        }
    }
}

/// 一个可选项：供应商 + 模型。
///
/// **清单由宿主提供**（`app::setup::model_choices`）：`features/*` 不认识供应商名单，
/// 那是装配层的知识（`D6`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelChoice {
    pub provider: String,
    pub model: String,
}

/// 对话视图。宿主持有 `Entity<ChatView>`，用 [`ChatView::apply`] 喂事件。
pub struct ChatView {
    /// 状态机：与 TUI 是同一个（`ui.rs`）。
    ui: ChatUi,
    /// 输入框（gpui-component 的 `InputState`，带 IME 支持）。
    input: Entity<InputState>,
    /// 转录区滚动句柄；`ChatUi` 说「跟随底部」时吸到底。
    scroll: ScrollHandle,
    /// 命令通道：用户操作发回 worker。
    cmd_tx: tokio::sync::mpsc::Sender<Cmd>,
    /// 取消令牌共享槽位（与 TUI 同一套约定：轮内不接命令，取消走槽位）。
    cancel_slot: Arc<Mutex<Option<CancellationToken>>>,
    status: StatusInfo,
    /// 候选清单（宿主给的，见 `ModelChoice`）+ 手输模型名（空态时把它当模型名用）。
    candidates: Vec<ModelChoice>,
    /// 模型选择器的搜索框。**必须在 `new` 里建一次**：`Popover::content` 每帧都重渲染，
    /// 在里面 `cx.new` 会每帧造一个 entity（`popover.rs:250` 明确警告）。
    model_search: Entity<InputState>,
    /// 弹层开关。受控：`.open(..)` 读它，`.on_open_change(..)` 写它——这样"点条目后关闭"
    /// 才由我们说了算（否则只能等点面板外面）。
    model_menu_open: bool,
    /// 弹层里模型列表的滚动句柄。
    model_scroll: ScrollHandle,
    /// 弹层里「管理模型」被点时的回调 —— **由宿主接线**（切片 B：切到设置页的模型分区）。
    ///
    /// `features/chat` 不认识设置页、也读不了偏好文件（`D5`/`D6`），所以只留一个口子。
    /// 用 `Rc<dyn Fn>` 而不是加一条通道：它只回传一个「用户想看设置」的信号，不是 `Cmd`
    /// （那个是发给 worker 的，见 `channel.rs`）。
    on_manage_models: Rc<dyn Fn(&mut App)>,
    sessions: Vec<SessionMeta>,
    session_id: Option<String>,
    session_title: Option<String>,
    /// 订阅必须持有，drop 即取消订阅。
    _sub: Subscription,
    /// 搜索框订阅（内容一变就重渲染 → 重新过滤）。同上，必须持有。
    _search_sub: Subscription,
}

impl ChatView {
    /// 建视图。`cmd_tx` / `cancel_slot` 与宿主 worker 共用（见 `app` 的宿主装配）。
    ///
    /// `candidates` 是**过滤后**的清单（被用户在「管理模型」里关掉的已剔除）——
    /// 过滤在装配层做（`app::setup::visible_choices`），因为偏好文件是宿主的活。
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        status: StatusInfo,
        candidates: Vec<ModelChoice>,
        cmd_tx: tokio::sync::mpsc::Sender<Cmd>,
        cancel_slot: Arc<Mutex<Option<CancellationToken>>>,
        on_manage_models: Rc<dyn Fn(&mut App)>,
    ) -> Self {
        let input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("输入消息，Enter 发送（Shift+Enter 换行）")
        });
        let model_search = cx.new(|cx| InputState::new(window, cx).placeholder("搜索模型…"));
        // 订阅输入事件：Enter 提交。`Input` 自己处理 IME 与组合串，这里只认「按了回车」。
        let sub = cx.subscribe_in(
            &input,
            window,
            |this, _state, event: &InputEvent, window, cx| {
                if let InputEvent::PressEnter { secondary, shift } = event
                    && !secondary
                    && !shift
                {
                    this.submit(window, cx);
                }
            },
        );
        // 搜索框只用来过滤候选：内容一变就重渲染，过滤在渲染期算（见 `model_capsule`）。
        let search_sub = cx.subscribe_in(
            &model_search,
            window,
            |_this, _state, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            },
        );
        // 焦点直接给输入框：打开窗口就能打字（`Focusable::focus_handle` 收 cx）
        let focus = input.focus_handle(cx);
        window.focus(&focus, cx);

        Self {
            ui: ChatUi::new(),
            input,
            scroll: ScrollHandle::new(),
            cmd_tx,
            cancel_slot,
            status,
            candidates,
            model_search,
            model_menu_open: false,
            model_scroll: ScrollHandle::new(),
            on_manage_models,
            sessions: Vec::new(),
            session_id: None,
            session_title: None,
            _sub: sub,
            _search_sub: search_sub,
        }
    }

    /// 换掉模型选择器的候选清单（宿主在「连接提供商」成功后调，无需重启）。
    ///
    /// 清单由宿主编排（`app::setup::visible_choices`）—— 本视图不认识供应商目录（`D6`）。
    pub fn set_candidates(&mut self, candidates: Vec<ModelChoice>, cx: &mut Context<Self>) {
        self.candidates = candidates;
        cx.notify();
    }

    /// 消费一个宿主事件（与 TUI 的 `apply` 一一对应）。
    pub fn apply(&mut self, event: UiEvent, cx: &mut Context<Self>) {
        match event {
            UiEvent::Sessions(list) => self.sessions = list,
            UiEvent::SessionLoaded { title, chat } => {
                self.ui.load_chat(&chat);
                self.session_title = Some(title);
            }
            UiEvent::Cleared => {
                self.ui.reset();
                self.session_id = None;
                self.session_title = None;
            }
            UiEvent::Loop(event) => self.ui.on_event(event),
            // 生效组合以 worker 为准（`channel.rs` 的约定：宿主不自己猜）——
            // 否则切换失败（缺密钥 / 供应商未知）时界面会显示成成功的
            UiEvent::ModelChanged { provider, model } => {
                self.status.provider = provider;
                self.status.model = model;
            }
            UiEvent::Rejected(message) => self.ui.rejected(message),
            UiEvent::Notice(message) => self.ui.set_notice(message),
            // 联网刷新（C2）由设置页发起；对话视图只报个提示 ——
            // 候选清单的重算在宿主侧（`setup::model_choices` 已把拉到的模型纳入）。
            UiEvent::ModelsRefreshed { provider, models } => self
                .ui
                .set_notice(format!("{provider} 模型清单已刷新（{} 个）", models.len())),
            // 失败：宿主会**另外**把它灌回设置页的状态栏（发起刷新的那一页，`C2d`）。
            // 这里报提示是给对话页兜底 —— 两个渲染端消费同一套事件，都得穷尽。
            UiEvent::ModelsRefreshFailed { provider, message } => self
                .ui
                .set_notice(format!("刷新 {provider} 模型失败：{message}")),
        }
        self.follow_bottom();
        cx.notify();
    }

    /// 生成中取消本轮（与 TUI 的 Ctrl-C 语义一致：丢弃未提交输出）。
    pub fn cancel(&mut self, cx: &mut Context<Self>) {
        if let Some(token) = self.cancel_slot.lock().expect("cancel slot").as_ref() {
            token.cancel();
        }
        cx.notify();
    }

    fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.ui.generating() {
            return;
        }
        let text = self.input.read(cx).value().trim().to_string();
        if text.is_empty() {
            return;
        }
        if self.cmd_tx.try_send(Cmd::Submit(text.clone())).is_err() {
            self.ui.set_notice("命令通道繁忙，稍后再试".to_string());
            cx.notify();
            return;
        }
        // 立刻上屏：取消也只丢模型输出，不丢用户输入（与 REPL / `ChatUi::push_user` 一致）
        self.ui.push_user(&text);
        self.input
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.follow_bottom();
        cx.notify();
    }

    fn pick_session(&mut self, id: String, title: String, cx: &mut Context<Self>) {
        if self.ui.generating() {
            self.ui.set_notice("生成中：先取消，再切会话".to_string());
            cx.notify();
            return;
        }
        if self
            .cmd_tx
            .try_send(Cmd::Load {
                id: id.clone(),
                title,
            })
            .is_err()
        {
            self.ui.set_notice("命令通道繁忙，稍后再试".to_string());
        } else {
            self.session_id = Some(id);
        }
        cx.notify();
    }

    fn new_session(&mut self, cx: &mut Context<Self>) {
        if self.ui.generating() {
            self.ui.set_notice("生成中：先取消，再新建".to_string());
            cx.notify();
            return;
        }
        if self.cmd_tx.try_send(Cmd::New).is_err() {
            self.ui.set_notice("命令通道繁忙，稍后再试".to_string());
        } else {
            self.session_id = None;
            self.session_title = None;
        }
        cx.notify();
    }

    /// 切模型：provider + model **一起**切（密钥按 provider 分别存，见 `Cmd::SelectModel`）。
    ///
    /// 界面**不做乐观更新**：状态栏等 worker 回 `UiEvent::ModelChanged` 才变，
    /// 这样「缺密钥切不过去」时不会先显示成功。
    fn pick_model(&mut self, choice: ModelChoice, cx: &mut Context<Self>) {
        if self.ui.generating() {
            self.ui.set_notice("生成中：先取消，再切模型".to_string());
            cx.notify();
            return;
        }
        if choice.provider == self.status.provider && choice.model == self.status.model {
            return;
        }
        if self
            .cmd_tx
            .try_send(Cmd::SelectModel {
                provider: choice.provider,
                model: choice.model,
            })
            .is_err()
        {
            self.ui.set_notice("命令通道繁忙，稍后再试".to_string());
        }
        cx.notify();
    }

    /// 用**手输的模型名**切换（provider 不变）—— 自定义端点（vLLM / 本地服务）用。
    ///
    /// 入口在弹层的空态里：搜不到任何候选时，把搜索框里那串字当成模型名。
    fn use_model_name(&mut self, model: String, cx: &mut Context<Self>) {
        let model = model.trim().to_string();
        if model.is_empty() {
            return;
        }
        let provider = self.status.provider.clone();
        self.pick_model(ModelChoice { provider, model }, cx);
    }

    /// `ChatUi` 说「跟随底部」（`scroll() == None`）时把转录吸到底。
    fn follow_bottom(&self) {
        if self.ui.scroll().is_none() {
            self.scroll.scroll_to_bottom();
        }
    }
}

// —— 渲染 ——

impl Render for ChatView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Palette::of(cx);
        div()
            .flex()
            .flex_row()
            .size_full()
            .bg(p.bg)
            .text_color(p.fg)
            .text_sm()
            .child(self.sidebar(cx, p))
            .child(self.main(cx, p))
    }
}

impl ChatView {
    fn sidebar(&self, cx: &mut Context<Self>, p: Palette) -> AnyElement {
        let current = self.session_id.clone();
        // `.id()` 不能省：`overflow_y_scroll` / `track_scroll` 属于 StatefulInteractiveElement
        let mut list = div()
            .id("session-list")
            .flex()
            .flex_col()
            .gap_0p5()
            .flex_1()
            .overflow_y_scroll();

        if self.sessions.is_empty() {
            list = list.child(
                div()
                    .px_2()
                    .py_2()
                    .text_xs()
                    .text_color(p.dim)
                    .child("还没有会话。发一条消息即新建。"),
            );
        }
        for meta in self.sessions.clone() {
            let selected = current.as_deref() == Some(meta.id.as_str());
            let (id, title) = (meta.id.clone(), meta.title.clone());
            list = list.child(
                div()
                    .id(ElementId::Name(id.clone().into()))
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .px_2()
                    .py_1p5()
                    .rounded_md()
                    .cursor_pointer()
                    // 选中用 accent 淡彩底（色相区分），悬浮用 secondary 实底（色阶区分）——
                    // 两者不再是同一个色，状态可辨。
                    .when(selected, |d| d.bg(p.accent.alpha(0.14)))
                    .hover(|d| d.bg(p.secondary))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.pick_session(id.clone(), title.clone(), cx)
                    }))
                    .child(
                        div()
                            .truncate()
                            .when(selected, |d| d.font_weight(FontWeight::SEMIBOLD))
                            .child(meta.title.clone()),
                    )
                    .child(div().text_xs().text_color(p.dim).truncate().child(format!(
                        "{} · {}",
                        meta.model,
                        ago(meta.updated_at)
                    ))),
            );
        }

        // 侧栏与主区只靠 surface / bg 的色阶分层，不画分隔线。
        div()
            .flex()
            .flex_col()
            .gap_2()
            .w(px(220.))
            .h_full()
            .p_2()
            .bg(p.surface)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.fg)
                            .child("会话"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(p.dim)
                            .child(self.sessions.len().to_string()),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("new-session")
                            .ghost()
                            .compact()
                            .icon(IconName::Plus)
                            .tooltip("新建会话")
                            .on_click(cx.listener(|this, _, _, cx| this.new_session(cx))),
                    ),
            )
            .child(list)
            .into_any_element()
    }

    fn main(&self, cx: &mut Context<Self>, p: Palette) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(0.))
            .h_full()
            .child(self.transcript(p))
            .child(self.status_bar(cx, p))
            .into_any_element()
    }

    fn transcript(&self, p: Palette) -> AnyElement {
        let mut rows = div()
            .id("transcript")
            .flex()
            .flex_col()
            .gap_4()
            .flex_1()
            .min_h(px(0.))
            .p_4()
            .overflow_y_scroll()
            .track_scroll(&self.scroll);

        if self.ui.entries().is_empty() {
            // 空态说清「做什么」，不只说「空着」。
            rows = rows.child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .flex_1()
                    .gap_2()
                    .child(div().text_lg().text_color(p.fg).child("问我点什么吧"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(p.dim)
                            .child("Enter 发送，Shift+Enter 换行"),
                    ),
            );
        }
        // 克隆一份再渲染：构建 element 树时不再借 `self`，避免与 `cx` 的借用打架。
        // （G-3 上万条消息要换虚拟列表，那时这条克隆也得去掉。）
        for entry in self.ui.entries().to_vec() {
            rows = rows.child(entry_row(entry, p));
        }
        rows.into_any_element()
    }

    fn status_bar(&self, cx: &mut Context<Self>, p: Palette) -> AnyElement {
        let usage = self.ui.total_usage();
        let generating = self.ui.generating();

        let hint = match (&self.session_title, self.ui.notice()) {
            (_, Some(notice)) => (notice.to_string(), p.err),
            (Some(title), None) => (title.clone(), p.dim),
            (None, None) => ("新会话".to_string(), p.dim),
        };

        // 生成中：取消用 outline（与发送的实心主按钮区分）。
        let action = if generating {
            Button::new("cancel")
                .outline()
                .label("取消")
                .on_click(cx.listener(|this, _, _, cx| this.cancel(cx)))
                .into_any_element()
        } else {
            Button::new("send")
                .label("发送")
                .on_click(cx.listener(|this, _, window, cx| this.submit(window, cx)))
                .into_any_element()
        };

        // 输入面板用 surface 底色从 bg 里浮起，不再画分隔线。
        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .bg(p.surface)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(self.model_capsule(cx, p))
                    // provider 跟在胶囊旁（原来底部另有一行 provider/model，信息重复）
                    .child(
                        div()
                            .text_xs()
                            .text_color(p.dim)
                            .truncate()
                            .child(self.status.provider.clone()),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_xs()
                            .text_color(p.dim)
                            .child(format!("{} tok", usage.total())),
                    )
                    .when(generating, |d| {
                        d.child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap_1()
                                .child(Spinner::new().color(p.thinking).with_size(px(14.)))
                                .child(div().text_xs().text_color(p.thinking).child("生成中")),
                        )
                    }),
            )
            .child(Input::new(&self.input))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .text_xs()
                            .text_color(hint.1)
                            .truncate()
                            .child(hint.0),
                    )
                    .child(action),
            )
            .into_any_element()
    }

    /// 输入条上方的模型选择器：一个胶囊 + 受控弹层（搜索 / 按供应商分组 / 打勾 / 管理模型）。
    ///
    /// 语义与 TUI 一致：点一项发 `Cmd::SelectModel { provider, model }` —— **一起切**
    /// （密钥按 provider 存，换 provider 必须连带重建实例，见 `channel.rs`）。
    /// 界面**不做乐观更新**：状态栏等 worker 回 `UiEvent::ModelChanged` 才变。
    fn model_capsule(&self, cx: &mut Context<Self>, p: Palette) -> AnyElement {
        let entity = cx.entity();
        let search = self.model_search.clone();
        let scroll = self.model_scroll.clone();
        let open = self.model_menu_open;
        let generating = self.ui.generating();

        // 过滤 + 按 provider 分组。候选几十到几百条，每帧重算足够。
        let query = self.model_search.read(cx).value().trim().to_lowercase();
        let typed = self.model_search.read(cx).value().trim().to_string();
        let mut groups: Vec<(String, Vec<ModelChoice>)> = Vec::new();
        for choice in &self.candidates {
            let hit = query.is_empty()
                || choice.model.to_lowercase().contains(&query)
                || choice.provider.to_lowercase().contains(&query);
            if !hit {
                continue;
            }
            match groups.last_mut() {
                // 去重：同一 (provider, model) 只留一条（行的 ElementId 由二者拼成）。
                Some((provider, rows))
                    if provider == &choice.provider
                        && !rows.iter().any(|row| row.model == choice.model) =>
                {
                    rows.push(choice.clone());
                }
                Some(_) => {}
                None => groups.push((choice.provider.clone(), vec![choice.clone()])),
            }
        }

        let current_provider = self.status.provider.clone();
        let current_model = self.status.model.clone();
        let (dim, accent) = (p.dim, p.accent);

        // `content` 是 `move` 闭包，会按值吃掉 `entity` —— 先留一份给 `on_open_change`。
        let content_entity = entity.clone();
        let content = move |_state: &mut PopoverState,
                            _window: &mut Window,
                            _cx: &mut Context<PopoverState>| {
            let mut list = div()
                .id("model-list")
                .flex()
                .flex_col()
                .max_h(px(320.))
                .overflow_y_scroll()
                .track_scroll(&scroll);

            for (provider, rows) in &groups {
                list = list.child(
                    div()
                        .px_2()
                        .pt_2()
                        .pb_1()
                        .text_xs()
                        .text_color(dim)
                        .child(short_provider(provider).to_string()),
                );
                for choice in rows {
                    let is_current =
                        choice.provider == current_provider && choice.model == current_model;
                    let label = choice.model.clone();
                    let id = format!("model::{}::{}", choice.provider, choice.model);
                    let choice = choice.clone();
                    let entity = content_entity.clone();
                    list = list.child(
                        ListItem::new(SharedString::from(id))
                            .selected(is_current)
                            // `check_icon` 只在 `confirmed` 为真时才画勾（`list_item.rs:245`）
                            .confirmed(is_current)
                            .check_icon(IconName::Check)
                            .on_click(move |_, _, cx| {
                                entity.update(cx, |view, cx| {
                                    view.model_menu_open = false;
                                    if generating {
                                        view.ui.set_notice("生成中：先取消，再切模型".to_string());
                                    } else {
                                        view.pick_model(choice.clone(), cx);
                                    }
                                    cx.notify();
                                });
                            })
                            .child(label),
                    );
                }
            }

            if groups.is_empty() {
                if typed.is_empty() {
                    list = list.child(
                        div()
                            .px_2()
                            .py_2()
                            .text_xs()
                            .text_color(dim)
                            .child("没有候选模型（宿主清单为空）"),
                    );
                } else {
                    // 搜不到 → 直接把这串字当模型名（自定义端点常见：服务端有不认识的模型）
                    let name = typed.clone();
                    let entity = content_entity.clone();
                    list = list.child(
                        ListItem::new("model-use-typed")
                            .on_click(move |_, _, cx| {
                                entity.update(cx, |view, cx| {
                                    view.model_menu_open = false;
                                    view.use_model_name(name.clone(), cx);
                                    cx.notify();
                                });
                            })
                            .child(div().text_color(accent).child(format!("使用「{typed}」"))),
                    );
                }
            }

            // 底部：管理模型（切片 B 接「开关 + 落盘」）。用间距分组，不画分隔线。
            list = list.child(
                div().mt_1().child(
                    ListItem::new("model-manage")
                        .on_click({
                            let entity = content_entity.clone();
                            move |_, _, cx| {
                                // 先把回调取出来再调：`update` 借 `view`，回调要 `&mut App`，
                                // 借完再调才不会撞借用（`Rc` 是 `clone` 出来的，不带借用）。
                                let callback = entity.update(cx, |view, cx| {
                                    view.model_menu_open = false;
                                    cx.notify();
                                    view.on_manage_models.clone()
                                });
                                callback(cx);
                            }
                        })
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap_2()
                                .child(Icon::new(IconName::Settings2).text_color(dim))
                                .child("管理模型"),
                        ),
                ),
            );

            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    Input::new(&search)
                        .prefix(Icon::new(IconName::Search).text_color(dim))
                        .cleanable(true),
                )
                .child(list)
                .into_any_element()
        };

        let on_open_change = {
            let entity = entity.clone();
            let search = self.model_search.clone();
            move |is_open: &bool, window: &mut Window, cx: &mut App| {
                let is_open = *is_open;
                entity.update(cx, |view, cx| {
                    view.model_menu_open = is_open;
                    cx.notify();
                });
                if is_open {
                    // 打开就把焦点给搜索框（不给的话还得再点一次才能打字）
                    let handle = search.focus_handle(cx);
                    window.focus(&handle, cx);
                }
            }
        };

        Popover::new("model-picker")
            // 面板开在胶囊**上方**：`BottomLeft` = 触发元素下边缘、左对齐（`popover.rs:157`）
            .anchor(Anchor::BottomLeft)
            .offset(px(6.))
            .open(open)
            .w(px(300.))
            .on_open_change(on_open_change)
            .trigger(
                Button::new("model-capsule")
                    .ghost()
                    .compact()
                    .icon(IconName::Asterisk)
                    .label(self.status.model.clone())
                    .dropdown_caret(true),
            )
            .content(content)
            .into_any_element()
    }
}

/// 供应商短名（分组标题上省地方）：`openai-compatible` → `openai`。
fn short_provider(provider: &str) -> &str {
    provider.strip_suffix("-compatible").unwrap_or(provider)
}

/// 一条转录条目 → 一个块。层级即信息：
/// 用户 = 右对齐实底气泡；助理 = 左对齐全墨正文；思考 = 小字暗紫；工具 = 图标 + 状态签。
fn entry_row(entry: Entry, p: Palette) -> AnyElement {
    match entry {
        // 用户消息：右对齐气泡，全墨正文（次要信息才用灰，正文不用）。
        Entry::User(text) => div()
            .flex()
            .flex_row()
            .justify_end()
            .child(
                div()
                    .max_w(relative(0.72))
                    .bg(p.secondary)
                    .rounded_lg()
                    .px_3()
                    .py_2()
                    .text_base()
                    .text_color(p.fg)
                    .child(text),
            )
            .into_any_element(),
        // 助理消息：左对齐，Bot 图标作身份，正文用 base 字号（14px 的全局小字不适合长文）。
        Entry::Assistant(text) => div()
            .flex()
            .flex_row()
            .items_start()
            .gap_2()
            .child(
                Icon::new(IconName::Bot)
                    .text_color(p.dim)
                    .flex_none()
                    .mt_0p5()
                    .with_size(px(16.)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .text_base()
                    .text_color(p.fg)
                    .child(text),
            )
            .into_any_element(),
        // 思考过程：小字、思考色，一个「思考」文字签（不用 glyph 图标）。
        Entry::Thinking(text) => div()
            .flex()
            .flex_row()
            .items_start()
            .gap_2()
            .child(
                div()
                    .flex_none()
                    .text_xs()
                    .text_color(p.thinking)
                    .mt_0p5()
                    .child("思考"),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .text_xs()
                    .text_color(p.thinking)
                    .child(text),
            )
            .into_any_element(),
        // 工具调用：终端图标（替代原来的 ⚙ emoji）+ 名称 + 状态签。
        Entry::Tool { name, status } => {
            let (label, color) = match status {
                Some(ToolStatus::Running) => ("运行中", p.tool),
                Some(ToolStatus::Ok) => ("完成", p.ok),
                Some(ToolStatus::Failed) => ("失败", p.err),
                None => ("", p.dim),
            };
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    Icon::new(IconName::SquareTerminal)
                        .text_color(p.tool)
                        .flex_none()
                        .with_size(px(16.)),
                )
                .child(div().flex_none().text_sm().text_color(p.fg).child(name))
                .when(!label.is_empty(), |d| {
                    d.child(div().flex_none().text_xs().text_color(color).child(label))
                })
                .into_any_element()
        }
        // 工具结果：缩进一档的次要文字。
        Entry::ToolResult(text) => div()
            .flex()
            .flex_row()
            .items_start()
            .gap_2()
            .child(div().flex_none().text_color(p.dim).child("↳"))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .text_sm()
                    .text_color(p.dim)
                    .child(text),
            )
            .into_any_element(),
        // 备注：最弱一档。
        Entry::Note(text) => div()
            .flex()
            .flex_row()
            .items_start()
            .gap_2()
            .child(div().flex_none().text_color(p.dim).child("—"))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .text_sm()
                    .text_color(p.dim)
                    .child(text),
            )
            .into_any_element(),
    }
}

/// 毫秒时间戳 → 相对时间。列表只是提示「哪个新」，不追求精确。
fn ago(ms: i64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let seconds = (now - ms).max(0) / 1000;
    if seconds < 60 {
        "刚刚".to_string()
    } else if seconds < 3_600 {
        format!("{} 分钟前", seconds / 60)
    } else if seconds < 86_400 {
        format!("{} 小时前", seconds / 3_600)
    } else {
        format!("{} 天前", seconds / 86_400)
    }
}
