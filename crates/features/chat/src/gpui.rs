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
//! 渲染全部走 **Ely GPUI Component**（`crates/kernel/ui-kit` 统一重导出）：
//! `Sidebar` + `ConversationItem` 做会话栏，`ChatContainer` + `MessageList` +
//! `MessageBubble` + `StreamingMarkdown` 做转录区（虚拟列表，`G-3` 顺带解决），
//! `PromptInput` 做输入区（自带 Enter 发送 / busy→stop / 拖动文件）。
//!
//! 本模块只在 `gui` feature 下编译：GPUI 的依赖树与体积不进一期 TUI 产物。

use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use agent_ui_kit::ely::buttons::{Button, ButtonVariant, IconButton};
use agent_ui_kit::ely::chat::{
    ChatContainer, Conversation, ConversationItem, MessageBubble, MessageList, PromptInput, Role,
    StreamingMarkdown,
};
use agent_ui_kit::ely::forms::{InputEvent as TextEvent, SearchInput, TextInput};
use agent_ui_kit::ely::layout::Sidebar;
use agent_ui_kit::ely::overlays::Popover;
use agent_ui_kit::ely::primitives::{Icon, IconName};
use agent_ui_kit::ely::theme::{ActiveTheme, ControlSize, IconSize, Palette, TextSize};
use agent_ui_kit::kit::prelude::FluentBuilder as _;
use agent_ui_kit::kit::*;
use jiff::Timestamp;
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

/// 一帧用的字号档位（`Styled::text_size` 收 `Into<AbsoluteLength>`，
/// 而 `Theme::text_size` 给的是 `Rems` —— 先取出来，避免每条元素都算一遍）。
#[derive(Debug, Clone, Copy)]
struct Type {
    xs: Rems,
    sm: Rems,
    lg: Rems,
}

