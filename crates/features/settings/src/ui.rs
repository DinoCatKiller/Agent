//! GPUI 设置面板（`gui` feature）。
//!
//! 语义按 `A` 方案：**保存只写盘，下次启动生效**（`S1` 切片 5 拍板）。界面因此只做三件事：
//! 读出来显示、校验后写回、把「需要重启」讲清楚。
//!
//! 两条安全线（`R1` / 硬约束 5）：
//! - 已存的密钥**不回显**，界面只显示「来源」；输入框里是本次要写入的新值。
//! - 读不动配置文件时（`load` 报错）**禁止保存** —— 否则一按保存就把用户手写的内容覆盖掉。

use std::sync::Arc;

use agent_config::{Preferences, SecretString};
use agent_ui_kit::UiTheme;
use agent_ui_kit::component::button::{Button, ButtonVariants as _};
use agent_ui_kit::component::input::{Input, InputEvent, InputState};
use agent_ui_kit::component::switch::Switch;
use agent_ui_kit::component::{Icon, IconName};
use agent_ui_kit::kit::*;

use crate::service::{KeySource, SettingsService};
use crate::settings::Draft;

/// 面板内的分区。
///
/// 默认「通用」；从对话页那个胶囊里的「管理模型」进来时直接落在「模型」
/// （宿主调 [`SettingsView::show_models`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    General,
    Models,
}

/// 面板状态。
pub struct SettingsView {
    service: Arc<SettingsService>,
    /// 当前分区。
    page: Page,
    provider: Entity<InputState>,
    model: Entity<InputState>,
    base_url: Entity<InputState>,
    proxy: Entity<InputState>,
    timeout: Entity<InputState>,
    /// **每个供应商一行密钥**：密钥在钥匙串里本来就是按 provider 存的
    /// （账号 `<provider>:api_key`，`R1`），所以界面也按 provider 分行。
    keys: Vec<ProviderKeyRow>,
    /// 可配密钥 / 可当默认供应商的条目（宿主给的）。
    ///
    /// 为什么用 chip 选「默认供应商」：它**真的生效**（`R1` §1.1），
    /// 手滑写出未知名字会让 `gui`（没有命令行参数的启动方式）直接起不来。
    providers: Vec<ProviderRow>,
    /// 全部候选模型（宿主编排给的）——「模型」页要看到**全量**才能把关掉的再打开。
    models: Vec<(String, String)>,
    /// 「模型」页的搜索框。
    model_search: Entity<InputState>,
    /// 偏好快照（哪些模型被关掉了）。开关是**受控**的：点一下 → 落盘 → 更新这里。
    hidden: Preferences,
    /// `Some` = 配置文件读不动（坏 TOML 等）：此时**禁止保存**。
    load_error: Option<String>,
    status: Option<Status>,
    config_file: String,
    preferences_file: String,
    /// 订阅必须持有，drop 即取消订阅。
    _search_sub: Subscription,
}

/// 设置页里的一个供应商条目 —— **宿主构造**（`features/*` 不认识供应商目录，`D6`）。
///
/// `app` 从 `agent_providers::catalog`（预设厂商）+ 配置文件里已连接的条目拼出来。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRow {
    /// 条目 id：钥匙串账号、`default_provider` 用它（`<id>:api_key`，`R1`）。
    pub id: String,
    /// 展示名（目录里的 `name`，自定义条目就是 id）。
    pub name: String,
    /// 约定环境变量名。`None` = 没有约定名（本地服务 / 自定义厂商）→ 只看钥匙串。
    pub env: Option<String>,
    /// 是否必须有密钥 —— 只用于提示文案：本地服务不该催用户去填。
    pub key_required: bool,
}

/// 一个供应商的密钥行。
struct ProviderKeyRow {
    row: ProviderRow,
    /// 本次要写入的新密钥（明文只活在输入框与那一次调用里；**已存值不回显**）。
    input: Entity<InputState>,
    /// 生效来源：环境变量 > 钥匙串 > 未配置（`R1`）。
    source: KeySource,
}

struct Status {
    message: String,
    error: bool,
}

