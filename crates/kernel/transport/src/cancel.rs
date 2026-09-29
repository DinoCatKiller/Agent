//! 取消（**机制**，对应 `A4` / `A1` §6「所有对外调用必须支持取消」）。
//!
//! 把 `CancellationToken`（即 `CallContext.cancel` 的底层类型）接进一次调用的两个阶段：
//!
//! - **请求阶段**：[`cancellable`] —— 在 `token` 取消前完成 future，取消即放弃等待；
//! - **流阶段**：[`CancelGuard`] / [`guard`] —— `token` 取消后立刻产出
//!   [`TransportError::Cancelled`] 并结束，不再向上游取数据。
//!
//! 流被 drop 时 reqwest 会关掉底层连接，所以「产出 Cancelled 后就结束」等价于
//! 真正停掉这次调用。
//!
//! ## 为什么这里不直接用 `CallContext`
//!
//! `CallContext` 定义在 `agent-providers`（`A2` §2），而 transport **不能**反向依赖
//! providers（`A1` §2「依赖只能向下」）。所以本模块只认 `CancellationToken` 本身；
//! 把 `ctx.cancel` 传进来是适配器的一行代码。
//!
//! ## 实现要点：取消的唤醒必须跨 poll 存活
//!
//! tokio-util 的 `cancelled()` 内部持有 `Notified`，**Drop 即注销**注册。若每次
//! `poll_next` 临时构造再丢弃，取消时就没人来唤醒我们——流要一直挂到上游下次产出
//! 才知道被取消了。因此 [`CancelGuard`] 持久保存一个 `'static` 的
//! [`WaitForCancellationFutureOwned`]（`token.clone().cancelled_owned()`），让注册一直有效。

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;
use tokio_util::sync::{CancellationToken, WaitForCancellationFutureOwned};

use crate::http::TransportError;

/// 在 `token` 取消前完成 `future`；取消则返回 [`TransportError::Cancelled`]。
///
/// 用于**请求阶段**：适配器应当用它代替裸 `await`，例如
/// `cancellable(&ctx.cancel, request.send().map_err(TransportError::from))`。
///
/// `biased` 保证先看取消——已经取消时**绝不**发起（或继续等待）真实请求。
pub async fn cancellable<F, T>(token: &CancellationToken, future: F) -> Result<T, TransportError>
where
    F: Future<Output = Result<T, TransportError>>,
{
    tokio::select! {
        biased;
        () = token.cancelled() => Err(TransportError::Cancelled),
        result = future => result,
    }
}

/// 取消感知的流适配器，由 [`guard`] 构造（或 [`CancelGuard::new`]）。
///
/// 语义：取消后产出**一次** [`TransportError::Cancelled`]，随后结束；
/// 上游的错误同样会被转发并结束本流（与 `sse` / `finish` 的收尾一致）。
pub struct CancelGuard<S> {
    inner: S,
    /// 供同步查询与构造 owned future。
    token: CancellationToken,
    /// 持久保存的取消未来——见模块文档「唤醒必须跨 poll 存活」。
    cancelled: Pin<Box<WaitForCancellationFutureOwned>>,
    done: bool,
}

impl<S> CancelGuard<S> {
    /// 用 `token` 守卫 `inner`。
    pub fn new(inner: S, token: CancellationToken) -> Self {
        let cancelled = Box::pin(token.clone().cancelled_owned());
        Self {
            inner,
            token,
            cancelled,
            done: false,
        }
    }

    /// 本守卫监听的取消令牌。
    pub fn token(&self) -> &CancellationToken {
        &self.token
    }

    /// 令牌是否已取消（收尾后可区分「被取消结束」与「正常结束」）。
    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }
}

