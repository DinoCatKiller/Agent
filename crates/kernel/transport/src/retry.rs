//! 退避重试（对应 `A6`，选型见 `X6` A3）。
//!
//! **机制层**：退避算法取自 `backoff`（指数退避 + 抖动），本模块只提供
//! 「按 `ProviderError::retryable` 判定 + 接取消」的控制流。
//!
//! 边界（`D6` §1）：
//! - **不做换模型兜底**——那是 `R2` 的 `Router` 的职责；本模块只管「同一目标重试几次」。
//! - **不含流式重试**：流只在「首个事件之前」失败才可重试，这需要首事件语义，
//!   随 `A6` 与适配器一起落地；本模块先覆盖一问一答（`chat` 类）调用。

use std::future::Future;
use std::time::Duration;

use backoff::ExponentialBackoff;
use backoff::backoff::Backoff;
use tokio_util::sync::CancellationToken;

use agent_common::{ErrorCategory, ProviderError};

/// 重试策略：最多 `max_attempts` 次尝试（含首次）。
///
/// 退避序列由 `backoff` 生成：`min(initial × multiplier^(n-1), max_interval)` 再按 `jitter` 抖动。
#[derive(Debug, Clone, PartialEq)]
pub struct RetryPolicy {
    /// 总尝试次数（含首次）。`1` 表示不重试。
    pub max_attempts: u32,
    /// 首次退避时长。
    pub initial_interval: Duration,
    /// 单次退避上限。
    pub max_interval: Duration,
    /// 退避倍数。
    pub multiplier: f64,
    /// 抖动比例（`0.0`–`1.0`）；`0.0` 便于确定性测试。
    pub jitter: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_interval: Duration::from_millis(500),
            max_interval: Duration::from_secs(8),
            multiplier: 2.0,
            jitter: 0.5,
        }
    }
}

impl RetryPolicy {
    /// 指定尝试次数，其余取默认值（至少 1 次）。
    pub fn new(max_attempts: u32) -> Self {
        Self {
            max_attempts: max_attempts.max(1),
            ..Self::default()
        }
    }

    /// 构造一份退避序列器。每次 [`retry`] 调用独立实例，避免跨调用串状态。
    fn backoff(&self) -> ExponentialBackoff {
        ExponentialBackoff {
            initial_interval: self.initial_interval,
            max_interval: self.max_interval,
            multiplier: self.multiplier,
            randomization_factor: self.jitter.clamp(0.0, 1.0),
            // 终止由 `max_attempts` 控制，不设时间窗口。
            max_elapsed_time: None,
            ..ExponentialBackoff::default()
        }
    }
}

/// 按策略重试一次调用。`op` 接收**尝试序号**（从 1 起），返回归一化错误。
///
/// - 只重试 `error.retryable == true`，否则立即返回；
/// - 退避期间取消 → 立刻返回「已取消」错误（`retryable == false`）；
/// - 尝试次数耗尽 → 返回最后一次错误。
pub async fn retry<T, F, Fut>(
    policy: &RetryPolicy,
    cancel: &CancellationToken,
    mut op: F,
) -> Result<T, ProviderError>
where
    F: FnMut(u32) -> Fut,
    Fut: Future<Output = Result<T, ProviderError>>,
{
    let mut backoff = policy.backoff();
    let mut attempt: u32 = 1;
    loop {
        match op(attempt).await {
            Ok(value) => return Ok(value),
            Err(error) => {
                if !error.retryable || attempt >= policy.max_attempts {
                    return Err(error);
                }
                let Some(delay) = backoff.next_backoff() else {
                    return Err(error);
                };
                tokio::select! {
                    biased;
                    _ = cancel.cancelled() => {
                        return Err(ProviderError::new(error.provider.clone(), ErrorCategory::Unknown)
                            .with_message("retry cancelled"));
                    }
                    _ = tokio::time::sleep(delay) => {}
                }
                attempt += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// 抖动设为 0，配合 `start_paused` 让退避确定性完成。
    fn policy(max_attempts: u32) -> RetryPolicy {
        RetryPolicy {
            max_attempts,
            initial_interval: Duration::from_millis(1),
            max_interval: Duration::from_millis(4),
            multiplier: 2.0,
            jitter: 0.0,
        }
    }

    fn retryable() -> ProviderError {
        ProviderError::new("test", ErrorCategory::RateLimit)
    }

    fn fatal() -> ProviderError {
        ProviderError::new("test", ErrorCategory::Auth)
    }

    #[tokio::test(start_paused = true)]
    async fn succeeds_after_a_retryable_failure() {
        let calls = Arc::new(AtomicU32::new(0));
        let counted = calls.clone();
        let policy = policy(3);
        let out: Result<&str, _> = retry(&policy, &CancellationToken::new(), move |attempt| {
            let counted = counted.clone();
            async move {
                counted.fetch_add(1, Ordering::SeqCst);
                if attempt == 1 {
                    Err(retryable())
                } else {
                    Ok("done")
                }
            }
        })
        .await;

        assert_eq!(out.expect("retry recovers"), "done");
        assert_eq!(calls.load(Ordering::SeqCst), 2, "失败一次后重试成功");
    }

    #[tokio::test(start_paused = true)]
    async fn non_retryable_returns_immediately() {
        let calls = Arc::new(AtomicU32::new(0));
        let counted = calls.clone();
        let policy = policy(5);
        let out: Result<(), _> = retry(&policy, &CancellationToken::new(), move |_| {
            let counted = counted.clone();
            async move {
                counted.fetch_add(1, Ordering::SeqCst);
                Err(fatal())
            }
        })
        .await;

        assert!(out.is_err(), "不可重试错误直接上抛");
        assert_eq!(calls.load(Ordering::SeqCst), 1, "不可重试错误只调一次");
    }

    #[tokio::test(start_paused = true)]
    async fn exhausts_max_attempts() {
        let calls = Arc::new(AtomicU32::new(0));
        let counted = calls.clone();
        let policy = policy(3);
        let out: Result<(), _> = retry(&policy, &CancellationToken::new(), move |_| {
            let counted = counted.clone();
            async move {
                counted.fetch_add(1, Ordering::SeqCst);
                Err(retryable())
            }
        })
        .await;

        let error = out.expect_err("耗尽后返回错误");
        assert_eq!(error.category, ErrorCategory::RateLimit);
        assert_eq!(calls.load(Ordering::SeqCst), 3, "恰好尝试 max_attempts 次");
    }

    #[tokio::test(start_paused = true)]
    async fn cancellation_during_backoff_stops_with_cancelled_error() {
        let cancel = CancellationToken::new();
        cancel.cancel();
        let calls = Arc::new(AtomicU32::new(0));
        let counted = calls.clone();
        let policy = policy(5);
        let out: Result<(), _> = retry(&policy, &cancel, move |_| {
            let counted = counted.clone();
            async move {
                counted.fetch_add(1, Ordering::SeqCst);
                Err(retryable())
            }
        })
        .await;

        let error = out.expect_err("取消后返回错误");
        assert!(!error.retryable, "取消错误不可再重试");
        assert_eq!(error.source_message.as_deref(), Some("retry cancelled"));
        assert_eq!(calls.load(Ordering::SeqCst), 1, "首次尝试后即因取消停止");
    }
}
