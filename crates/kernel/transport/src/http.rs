//! HTTP 客户端封装（对应 `A4`）。
//!
//! 只负责「可靠地发出去、可靠地读回来」，不碰厂商字段：
//! - 连接超时 / 可选的整体读超时；
//! - **流式空闲超时**：部分厂商不发结束记录，见 `X1` §2；
//! - 代理、rustls（无 OpenSSL 依赖）。
//!
//! 鉴权头与请求体构造属于 `agent-providers` 的适配器，不在这里。
//! 本模块**不认识** `ProviderError`：错误在这里是 [`TransportError`]，
//! 由适配器按 `A2` §5 映射成 `ProviderError`。

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use futures::Stream;
use reqwest::{Client, ClientBuilder, Proxy};

/// 传输层错误。
///
/// 刻意不直接产出 [`agent_common::ProviderError`]：那需要 provider 名称与
/// HTTP 状态 → 错误类别的映射表，属于适配器职责（`A2` §5）。
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("failed to build http client: {0}")]
    Build(String),
    #[error("transport timeout: {0}")]
    Timeout(String),
    #[error("transport network error: {0}")]
    Network(String),
    /// 流式响应两次数据之间超过空闲上限。**必须**上抛，不得静默当成功
    /// （`A2` §4 的截断语义之一）。
    #[error("stream stalled for {0:?} (idle timeout)")]
    IdleTimeout(Duration),
    #[error("request cancelled")]
    Cancelled,
}

impl From<reqwest::Error> for TransportError {
    /// 把 reqwest 的底层错误归一为传输层错误（超时 vs 其它网络问题）。
    ///
    /// 供 `bytes_stream().map(TransportError::from)` 使用，把字节流统一成
    /// `Stream<Item = Result<Bytes, TransportError>>`，下游（SSE / 适配器）只认一种错误。
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            Self::Timeout(err.to_string())
        } else {
            Self::Network(err.to_string())
        }
    }
}

/// 传输层配置。字段全部来自配置层（`R1`），不在此处读环境变量或文件。
#[derive(Debug, Clone)]
pub struct HttpConfig {
    /// 建立连接（含 TLS 握手）超时。
    pub connect_timeout: Duration,
    /// 整体请求超时。**流式请求应为 `None`**，改用 [`Self::stream_idle_timeout`] 兜底。
    pub request_timeout: Option<Duration>,
    /// 流式响应两次数据之间的最大间隔。
    pub stream_idle_timeout: Duration,
    /// 代理地址（如 `http://127.0.0.1:7890`）。`None` 表示直连。
    pub proxy: Option<String>,
    /// User-Agent。`None` 表示用 reqwest 默认值。
    pub user_agent: Option<String>,
    /// 连接池中空闲连接的存活时间。
    pub pool_idle_timeout: Duration,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(10),
            // 流式调用不设整体超时，由空闲超时负责发现「卡死」。
            request_timeout: None,
            stream_idle_timeout: Duration::from_secs(60),
            proxy: None,
            user_agent: Some(concat!("agent-transport/", env!("CARGO_PKG_VERSION")).to_string()),
            pool_idle_timeout: Duration::from_secs(90),
        }
    }
}

/// 共享的 HTTP 客户端。`reqwest::Client` 内部是 `Arc`，clone 代价极低。
#[derive(Debug, Clone)]
pub struct HttpClient {
    client: Client,
    config: HttpConfig,
}

impl HttpClient {
    /// 按配置构建客户端。代理字符串非法或 TLS 后端不可用时在此 fast-fail。
    pub fn new(config: HttpConfig) -> Result<Self, TransportError> {
        let mut builder = ClientBuilder::new()
            .use_rustls_tls()
            .connect_timeout(config.connect_timeout)
            .pool_idle_timeout(config.pool_idle_timeout);

        if let Some(timeout) = config.request_timeout {
            builder = builder.timeout(timeout);
        }
        if let Some(user_agent) = &config.user_agent {
            builder = builder.user_agent(user_agent.clone());
        }
        if let Some(proxy) = &config.proxy {
            let proxy = Proxy::all(proxy).map_err(|e| TransportError::Build(e.to_string()))?;
            builder = builder.proxy(proxy);
        }

        let client = builder
            .build()
            .map_err(|e| TransportError::Build(e.to_string()))?;
        Ok(Self { client, config })
    }

    /// 底层客户端，用于 `get` / `post` 等构造请求。
    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn config(&self) -> &HttpConfig {
        &self.config
    }

    pub fn stream_idle_timeout(&self) -> Duration {
        self.config.stream_idle_timeout
    }

    /// 给响应字节流套上空闲超时。SSE 解析前调用（见 `A4`）。
    pub fn guard_idle<S>(&self, stream: S) -> IdleTimeout<S>
    where
        S: Stream + Unpin,
    {
        IdleTimeout::new(stream, self.config.stream_idle_timeout)
    }
}

/// 空闲超时流包装：两次数据之间超过 `idle` 即产出
/// [`TransportError::IdleTimeout`]，收到数据则重置计时。
///
/// 用「相邻数据间隔」而非「整体耗时」作为判据，长回答不会误杀。
pub struct IdleTimeout<S> {
    stream: S,
    idle: Duration,
    sleep: Pin<Box<tokio::time::Sleep>>,
}

impl<S> IdleTimeout<S> {
    pub fn new(stream: S, idle: Duration) -> Self {
        Self {
            stream,
            idle,
            sleep: Box::pin(tokio::time::sleep(idle)),
        }
    }
}

impl<S: Stream + Unpin> Stream for IdleTimeout<S> {
    type Item = Result<S::Item, TransportError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        match Pin::new(&mut this.stream).poll_next(cx) {
            Poll::Ready(Some(item)) => {
                this.sleep
                    .as_mut()
                    .reset(tokio::time::Instant::now() + this.idle);
                Poll::Ready(Some(Ok(item)))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => match this.sleep.as_mut().poll(cx) {
                Poll::Ready(()) => Poll::Ready(Some(Err(TransportError::IdleTimeout(this.idle)))),
                Poll::Pending => Poll::Pending,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[test]
    fn default_config_leaves_streaming_without_overall_timeout() {
        let config = HttpConfig::default();
        assert!(config.request_timeout.is_none());
        assert!(config.stream_idle_timeout > Duration::ZERO);
    }

    #[test]
    fn client_builds_with_default_config() {
        assert!(HttpClient::new(HttpConfig::default()).is_ok());
    }

    #[test]
    fn invalid_proxy_fails_fast() {
        let config = HttpConfig {
            proxy: Some("not a url".into()),
            ..HttpConfig::default()
        };
        assert!(matches!(
            HttpClient::new(config),
            Err(TransportError::Build(_))
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn idle_timeout_fires_when_stream_stalls() {
        let idle = Duration::from_millis(50);
        let mut stream = IdleTimeout::new(futures::stream::pending::<u8>(), idle);

        let got = tokio::time::timeout(Duration::from_secs(5), stream.next())
            .await
            .expect("paused clock should advance to the idle timer");

        match got {
            Some(Err(TransportError::IdleTimeout(d))) => assert_eq!(d, idle),
            other => panic!("expected idle timeout, got {other:?}"),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn idle_timeout_passes_items_and_completion_through() {
        let items: Vec<u8> = vec![1, 2, 3];
        let mut stream = IdleTimeout::new(
            futures::stream::iter(items.clone()),
            Duration::from_millis(50),
        );

        let mut got = Vec::new();
        while let Some(item) = stream.next().await {
            got.push(item.expect("items arrive immediately, no timeout"));
        }
        assert_eq!(got, items);
    }
}
