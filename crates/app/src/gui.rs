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

use std::rc::Rc;
use std::sync::Arc;

use agent_chat::{ChatView, Cmd, ModelChoice, StatusInfo, UiEvent};
use agent_settings::SettingsService;
use agent_settings::ui::{ApplySettingsCallback, RefreshModelsCallback, SettingsView};
use agent_ui_kit::ely;
use agent_ui_kit::ely::forms::Choice;
use agent_ui_kit::ely::navigation::Tabs;
use agent_ui_kit::ely::primitives::{FocusScope, Icon, IconName};
use agent_ui_kit::ely::theme::{ActiveTheme, IconSize, TextSize};
use agent_ui_kit::kit::assets::Assets as KitAssets;
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

    // 生效组合（provider + model）与「重建 provider 的上下文」——界面里切模型时 worker 用它。
    // 初始 provider 在宿主先构造好：未知供应商能在开窗口之前就退出（`R1`）。
    let selection = worker::Selection::new(chat_args.provider.clone(), chat_args.model.clone());
    let factory = worker::ProviderFactory::new(&chat_args, setup::demo_tools());

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
        selection,
        built,
        factory,
        repo,
        cancel_slot.clone(),
        cmd_rx,
        ui_tx,
    ));

    let status = StatusInfo {
        provider: args.provider.clone(),
        model: args.model.clone(),
    };
    // 选择器的候选清单：装配层给（`features/*` 不认识供应商名单）
    //
    // 两份用途不同的清单：
    // - `all_choices` 给设置页的「管理模型」——**全量**，否则关掉的模型没法再打开；
    // - `visible` 给对话页的选择器 —— 已按偏好剔除，且当前生效的那对永远保留。
    let all_choices = setup::model_choices(&chat_args);
    let prefs = setup::load_preferences(chat_args.config.as_deref());
    let visible =
        setup::visible_choices(&all_choices, &prefs, &chat_args.provider, &chat_args.model);
    // 设置页的供应商行（展示名与约定环境变量名都来自 `kernel/providers` 的目录）
    let provider_rows = setup::provider_rows(&chat_args);

    // 设置面板的数据入口。定位配置目录失败不该挡启动（`R1`：配置问题不挡「跑起来」），
    // 只是设置页不可用——所以这里降级为一句可读的提示。
    let (settings_service, settings_error) = match SettingsService::discover() {
        Ok(service) => (Some(Arc::new(service)), None),
        Err(error) => (
            None,
            Some(format!("定位配置目录失败，设置页不可用：{error}")),
        ),
    };

    // `.with_assets` 不是可选项：GPUI 默认的 asset source 是 `impl AssetSource for ()`
    // （空实现），不带它**所有 SVG 图标都画不出来**（勾、放大镜、下拉箭头全空白）。
    //
    // 两份资产要**叠成一份**：Ely 的字体与图标在前，Kit 的图标在后。`ely::init` 会校验
    // 资产源能加载 Ely 的每一个文件，漏了它直接报错（这正是 `Before` 的用途）。
    application()
        .with_assets(ely::Assets::before(KitAssets))
        .run(move |cx| {
        // 组件库 + 设计令牌主题（必须在开窗之前）
        agent_ui_kit::init(cx);
        open_window(window_options(), cx, |window, cx| {
            // 顺序有讲究：设置页先建（它要**全量**清单），外壳次之，对话页最后 ——
            // 对话页的「管理模型」回调要回指外壳，而外壳得先存在。
            let settings = settings_service.as_ref().map(|service| {
                cx.new(|cx| {
                    SettingsView::new(
                        window,
                        cx,
                        service.clone(),
                        provider_rows.clone(),
                        all_choices.clone(),
                        setup::catalog_rows(),
                        // 真正的回调要回指对话页，而对话页还没建出来 —— 先给空回调，随后注入。
                        Rc::new(|_: &mut App| {}),
                    )
                })
            });
            let settings_handle = settings.clone();

            // 「刷新模型」按钮（`C2d`）：设置页只发起，宿主把它转成 `Cmd::RefreshModels` 发回 worker。
            // 跨功能的线只由宿主接（`D6`）；结果由下面的事件桥回灌（`set_models` / `set_status`）。
            let on_refresh_models: RefreshModelsCallback = {
                let cmd_tx = cmd_tx.clone();
                Rc::new(move |provider: &str, _cx: &mut App| {
                    // GPUI 主线程上不能 await，用 try_send；队列满（罕见）时静默丢弃。
                    let _ = cmd_tx.try_send(Cmd::RefreshModels {
                        provider: provider.to_string(),
                    });
                })
            };
            if let Some(settings) = settings.as_ref() {
                settings.update(cx, |view, _cx| view.set_on_refresh_models(on_refresh_models));
            }

            // 「保存后生效」（B 方案）：设置页保存成功后把 `Cmd::ApplySettings` 发回 worker，
            // 用**当前组合**重建 provider（重读磁盘的密钥 / 端点 / 代理 / 超时）。
            // 生成中发去也只是排队（worker 轮内不处理命令），本轮结束自动生效。
            // 结果由下方事件桥回灌设置页（`SettingsApplied` / `SettingsApplyFailed`）。
            let on_apply_settings: ApplySettingsCallback = {
                let cmd_tx = cmd_tx.clone();
                Rc::new(move |_cx: &mut App| {
                    let _ = cmd_tx.try_send(Cmd::ApplySettings);
                })
            };
            if let Some(settings) = settings.as_ref() {
                settings.update(cx, |view, _cx| view.set_on_apply_settings(on_apply_settings));
            }

            let shell = cx.new(|cx| Shell {
                chat: None,
                settings,
                settings_error: settings_error.clone(),
                tab: Tab::Chat,
                open_models_page: false,
                focus: cx.focus_handle(),
            });

            // 对话页那个胶囊底部的「管理模型」→ 切到设置页并落在模型分区。
            //
            // 跨功能的这条线**只能由宿主接**（`D6`：各功能不知道对方存在）。
            // 这里只置旗标、不直接调设置页：设输入框的值要 `&mut Window`，回调里拿不到 ——
            // 真正的切换发生在 `Shell::render`（那里有 window）。
            let on_manage_models: Rc<dyn Fn(&mut App)> = {
                let shell = shell.downgrade();
                Rc::new(move |cx: &mut App| {
                    if let Some(shell) = shell.upgrade() {
                        shell.update(cx, |shell, cx| {
                            shell.tab = Tab::Settings;
                            shell.open_models_page = true;
                            cx.notify();
                        });
                    }
                })
            };

            let chat = cx.new(|cx| {
                ChatView::new(
                    window,
                    cx,
                    status.clone(),
                    visible
                        .iter()
                        .map(|(provider, model)| ModelChoice {
                            provider: provider.clone(),
                            model: model.clone(),
                        })
                        .collect(),
                    cmd_tx.clone(),
                    cancel_slot.clone(),
                    on_manage_models,
                )
            });
            shell.update(cx, |shell, _cx| shell.chat = Some(chat.clone()));

            // 「连接提供商」成功后：重算可见候选并推给对话页（**无需重启**，切片 C1c A 方案）；
            // 再补一发 `Cmd::ApplySettings`（B 方案）——重连的若是**当前生效**的条目
            // （改密钥 / 端点），worker 重建后立刻用上新值；连的是新条目则重建无感。
            // 与 `on_manage_models` 同一条规矩：跨功能的线只由宿主接（`D6`）。
            if let Some(settings) = settings_handle.as_ref() {
                let shell = shell.downgrade();
                let args = chat_args.clone();
                let cmd_tx = cmd_tx.clone();
                // 注意：这里是**同步**回调（`set_on_connected` 的契约是 `Fn(&mut App)`），
                // 会就地读配置 + 偏好。可接受：连接是低频的用户动作、读的是两个小文件；
                // 高频路径（刷新模型）已放到后台执行器，见下方事件泵。
                let on_connected: Rc<dyn Fn(&mut App)> = Rc::new(move |cx: &mut App| {
                    let Some(shell) = shell.upgrade() else {
                        return;
                    };
                    shell.update(cx, |shell, cx| {
                        let Some(chat) = shell.chat.clone() else {
                            return;
                        };
                        let all = setup::model_choices(&args);
                        let prefs = setup::load_preferences(args.config.as_deref());
                        let visible =
                            setup::visible_choices(&all, &prefs, &args.provider, &args.model);
                        chat.update(cx, |view, cx| {
                            view.set_candidates(
                                visible
                                    .into_iter()
                                    .map(|(provider, model)| ModelChoice { provider, model })
                                    .collect(),
                                cx,
                            );
                        });
                    });
                    // 生效命令（B 方案）：worker 轮内不处理命令，生成中发去也只是排队。
                    let _ = cmd_tx.try_send(Cmd::ApplySettings);
                });
                settings.update(cx, |view, _cx| view.set_on_connected(on_connected));
            }

            // 桥：worker 的事件流 → 视图。`rx.recv().await` 在 GPUI 的执行器里轮询
            // （`X2` §8 已验证过这个形态）。
            //
            // 除对话页外，联网刷新（`C2d`）与「保存生效」（B 方案）的结果都要回灌**设置页**：
            // 按钮与保存动作在那边，成功 / 失败得就地看到；成功还要重算候选 ——
            // 这几条跨功能的线只由宿主接（`D6`）。
            let settings_for_events = settings_handle.clone();
            let args_for_events = chat_args.clone();
            chat.update(cx, |_view, cx| {
                cx.spawn(async move |this, cx| {
                    while let Some(event) = ui_rx.recv().await {
                        match &event {
                            UiEvent::ModelsRefreshed { provider, models } => {
                                let (provider, count) = (provider.clone(), models.len());
                                // 重算候选：worker 已把拉到的 id 写回配置，这里读得到。
                                // 文件读放**后台执行器**（硬约束 5：UI 线程禁阻塞）。
                                let args_bg = args_for_events.clone();
                                let (all, visible) = cx
                                    .background_executor()
                                    .spawn(async move { recompute_choices(&args_bg) })
                                    .await;
                                let all_for_settings = all;
                                if let Some(settings) = settings_for_events.as_ref() {
                                    settings.update(cx, |view, cx| {
                                        view.set_models(all_for_settings);
                                        view.set_status(
                                            format!(
                                                "已刷新 {provider}：拉到 {count} 个模型（新模型默认显示）"
                                            ),
                                            false,
                                        );
                                        cx.notify();
                                    });
                                }
                                let choices = visible
                                    .into_iter()
                                    .map(|(provider, model)| ModelChoice { provider, model })
                                    .collect();
                                if this
                                    .update(cx, |view, cx| view.set_candidates(choices, cx))
                                    .is_err()
                                {
                                    break; // 视图已销毁
                                }
                            }
                            UiEvent::ModelsRefreshFailed { provider, message } => {
                                let (provider, message) = (provider.clone(), message.clone());
                                if let Some(settings) = settings_for_events.as_ref() {
                                    settings.update(cx, |view, cx| {
                                        view.set_status(
                                            format!("刷新 {provider} 失败：{message}"),
                                            true,
                                        );
                                        cx.notify();
                                    });
                                }
                            }
                            UiEvent::SettingsApplied { provider, model } => {
                                // 配置可能变了（默认模型、条目端点…）：同 ModelsRefreshed
                                // 重算候选；设置页状态条就地确认（B 方案）。
                                let (provider, model) = (provider.clone(), model.clone());
                                let args_bg = args_for_events.clone();
                                let (all, visible) = cx
                                    .background_executor()
                                    .spawn(async move { recompute_choices(&args_bg) })
                                    .await;
                                let all_for_settings = all;
                                if let Some(settings) = settings_for_events.as_ref() {
                                    settings.update(cx, |view, cx| {
                                        view.set_models(all_for_settings);
                                        view.set_status(
                                            format!("已生效：{provider} / {model}"),
                                            false,
                                        );
                                        cx.notify();
                                    });
                                }
                                let choices = visible
                                    .into_iter()
                                    .map(|(provider, model)| ModelChoice { provider, model })
                                    .collect();
                                if this
                                    .update(cx, |view, cx| view.set_candidates(choices, cx))
                                    .is_err()
                                {
                                    break; // 视图已销毁
                                }
                            }
                            UiEvent::SettingsApplyFailed { message } => {
                                // worker 的 message 已自带「旧连接保留」，这里原样转达。
                                let message = message.clone();
                                if let Some(settings) = settings_for_events.as_ref() {
                                    settings.update(cx, |view, cx| {
                                        view.set_status(message, true);
                                        cx.notify();
                                    });
                                }
                            }
                            _ => {}
                        }
                        if this.update(cx, |view, cx| view.apply(event, cx)).is_err() {
                            break; // 视图已销毁
                        }
                    }
                })
                .detach();
            });

            shell
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

/// 重算候选清单（读配置 + 读偏好，两次文件读）：只该跑在**后台执行器**
/// （硬约束 5：UI 线程禁阻塞）。`ModelsRefreshed`（C2d）与 `SettingsApplied`（B 方案）
/// 两条回灌路径共用。
///
/// 返回 `(全量, 可见)`：全量给设置页的「管理模型」（关掉的还要能再打开），
/// 可见给对话页的选择器。
fn recompute_choices(args: &ChatArgs) -> (Vec<(String, String)>, Vec<(String, String)>) {
    let all = setup::model_choices(args);
    let prefs = setup::load_preferences(args.config.as_deref());
    let visible = setup::visible_choices(&all, &prefs, &args.provider, &args.model);
    (all, visible)
}

// —— 外壳：标签页（对话 / 设置） ——
//
// 放在 `app` 是因为它**跨功能**组装两个 `features/*` 的视图（`D6`：组装归宿主）。
// 各功能自己不知道对方存在。
//
// 渲染全部走 Ely（`D4` 的组件层）：顶栏是 `navigation::Tabs`（自带滑动指示条、
// 方向键切页、a11y 角色），整棵视图包在 `primitives::FocusScope` 的根作用域里 ——
// 那是 Ely 接管 Tab / Shift-Tab 的前提（`FocusScope` 的 key context 才绑了这两个键）。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Chat,
    Settings,
}

impl Tab {
    /// `Tabs` 的取值（`Choice.value`）。
    fn value(self) -> &'static str {
        match self {
            Tab::Chat => "chat",
            Tab::Settings => "settings",
        }
    }
}

struct Shell {
    /// `None` 只在构造的**那一瞬间**成立：外壳要先于对话页建出来（对话页的回调要回指它），
    /// 建完对话页立刻填回来。渲染发生在构造之后，所以渲染期它一定在。
    chat: Option<Entity<ChatView>>,
    /// `None` = 定位配置目录失败（原因在 `settings_error`）。
    settings: Option<Entity<SettingsView>>,
    settings_error: Option<String>,
    tab: Tab,
    /// 对话页点了「管理模型」→ 下一帧切到设置页的模型分区（见 `on_manage_models`）。
    open_models_page: bool,
    /// 根焦点作用域：焦点丢失时它接住，并把 Tab / Shift-Tab 绑给整棵视图。
    focus: FocusHandle,
}

impl Render for Shell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 对话页刚点了「管理模型」：这里才切得到模型分区。
        if self.open_models_page {
            self.open_models_page = false;
            if let Some(settings) = &self.settings {
                settings.update(cx, |view, cx| view.show_models(cx));
            }
        }

        let colors = cx.theme().colors.clone();
        let body = match self.tab {
            // `children` 而不是 `child`：`Option<Entity<_>>` 实现了 `IntoIterator`，
            // `None` 时什么都不画（`Option` 本身没有 `IntoElement` 实现，`child` 收不了它）。
            Tab::Chat => div()
                .flex()
                .flex_1()
                .min_h(px(0.))
                .children(self.chat.clone()),
            Tab::Settings => match &self.settings {
                Some(view) => div().flex().flex_1().min_h(px(0.)).child(view.clone()),
                None => div().p_4().text_color(colors.fg_muted).child(
                    self.settings_error
                        .clone()
                        .unwrap_or_else(|| "设置页不可用".to_string()),
                ),
            },
        };

        FocusScope::new(&self.focus)
            .root()
            .flex()
            .flex_col()
            .size_full()
            .bg(colors.bg)
            .text_color(colors.fg)
            .text_size(cx.theme().text_size(TextSize::Sm))
            .child(self.header(cx))
            .child(body)
    }
}