impl SettingsView {
    /// `providers` / `models` 都由宿主给（`features/*` 不认识供应商名单，`D6`）。
    ///
    /// `models` 是**全量**候选（含被关掉的）：关掉的也得列出来，否则用户没法再打开。
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        service: Arc<SettingsService>,
        providers: Vec<ProviderRow>,
        models: Vec<(String, String)>,
    ) -> Self {
        let (draft, load_error) = match service.load() {
            Ok(settings) => (Draft::from_settings(&settings), None),
            Err(error) => (Draft::default(), Some(error.to_string())),
        };

        // 逐个显式建输入框，别用「捕获 `window` 的闭包」批量建 —— 那样会和下面循环里的闭包
        // 同时要求 `*window` 的唯一访问权（E0524）。
        let provider_input =
            cx.new(|cx| InputState::new(window, cx).default_value(draft.default_provider.clone()));
        let model_input =
            cx.new(|cx| InputState::new(window, cx).default_value(draft.default_model.clone()));
        let base_url_input =
            cx.new(|cx| InputState::new(window, cx).default_value(draft.base_url.clone()));
        let proxy_input =
            cx.new(|cx| InputState::new(window, cx).default_value(draft.proxy.clone()));
        let timeout_input = cx.new(|cx| {
            InputState::new(window, cx).default_value(draft.stream_idle_timeout_secs.clone())
        });

        let provider_options = providers.clone();
        let mut keys = Vec::new();
        for row in providers {
            let source = service
                .key_source(&row.id, row.env.as_deref())
                .unwrap_or(KeySource::None);
            let key_input = cx.new(|cx| {
                InputState::new(window, cx)
                    .masked(true)
                    .placeholder("粘贴新密钥（已存的不回显）")
            });
            keys.push(ProviderKeyRow {
                row,
                input: key_input,
                source,
            });
        }

        // 「模型」页的搜索框 + 其订阅（内容一变就重渲染 → 重新过滤）。
        let model_search = cx.new(|cx| InputState::new(window, cx).placeholder("搜索模型…"));
        let search_sub = cx.subscribe_in(
            &model_search,
            window,
            |_this, _state, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            },
        );

        let config_file = service.config_file().display().to_string();
        let preferences_file = service.preferences_file().display().to_string();
        // 偏好**降级不报错**（`agent_config::prefs` 的口径）：坏了就当没关过任何模型。
        let hidden = service.preferences();

        Self {
            service,
            page: Page::General,
            provider: provider_input,
            model: model_input,
            base_url: base_url_input,
            proxy: proxy_input,
            timeout: timeout_input,
            keys,
            providers: provider_options,
            models,
            model_search,
            hidden,
            load_error,
            status: None,
            config_file,
            preferences_file,
            _search_sub: search_sub,
        }
    }

    /// 切到「模型」页（宿主在对话页点「管理模型」时调）。
    pub fn show_models(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.page = Page::Models;
        self.status = None;
        // 重新读一次：别的地方（手改文件）动过偏好，这里要跟上。
        self.hidden = self.service.preferences();
        self.model_search
            .update(cx, |state, cx| state.set_value("", window, cx));
        cx.notify();
    }

    /// 切回「通用」页。
    fn show_general(&mut self, cx: &mut Context<Self>) {
        self.page = Page::General;
        cx.notify();
    }

    /// 开关一个模型是否出现在选择器里。**立即落盘**（偏好文件很小，且不存在覆盖用户内容的风险）。
    ///
    /// 界面**不做乐观更新**：落盘成功才改本地快照，失败就保持原样并报错 ——
    /// 否则"开关弹回去了但文件里已经写进去"这种状态最难查。
    fn toggle_model(
        &mut self,
        provider: String,
        model: String,
        visible: bool,
        cx: &mut Context<Self>,
    ) {
        match self.service.set_model_hidden(&provider, &model, !visible) {
            Ok(()) => {
                self.hidden = self.service.preferences();
                self.status = Some(Status {
                    message: if visible {
                        format!("已显示 {model}")
                    } else {
                        format!("已隐藏 {model}（对话页的模型菜单里不再出现）")
                    },
                    error: false,
                });
            }
            Err(error) => {
                self.status = Some(Status {
                    message: format!("写偏好失败：{error}"),
                    error: true,
                });
            }
        }
        cx.notify();
    }

    /// 用 chip 选默认供应商（写进输入框，保存时随其余配置一起落盘）。
    fn pick_provider(&mut self, provider: String, window: &mut Window, cx: &mut Context<Self>) {
        self.provider
            .update(cx, |state, cx| state.set_value(provider, window, cx));
        cx.notify();
    }

    /// 重新从磁盘读（切到本页、或用户手改过文件之后）。
    pub fn reload(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.status = None;
        let settings = match self.service.load() {
            Ok(settings) => {
                self.load_error = None;
                settings
            }
            Err(error) => {
                self.load_error = Some(error.to_string());
                return;
            }
        };
        let draft = Draft::from_settings(&settings);
        let pairs: [(Entity<InputState>, String); 5] = [
            (self.provider.clone(), draft.default_provider),
            (self.model.clone(), draft.default_model),
            (self.base_url.clone(), draft.base_url),
            (self.proxy.clone(), draft.proxy),
            (self.timeout.clone(), draft.stream_idle_timeout_secs),
        ];
        for (state, value) in pairs {
            state.update(cx, |state, cx| state.set_value(value, window, cx));
        }
        self.refresh_key_sources();
        cx.notify();
    }

    /// 保存：校验 → 写配置 → （填了新密钥才）写钥匙串。
    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(error) = &self.load_error {
            self.status = Some(Status {
                message: format!("配置文件读不动，先修好再保存：{error}"),
                error: true,
            });
            cx.notify();
            return;
        }

        let draft = Draft {
            default_provider: self.provider.read(cx).value().to_string(),
            default_model: self.model.read(cx).value().to_string(),
            base_url: self.base_url.read(cx).value().to_string(),
            proxy: self.proxy.read(cx).value().to_string(),
            stream_idle_timeout_secs: self.timeout.read(cx).value().to_string(),
        };

        let settings = match draft.to_settings() {
            Ok(settings) => settings,
            Err(errors) => {
                let list = errors
                    .iter()
                    .map(|error| error.to_string())
                    .collect::<Vec<_>>()
                    .join("；");
                self.status = Some(Status {
                    message: format!("有 {list}"),
                    error: true,
                });
                cx.notify();
                return;
            }
        };

        if let Err(error) = self.service.save(&settings) {
            self.status = Some(Status {
                message: format!("写配置失败：{error}"),
                error: true,
            });
            cx.notify();
            return;
        }

        // 逐供应商写密钥：**只写填了值的那几行**（空 = 不动它现在那把）
        let mut key_note = String::new();
        for index in 0..self.keys.len() {
            let (provider, state) = {
                let row = &self.keys[index];
                (row.row.id.clone(), row.input.clone())
            };
            let raw = state.read(cx).value().trim().to_string();
            if raw.is_empty() {
                continue;
            }
            match self.service.save_key(&provider, &SecretString::from(raw)) {
                Ok(()) => {
                    key_note = "，密钥已存入系统钥匙串".to_string();
                    state.update(cx, |state, cx| state.set_value("", window, cx));
                }
                Err(error) => {
                    self.status = Some(Status {
                        message: format!("保存 {provider} 的密钥失败：{error}"),
                        error: true,
                    });
                    cx.notify();
                    return;
                }
            }
        }

        self.refresh_key_sources();
        self.status = Some(Status {
            message: format!("已保存{key_note}（**重启后生效**）"),
            error: false,
        });
        cx.notify();
    }

    /// 清掉某个供应商在钥匙串里的密钥（环境变量那份不受影响，且优先级更高）。
    fn clear_key(&mut self, provider: String, cx: &mut Context<Self>) {
        match self.service.delete_key(&provider) {
            Ok(()) => {
                self.status = Some(Status {
                    message: format!("已清除 {provider} 的钥匙串密钥"),
                    error: false,
                });
                self.refresh_key_sources();
            }
            Err(error) => {
                self.status = Some(Status {
                    message: format!("清除 {provider} 失败：{error}"),
                    error: true,
                });
            }
        }
        cx.notify();
    }

    /// 刷新每行的密钥来源（保存 / 清除 / 重新载入后都要跟一下）。
    fn refresh_key_sources(&mut self) {
        for row in &mut self.keys {
            row.source = self
                .service
                .key_source(&row.row.id, row.row.env.as_deref())
                .unwrap_or(KeySource::None);
        }
    }
}

