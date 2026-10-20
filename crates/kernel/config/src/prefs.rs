//! UI 偏好（**独立文件**，不并进 `config.toml`）。
//!
//! 为什么不并进 `config.toml`：`AppConfig` 带 `deny_unknown_fields`（`file.rs:16`），
//! 往里加键会让**旧版本二进制读到新配置直接解析失败**（那条约束有测试钉着）。
//! 偏好是「锦上添花」的数据，不该让用户为它承担一次单向兼容。分开存则两边互不影响。
//!
//! 与 `config.toml` 刻意相反的两条取舍：
//!
//! - **容忍未知键**（不写 `deny_unknown_fields`）：新版写的偏好，旧版照样能读，
//!   只是忽略不认识的部分。配置那边"宁可报错"是为了抓拼写错误；偏好没有这个问题。
//! - **坏了不报错**：文件缺失 / 解析失败 / 读不动一律降级为默认值 + 一条日志。
//!   偏好损坏不该挡启动（`R1`：配置问题不挡「跑起来」）。

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::ConfigError;
use crate::file::ConfigPaths;

/// 偏好文件名（与 `config.toml` **同目录**）。
const PREFS_FILE: &str = "models.toml";

impl ConfigPaths {
    /// 偏好文件路径：配置文件同目录下的 `models.toml`。
    ///
    /// 与 `--config` 联动：显式指定了配置文件，偏好也落在它旁边（测试与多套配置都靠这个）。
    pub fn preferences_file(&self) -> PathBuf {
        self.file().with_file_name(PREFS_FILE)
    }
}

/// UI 偏好。目前只有一项：模型选择器里**不显示**哪些模型。
///
/// 存「隐藏」而不是「显示」：新出现的模型默认可见（内置目录会随版本更新），
/// 用户不该为每个新模型回头补一次勾。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    /// provider → 被隐藏的模型 id。
    ///
    /// 值用模型 id 原样（不拼 `provider/model`）：模型 id 里**可以带 `/`**
    /// （OpenRouter 的 `deepseek/deepseek-chat` 就是），拼字符串会歧义。
    pub hidden_models: BTreeMap<String, Vec<String>>,
}

impl Preferences {
    /// 某模型是否被隐藏。
    pub fn is_hidden(&self, provider: &str, model: &str) -> bool {
        self.hidden_models
            .get(provider)
            .is_some_and(|models| models.iter().any(|it| it == model))
    }

    /// 设置某模型的显隐。幂等；该 provider 的列表空了就把键删掉（文件保持干净）。
    pub fn set_hidden(&mut self, provider: &str, model: &str, hidden: bool) {
        let models = self.hidden_models.entry(provider.to_string()).or_default();
        models.retain(|it| it != model);
        if hidden {
            models.push(model.to_string());
            models.sort();
        }
        if models.is_empty() {
            self.hidden_models.remove(provider);
        }
    }

    /// 读取偏好。**任何失败都降级为默认值**（不存在 / 解析失败 / 读不动），
    /// 解析失败时留一条 warn——否则用户会发现"勾选莫名失效"却查不到原因。
    pub fn load(paths: &ConfigPaths) -> Self {
        let path = paths.preferences_file();
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Self::default(),
            Err(error) => {
                tracing::warn!(
                    target: "agent_config",
                    path = %path.display(),
                    reason = %error,
                    "偏好文件读取失败，忽略"
                );
                return Self::default();
            }
        };
        match toml::from_str(&text) {
            Ok(prefs) => prefs,
            Err(error) => {
                // 与配置一样只用 `message()`：`Display` 会回显原文（`X1` §4 的口径）。
                tracing::warn!(
                    target: "agent_config",
                    path = %path.display(),
                    reason = %error.message(),
                    "偏好文件解析失败，忽略"
                );
                Self::default()
            }
        }
    }

    /// 写回偏好（必要时建目录）。
    pub fn save(&self, paths: &ConfigPaths) -> Result<(), ConfigError> {
        let path = paths.preferences_file();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| ConfigError::Write {
                path: parent.display().to_string(),
                reason: error.to_string(),
            })?;
        }
        let text = toml::to_string_pretty(self).map_err(|error| ConfigError::Write {
            path: path.display().to_string(),
            reason: error.to_string(),
        })?;
        std::fs::write(&path, text).map_err(|error| ConfigError::Write {
            path: path.display().to_string(),
            reason: error.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_paths(name: &str) -> ConfigPaths {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "agent-config-prefs-{}-{name}/config.toml",
            std::process::id()
        ));
        ConfigPaths::from_file(path)
    }

    #[test]
    fn preferences_file_sits_next_to_config() {
        let paths = temp_paths("sibling");
        let prefs = paths.preferences_file();
        assert_eq!(prefs.file_name().unwrap(), "models.toml");
        assert_eq!(prefs.parent(), paths.file().parent());
    }

    #[test]
    fn missing_file_yields_default() {
        let paths = temp_paths("missing");
        assert_eq!(Preferences::load(&paths), Preferences::default());
    }

    #[test]
    fn round_trip_hidden_models() {
        let paths = temp_paths("roundtrip");
        let mut prefs = Preferences::default();
        prefs.set_hidden("openai-compatible", "gpt-4o-mini", true);
        prefs.set_hidden("anthropic", "claude-x", true);
        prefs.save(&paths).expect("save");

        let loaded = Preferences::load(&paths);
        assert!(loaded.is_hidden("openai-compatible", "gpt-4o-mini"));
        assert!(loaded.is_hidden("anthropic", "claude-x"));
        assert!(!loaded.is_hidden("openai-compatible", "gpt-4o"));

        let _ = std::fs::remove_dir_all(paths.file().parent().unwrap());
    }

    #[test]
    fn set_hidden_is_idempotent_and_cleans_up() {
        let mut prefs = Preferences::default();
        prefs.set_hidden("openai-compatible", "m", true);
        prefs.set_hidden("openai-compatible", "m", true);
        assert_eq!(prefs.hidden_models["openai-compatible"], vec!["m"]);

        prefs.set_hidden("openai-compatible", "m", false);
        assert!(prefs.hidden_models.is_empty(), "空的 provider 键要删掉");
    }

    #[test]
    fn model_id_with_slash_is_not_ambiguous() {
        let mut prefs = Preferences::default();
        prefs.set_hidden("openai-compatible", "deepseek/deepseek-chat", true);
        assert!(prefs.is_hidden("openai-compatible", "deepseek/deepseek-chat"));
        assert!(!prefs.is_hidden("openai-compatible", "deepseek"));
    }

    #[test]
    fn unknown_keys_are_tolerated() {
        let paths = temp_paths("unknown");
        std::fs::create_dir_all(paths.preferences_file().parent().unwrap()).unwrap();
        std::fs::write(
            paths.preferences_file(),
            "future_field = 1\n[hidden_models]\nanthropic = [\"a\"]\n",
        )
        .unwrap();

        // 不报错、不降级：认识的键照读（这正是与 config.toml 的差别）
        let loaded = Preferences::load(&paths);
        assert!(loaded.is_hidden("anthropic", "a"));

        let _ = std::fs::remove_dir_all(paths.file().parent().unwrap());
    }

    #[test]
    fn corrupt_file_degrades_to_default() {
        let paths = temp_paths("corrupt");
        std::fs::create_dir_all(paths.preferences_file().parent().unwrap()).unwrap();
        std::fs::write(paths.preferences_file(), "this is not toml = = =").unwrap();

        assert_eq!(Preferences::load(&paths), Preferences::default());

        let _ = std::fs::remove_dir_all(paths.file().parent().unwrap());
    }
}
