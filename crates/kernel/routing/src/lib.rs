//! 模型路由与降级链（规格：`R2`）。
//!
//! 机制层：只知道「模型 → 候选链 → 能力协商 → 失败换下一家」，不含对话语义（`D6`）。
//! [`Router`] 实现 [`Provider`] 契约——编排层（`features/chat`）与宿主把它当一个
//! 普通供应商使用，感知不到降级的发生（除 `ProviderError.provider` 归因外）。
//!
//! # 降级策略（`R2` §3）
//!
//! | 失败类别 | 行为 |
//! |---|---|
//! | 可重试（429 / 5xx / 超时 / 网络断）| 换下一候选 |
//! | `Auth`（密钥 / 权限是**该供应商**的配置问题）| 换下一候选 |
//! | `InvalidRequest` / `ContextOverflow` / `ContentFilter` | 立即失败（换模型也救不了请求本身；按窗口大小选型随 `R2` 后续）|
//!
//! 流式的降级窗口：**只看首个事件**。首事件是可降级错误且还有候选 → 换下一家；
//! `Start` 已发出后出错 → 原样传播（中途换流会重复输出，重试属 `A6` 范畴）。

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use agent_common::{
    ErrorCategory, ModelRequest, ModelResponse, ModelSpec, ProviderError, StreamEvent,
};
use agent_providers::{CallContext, ErasedProvider, EventStream, Provider, required_capabilities};
use async_stream::stream;
use futures::StreamExt;

/// 路由器对外 id（错误归因用；真实供应商 id 看各候选的 `ProviderError.provider`）。
pub const ROUTER_ID: &str = "router";

/// 一条候选：某个供应商上的某个具体模型。
#[derive(Clone)]
struct Candidate {
    provider: Arc<dyn ErasedProvider>,
    model: String,
}

impl Candidate {
    /// 候选模型在本供应商的登记规格（未登记 = `None`，放行交供应商报错，与适配器一致）。
    fn spec(&self) -> Option<ModelSpec> {
        self.provider
            .list_models()
            .into_iter()
            .find(|spec| spec.id == self.model)
    }

    /// 把请求改写为指向本候选的模型。
    fn request(&self, req: &ModelRequest) -> ModelRequest {
        let mut request = req.clone();
        request.model = self.model.clone();
        request
    }
}

/// 该失败是否值得换下一候选（`R2` §3）。
fn should_fallback(error: &ProviderError) -> bool {
    error.retryable || matches!(error.category, ErrorCategory::Auth)
}

fn no_route(model: &str) -> ProviderError {
    ProviderError::new(ROUTER_ID, ErrorCategory::InvalidRequest)
        .with_message(format!("no route for model '{model}'"))
}

/// 模型路由器。
///
/// - **隐式路由**：未登记显式路由的模型，按注册顺序找「清单里有该模型」的供应商；
/// - **显式兜底链**（[`Router::with_route`]）：请求 `target` 时按候选依次尝试——
///   主模型挂了换备模型、跨供应商别名都靠它。
#[derive(Clone, Default)]
pub struct Router {
    providers: Vec<Arc<dyn ErasedProvider>>,
    routes: HashMap<String, Vec<Candidate>>,
}

impl Router {
    pub fn new() -> Self {
        Self::default()
    }

    /// 登记一个供应商；注册顺序即隐式路由的优先级。
    pub fn with_provider<P: Provider>(mut self, provider: P) -> Self {
        self.providers.push(Arc::new(provider));
        self
    }

    /// 登记一个已擦除的供应商（调用方手里是 `Arc<dyn ErasedProvider>` 时用）。
    pub fn with_erased(mut self, provider: Arc<dyn ErasedProvider>) -> Self {
        self.providers.push(provider);
        self
    }

    /// 显式兜底链：请求 `target` 时按 `candidates`（`(provider_id, model_id)`）依次尝试。
    /// `provider_id` 未登记的候选被忽略（由测试与 `resolve` 的长度约束此行为）。
    pub fn with_route(
        mut self,
        target: impl Into<String>,
        candidates: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        let target = target.into();
        let chain = candidates
            .into_iter()
            .filter_map(|(provider_id, model)| {
                let provider_id = provider_id.into();
                let provider = self.provider(&provider_id)?;
                Some(Candidate {
                    provider,
                    model: model.into(),
                })
            })
            .collect();
        self.routes.insert(target, chain);
        self
    }