// —— 渲染 ——

impl SettingsView {
    /// 「模型」页：搜索 + 按供应商分组 + 每个模型一个开关（控制它是否出现在对话页的选择器里）。
    ///
    /// 关掉是**加法语义**：偏好文件里只记被隐藏的那些，新模型默认可见。
    fn models_page(&self, cx: &mut Context<Self>, fg: Hsla, dim: Hsla, err: Hsla) -> AnyElement {
        let query = self.model_search.read(cx).value().trim().to_lowercase();
        let mut groups: Vec<(String, Vec<String>)> = Vec::new();
        for (provider, model) in &self.models {
            let hit = query.is_empty()
                || model.to_lowercase().contains(&query)
                || provider.to_lowercase().contains(&query);
            if !hit {
                continue;
            }
            match groups.last_mut() {
                Some((it, rows)) if it == provider && !rows.iter().any(|row| row == model) => {
                    rows.push(model.clone());
                }
                Some(_) => {}
                None => groups.push((provider.clone(), vec![model.clone()])),
            }
        }

        let mut rows = div().flex().flex_col().gap_1();
        for (provider, models) in &groups {
            rows = rows.child(
                div()
                    .pt_2()
                    .text_xs()
                    .text_color(dim)
                    .child(provider.clone()),
            );
            for model in models {
                let visible = !self.hidden.is_hidden(provider, model);
                let (row_provider, row_model) = (provider.clone(), model.clone());
                rows = rows.child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_3()
                        .child(div().flex_1().truncate().child(model.clone()))
                        .child(
                            Switch::new(ElementId::Name(
                                format!("model-switch::{provider}::{model}").into(),
                            ))
                            .checked(visible)
                            .on_change(cx.listener(
                                move |this, wanted: &bool, _, cx| {
                                    this.toggle_model(
                                        row_provider.clone(),
                                        row_model.clone(),
                                        *wanted,
                                        cx,
                                    )
                                },
                            )),
                        ),
                );
            }
        }
        if groups.is_empty() {
            rows = rows.child(div().py_2().text_color(dim).child(if query.is_empty() {
                "没有候选模型（宿主清单为空）"
            } else {
                "没有匹配的模型"
            }));
        }

