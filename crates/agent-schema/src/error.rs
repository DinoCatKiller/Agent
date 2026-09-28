//! 错误分类与统一包装（对应 A2 §5）。
//!
//! 这里刻意**不用** `Box<dyn Error>` 作为 `source`：错误对象需要能进
//! `StreamEvent`、能进日志与 fixtures，因此必须 `Clone + Serialize`。
//! 原始原因以字符串形式保留在 `source_message`。

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 错误分类。每个适配器都要提供「HTTP 状态 / 错误体 → 本枚举」的映射表。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    /// 401 / 403 / 密钥无效：不重试，告警。
    Auth,
    /// 429：指数退避 + 抖动，必要时换模型。
    RateLimit,
    /// 超时（含流式空闲超时）。
    Timeout,
    /// 400：参数或模型不支持。不重试，可降级到兼容模型。
    InvalidRequest,
    /// 5xx：退避重试。
    Server,
    /// 超上下文窗口：裁剪 / 摘要后重试一次。
    ContextOverflow,
    /// 命中内容审核：不重试，上抛业务。
    ContentFilter,
    /// 流提前结束。
    Truncated,
    Unknown,
}

impl ErrorCategory {
    /// 默认是否可重试。策略层可在 `ProviderError::retryable` 上覆盖。
    pub fn retryable(self) -> bool {
        matches!(
            self,
            Self::RateLimit | Self::Timeout | Self::Server | Self::Truncated
        )
    }
}

/// 统一错误包装。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderError {
    pub provider: String,
    pub category: ErrorCategory,
    /// 是否可重试。默认取自 `category`，策略层可覆盖。
    pub retryable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// 供应商原始错误体，排错与 fixtures 用。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<Value>,
    /// 底层原因（已字符串化，保证可序列化）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_message: Option<String>,
}

impl ProviderError {
    pub fn new(provider: impl Into<String>, category: ErrorCategory) -> Self {
        Self {
            provider: provider.into(),
            category,
            retryable: category.retryable(),
            status: None,
            request_id: None,
            raw: None,
            source_message: None,
        }
    }

    pub fn with_status(mut self, status: u16) -> Self {
        self.status = Some(status);
        self
    }

    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    pub fn with_raw(mut self, raw: Value) -> Self {
        self.raw = Some(raw);
        self
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.source_message = Some(message.into());
        self
    }

    /// 覆盖默认重试判定（例如业务判定某类 429 不该重试）。
    pub fn retryable_override(mut self, retryable: bool) -> Self {
        self.retryable = retryable;
        self
    }
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {:?}", self.provider, self.category)?;
        if let Some(status) = self.status {
            write!(f, " status={status}")?;
        }
        if let Some(message) = &self.source_message {
            write!(f, ": {message}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ProviderError {}