    fn provider(&self, id: &str) -> Option<Arc<dyn ErasedProvider>> {
        self.providers.iter().find(|p| p.id() == id).cloned()
    }

    /// 解析候选链：显式路由优先；否则按注册顺序匹配模型清单。
    pub(crate) fn resolve(&self, model: &str) -> Vec<Candidate> {
        if let Some(chain) = self.routes.get(model) {
            return chain.clone();
        }
        self.providers
            .iter()
            .filter(|p| p.list_models().iter().any(|spec| spec.id == model))
            .map(|p| Candidate {
                provider: p.clone(),
                model: model.to_string(),
            })
            .collect()
    }

    /// 能力协商（`A2` §1）：清单已登记且能力不足的候选在发请求**之前**剔除。
    /// 返回 `(可用候选, 被剔除的理由)`；全被剔除时上层直接以首个理由失败。
    fn viable(
        &self,
        candidates: Vec<Candidate>,
        req: &ModelRequest,
    ) -> (Vec<Candidate>, Vec<ProviderError>) {
        let required = required_capabilities(req);
        let mut ok = Vec::new();
        let mut rejected = Vec::new();
        for candidate in candidates {
            match candidate.spec() {
                Some(spec) if !spec.missing(&required).is_empty() => rejected.push(
                    ProviderError::new(ROUTER_ID, ErrorCategory::InvalidRequest).with_message(
                        format!(
                            "candidate '{}:{}' lacks required capabilities: {:?}",
                            candidate.provider.id(),
                            candidate.model,
                            spec.missing(&required)
                        ),
                    ),
                ),
                _ => ok.push(candidate),
            }
        }
        (ok, rejected)
    }

    fn first_failure(rejected: Vec<ProviderError>, model: &str) -> ProviderError {
        rejected
            .into_iter()
            .next()
            .unwrap_or_else(|| no_route(model))
    }

    async fn chat_routed(
        &self,
        req: ModelRequest,
        ctx: CallContext,
    ) -> Result<ModelResponse, ProviderError> {
        let (candidates, rejected) = self.viable(self.resolve(&req.model), &req);
        if candidates.is_empty() {
            return Err(Self::first_failure(rejected, &req.model));
        }
        let mut last: Option<ProviderError> = None;
        for candidate in &candidates {
            if ctx.cancel.is_cancelled() {
                break;
            }
            match candidate
                .provider
                .chat_boxed(candidate.request(&req), ctx.clone())
                .await
            {
                Ok(response) => return Ok(response),
                Err(error) if should_fallback(&error) => last = Some(error),
                Err(error) => return Err(error),
            }
        }
        Err(last.unwrap_or_else(|| {
            ProviderError::new(ROUTER_ID, ErrorCategory::Unknown)
                .with_message("routing cancelled before any candidate")
        }))
    }
}