impl Type {
    fn of(cx: &App) -> Self {
        let theme = cx.theme();
        Self {
            xs: theme.text_size(TextSize::Xs),
            sm: theme.text_size(TextSize::Sm),
            lg: theme.text_size(TextSize::Lg),
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
    /// 输入框（Ely 的 `TextInput`，带 IME / 多行 / 撤销）。
    input: Entity<TextInput>,
    /// 转录快照：`MessageList` 的渲染闭包要 `'static`，拿不到 `&self`，
    /// 所以在状态变化后整份拷成一个 `Rc`（渲染时只做引用计数）。
    entries: Rc<Vec<Entry>>,
    /// 命令通道：用户操作发回 worker。
    cmd_tx: tokio::sync::mpsc::Sender<Cmd>,
    /// 取消令牌共享槽位（与 TUI 同一套约定：轮内不接命令，取消走槽位）。
    cancel_slot: Arc<Mutex<Option<CancellationToken>>>,
    status: StatusInfo,
    /// 候选清单（宿主给的，见 `ModelChoice`）+ 手输模型名（空态时把它当模型名用）。
    candidates: Vec<ModelChoice>,
    /// 模型选择器的搜索框。**必须在 `new` 里建一次**：弹层每帧重渲染，
    /// 在里面 `cx.new` 会每帧造一个 entity。
    model_search: Entity<TextInput>,
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
        // 单行起、最多 8 行：Ely 的 `PromptInput` 自己处理 Enter / Shift-Enter，
        // 这里只要把多行的上限给它。
        let input = cx.new(|cx| {
            TextInput::new(window, cx)
                .multi_line(1, 8)
                .placeholder("输入消息，Enter 发送（Shift+Enter 换行）")
        });
        let model_search = cx.new(|cx| TextInput::new(window, cx).placeholder("搜索模型…"));
        // 输入框只订阅「内容变了」：发送按钮的可用态要跟着重算（Enter 由 `PromptInput` 接）。
        let sub = cx.subscribe_in(
            &input,
            window,
            |_this, _state, event: &TextEvent, _window, cx| {
                if matches!(event, TextEvent::Changed) {
                    cx.notify();
                }
            },
        );
        // 搜索框只用来过滤候选：内容一变就重渲染，过滤在弹层构建时算。
        let search_sub = cx.subscribe_in(
            &model_search,
            window,
            |_this, _state, event: &TextEvent, _window, cx| {
                if matches!(event, TextEvent::Changed) {
                    cx.notify();
                }
            },
        );
        // 焦点直接给输入框：打开窗口就能打字
        let focus = input.read(cx).focus_handle(cx);
        window.focus(&focus, cx);

        Self {
            ui: ChatUi::new(),
            input,
            entries: Rc::new(Vec::new()),
            cmd_tx,
            cancel_slot,
            status,
            candidates,
            model_search,
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
        self.snapshot();
        cx.notify();
    }

    /// 生成中取消本轮（与 TUI 的 Ctrl-C 语义一致：丢弃未提交输出）。
    pub fn cancel(&mut self, cx: &mut Context<Self>) {
        if let Some(token) = self.cancel_slot.lock().expect("cancel slot").as_ref() {
            token.cancel();
        }
        cx.notify();
    }

    fn submit(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.ui.generating() {
            return;
        }
        let text = self.input.read(cx).text().trim().to_string();
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
        self.input.update(cx, |input, cx| input.set_text("", cx));
        self.snapshot();
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

    /// 把状态机的条目拷成渲染闭包能拿的 `Rc`（只在状态变化后做一次）。
    fn snapshot(&mut self) {
        self.entries = Rc::new(self.ui.entries().to_vec());
    }
}

// —— 渲染 ——

impl Render for ChatView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors.clone();
        let ty = Type::of(cx);
        div()
            .flex()
            .flex_row()
            .size_full()
            .bg(colors.bg)
            .text_color(colors.fg)
            .text_size(ty.sm)
            .child(self.sidebar(cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w(px(0.))
                    .h_full()
                    .child(self.main_area(cx)),
            )
    }
}

impl ChatView {
    /// 会话栏：`Sidebar`（自带宽度与右边框）+ 按天分组的中文标题 + `ConversationItem`。
    fn sidebar(&self, cx: &mut Context<Self>) -> AnyElement {
        let colors = cx.theme().colors.clone();
        let ty = Type::of(cx);
        let view = cx.entity().downgrade();
        let current = self.session_id.clone();
        let now = now_ms();

        let mut list = div()
            .id("session-list")
            .flex()
            .flex_col()
            .gap_0p5()
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll();

        if self.sessions.is_empty() {
            list = list.child(
                div()
                    .px_2()
                    .py_2()
                    .text_size(ty.xs)
                    .text_color(colors.fg_muted)
                    .child("还没有会话。发一条消息即新建。"),
            );
        }
        let mut bucket = "";
        for meta in &self.sessions {
            let day = day_bucket(meta.updated_at, now);
            if day != bucket {
                list = list.child(heading(day, &colors, ty));
                bucket = day;
            }
            let at =
                Timestamp::from_millisecond(meta.updated_at).unwrap_or_else(|_| Timestamp::now());
            let conversation = Conversation {
                key: meta.id.clone().into(),
                title: meta.title.clone().into(),
                at,
                pinned: false,
            };
            let (id, title) = (meta.id.clone(), meta.title.clone());
            let view = view.clone();
            list = list.child(
                ConversationItem::new(
                    ElementId::Name(format!("session-{}", meta.id).into()),
                    conversation,
                )
                .active(current.as_deref() == Some(meta.id.as_str()))
                .on_select(move |_, _, cx| {
                    let Some(view) = view.upgrade() else {
                        return;
                    };
                    view.update(cx, |view, cx| {
                        view.pick_session(id.clone(), title.clone(), cx);
                    });
                }),
            );
        }

        let new_session_view = view.clone();

        Sidebar::new("chat-sidebar", false)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_2()
                    .h_full()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .px_1()
                            .child(
                                div()
                                    .text_size(ty.xs)
                                    .text_color(colors.fg_muted)
                                    .child("会话"),
                            )
                            .child(
                                div()
                                    .text_size(ty.xs)
                                    .text_color(colors.fg_subtle)
                                    .child(self.sessions.len().to_string()),
                            )
                            .child(div().flex_1())
                            .child(
                                IconButton::new("new-session", IconName::Plus)
                                    .variant(ButtonVariant::Ghost)
                                    .size(ControlSize::Sm)
                                    .tooltip("新建会话")
                                    .on_click(move |_, _, cx| {
                                        let Some(view) = new_session_view.upgrade() else {
                                            return;
                                        };
                                        view.update(cx, |view, cx| view.new_session(cx));
                                    }),
                            ),
                    )
                    .child(list),
            )
            .into_any_element()
    }

