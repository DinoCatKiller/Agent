//! 配置文件（TOML）读写（`X1` §4）。
//!
//! **这里只放非敏感项。** 任何形如 `api_key` 的字段都不该出现在本文件中——
//! 密钥一律走 [`crate::SecretStore`]（系统钥匙串）。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::error::ConfigError;

/// 配置文件内容（全部可选；缺省即「用默认值」）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AppConfig {
    /// 默认供应商 id（如 `openai-compatible` / `anthropic`）。
    pub default_provider: Option<String>,
    /// 默认模型 id。
    pub default_model: Option<String>,
    /// 兜底端点（未在 `providers` 中单独配置时使用）。
    pub base_url: Option<String>,
    /// HTTP 代理（如 `http://127.0.0.1:7890`）。
    pub proxy: Option<String>,
    /// 流式空闲超时（秒）。
    pub stream_idle_timeout_secs: Option<u64>,
    /// 各供应商的端点 / 默认模型覆盖。
    pub providers: BTreeMap<String, ProviderConfig>,
}

impl AppConfig {
    /// 某供应商的生效端点：先查 `providers[<id>].base_url`，再回退到顶层 `base_url`。
    pub fn base_url_for(&self, provider: &str) -> Option<&str> {
        self.providers
            .get(provider)
            .and_then(|config| config.base_url.as_deref())
            .or(self.base_url.as_deref())
    }

    /// 某供应商的默认模型：先查 `providers[<id>].model`，再回退到顶层 `default_model`。
    pub fn model_for(&self, provider: &str) -> Option<&str> {
        self.providers
            .get(provider)
            .and_then(|config| config.model.as_deref())
            .or(self.default_model.as_deref())
    }
}

/// 单个供应商条目的非敏感配置。
///
/// 条目 id 可以是**目录里的名字**（`ollama` / `openrouter`，见 `agent_providers::catalog`），
/// 也可以是用户自己起的名字 —— 后者必须写 `adapter`，因为目录推断不出来。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProviderConfig {
    /// 用哪个适配器（`openai-compatible` / `anthropic`）。目录里有的 id 可省。
    pub adapter: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    /// 该端点已知的**模型 id 清单**（C2）：联网拉取 `GET /models` 的结果写这里，
    /// 也允许用户手写。选择器的候选与本地能力协商都读它 —— 于是「拉到的模型」
    /// 复用**已有的逐模型显隐偏好**（`models.toml`），不引入额外状态。
    pub models: Vec<String>,
}

/// 配置文件位置。
///
/// 默认走平台标准目录（Windows `%APPDATA%\CodingRocket\config\config.toml` ——
/// 注意 `config` 子目录与文件名同名、macOS `~/Library/Application Support/CodingRocket/`、
/// Linux `~/.config/CodingRocket/`）；
/// 宿主可用 CLI 参数显式指定（[`ConfigPaths::from_file`]）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigPaths {
    file: PathBuf,
}

/// 平台标准配置目录下的文件名。
const CONFIG_FILE: &str = "config.toml";

impl ConfigPaths {
    /// 按平台标准目录定位配置文件。（不保证文件存在——不存在时 [`load`] 返回默认值。）
    pub fn discover() -> Result<Self, ConfigError> {
        let dirs = ProjectDirs::from("dev", "", "CodingRocket")
            .ok_or_else(|| ConfigError::Dir("ProjectDirs::from 返回 None".into()))?;
        Ok(Self {
            file: dirs.config_dir().join(CONFIG_FILE),
        })
    }

    /// 显式指定配置文件路径（测试与 `--config` 参数用）。
    pub fn from_file(path: impl Into<PathBuf>) -> Self {
        Self { file: path.into() }
    }

    /// 配置文件路径。
    pub fn file(&self) -> &Path {
        &self.file
    }
}

/// 读取配置。**文件不存在**返回 [`AppConfig::default`]（不是错误：零配置即可运行）。
pub fn load(paths: &ConfigPaths) -> Result<AppConfig, ConfigError> {
    let path = paths.file();
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            tracing::debug!(target: "agent_config", path = %path.display(), "配置文件不存在，使用默认配置");
            return Ok(AppConfig::default());
        }
        Err(error) => {
            return Err(ConfigError::Read {
                path: path.display().to_string(),
                reason: error.to_string(),
            });
        }
    };
    // 有意只用 `message()`：toml 的 `Display` 会**回显出错那一行的原文**，
    // 若用户误把密钥写进配置文件，明文就会随错误信息一起泄漏（`X1` §4）。
    toml::from_str(&text).map_err(|error| ConfigError::Parse {
        path: path.display().to_string(),
        reason: error.message().to_string(),
    })
}

