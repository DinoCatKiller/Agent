//! 测试支撑（仅 `cfg(test)` 编译）：可编程的假供应商。
//!
//! 脚本按调用次序弹出；脚本耗尽时走默认行为（chat 成功空响应、stream 立即结束）。
//! 每次 `chat` / `stream` 调用都会记录 `"{provider_id}:{model}"` 到 [`StubProvider::calls`]。

use std::collections::VecDeque;
use std::future::Future;
use std::sync::{Arc, Mutex};

use agent_common::{
    FinishReason, ModelRequest, ModelResponse, ModelSpec, ProviderError, StreamEvent, Usage,
};
use agent_providers::{CallContext, EventStream, Provider};
use futures::future::BoxFuture;

/// 一次 stream 调用的脚本：同步 `Err`（发请求前失败）或要吐的事件序列。
type StreamScriptItem = Result<Vec<StreamEvent>, ProviderError>;

/// 可编程假供应商（[`Provider`] 全实现，供路由测试编排失败 / 成功序列）。
#[derive(Clone)]
pub struct StubProvider {
    id: &'static str,
    models: Vec<ModelSpec>,
    chat_script: Arc<Mutex<VecDeque<Result<ModelResponse, ProviderError>>>>,
    stream_script: Arc<Mutex<VecDeque<StreamScriptItem>>>,
    count_script: Arc<Mutex<VecDeque<Option<u32>>>>,
    pub(crate) calls: Arc<Mutex<Vec<String>>>,
}

/// 极简模型规格：Text 能力、8k 窗口（路由测试够用）。
pub fn simple_spec(model: &str, provider: &str) -> ModelSpec {
    ModelSpec {
        id: model.into(),
        provider: provider.into(),
        context_window: 8_000,
        max_output: 1_000,
        capabilities: vec![agent_common::Capability::Text],
        pricing: None,
        deprecated: false,
    }
}

/// 建一个假供应商：`id` + 模型清单。
pub fn stub(id: &'static str, models: &[ModelSpec]) -> StubProvider {
    StubProvider {
        id,
        models: models.to_vec(),
        chat_script: Arc::new(Mutex::new(VecDeque::new())),
        stream_script: Arc::new(Mutex::new(VecDeque::new())),
        count_script: Arc::new(Mutex::new(VecDeque::new())),
        calls: Arc::new(Mutex::new(Vec::new())),
    }
}

/// `StreamEvent` 的测试构造器（语义见 `A2` §4）。
pub trait StreamEventExt {
    fn start(provider: &str) -> StreamEvent;
    fn delta_text(text: &str) -> StreamEvent;
    fn end() -> StreamEvent;
    fn error_rate_limit(provider: &str) -> StreamEvent;
}

impl StreamEventExt for StreamEvent {
    fn start(provider: &str) -> StreamEvent {
        StreamEvent::Start {
            response_id: format!("stub-{provider}"),
            model: "m".into(),
        }
    }

    fn delta_text(text: &str) -> StreamEvent {
        StreamEvent::Delta {
            kind: agent_common::DeltaKind::Text,
            text: text.into(),
        }
    }

    fn end() -> StreamEvent {
        StreamEvent::End {
            finish_reason: FinishReason::Stop,
        }
    }

    fn error_rate_limit(provider: &str) -> StreamEvent {
        StreamEvent::Error {
            error: ProviderError::new(provider, agent_common::ErrorCategory::RateLimit)
                .with_message("slow down"),
        }
    }
}

impl StubProvider {
    fn ok_response(model: &str) -> ModelResponse {
        ModelResponse {
            id: "stub".into(),
            model: model.into(),
            text: String::new(),
            tool_calls: Vec::new(),
            finish_reason: FinishReason::Stop,
            usage: Usage::default(),
            raw: None,
        }
    }

    /// 追加一次 chat 的脚本结果。
    pub fn chat_once(self, result: Result<ModelResponse, ProviderError>) -> Self {
        self.chat_script.lock().unwrap().push_back(result);
        self
    }

    /// chat 失败一次（构建 [`ProviderError`] 的语法糖）。
    pub fn chat_err(self, error: ProviderError) -> Self {
        self.chat_once(Err(error))
    }

    /// 追加一次 stream 的脚本：完整事件序列（原样吐出）。
    pub fn stream_once(self, events: Vec<StreamEvent>) -> Self {
        self.stream_script.lock().unwrap().push_back(Ok(events));
        self
    }

    /// stream 同步失败一次（`stream()` 直接返回 `Err`，对应「发请求前就暴露」）。
    pub fn stream_sync_err(self, error: ProviderError) -> Self {
        self.stream_script.lock().unwrap().push_back(Err(error));
        self
    }

    /// 追加一次 count_tokens 的脚本；`None` = 该供应商无法回答（路由继续找下一家）。
    pub fn count_once(self, tokens: Option<u32>) -> Self {
        self.count_script.lock().unwrap().push_back(tokens);
        self
    }

    fn record(&self, model: &str) {
        self.calls
            .lock()
            .unwrap()
            .push(format!("{}:{}", self.id, model));
    }
}

impl Provider for StubProvider {
    fn id(&self) -> &'static str {
        self.id
    }

    fn list_models(&self) -> Vec<ModelSpec> {
        self.models.clone()
    }

    fn chat(
        &self,
        req: ModelRequest,
        _ctx: CallContext,
    ) -> impl Future<Output = Result<ModelResponse, ProviderError>> + Send {
        self.record(&req.model);
        let result = self
            .chat_script
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| Ok(Self::ok_response(&req.model)));
        Box::pin(async move { result }) as BoxFuture<'static, Result<ModelResponse, ProviderError>>
    }

    fn stream(&self, req: ModelRequest, _ctx: CallContext) -> Result<EventStream, ProviderError> {
        self.record(&req.model);
        let scripted = self
            .stream_script
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| Ok(vec![StreamEvent::start(self.id)]));
        match scripted {
            Ok(events) => Ok(Box::pin(futures::stream::iter(events))),
            Err(error) => Err(error),
        }
    }

    fn count_tokens(&self, _req: &ModelRequest) -> Option<u32> {
        self.count_script.lock().unwrap().pop_front().flatten()
    }
}
