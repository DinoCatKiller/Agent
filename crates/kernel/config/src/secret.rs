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

/// 解析某条目的密钥：**环境变量优先，其次钥匙串**（`X1` §4）。
///
/// `env` 是**调用方给的**环境变量名 —— `config` 是机制层，不认识供应商目录
/// （`D6`）：厂商 → 变量名的映射在 `agent_providers::catalog`。没有约定名的条目
/// （本地服务、自定义厂商）传 `None`，只看钥匙串。
///
/// 两者都没有 → `Ok(None)`（调用方决定是否提示）。
/// 钥匙串后端不可用 → 不阻断，`Ok(None)`（上层提示「未配置密钥」）。
pub fn resolve_api_key(
    provider: &str,
    env: Option<&str>,
) -> Result<Option<SecretString>, ConfigError> {
    if let Some(secret) = env.and_then(env_api_key) {
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
///
/// **按条目 id**（`ollama:api_key` / `openrouter:api_key`），不按适配器：
/// 同一个适配器下接多家厂商是很正常的（`C1` 之后尤其如此）。
pub fn account_for(provider: &str) -> String {
    format!("{provider}:api_key")
}

fn env_api_key(name: &str) -> Option<SecretString> {
    std::env::var(name)
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

    /// 环境变量名由调用方给（映射在 `agent_providers::catalog`）——
    /// 这里只验「给了名字就查，没给就不查」这一件事。
    ///
    /// 用**不存在的条目 id** 保证钥匙串里一定没有它，测试不受本机状态影响。
    #[test]
    fn env_name_is_supplied_by_the_caller() {
        assert!(
            resolve_api_key("agent-config-test-absent", None)
                .expect("不报错")
                .is_none(),
            "没给环境变量名 + 钥匙串没存过 → 就是没有密钥"
        );
    }

    #[test]
    fn account_is_namespaced_by_provider() {
        assert_eq!(account_for("anthropic"), "anthropic:api_key");
    }
}