/// 写回配置（保留用户手写的注释是不可能的——TOML 序列化会重排；见 `R1` §取舍）。
pub fn save(paths: &ConfigPaths, config: &AppConfig) -> Result<(), ConfigError> {
    let path = paths.file();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| ConfigError::Write {
            path: parent.display().to_string(),
            reason: error.to_string(),
        })?;
    }
    let text = toml::to_string_pretty(config).map_err(|error| ConfigError::Write {
        path: path.display().to_string(),
        reason: error.to_string(),
    })?;
    std::fs::write(path, text).map_err(|error| ConfigError::Write {
        path: path.display().to_string(),
        reason: error.to_string(),
    })
}

/// 进程内的配置文件写锁（C2）。
///
/// **为什么需要**：配置的修改一律是「读全量 → 改本切片管的字段 → 写回」，`AppConfig` 用的是
/// `BTreeMap` 而非原地更新。若两个写者并发（同一进程内：worker 写回拉取的模型清单 vs
/// 设置界面保存设置 / 连接条目），后写者基于**过期快照**写回，会把先写者的改动整段抹掉。
/// 进程内的 `Mutex` 串行化所有「读改写」即可消除这类丢失更新（跨进程不在范围：
/// 同一份配置只会有一个 `CodingRocket` 在跑）。
static FILE_LOCK: Mutex<()> = Mutex::new(());

/// 「读全量 → 改 → 写回」的**唯一**入口：全程持进程内锁，杜绝并发写相互覆盖。
///
/// 所有需要修改配置的调用方（设置保存、连接条目、写回模型清单）都应走这里，
/// 而不是各自 `load` + `save`。`edit` 里只改自己管的字段，其余原样保留。
pub fn update(paths: &ConfigPaths, edit: impl FnOnce(&mut AppConfig)) -> Result<(), ConfigError> {
    let _guard = FILE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut config = load(paths)?;
    edit(&mut config);
    save(paths, &config)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "agent-config-test-{}-{name}.toml",
            std::process::id()
        ));
        path
    }

    #[test]
    fn missing_file_yields_default_config() {
        let paths = ConfigPaths::from_file(temp_path("missing"));
        assert_eq!(
            load(&paths).expect("missing is not an error"),
            AppConfig::default()
        );
    }

    #[test]
    fn round_trips_through_toml() {
        let paths = ConfigPaths::from_file(temp_path("roundtrip"));
        let mut config = AppConfig {
            default_provider: Some("openai-compatible".into()),
            default_model: Some("gpt-4o".into()),
            proxy: Some("http://127.0.0.1:7890".into()),
            stream_idle_timeout_secs: Some(60),
            ..AppConfig::default()
        };
        config.providers.insert(
            "anthropic".into(),
            ProviderConfig {
                adapter: None,
                base_url: Some("https://api.anthropic.com".into()),
                model: None,
                models: vec!["claude-3-5-sonnet".into()],
            },
        );

        save(&paths, &config).expect("save");
        let loaded = load(&paths).expect("load");
        assert_eq!(loaded, config);
        let _ = std::fs::remove_file(paths.file());
    }

    #[test]
    fn provider_lookup_falls_back_to_top_level() {
        let mut config = AppConfig {
            base_url: Some("https://fallback.example".into()),
            default_model: Some("m-default".into()),
            ..AppConfig::default()
        };
        config.providers.insert(
            "anthropic".into(),
            ProviderConfig {
                adapter: None,
                base_url: Some("https://api.anthropic.com".into()),
                model: None,
                models: Vec::new(),
            },
        );

        assert_eq!(
            config.base_url_for("anthropic"),
            Some("https://api.anthropic.com"),
            "供应商级覆盖优先"
        );
        assert_eq!(
            config.base_url_for("openai-compatible"),
            Some("https://fallback.example"),
            "未单独配置则回退顶层"
        );
        assert_eq!(config.model_for("anthropic"), Some("m-default"));
    }

    #[test]
    fn unknown_field_is_rejected() {
        let path = temp_path("unknown-field");
        std::fs::write(
            &path,
            "default_model = \"m\"\napi_key = \"should-not-be-here\"\n",
        )
        .unwrap();
        let paths = ConfigPaths::from_file(&path);
        let error = load(&paths).expect_err("密钥写进配置文件必须报错");
        assert!(matches!(error, ConfigError::Parse { .. }));
        assert!(
            !error.to_string().contains("should-not-be-here"),
            "错误信息不得回显密钥内容"
        );
        let _ = std::fs::remove_file(&path);
    }
}