impl Provider for Router {
    fn id(&self) -> &'static str {
        ROUTER_ID
    }

    fn list_models(&self) -> Vec<ModelSpec> {
        let mut specs: Vec<ModelSpec> = self
            .providers
            .iter()
            .flat_map(|p| p.list_models())
            .collect();
        // 显式别名（路由目标不在任何清单里）：借用主候选的规格，供裁剪 / 展示查询。
        for (target, chain) in &self.routes {
            if specs.iter().any(|spec| spec.id == *target) {
                continue;
            }
            if let Some(mut spec) = chain.first().and_then(Candidate::spec) {
                spec.id = target.clone();
                specs.push(spec);
            }
        }
        specs
    }

    fn chat(
        &self,
        req: ModelRequest,
        ctx: CallContext,
    ) -> impl Future<Output = Result<ModelResponse, ProviderError>> + Send {
        let this = self.clone();
        async move { this.chat_routed(req, ctx).await }
    }

    fn stream(&self, req: ModelRequest, ctx: CallContext) -> Result<EventStream, ProviderError> {
        let (candidates, rejected) = self.viable(self.resolve(&req.model), &req);
        if candidates.is_empty() {
            return Err(Self::first_failure(rejected, &req.model));
        }
        let out = stream! {
            let total = candidates.len();
            let mut last_error: Option<ProviderError> = None;
            for (index, candidate) in candidates.into_iter().enumerate() {
                if ctx.cancel.is_cancelled() {
                    return;
                }
                let mut inner = match candidate.provider.stream(candidate.request(&req), ctx.clone()) {
                    Ok(inner) => inner,
                    Err(error) if should_fallback(&error) => {
                        last_error = Some(error);
                        continue;
                    }
                    Err(error) => {
                        yield StreamEvent::Error { error };
                        return;
                    }
                };
                // 偷看首事件决定是否降级（`R2` §3：Start 之后不再换流）。
                match inner.next().await {
                    Some(StreamEvent::Error { error })
                        if should_fallback(&error) && index + 1 < total =>
                    {
                        last_error = Some(error);
                        continue;
                    }
                    Some(event) => {
                        yield event;
                        while let Some(next) = inner.next().await {
                            yield next;
                        }
                        return;
                    }
                    None => {
                        // 无事件即终止：正常流保证 Start 起步（A2 §4），视作可降级失败。
                        last_error = Some(
                            ProviderError::new(candidate.provider.id(), ErrorCategory::Truncated)
                                .with_message("candidate stream ended without events"),
                        );
                        continue;
                    }
                }
            }
            yield StreamEvent::Error {
                error: last_error.unwrap_or_else(|| {
                    ProviderError::new(ROUTER_ID, ErrorCategory::Unknown)
                        .with_message("no candidate produced events")
                }),
            };
        };
        Ok(Box::pin(out))
    }

    fn count_tokens(&self, req: &ModelRequest) -> Option<u32> {
        self.resolve(&req.model)
            .into_iter()
            .find_map(|candidate| candidate.provider.count_tokens(&candidate.request(req)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests_support::stub;
    use agent_common::Capability;

    fn spec(model: &str, provider: &str, caps: &[Capability]) -> ModelSpec {
        ModelSpec {
            id: model.into(),
            provider: provider.into(),
            context_window: 8_000,
            max_output: 1_000,
            capabilities: caps.to_vec(),
            pricing: None,
            deprecated: false,
        }
    }

    #[test]
    fn implicit_route_follows_registration_order() {
        let a = stub("a", &[spec("m", "a", &[Capability::Text])]);
        let b = stub("b", &[spec("m", "b", &[Capability::Text])]);
        let router = Router::new().with_provider(a).with_provider(b);
        let chain = router.resolve("m");
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].provider.id(), "a", "注册顺序即优先级");
        assert!(router.resolve("nope").is_empty());
    }

    #[test]
    fn explicit_route_overrides_and_drops_unknown_providers() {
        let a = stub("a", &[spec("m", "a", &[Capability::Text])]);
        let router = Router::new()
            .with_provider(a)
            .with_route("alias", [("ghost", "x"), ("a", "m")]);
        let chain = router.resolve("alias");
        assert_eq!(chain.len(), 1, "未登记的 provider 候选被忽略");
        assert_eq!(chain[0].model, "m");
        // 别名进清单：主候选的规格换 id，供裁剪查询
        let alias = Provider::list_models(&router)
            .into_iter()
            .find(|s| s.id == "alias")
            .expect("alias listed");
        assert_eq!(alias.context_window, 8_000);
    }

    #[test]
    fn capability_missing_candidate_is_rejected_before_any_call() {
        let weak = stub("weak", &[spec("m", "weak", &[Capability::Text])]);
        let strong = stub(
            "strong",
            &[spec("m", "strong", &[Capability::Text, Capability::Tools])],
        );
        let mut req = ModelRequest::new("m", vec![agent_common::Message::user("hi")]);
        req.tools = vec![agent_common::ToolDefinition {
            name: "t".into(),
            description: "d".into(),
            parameters: serde_json::json!({ "type": "object" }),
        }];
        let router = Router::new().with_provider(weak).with_provider(strong);
        let (candidates, rejected) = router.viable(router.resolve("m"), &req);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].provider.id(), "strong");
        assert_eq!(rejected.len(), 1);
        assert_eq!(rejected[0].category, ErrorCategory::InvalidRequest);
    }
}

