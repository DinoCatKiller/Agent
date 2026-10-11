//! 配置与密钥（规格：`R1`）。
//!
//! **机制层**（`D6` §1）：只负责「配置从哪读、密钥往哪存」，不认识任何供应商语义，
//! 也不决定「用哪个 provider」——那是 `features/settings` 与宿主的事。
//!
//! 两条边界（`X1` §4）：
//! - **配置文件只存非敏感项**（provider / base_url / model / 代理 / 超时）；
//!   `api_key` **一律**进系统钥匙串，绝不落文件、绝不进日志。
//! - 密钥按**环境变量 > 系统钥匙串**的顺序解析（CLI 参数由调用方更优先处理）；
//!   钥匙串不可用时的降级路径见 `R1` §降级。

mod error;
mod file;
mod secret;

pub use error::ConfigError;
pub use file::{AppConfig, ConfigPaths, ProviderConfig, load, save};
pub use secret::{SecretStore, api_key_env, resolve_api_key};
// 密钥载体由本 crate 统一出口，使用方不必再直接依赖 `secrecy`。
pub use secrecy::{ExposeSecret, SecretString};
