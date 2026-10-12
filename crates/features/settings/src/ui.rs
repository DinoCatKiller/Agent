//! GPUI 设置面板（`gui` feature）。
//!
//! 语义按 `A` 方案：**保存只写盘，下次启动生效**（`S1` 切片 5 拍板）。界面因此只做三件事：
//! 读出来显示、校验后写回、把「需要重启」讲清楚。
//!
//! 两条安全线（`R1` / 硬约束 5）：
//! - 已存的密钥**不回显**，界面只显示「来源」；输入框里是本次要写入的新值。
//! - 读不动配置文件时（`load` 报错）**禁止保存** —— 否则一按保存就把用户手写的内容覆盖掉。

use std::sync::Arc;

use agent_config::SecretString;
use agent_ui_kit::UiTheme;
use agent_ui_kit::component::button::Button;
use agent_ui_kit::component::input::{Input, InputState};
use agent_ui_kit::kit::*;

use crate::service::{KeySource, SettingsService};
use crate::settings::Draft;

/// 面板状态。
pub struct SettingsView {
    service: Arc<SettingsService>,
    provider: Entity<InputState>,
    model: Entity<InputState>,
    base_url: Entity<InputState>,
    proxy: Entity<InputState>,
    timeout: Entity<InputState>,
    /// **每个供应商一行密钥**：密钥在钥匙串里本来就是按 provider 存的
    /// （账号 `<provider>:api_key`，`R1`），所以界面也按 provider 分行。
    keys: Vec<ProviderKeyRow>,
    /// 已知供应商（宿主给的）——「默认供应商」用 chip 选，避免手滑写出未知名字：
    /// 它现在**真的生效**（`R1` §1.1），写错会让 `gui` 直接起不来。
    providers: Vec<String>,
    /// `Some` = 配置文件读不动（坏 TOML 等）：此时**禁止保存**。
    load_error: Option<String>,
    status: Option<Status>,
    config_file: String,
}

/// 一个供应商的密钥行。
struct ProviderKeyRow {
    provider: String,
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
    /// `providers` 由宿主给（`features/*` 不认识供应商名单，`D6`）。
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        service: Arc<SettingsService>,
        providers: Vec<String>,
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
        for provider in providers {
            let source = service.key_source(&provider).unwrap_or(KeySource::None);
            let key_input = cx.new(|cx| {
                InputState::new(window, cx)
                    .masked(true)
                    .placeholder("粘贴新密钥（已存的不回显）")
            });
            keys.push(ProviderKeyRow {
                provider,
                input: key_input,
                source,
            });
        }

        Self {
            config_file: service.config_file().display().to_string(),
            service,
            provider: provider_input,
            model: model_input,
            base_url: base_url_input,
            proxy: proxy_input,
            timeout: timeout_input,
            keys,
            providers: provider_options,
            load_error,
            status: None,
        }
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
                (row.provider.clone(), row.input.clone())
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
                .key_source(&row.provider)
                .unwrap_or(KeySource::None);
        }
    }
}

// —— 渲染 ——

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
            let (provider, state, source) = {
                let row = &self.keys[index];
                (row.provider.clone(), row.input.clone(), row.source)
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
                            .child(div().text_color(dim).child(provider.clone()))
                            .child(div().text_color(dim).child(key_hint(source)))
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

        div()
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
            .into_any_element()
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
        for provider in self.providers.clone() {
            let selected = provider == current;
            let label = provider.clone();
            chips = chips.child(
                div()
                    .id(ElementId::Name(format!("provider-{provider}").into()))
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
                    .child(provider),
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

fn key_hint(source: KeySource) -> &'static str {
    match source {
        KeySource::Env => "密钥来源：环境变量（优先级更高，会盖过钥匙串）",
        KeySource::Keyring => "密钥来源：系统钥匙串（已配置，不回显）",
        KeySource::None => "密钥来源：未配置",
    }
}