/// 降级链行为测试（`Q1`：假供应商，断网）。
#[cfg(test)]
mod behavior {
    use super::*;
    use crate::tests_support::{StreamEventExt, simple_spec, stub};
    use futures::StreamExt;

    async fn collect(stream: EventStream) -> Vec<StreamEvent> {
        let mut events = Vec::new();
        futures::pin_mut!(stream);
        while let Some(event) = stream.next().await {
            events.push(event);
        }
        events
    }

    fn rate_limit(provider: &str) -> ProviderError {
        ProviderError::new(provider, ErrorCategory::RateLimit).with_message("slow down")
    }

    /// 按运行时真实形态调用：擦除后只剩一套 `ErasedProvider` 方法，无歧义。
    fn erased(router: Router) -> Arc<dyn ErasedProvider> {
        Arc::new(router)
    }

    #[tokio::test]
    async fn stream_falls_back_on_retryable_first_event() {
        let a = stub("a", &[simple_spec("m", "a")])
            .stream_once(vec![StreamEvent::error_rate_limit("a")]);
        let b = stub("b", &[simple_spec("m", "b")]).stream_once(vec![
            StreamEvent::start("b"),
            StreamEvent::delta_text("ok"),
            StreamEvent::end(),
        ]);
        let calls_a = a.calls.clone();
        let router = erased(Router::new().with_provider(a).with_provider(b));

        let events = collect(
            router
                .stream(
                    ModelRequest::new("m", vec![agent_common::Message::user("hi")]),
                    CallContext::default(),
                )
                .expect("route exists"),
        )
        .await;

        assert_eq!(
            events,
            vec![
                StreamEvent::start("b"),
                StreamEvent::delta_text("ok"),
                StreamEvent::end(),
            ],
            "A 的首个可重试错误被吞掉（对消费者透明），B 完整接管"
        );
        assert_eq!(*calls_a.lock().unwrap(), vec!["a:m".to_string()]);
    }

    #[tokio::test]
    async fn stream_propagates_non_fallback_error_without_trying_next() {
        let invalid = ProviderError::new("a", ErrorCategory::InvalidRequest).with_message("bad");
        let a = stub("a", &[simple_spec("m", "a")]).stream_once(vec![StreamEvent::Error {
            error: invalid.clone(),
        }]);
        let b = stub("b", &[simple_spec("m", "b")]);
        let calls_b = b.calls.clone();
        let router = erased(Router::new().with_provider(a).with_provider(b));

        let events = collect(
            router
                .stream(
                    ModelRequest::new("m", vec![agent_common::Message::user("hi")]),
                    CallContext::default(),
                )
                .unwrap(),
        )
        .await;

        assert_eq!(events, vec![StreamEvent::Error { error: invalid }]);
        assert!(calls_b.lock().unwrap().is_empty(), "请求本身有错，不换家");
    }

    #[tokio::test]
    async fn stream_sync_error_falls_back() {
        let a = stub("a", &[simple_spec("m", "a")]).stream_sync_err(rate_limit("a"));
        let b = stub("b", &[simple_spec("m", "b")])
            .stream_once(vec![StreamEvent::start("b"), StreamEvent::end()]);
        let router = erased(Router::new().with_provider(a).with_provider(b));

        let events = collect(
            router
                .stream(
                    ModelRequest::new("m", vec![agent_common::Message::user("hi")]),
                    CallContext::default(),
                )
                .unwrap(),
        )
        .await;
        assert_eq!(events, vec![StreamEvent::start("b"), StreamEvent::end()]);
    }