        let hidden_count: usize = self.hidden.hidden_models.values().map(Vec::len).sum();

        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .size_full()
            .max_w(px(560.))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::new("models-back")
                            .ghost()
                            .compact()
                            .icon(IconName::ChevronLeft)
                            .label("设置")
                            .on_click(cx.listener(|this, _, _, cx| this.show_general(cx))),
                    )
                    .child(div().text_color(fg).child("管理模型"))
                    .child(div().flex_1())
                    .child(
                        // 切片 C1 接：从预设供应商表里挑一个连上
                        Button::new("connect-provider")
                            .outline()
                            .icon(IconName::Plus)
                            .label("连接提供商")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.status = Some(Status {
                                    message: "连接提供商：下一步接入（切片 C1）".to_string(),
                                    error: false,
                                });
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .text_xs()
                    .text_color(dim)
                    .child("自定义模型选择器中显示的模型。")
                    .child(format!(
                        "偏好文件：{}（已隐藏 {hidden_count} 个）",
                        self.preferences_file
                    )),
            )
            .child(div().border_b_1().border_color(theme_border(cx)))
            .child(
                Input::new(&self.model_search)
                    .prefix(Icon::new(IconName::Search).text_color(dim))
                    .cleanable(true),
            )
            .child(rows)
            .children(self.status.as_ref().map(|status| {
                div()
                    .text_xs()
                    .text_color(if status.error { err } else { fg })
                    .child(status.message.clone())
            }))
            .into_any_element()
    }
}

/// 分隔线颜色（`div()` 自己取不到主题，单开一个口子给它）。
fn theme_border(cx: &App) -> Hsla {
    UiTheme::global(cx).colors.border
}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = UiTheme::global(cx);
        let (fg, dim, err, border, accent) = (
            theme.colors.foreground,
            theme.colors.muted_foreground,
            theme.colors.destructive,
            theme.colors.border,
            theme.colors.accent,
        );

        let mut form = div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .max_w(px(560.))
            .child(self.provider_field(cx, dim, accent, border))
            .child(field("默认模型", &self.model, dim))
            .child(field("Base URL（留空用内置默认）", &self.base_url, dim))
            .child(field("代理（http/https/socks5）", &self.proxy, dim))
            .child(field("流式空闲超时（秒）", &self.timeout, dim));

        // 密钥：**每个供应商一行**（钥匙串里本来就是按 provider 存的）。只显示来源，不回显已存值
        form = form.child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .text_xs()
                .text_color(dim)
                .child("API Key（每行单独保存；留空 = 不动它现在那把）"),
        );
        for index in 0..self.keys.len() {
            let (provider, name, env, required, state, source) = {
                let row = &self.keys[index];
                (
                    row.row.id.clone(),
                    row.row.name.clone(),
                    row.row.env.clone(),
                    row.row.key_required,
                    row.input.clone(),
                    row.source,
                )
            };
            form = form.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .text_xs()
                            // 展示名在前、id 在后：id 才是钥匙串账号与 `default_provider` 用的那个
                            .child(div().text_color(dim).child(name))
                            .child(div().text_color(dim).child(format!("({provider})")))
                            .child(div().text_color(dim).child(key_hint(
                                source,
                                env.as_deref(),
                                required,
                            )))
                            .child(
                                Button::new(ElementId::Name(
                                    format!("clear-key-{provider}").into(),
                                ))
                                .label("清除")
                                .on_click(cx.listener(
                                    move |this, _, _, cx| this.clear_key(provider.clone(), cx),
                                )),
                            ),
                    )
                    .child(Input::new(&state)),
            );
        }

        form = form.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    Button::new("save-settings")
                        .label("保存")
                        .on_click(cx.listener(|this, _, window, cx| this.save(window, cx))),
                )
                .child(
                    Button::new("reload-settings")
                        .label("重新载入")
                        .on_click(cx.listener(|this, _, window, cx| this.reload(window, cx))),
                )
                .child(
                    // 模型分区也放个入口：不然它只能在对话页的胶囊弹层里找到
                    Button::new("open-models")
                        .ghost()
                        .icon(IconName::Settings2)
                        .label("管理模型")
                        .on_click(cx.listener(|this, _, window, cx| this.show_models(window, cx))),
                ),
        );

        if let Some(status) = &self.status {
            form = form.child(
                div()
                    .text_xs()
                    .text_color(if status.error { err } else { fg })
                    .child(status.message.clone()),
            );
        }
        if let Some(error) = &self.load_error {
            form = form.child(
                div()
                    .text_xs()
                    .text_color(err)
                    .child(format!("配置文件读不动：{error}")),
            );
        }

        match self.page {
            Page::Models => self.models_page(cx, fg, dim, err),
            Page::General => div()
                .flex()
                .flex_col()
                .gap_3()
                .size_full()
                .p_4()
                .child(div().text_color(fg).child("设置"))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .text_xs()
                        .text_color(dim)
                        .child(format!("配置文件：{}", self.config_file))
                        .child("保存只写盘，重启后生效（`S1` 切片 5：B 方案的热切换还没做）"),
                )
                .child(div().border_b_1().border_color(border))
                .child(form)
                .into_any_element(),
        }
    }
}