impl<S, T> Stream for CancelGuard<S>
where
    S: Stream<Item = Result<T, TransportError>> + Unpin,
{
    type Item = Result<T, TransportError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if this.done {
            return Poll::Ready(None);
        }
        // 取消优先：先登记/检查取消，再碰上游。
        if this.cancelled.as_mut().poll(cx).is_ready() {
            this.done = true;
            return Poll::Ready(Some(Err(TransportError::Cancelled)));
        }

        match Pin::new(&mut this.inner).poll_next(cx) {
            Poll::Ready(Some(item)) => {
                if item.is_err() {
                    this.done = true;
                }
                Poll::Ready(Some(item))
            }
            Poll::Ready(None) => {
                this.done = true;
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// 给流套上取消守卫。典型位置是**流水线最外层**：
/// `guard(finalize(parse_sse(bytes), policy), ctx.cancel)`。
pub fn guard<S>(stream: S, token: CancellationToken) -> CancelGuard<S> {
    CancelGuard::new(stream, token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use futures::{StreamExt, stream};
    use std::time::Duration;
    use tokio::sync::oneshot;

    fn network_error() -> TransportError {
        TransportError::Network("boom".into())
    }

    #[tokio::test]
    async fn cancel_before_first_poll_yields_cancelled() {
        let token = CancellationToken::new();
        token.cancel();

        let items = vec![Ok::<u8, TransportError>(1), Ok(2)];
        let mut guarded = guard(stream::iter(items), token);

        assert!(matches!(
            guarded.next().await,
            Some(Err(TransportError::Cancelled))
        ));
        assert!(guarded.next().await.is_none(), "取消后必须结束");
    }

    #[tokio::test]
    async fn items_before_cancel_are_delivered_then_cancelled() {
        let token = CancellationToken::new();
        let items = vec![Ok::<u8, TransportError>(1), Ok(2), Ok(3)];
        let mut guarded = guard(stream::iter(items), token.clone());

        assert!(
            matches!(guarded.next().await, Some(Ok(1))),
            "取消前的数据照常送达"
        );
        token.cancel();
        assert!(matches!(
            guarded.next().await,
            Some(Err(TransportError::Cancelled))
        ));
        assert!(guarded.next().await.is_none(), "取消后必须结束");
    }

    #[tokio::test]
    async fn stream_ends_normally_when_not_cancelled() {
        let token = CancellationToken::new();
        let items = vec![Ok::<u8, TransportError>(1), Ok(2)];
        let mut guarded = guard(stream::iter(items), token);

        assert!(matches!(guarded.next().await, Some(Ok(1))));
        assert!(matches!(guarded.next().await, Some(Ok(2))));
        assert!(guarded.next().await.is_none());
        assert!(!guarded.is_cancelled(), "正常结束不应被标记为取消");
    }

    #[tokio::test]
    async fn upstream_error_propagates_and_stops() {
        let token = CancellationToken::new();
        let items = vec![Err::<u8, TransportError>(network_error()), Ok(2)];
        let mut guarded = guard(stream::iter(items), token);

        assert!(matches!(
            guarded.next().await,
            Some(Err(TransportError::Network(_)))
        ));
        assert!(guarded.next().await.is_none(), "上游错误之后必须结束");
    }

    /// 「首次被 poll 就通知外部、但永不产出」的上游：配合 oneshot 精确断言
    /// 「守卫已经挂起并注册了唤醒」，从而验证取消唤醒路径（而非只是查标志位）。
    struct SignalOnce(Option<oneshot::Sender<()>>);

    impl Stream for SignalOnce {
        type Item = Result<u8, TransportError>;

        fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            if let Some(tx) = self.0.take() {
                let _ = tx.send(());
            }
            Poll::Pending
        }
    }

    #[tokio::test]
    async fn cancel_wakes_a_parked_stream() {
        let token = CancellationToken::new();
        let (tx, rx) = oneshot::channel();
        let mut guarded = guard(SignalOnce(Some(tx)), token.clone());

        let task = tokio::spawn(async move { guarded.next().await.is_some() });

        // 等到守卫确实 poll 过一次（此刻已挂起并注册了取消唤醒）。
        rx.await.expect("守卫应当已经 poll 过一次");
        token.cancel();

        let got = tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .expect("取消必须唤醒挂起的流")
            .expect("任务不应 panic");
        assert!(got, "取消后应当产出一个事件（Cancelled）");
    }

    #[tokio::test]
    async fn cancellable_returns_error_when_already_cancelled() {
        let token = CancellationToken::new();
        token.cancel();

        let result = cancellable(&token, async { Ok::<u8, TransportError>(42) }).await;
        assert!(matches!(result, Err(TransportError::Cancelled)));
    }

    #[tokio::test]
    async fn cancellable_passes_result_through_when_not_cancelled() {
        let token = CancellationToken::new();

        let result = cancellable(&token, async { Ok::<u8, TransportError>(7) }).await;
        assert!(matches!(result, Ok(7)));
    }

    #[tokio::test]
    async fn cancellable_aborts_a_pending_request() {
        // 请求挂起期间取消：必须立刻返回 Cancelled，而不是继续等下去。
        let token = CancellationToken::new();
        let canceller = token.clone();
        let stuck = async move {
            canceller.cancel();
            futures::future::pending::<Result<(), TransportError>>().await
        };

        let result = cancellable(&token, stuck).await;
        assert!(matches!(result, Err(TransportError::Cancelled)));
    }

    #[tokio::test]
    async fn guard_wraps_the_finalized_sse_pipeline() {
        use crate::finish::{Frame, FramePolicy, finalize_bytes};
        use crate::sse::SseEvent;

        /// 最小判决：`[DONE]` 是终止记录，其余按 data 当文本。
        struct TextPolicy;

        impl FramePolicy<SseEvent> for TextPolicy {
            type Output = String;

            fn classify(&mut self, frame: SseEvent) -> Frame<String> {
                if frame.is_done() {
                    Frame::end()
                } else {
                    Frame::one(frame.data)
                }
            }

            fn on_truncated(&mut self) -> Vec<String> {
                vec!["<truncated>".to_string()]
            }
        }

        let token = CancellationToken::new();
        let bytes: Vec<Result<Bytes, TransportError>> = vec![
            Ok(Bytes::from_static(b"data: a\n\n")),
            Ok(Bytes::from_static(b"data: b\n\n")),
        ];
        let mut guarded = guard(
            finalize_bytes(stream::iter(bytes), TextPolicy),
            token.clone(),
        );

        assert!(
            matches!(guarded.next().await, Some(Ok(text)) if text == "a"),
            "取消前的事件应正常送达"
        );
        token.cancel();
        // 取消优先于后面还没吐出的帧（b）与收尾兜底。
        assert!(matches!(
            guarded.next().await,
            Some(Err(TransportError::Cancelled))
        ));
        assert!(guarded.next().await.is_none());
    }
}