    #[tokio::test]
    async fn stream_does_not_fallback_after_start() {
        let rate = rate_limit("a");
        let a = stub("a", &[simple_spec("m", "a")]).stream_once(vec![
            StreamEvent::start("a"),
            StreamEvent::Error {
                error: rate.clone(),
            },
        ]);
        let b = stub("b", &[simple_spec("m", "b")]);
        let calls_b = b.calls.clone();
        let router = erased(Router::new().with_provider(a).with_provider(b));

        let events = collect(
            router
                .stream(
                    ModelRequest::new("m", vec![agent_common::Message::user("hi")]),
                    CallContext::default(),
                )
                .unwrap(),
        )
        .await;

        assert_eq!(
            events,
            vec![StreamEvent::start("a"), StreamEvent::Error { error: rate }],
            "Start 已发出：中途换流会重复输出，只能传播"
        );
        assert!(calls_b.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn exhausted_candidates_surface_last_error() {
        let a = stub("a", &[simple_spec("m", "a")])
            .stream_once(vec![StreamEvent::error_rate_limit("a")]);
        let b = stub("b", &[simple_spec("m", "b")])
            .stream_once(vec![StreamEvent::error_rate_limit("b")]);
        let router = erased(Router::new().with_provider(a).with_provider(b));

        let events = collect(
            router
                .stream(
                    ModelRequest::new("m", vec![agent_common::Message::user("hi")]),
                    CallContext::default(),
                )
                .unwrap(),
        )
        .await;
        assert_eq!(events.len(), 1, "中间候选的失败对消费者不可见");
        assert!(
            matches!(
                &events[0],
                StreamEvent::Error { error } if error.provider == "b"
            ),
            "耗尽后浮出最后一个候选的错误"
        );
    }

    #[tokio::test]
    async fn chat_falls_back_on_auth_fails_fast_on_invalid_request() {
        // Auth = 该供应商的配置问题 → 换家
        let a = stub("a", &[simple_spec("m", "a")])
            .chat_err(ProviderError::new("a", ErrorCategory::Auth).with_message("bad key"));
        let b = stub("b", &[simple_spec("m", "b")]);
        let router = erased(Router::new().with_provider(a).with_provider(b));
        let ok = router
            .chat_boxed(
                ModelRequest::new("m", vec![agent_common::Message::user("hi")]),
                CallContext::default(),
            )
            .await;
        assert!(ok.is_ok(), "Auth 换下一家后成功");

        // InvalidRequest = 请求本身的问题 → 立即失败
        let a = stub("a", &[simple_spec("m", "a")])
            .chat_err(ProviderError::new("a", ErrorCategory::InvalidRequest).with_message("bad"));
        let b = stub("b", &[simple_spec("m", "b")]);
        let calls_b = b.calls.clone();
        let router = erased(Router::new().with_provider(a).with_provider(b));
        let err = router
            .chat_boxed(
                ModelRequest::new("m", vec![agent_common::Message::user("hi")]),
                CallContext::default(),
            )
            .await
            .expect_err("InvalidRequest 不换家");
        assert_eq!(err.category, ErrorCategory::InvalidRequest);
        assert!(calls_b.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn cancelled_context_produces_no_events_and_no_calls() {
        let a = stub("a", &[simple_spec("m", "a")])
            .stream_once(vec![StreamEvent::start("a"), StreamEvent::end()]);
        let calls_a = a.calls.clone();
        let router = erased(Router::new().with_provider(a));

        let ctx = CallContext::default();
        ctx.cancel.cancel();
        let events = collect(
            router
                .stream(
                    ModelRequest::new("m", vec![agent_common::Message::user("hi")]),
                    ctx,
                )
                .unwrap(),
        )
        .await;
        assert!(events.is_empty(), "已取消：不产生任何事件");
        assert!(calls_a.lock().unwrap().is_empty(), "已取消：不打到供应商");
    }

    #[tokio::test]
    async fn no_route_and_count_tokens_delegation() {
        let router = erased(Router::new());
        let err = match router.stream(
            ModelRequest::new("ghost", vec![agent_common::Message::user("hi")]),
            CallContext::default(),
        ) {
            Err(error) => error,
            Ok(_) => panic!("无路由应同步失败"),
        };
        assert_eq!(err.category, ErrorCategory::InvalidRequest);

        let a = stub("a", &[simple_spec("m", "a")]).count_once(Some(7));
        let b = stub("b", &[simple_spec("m", "b")]).count_once(Some(42));
        let router = erased(Router::new().with_provider(a).with_provider(b));
        let tokens = router.count_tokens(&ModelRequest::new(
            "m",
            vec![agent_common::Message::user("hi")],
        ));
        assert_eq!(tokens, Some(7), "按候选链首个能回答的委托");
    }
}

/// 测试支撑（仅 `cfg(test)` 编译）：可编程假供应商。
#[cfg(test)]
pub(crate) mod tests_support;