impl Shell {
    fn select(&mut self, tab: Tab, cx: &mut Context<Self>) {
        self.tab = tab;
        cx.notify();
    }

    /// 切到设置页时重新读盘：用户可能刚在外部改过（也可能刚把它改坏）。
    fn select_settings(&mut self, cx: &mut Context<Self>) {
        self.tab = Tab::Settings;
        if let Some(view) = self.settings.clone() {
            view.update(cx, |view, cx| view.reload(cx));
        }
        cx.notify();
    }

    /// 顶栏：左侧是应用身份，右侧是 Ely 的标签条。
    fn header(&self, cx: &mut Context<Self>) -> AnyElement {
        let colors = cx.theme().colors.clone();
        let text = cx.theme().text_size(TextSize::Sm);
        let shell = cx.entity().downgrade();

        div()
            .flex()
            .flex_none()
            .flex_row()
            .items_center()
            .gap_6()
            .px_4()
            .h(px(44.))
            .bg(colors.bg)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(
                        Icon::new(IconName::Sparkles)
                            .size(IconSize::Sm)
                            .color(colors.accent),
                    )
                    .child(
                        div()
                            .text_size(text)
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.fg)
                            .child("CodingRocket"),
                    ),
            )
            .child(
                Tabs::new(
                    "shell-tabs",
                    [
                        Choice::new(Tab::Chat.value(), "对话").icon(IconName::MessageSquare),
                        Choice::new(Tab::Settings.value(), "设置").icon(IconName::Settings),
                    ],
                    self.tab.value(),
                )
                .on_change(move |value, _window, cx| {
                    let Some(shell) = shell.upgrade() else {
                        return;
                    };
                    shell.update(cx, |shell, cx| match value.as_ref() {
                        "settings" => shell.select_settings(cx),
                        _ => shell.select(Tab::Chat, cx),
                    });
                }),
            )
            .into_any_element()
    }
}
