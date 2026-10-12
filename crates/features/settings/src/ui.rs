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
    /// 本次要写入钥匙串的新密钥（明文只活在输入框与这一次调用里；已存值不回显）。
    api_key: Entity<InputState>,
    key_source: KeySource,
    /// `Some` = 配置文件读不动（坏 TOML 等）：此时**禁止保存**。
    load_error: Option<String>,
    status: Option<Status>,
    config_file: String,
}

struct Status {
    message: String,
    error: bool,
}

impl SettingsView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>, service: Arc<SettingsService>) -> Self {
        let (draft, load_error, provider_id) = match service.load() {
            Ok(settings) => {
                let provider = settings.default_provider.clone();
                (Draft::from_settings(&settings), None, provider)
            }
            Err(error) => (Draft::default(), Some(error.to_string()), None),
        };
        let key_source = service
            .key_source(provider_id.as_deref().unwrap_or("openai-compatible"))
            .unwrap_or(KeySource::None);

        let mut input = |cx: &mut Context<Self>, value: &str| {
            cx.new(|cx| InputState::new(window, cx).default_value(value.to_string()))
        };

        Self {
            config_file: service.config_file().display().to_string(),
            service,
            provider: input(cx, &draft.default_provider),
            model: input(cx, &draft.default_model),
            base_url: input(cx, &draft.base_url),
            proxy: input(cx, &draft.proxy),
            timeout: input(cx, &draft.stream_idle_timeout_secs),
            api_key: cx.new(|cx| {
                InputState::new(window, cx)
                    .masked(true)
                    .placeholder("粘贴 API Key（已存的值不会回显）")
            }),
            key_source,
            load_error,
            status: None,
        }
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
        self.refresh_key_source(settings.default_provider.as_deref());
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
            api_key: self.api_key.read(cx).value().to_string(),
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

        // 空 = 本次不改密钥（已存的那个继续用）
        let mut key_note = String::new();
        if !draft.api_key.trim().is_empty() {
            let secret = SecretString::from(draft.api_key.clone());
            match self.service.save_key(
                settings
                    .default_provider
                    .as_deref()
                    .unwrap_or("openai-compatible"),
                &secret,
            ) {
                Ok(()) => {
                    key_note = "，密钥已存入系统钥匙串".to_string();
                    self.api_key
                        .update(cx, |state, cx| state.set_value("", window, cx));
                }
                Err(error) => {
                    self.status = Some(Status {
                        message: format!("保存密钥失败：{error}"),
                        error: true,
                    });
                    cx.notify();
                    return;
                }
            }
        }

        self.refresh_key_source(settings.default_provider.as_deref());
        self.status = Some(Status {
            message: format!("已保存{key_note}（**重启后生效**）"),
            error: false,
        });
        cx.notify();
    }

    /// 清掉钥匙串里的密钥（环境变量那份不受影响，且优先级更高）。
    fn clear_key(&mut self, cx: &mut Context<Self>) {
        let provider = self.provider.read(cx).value().trim().to_string();
        let provider = if provider.is_empty() {
            "openai-compatible".to_string()
        } else {
            provider
        };
        match self.service.delete_key(&provider) {
            Ok(()) => {
                self.status = Some(Status {
                    message: format!("已清除 {provider} 的钥匙串密钥"),
                    error: false,
                });
                self.refresh_key_source(Some(&provider));
            }
            Err(error) => {
                self.status = Some(Status {
                    message: format!("清除失败：{error}"),
                    error: true,
                });
            }
        }
        cx.notify();
    }

    fn refresh_key_source(&mut self, provider: Option<&str>) {
        self.key_source = self
            .service
            .key_source(provider.unwrap_or("openai-compatible"))
            .unwrap_or(KeySource::None);
    }
}

// —— 渲染 ——

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = UiTheme::global(cx);
        let (fg, dim, err, border) = (
            theme.colors.foreground,
            theme.colors.muted_foreground,
            theme.colors.destructive,
            theme.colors.border,
        );

        let mut form = div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .max_w(px(560.))
            .child(field("默认供应商", &self.provider, dim))
            .child(field("默认模型", &self.model, dim))
            .child(field("Base URL（留空用内置默认）", &self.base_url, dim))
            .child(field("代理（http/https/socks5）", &self.proxy, dim))
            .child(field("流式空闲超时（秒）", &self.timeout, dim));

        // 密钥行：只显示来源，不回显已存值
        form = form.child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_xs().text_color(dim).child("API Key"))
                .child(Input::new(&self.api_key))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .text_xs()
                        .child(div().text_color(dim).child(key_hint(self.key_source)))
                        .child(
                            Button::new("clear-key")
                                .label("清除钥匙串密钥")
                                .on_click(cx.listener(|this, _, _, cx| this.clear_key(cx))),
                        ),
                ),
        );

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
