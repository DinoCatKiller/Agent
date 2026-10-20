//! 设置的读取 / 写回编排。**机制全部走 `kernel/config`**（`R1`）。
//!
//! 这里没有 `std::fs`、没有 `keyring`：文件与钥匙串的读写、平台后端的选择、错误文案的
//! 「不回显原文」都由 `agent-config` 兜住。本模块只负责**编排顺序**与**保住用户内容**。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use agent_config::{
    ConfigError, ConfigPaths, ExposeSecret, Preferences, ProviderConfig, SecretStore, SecretString,
    account_for, load, update,
};

use crate::settings::Settings;

/// 密钥后端。
///
/// 抽成 trait 的唯一理由是**测试不能碰真实钥匙串**（`settings/README.md` / `Q1`）：
/// 生产用 [`KeyringBackend`]，测试用 [`MemoryBackend`]。
pub trait SecretBackend: Send + Sync {
    /// `Ok(None)` = 没存过（**不是错误**，`R1`）。
    fn get(&self, provider: &str) -> Result<Option<SecretString>, ConfigError>;
    fn set(&self, provider: &str, secret: &SecretString) -> Result<(), ConfigError>;
    /// 幂等：没存过也算成功。
    fn delete(&self, provider: &str) -> Result<(), ConfigError>;
}

/// 系统钥匙串（账号名唯一来源：`agent_config::account_for`，见 `R1`）。
#[derive(Debug, Clone)]
pub struct KeyringBackend {
    store: SecretStore,
}

impl KeyringBackend {
    pub fn new() -> Self {
        Self {
            store: SecretStore::new(),
        }
    }
}

impl Default for KeyringBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretBackend for KeyringBackend {
    fn get(&self, provider: &str) -> Result<Option<SecretString>, ConfigError> {
        self.store.get(&account_for(provider))
    }

    fn set(&self, provider: &str, secret: &SecretString) -> Result<(), ConfigError> {
        self.store
            .set(&account_for(provider), secret.expose_secret())
    }

    fn delete(&self, provider: &str) -> Result<(), ConfigError> {
        self.store.delete(&account_for(provider))
    }
}

/// 内存后端：测试与嵌入场景用。**绝不落盘、不碰系统凭据管理器。**
#[derive(Debug, Default)]
pub struct MemoryBackend {
    entries: Mutex<HashMap<String, String>>,
}

impl MemoryBackend {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SecretBackend for MemoryBackend {
    fn get(&self, provider: &str) -> Result<Option<SecretString>, ConfigError> {
        let entries = self.entries.lock().expect("memory backend");
        Ok(entries
            .get(&account_for(provider))
            .map(|raw| SecretString::from(raw.clone())))
    }

    fn set(&self, provider: &str, secret: &SecretString) -> Result<(), ConfigError> {
        let mut entries = self.entries.lock().expect("memory backend");
        entries.insert(account_for(provider), secret.expose_secret().to_string());
        Ok(())
    }

    fn delete(&self, provider: &str) -> Result<(), ConfigError> {
        let mut entries = self.entries.lock().expect("memory backend");
        entries.remove(&account_for(provider));
        Ok(())
    }
}

/// 密钥来源（界面提示用）。与 `R1` 的优先级一致，CLI 参数由宿主更优先处理。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeySource {
    /// 环境变量（`OPENAI_API_KEY` / `ANTHROPIC_API_KEY`）已提供，**优先级高于钥匙串**。
    Env,
    /// 钥匙串里有。
    Keyring,
    /// 两处都没有。
    None,
}

/// 设置读写入口。
pub struct SettingsService {
    paths: ConfigPaths,
    secrets: Arc<dyn SecretBackend>,
}

impl SettingsService {
    /// 平台标准目录 + 系统钥匙串（`R1`）。
    pub fn discover() -> Result<Self, ConfigError> {
        Ok(Self {
            paths: ConfigPaths::discover()?,
            secrets: Arc::new(KeyringBackend::new()),
        })
    }

    /// 指定配置路径与密钥后端（测试 / 定制用）。
    pub fn with_parts(paths: ConfigPaths, secrets: Arc<dyn SecretBackend>) -> Self {
        Self { paths, secrets }
    }

    /// 配置文件路径（界面里显示给用户，便于手改）。
    pub fn config_file(&self) -> &Path {
        self.paths.file()
    }