    /// 右栏：阅读列宽的消息区 + 底部输入区（`ChatContainer` 管这条竖排）。
    fn main_area(&self, cx: &mut Context<Self>) -> AnyElement {
        ChatContainer::new(self.messages(cx))
            .composer(self.composer(cx))
            .into_any_element()
    }

    fn messages(&self, cx: &mut Context<Self>) -> AnyElement {
        let colors = cx.theme().colors.clone();
        let ty = Type::of(cx);
        let entries = self.entries.clone();
        if entries.is_empty() {
            // 空态说清「做什么」，不只说「空着」。
            return div()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_2()
                .child(
                    Icon::new(IconName::Sparkles)
                        .size(IconSize::Lg)
                        .color(colors.accent),
                )
                .child(
                    div()
                        .text_size(ty.lg)
                        .text_color(colors.fg)
                        .child("问我点什么吧"),
                )
                .child(
                    div()
                        .text_size(ty.sm)
                        .text_color(colors.fg_muted)
                        .child("Enter 发送，Shift+Enter 换行"),
                )
                .into_any_element();
        }

        let generating = self.ui.generating();
        let last = entries.len() - 1;
        let list = MessageList::new("chat-messages", entries.len(), move |ix, _window, cx| {
            let colors = cx.theme().colors.clone();
            let ty = Type::of(cx);
            let live = generating && ix == last;
            div()
                .px_6()
                .py_2()
                .child(entry_row(entries[ix].clone(), ix, live, &colors, ty))
                .into_any_element()
        });
        // 转录按阅读列宽居中；列表自己虚拟化并对齐到底部（`G-3`）。
        div()
            .size_full()
            .flex()
            .justify_center()
            .child(div().w_full().max_w(px(820.)).h_full().child(list))
            .into_any_element()
    }

    /// 输入区：`PromptInput` 自带 Enter 发送 / busy→stop / Composer 边框；
    /// 工具行放模型胶囊、provider 与 token 用量。
    fn composer(&self, cx: &mut Context<Self>) -> AnyElement {
        let colors = cx.theme().colors.clone();
        let ty = Type::of(cx);
        let view = cx.entity().downgrade();
        let generating = self.ui.generating();

        let send = {
            let view = view.clone();
            move |window: &mut Window, cx: &mut App| {
                let Some(view) = view.upgrade() else {
                    return;
                };
                view.update(cx, |view, cx| view.submit(window, cx));
            }
        };
        let mut prompt = PromptInput::new("chat-prompt", &self.input, send);
        if generating {
            let stop = {
                let view = view.clone();
                move |_window: &mut Window, cx: &mut App| {
                    let Some(view) = view.upgrade() else {
                        return;
                    };
                    view.update(cx, |view, cx| view.cancel(cx));
                }
            };
            prompt = prompt.busy(stop);
        }
        if let Some(notice) = self.ui.notice() {
            prompt = prompt.above(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_1()
                    .text_size(ty.xs)
                    .text_color(colors.danger)
                    .child(
                        Icon::new(IconName::TriangleAlert)
                            .size(IconSize::Xs)
                            .color(colors.danger),
                    )
                    .child(notice.to_string()),
            );
        }
        prompt = prompt.tool(self.model_picker(cx));
        if let Some(title) = self.session_title.clone() {
            prompt = prompt.tool(
                div()
                    .min_w(px(0.))
                    .max_w(px(200.))
                    .truncate()
                    .text_size(ty.xs)
                    .text_color(colors.fg_muted)
                    .child(title),
            );
        }
        prompt
            .tool(
                div()
                    .text_size(ty.xs)
                    .text_color(colors.fg_muted)
                    .child(self.status.provider.clone()),
            )
            .tool(
                div()
                    .text_size(ty.xs)
                    .text_color(colors.fg_subtle)
                    .child(format!("{} tok", self.ui.total_usage().total())),
            )
            .into_any_element()
    }