impl SettingsView {
    /// 「默认供应商」：已知的用 chip 点选，也能手输（自定义适配器）。
    ///
    /// 为什么要 chip：`default_provider` 现在**真的生效**（`R1` §1.1），
    /// 手滑写成未知名字会让 `gui`（没有命令行参数的启动方式）直接起不来。
    fn provider_field(
        &self,
        cx: &mut Context<Self>,
        dim: Hsla,
        accent: Hsla,
        border: Hsla,
    ) -> AnyElement {
        let current = self.provider.read(cx).value().trim().to_string();
        let mut chips = div().flex().flex_row().flex_wrap().items_center().gap_1();
        for row in self.providers.clone() {
            let selected = row.id == current;
            let label = row.id.clone();
            let shown = row.name.clone();
            chips = chips.child(
                div()
                    .id(ElementId::Name(format!("provider-{}", row.id).into()))
                    .px_2()
                    .py_0p5()
                    .rounded_md()
                    .cursor_pointer()
                    .text_xs()
                    .border_1()
                    .border_color(if selected { accent } else { border })
                    .text_color(if selected { accent } else { dim })
                    .hover(|d| d.text_color(accent))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.pick_provider(label.clone(), window, cx)
                    }))
                    // chip 上写**展示名**，点下去写进输入框的是 id ——
                    // 用户不用记 `openai-compatible` 这种内部名字。
                    .child(shown),
            );
        }

        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_xs().text_color(dim).child("默认供应商"))
            .child(chips)
            .child(Input::new(&self.provider))
            .into_any_element()
    }
}

/// 一行「标签 + 输入框」。
fn field(label: &str, state: &Entity<InputState>, dim: Hsla) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_xs().text_color(dim).child(label.to_string()))
        .child(Input::new(state))
        .into_any_element()
}

/// 密钥来源提示。`env` / `required` 用来把"该不该填"讲清楚 ——
/// 本地服务不该催用户填密钥，有约定变量名的则应当告诉他变量叫什么。
fn key_hint(source: KeySource, env: Option<&str>, required: bool) -> String {
    match source {
        KeySource::Env => format!(
            "密钥来源：环境变量 {}（优先级更高，会盖过钥匙串）",
            env.unwrap_or("(未知)")
        ),
        KeySource::Keyring => "密钥来源：系统钥匙串（已配置，不回显）".to_string(),
        KeySource::None if !required => {
            format!(
                "密钥来源：未配置（{} 通常不需要密钥，留空即可）",
                "本地服务"
            )
        }
        KeySource::None => match env {
            Some(env) => format!("密钥来源：未配置（可设环境变量 {env}，或在这里粘贴）"),
            None => "密钥来源：未配置（没有约定环境变量名，只能存钥匙串）".to_string(),
        },
    }
}