    /// 读设置。
    ///
    /// **刻意不降级**：`app` 启动时读到坏文件可以退回默认继续跑，但设置界面**必须报错** ——
    /// 否则用户一按保存，就把那个「读不动的文件」连同他手写的内容一起覆盖掉了。
    pub fn load(&self) -> Result<Settings, ConfigError> {
        Ok(Settings::from_config(&load(&self.paths)?))
    }

    /// 写回设置。
    ///
    /// 先读全量、只改本切片管的字段、再写回 —— `[providers.*]` 等本界面不管的内容原样保留。
    /// 走 [`update`]（持进程内写锁）：避免与 worker 写回模型清单并发时相互覆盖。
    /// 校验请用 [`crate::settings::Draft::to_settings`]，这里只管写。
    pub fn save(&self, settings: &Settings) -> Result<(), ConfigError> {
        update(&self.paths, |config| settings.apply_to(config))
    }

    /// 「连接提供商」：新增 / 覆盖 `[providers.<id>]` 这一个条目。
    ///
    /// 与 [`Self::save`] 同一条约定：**先读全量再写回**（持进程内写锁），其余内容
    /// （含用户手写的条目）原样保留。只写非敏感项；密钥走 [`Self::save_key`]（钥匙串），
    /// 这里**不碰**。
    pub fn connect_provider(
        &self,
        id: &str,
        adapter: &str,
        base_url: Option<&str>,
        model: Option<&str>,
    ) -> Result<(), ConfigError> {
        update(&self.paths, |config| {
            // 重连同一条目时**保住已拉取的模型清单**（C2）：否则用户每连一次就把模型列表清空。
            let models = config
                .providers
                .get(id)
                .map(|entry| entry.models.clone())
                .unwrap_or_default();
            config.providers.insert(
                id.to_string(),
                ProviderConfig {
                    adapter: Some(adapter.to_string()),
                    base_url: base_url.map(str::to_string),
                    model: model.map(str::to_string),
                    models,
                },
            );
        })
    }

    /// 该条目的密钥来源（只回来源，**不回明文**）。
    ///
    /// `env` 是约定环境变量名，由调用方给（映射在 `agent_providers::catalog`）——
    /// `None` = 该条目没有约定名（本地服务 / 自定义厂商），只看钥匙串。
    pub fn key_source(&self, provider: &str, env: Option<&str>) -> Result<KeySource, ConfigError> {
        if env.is_some_and(env_key_present) {
            return Ok(KeySource::Env);
        }
        match self.secrets.get(provider)? {
            Some(_) => Ok(KeySource::Keyring),
            None => Ok(KeySource::None),
        }
    }

    /// 写密钥进钥匙串。明文只经过这一个调用，**不进日志、不进错误、不入库**（`R1` / 硬约束 5）。
    pub fn save_key(&self, provider: &str, secret: &SecretString) -> Result<(), ConfigError> {
        self.secrets.set(provider, secret)
    }

    /// 删除钥匙串里的密钥（幂等）。
    pub fn delete_key(&self, provider: &str) -> Result<(), ConfigError> {
        self.secrets.delete(provider)
    }

    /// 偏好文件路径（`models.toml`，与 `config.toml` 同目录）。
    pub fn preferences_file(&self) -> PathBuf {
        self.paths.preferences_file()
    }

    /// 读 UI 偏好（模型选择器里隐藏了哪些模型）。
    ///
    /// 与 [`Self::load`] **刻意相反**：这里降级不报错 —— 偏好坏了不该挡用，
    /// 理由写在 `agent_config::prefs` 的模块文档里。
    pub fn preferences(&self) -> Preferences {
        Preferences::load(&self.paths)
    }

    /// 设置某模型在选择器里的显隐，并**立即落盘**（幂等）。
    ///
    /// 不像 [`Self::save`] 那样先全量读再写：偏好是"锦上添花"的数据，不存在把用户
    /// 手写内容覆盖掉的风险，文件也小。
    pub fn set_model_hidden(
        &self,
        provider: &str,
        model: &str,
        hidden: bool,
    ) -> Result<(), ConfigError> {
        let mut prefs = self.preferences();
        prefs.set_hidden(provider, model, hidden);
        prefs.save(&self.paths)
    }
}

/// 环境变量里是否已提供密钥（只判「有没有」，不看值）。
///
/// 参数是**环境变量名**而不是条目 id：名字由宿主从 `agent_providers::catalog` 查好传来。
pub fn env_key_present(env: &str) -> bool {
    std::env::var_os(env).is_some_and(|value| !value.is_empty())
}
