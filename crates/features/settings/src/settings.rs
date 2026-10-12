//! 设置项的业务模型与校验（`D6`：语义住在这里，机制在 `kernel/config`）。
//!
//! 为什么不直接把 `AppConfig` 当设置模型：`AppConfig` 是**文件格式**（`deny_unknown_fields`、
//! 字段随机制演进），而「有哪些设置项、怎么校验、怎么呈现」是产品语义。两者用
//! [`Settings::from_config`] / [`Settings::apply_to`] 单向转接——`apply_to` 只动本切片管的
//! 那几项，`[providers.*]` 等其余内容**原样保留**（否则保存会清掉用户手写的东西）。

use agent_config::AppConfig;

/// 设置项（本切片负责的那一部分）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Settings {
    pub default_provider: Option<String>,
    pub default_model: Option<String>,
    pub base_url: Option<String>,
    pub proxy: Option<String>,
    pub stream_idle_timeout_secs: Option<u64>,
}

impl Settings {
    /// 从配置文件里取出本切片管的项。
    pub fn from_config(config: &AppConfig) -> Self {
        Self {
            default_provider: config.default_provider.clone(),
            default_model: config.default_model.clone(),
            base_url: config.base_url.clone(),
            proxy: config.proxy.clone(),
            stream_idle_timeout_secs: config.stream_idle_timeout_secs,
        }
    }

    /// 写回配置文件对象。**只覆盖本切片管的字段**，其余（`providers` 等）保持原样。
    pub fn apply_to(&self, config: &mut AppConfig) {
        config.default_provider = self.default_provider.clone();
        config.default_model = self.default_model.clone();
        config.base_url = self.base_url.clone();
        config.proxy = self.proxy.clone();
        config.stream_idle_timeout_secs = self.stream_idle_timeout_secs;
    }

    /// 业务校验。返回**全部**问题（不是第一个），界面才好逐项标红。
    ///
    /// 判据（刻意从宽）：只挡「明显写错」——空串、非法 scheme、离谱的超时。
    /// 真正的可达性校验（端点是否活着）不在这里，也不该在保存时做。
    pub fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut errors = Vec::new();

        if let Some(provider) = &self.default_provider
            && provider.trim().is_empty()
        {
            errors.push(FieldError::new("default_provider", "不能是空白"));
        }
        if let Some(model) = &self.default_model
            && model.trim().is_empty()
        {
            errors.push(FieldError::new("default_model", "不能是空白"));
        }
        if let Some(base_url) = &self.base_url
            && !is_http_url(base_url)
        {
            errors.push(FieldError::new(
                "base_url",
                "必须是 http:// 或 https:// 开头的地址",
            ));
        }
        if let Some(proxy) = &self.proxy
            && !(is_http_url(proxy) || proxy.trim().starts_with("socks5://"))
        {
            errors.push(FieldError::new(
                "proxy",
                "必须是 http:// / https:// / socks5:// 开头",
            ));
        }
        if let Some(secs) = self.stream_idle_timeout_secs
            && !(1..=3600).contains(&secs)
        {
            errors.push(FieldError::new(
                "stream_idle_timeout_secs",
                "取值范围 1–3600 秒",
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// 界面草稿：字段都是字符串（输入框的天然形态），保存时转成 [`Settings`]。
///
/// **`api_key` 的明文只活在这个结构里的一小段时间**：它不是 [`Settings`] 的一部分（不进
/// 配置文件）、没有 `Debug` 实现（不会被顺手 `{:?}` 打出来）、保存后由界面清空。
/// 已存进钥匙串的密钥**不回显**——界面只显示「已配置 / 未配置」。
#[derive(Clone, Default)]
pub struct Draft {
    pub default_provider: String,
    pub default_model: String,
    pub base_url: String,
    pub proxy: String,
    pub stream_idle_timeout_secs: String,
    pub api_key: String,
}

impl Draft {
    /// 用已存的设置填充草稿（密钥槽留空：不回显）。
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            default_provider: settings.default_provider.clone().unwrap_or_default(),
            default_model: settings.default_model.clone().unwrap_or_default(),
            base_url: settings.base_url.clone().unwrap_or_default(),
            proxy: settings.proxy.clone().unwrap_or_default(),
            stream_idle_timeout_secs: settings
                .stream_idle_timeout_secs
                .map(|s| s.to_string())
                .unwrap_or_default(),
            api_key: String::new(),
        }
    }

    /// 校验并转成 [`Settings`]。
    ///
    /// 归一化在这里做：首尾空白去掉、空串归 `None`（空输入 = 「不设置这一项」，
    /// 而不是「设置成空字符串」——后者会被 `deny_unknown_fields` 之外的语义坑到）。
    pub fn to_settings(&self) -> Result<Settings, Vec<FieldError>> {
        let mut errors = Vec::new();
        let timeout = match trim_opt(&self.stream_idle_timeout_secs) {
            Some(raw) => match raw.parse::<u64>() {
                Ok(secs) => Some(secs),
                Err(_) => {
                    errors.push(FieldError::new(
                        "stream_idle_timeout_secs",
                        "必须是整数秒数",
                    ));
                    None
                }
            },
            None => None,
        };

        let settings = Settings {
            default_provider: trim_opt(&self.default_provider),
            default_model: trim_opt(&self.default_model),
            base_url: trim_opt(&self.base_url),
            proxy: trim_opt(&self.proxy),
            stream_idle_timeout_secs: timeout,
        };
        if let Err(mut more) = settings.validate() {
            errors.append(&mut more);
        }
        if errors.is_empty() {
            Ok(settings)
        } else {
            Err(errors)
        }
    }
}

/// 单项校验错误（字段名用配置文件里的键名，界面据此定位输入框）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldError {
    pub field: &'static str,
    pub message: String,
}

impl FieldError {
    pub fn new(field: &'static str, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for FieldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}：{}", self.field, self.message)
    }
}

fn trim_opt(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn is_http_url(raw: &str) -> bool {
    let trimmed = raw.trim();
    trimmed.starts_with("http://") || trimmed.starts_with("https://")
}
