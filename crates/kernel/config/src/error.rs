//! 配置层错误。
//!
//! 刻意**不**复用 `agent-common` 的 `ProviderError`：那是「一次模型调用」的错误契约
//! （能进 `StreamEvent`、能回放），配置错误发生在调用之前，语义与生命周期都不同
//! （`D6` §1：机制层不依赖上层的语义类型）。
//!
//! 所有 `Display` 实现都**不得**包含密钥明文（`X1` §4）。

/// 配置与密钥层错误。
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// 定位平台标准配置目录失败。
    #[error("无法定位配置目录：{0}")]
    Dir(String),
    /// 读配置文件失败（不存在不算错误——上层把「无配置」当作默认值）。
    ///
    /// 字段名用 `reason` 而非 `source`：`thiserror` 会把 `source` 当成
    /// `Error::source()` 的链，要求它实现 `StdError`（见 `kernel/store` 的同类写法）。
    #[error("读取配置失败（{path}）：{reason}")]
    Read { path: String, reason: String },
    /// 解析配置文件失败。
    #[error("解析配置失败（{path}）：{reason}")]
    Parse { path: String, reason: String },
    /// 写配置文件失败。
    #[error("写入配置失败（{path}）：{reason}")]
    Write { path: String, reason: String },
    /// 系统钥匙串访问失败（后端不可用 / 权限不足）。
    ///
    /// 注意：**「没找到」不是错误**（返回 `Ok(None)`），只有真正的故障才走这里。
    #[error("钥匙串访问失败：{0}")]
    Keyring(String),
}
