//! 密钥存取：系统钥匙串（`X1` §4）。
//!
//! 优先级（`X1` §4）：**环境变量 > 系统钥匙串**。CLI 参数由调用方（`app`）更优先处理。
//! 密钥一律用 [`SecretString`] 承载，其 `Debug` 实现**打码**，避免误入日志或 `{:?}`。

use keyring::Entry;
use secrecy::SecretString;

use crate::error::ConfigError;

/// 钥匙串中的服务名（同一服务下按账号区分密钥）。
const SERVICE: &str = "CodingRocket";

/// 系统钥匙串读写。
///
/// Windows 走 Credential Manager，macOS 走 Keychain，Linux 走 Secret Service。
/// 服务不可用时返回 [`ConfigError::Keyring`]，由上层决定是否降级（`R1` §降级）。
#[derive(Debug, Clone)]
pub struct SecretStore {
    service: String,
}

impl Default for SecretStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore {
    /// 使用默认服务名 `CodingRocket`。
    pub fn new() -> Self {
        Self {
            service: SERVICE.to_string(),
        }
    }

    /// 自定义服务名（测试用，避免污染真实凭据）。
    pub fn with_service(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    pub fn service(&self) -> &str {
        &self.service
    }

    fn entry(&self, account: &str) -> Result<Entry, ConfigError> {
        Entry::new(&self.service, account).map_err(|error| ConfigError::Keyring(error.to_string()))
    }

    /// 读取密钥；**未存过**返回 `Ok(None)`（不是错误）。
    pub fn get(&self, account: &str) -> Result<Option<SecretString>, ConfigError> {
        match self.entry(account)?.get_password() {
            Ok(secret) => Ok(Some(SecretString::from(secret))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(ConfigError::Keyring(error.to_string())),
        }
    }

    /// 写入 / 覆盖密钥。
    pub fn set(&self, account: &str, secret: &str) -> Result<(), ConfigError> {
        self.entry(account)?
            .set_password(secret)
            .map_err(|error| ConfigError::Keyring(error.to_string()))
    }

    /// 删除密钥；**本来就没有**也算成功（幂等）。
    pub fn delete(&self, account: &str) -> Result<(), ConfigError> {
        match self.entry(account)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(ConfigError::Keyring(error.to_string())),
        }
    }
}

/// 某供应商的密钥环境变量名（`R1` 的过渡约定，`A2` §6 要求密钥不写死）。
pub fn api_key_env(provider: &str) -> &'static str {
    match provider {
        "anthropic" => "ANTHROPIC_API_KEY",
        _ => "OPENAI_API_KEY",
    }
}

/// 解析某供应商的密钥：**环境变量优先，其次钥匙串**（`X1` §4）。
///
/// 两者都没有 → `Ok(None)`（调用方决定是否提示）。
/// 钥匙串后端不可用 → 环境变量仍有效时不算失败；否则返回 [`ConfigError::Keyring`]。
pub fn resolve_api_key(provider: &str) -> Result<Option<SecretString>, ConfigError> {
    if let Some(secret) = env_api_key(provider) {
        tracing::debug!(target: "agent_config", provider, "密钥来自环境变量");
        return Ok(Some(secret));
    }
    match SecretStore::new().get(&account_for(provider)) {
        Ok(found) => {
            if found.is_some() {
                tracing::debug!(target: "agent_config", provider, "密钥来自系统钥匙串");
            }
            Ok(found)
        }
        // 钥匙串不可用（如无 Secret Service 的 Linux）：不阻断，交上层提示「未配置密钥」。
        Err(error) => {
            tracing::warn!(target: "agent_config", provider, %error, "钥匙串不可用，跳过该来源");
            Ok(None)
        }
    }
}

/// 钥匙串账号名：按 provider 区分（同一服务下多把密钥）。
pub fn account_for(provider: &str) -> String {
    format!("{provider}:api_key")
}

fn env_api_key(provider: &str) -> Option<SecretString> {
    std::env::var(api_key_env(provider))
        .ok()
        .filter(|value| !value.is_empty())
        .map(SecretString::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;

    /// 钥匙串真实读写验证（`X1` §6 挂到 `R1` 的待办）。
    ///
    /// 默认 **ignored**：它会写入真实系统凭据管理器，不宜混在常规单测里。
    /// 手动跑：`cargo test -p agent-config -- --ignored --nocapture`
    #[test]
    #[ignore = "会写入真实系统钥匙串，手动执行"]
    fn keyring_round_trip_on_this_machine() {
        // 用独立服务名，避免污染产品凭据。
        let store = SecretStore::with_service("CodingRocket-ConfigTest");
        let account = "round-trip";
        store.delete(account).expect("cleanup before");
        assert!(
            store.get(account).expect("get").is_none(),
            "未写入时应为 None"
        );

        store.set(account, "s3cret-value").expect("set");
        let got = store.get(account).expect("get").expect("present");
        assert_eq!(got.expose_secret(), "s3cret-value");

        store.delete(account).expect("delete");
        assert!(
            store.get(account).expect("get after delete").is_none(),
            "删除后应读不到"
        );
    }

    #[test]
    fn secret_debug_is_redacted() {
        let secret = SecretString::from("s3cret-value");
        let rendered = format!("{secret:?}");
        assert!(
            !rendered.contains("s3cret-value"),
            "SecretString 的 Debug 必须打码，实际：{rendered}"
        );
    }

    #[test]
    fn env_var_name_depends_on_provider() {
        assert_eq!(api_key_env("anthropic"), "ANTHROPIC_API_KEY");
        assert_eq!(api_key_env("openai-compatible"), "OPENAI_API_KEY");
        assert_eq!(
            api_key_env("vllm"),
            "OPENAI_API_KEY",
            "兼容族共用同一环境变量"
        );
    }

    #[test]
    fn account_is_namespaced_by_provider() {
        assert_eq!(account_for("anthropic"), "anthropic:api_key");
    }
}