    /// 模型选择器：胶囊 + `Popover`（搜索 / 按供应商分组 / 打勾 / 管理模型）。
    ///
    /// 语义与 TUI 一致：点一项发 `Cmd::SelectModel { provider, model }` —— **一起切**
    /// （密钥按 provider 存，换 provider 必须连带重建实例，见 `channel.rs`）。
    /// 界面**不做乐观更新**：状态栏等 worker 回 `UiEvent::ModelChanged` 才变。
    fn model_picker(&self, cx: &mut Context<Self>) -> AnyElement {
        let view = cx.entity().downgrade();
        let candidates = self.candidates.clone();
        let search = self.model_search.clone();
        let manage = self.on_manage_models.clone();
        let label = self.status.model.clone();
        let current = (self.status.provider.clone(), self.status.model.clone());

        Popover::with_opener(
            "model-picker",
            move |toggle| {
                Button::new("model-capsule", label)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .icon(IconName::Bot)
                    .on_click(move |_, window, cx| toggle(window, cx))
            },
            move |close, _window, cx| {
                let colors = cx.theme().colors.clone();
                let ty = Type::of(cx);
                // 查询串在这一刻读：弹层每次重渲染都会重新构建内容。
                let typed = search.read(cx).text().trim().to_string();
                let query = typed.to_lowercase();
                let mut groups: Vec<(String, Vec<ModelChoice>)> = Vec::new();
                for choice in &candidates {
                    let hit = query.is_empty()
                        || choice.model.to_lowercase().contains(&query)
                        || choice.provider.to_lowercase().contains(&query);
                    if !hit {
                        continue;
                    }
                    match groups.last_mut() {
                        // 去重：同一 (provider, model) 只留一条。
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

                let mut list = div()
                    .id("model-list")
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .max_h(px(320.))
                    .overflow_y_scroll();

                let mut row = 0_usize;
                for (provider, rows) in &groups {
                    list = list.child(
                        div()
                            .px_2()
                            .pt_2()
                            .pb_1()
                            .text_size(ty.xs)
                            .text_color(colors.fg_subtle)
                            .child(short_provider(provider).to_string()),
                    );
                    for choice in rows {
                        let is_current = choice.provider == current.0 && choice.model == current.1;
                        let label = choice.model.clone();
                        let choice = choice.clone();
                        let view = view.clone();
                        let close = close.clone();
                        list = list.child(
                            div()
                                .id(("model-row", row))
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap_2()
                                .px_2()
                                .py_1p5()
                                .rounded_md()
                                .cursor_pointer()
                                .hover(|style| style.bg(colors.hover))
                                .on_click(move |_, window, cx| {
                                    close(window, cx);
                                    let Some(view) = view.upgrade() else {
                                        return;
                                    };
                                    view.update(cx, |view, cx| view.pick_model(choice.clone(), cx));
                                })
                                .child(div().w(px(16.)).children(is_current.then(|| {
                                    Icon::new(IconName::Check)
                                        .size(IconSize::Xs)
                                        .color(colors.accent)
                                })))
                                .child(
                                    div()
                                        .min_w(px(0.))
                                        .truncate()
                                        .text_size(ty.sm)
                                        .child(label.clone()),
                                ),
                        );
                        row += 1;
                    }
                }

                if groups.is_empty() {
                    if typed.is_empty() {
                        list = list.child(
                            div()
                                .px_2()
                                .py_2()
                                .text_size(ty.xs)
                                .text_color(colors.fg_muted)
                                .child("没有候选模型（宿主清单为空）"),
                        );
                    } else {
                        // 搜不到 → 直接把这串字当模型名（自定义端点常见：服务端有不认识的模型）
                        let view = view.clone();
                        let close = close.clone();
                        let name = typed.clone();
                        list = list.child(
                            div()
                                .id("model-use-typed")
                                .px_2()
                                .py_1p5()
                                .rounded_md()
                                .cursor_pointer()
                                .hover(|style| style.bg(colors.hover))
                                .on_click(move |_, window, cx| {
                                    close(window, cx);
                                    let Some(view) = view.upgrade() else {
                                        return;
                                    };
                                    view.update(cx, |view, cx| {
                                        view.use_model_name(name.clone(), cx)
                                    });
                                })
                                .text_size(ty.sm)
                                .text_color(colors.accent)
                                .child(format!("使用「{typed}」")),
                        );
                    }
                }

                // 底部：管理模型（切片 B 接「开关 + 落盘」）。
                let view = view.clone();
                let close = close.clone();
                let manage = manage.clone();
                list = list.child(
                    div().mt_1().child(
                        div()
                            .id("model-manage")
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .px_2()
                            .py_1p5()
                            .rounded_md()
                            .cursor_pointer()
                            .hover(|style| style.bg(colors.hover))
                            .text_size(ty.sm)
                            .on_click(move |_, window, cx| {
                                close(window, cx);
                                let Some(view) = view.upgrade() else {
                                    return;
                                };
                                view.update(cx, |_, cx| cx.notify());
                                manage(cx);
                            })
                            .child(
                                Icon::new(IconName::Settings)
                                    .size(IconSize::Sm)
                                    .color(colors.fg_muted),
                            )
                            .child("管理模型"),
                    ),
                );

                div()
                    .w(px(320.))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(SearchInput::new("model-search", &search))
                    .child(list)
            },
        )
        .into_any_element()
    }
}

/// 供应商短名（分组标题上省地方）：`openai-compatible` → `openai`。
fn short_provider(provider: &str) -> &str {
    provider.strip_suffix("-compatible").unwrap_or(provider)
}

/// 一天的分组标题（Ely 的 `ConversationList` 标题写死英文，所以会话栏自己分组）。
fn heading(day: &str, colors: &Palette, ty: Type) -> AnyElement {
    div()
        .px_2()
        .pt_2()
        .pb_1()
        .text_size(ty.xs)
        .text_color(colors.fg_subtle)
        .child(day.to_string())
        .into_any_element()
}

/// 会话按「最近一次更新」归到哪一档。
fn day_bucket(updated_at: i64, now: i64) -> &'static str {
    let days = (now - updated_at).max(0) / 86_400_000;
    match days {
        0 => "今天",
        1 => "昨天",
        2..=7 => "最近 7 天",
        _ => "更早",
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 一条转录条目 → 一个块。层级即信息：
/// 用户 = 右对齐气泡；助理 = 头像 + Markdown；思考 = 小字弱色；
/// 工具 = 图标 + 状态签；备注 = 居中小字。
fn entry_row(entry: Entry, ix: usize, live: bool, colors: &Palette, ty: Type) -> AnyElement {
    match entry {
        Entry::User(text) => MessageBubble::new(Role::User)
            .child(text)
            .into_any_element(),
        // 助理正文走 Markdown（标题 / 列表 / 代码块都能成形）；流式时 `live` 逐字揭示。
        Entry::Assistant(text) => MessageBubble::new(Role::Assistant)
            .child(StreamingMarkdown::new(("message", ix), text, live))
            .into_any_element(),
        // 思考过程：小字、弱色，一个「思考」文字签（不用 glyph 图标）。
        Entry::Thinking(text) => div()
            .flex()
            .flex_row()
            .items_start()
            .gap_2()
            .child(
                div()
                    .flex_none()
                    .text_size(ty.xs)
                    .text_color(colors.accent)
                    .mt_0p5()
                    .child("思考"),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .text_size(ty.xs)
                    .text_color(colors.fg_muted)
                    .child(text),
            )
            .into_any_element(),
        // 工具调用：终端图标 + 名称 + 状态签。
        Entry::Tool { name, status } => {
            let (label, color) = match status {
                Some(ToolStatus::Running) => ("运行中", colors.warning),
                Some(ToolStatus::Ok) => ("完成", colors.success),
                Some(ToolStatus::Failed) => ("失败", colors.danger),
                None => ("", colors.fg_subtle),
            };
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    Icon::new(IconName::Terminal)
                        .size(IconSize::Sm)
                        .color(colors.accent),
                )
                .child(
                    div()
                        .flex_none()
                        .text_size(ty.sm)
                        .text_color(colors.fg)
                        .child(name),
                )
                .when(!label.is_empty(), |d| {
                    d.child(
                        div()
                            .flex_none()
                            .text_size(ty.xs)
                            .text_color(color)
                            .child(label),
                    )
                })
                .into_any_element()
        }
        // 工具结果：缩进一档的次要文字。
        Entry::ToolResult(text) => div()
            .flex()
            .flex_row()
            .items_start()
            .gap_2()
            .child(div().flex_none().text_color(colors.fg_subtle).child("↳"))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .text_size(ty.sm)
                    .text_color(colors.fg_muted)
                    .child(text),
            )
            .into_any_element(),
        // 备注：最弱一档，居中（`Role::System` 的样式）。
        Entry::Note(text) => MessageBubble::new(Role::System)
            .child(text)
            .into_any_element(),
    }
}
