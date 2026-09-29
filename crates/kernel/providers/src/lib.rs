//! Provider 契约与运行时注册表（对应 `A2`，ID `A2`）。
//!
//! 分工：
//! - 适配器实现 [`Provider`]（每家一个，见 [`openai`]）；
//! - 路由层通过 [`ProviderRegistry`] 拿到 `dyn ErasedProvider`，不依赖任何具体实现。

pub mod openai;

pub use openai::{
    OpenAiCompatible, OpenAiConfig, OpenAiFramePolicy, map_finish_reason, map_http_error,
    openai_default_models, parse_chat_response,
};

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use agent_common::{
    Capability, ContentPart, ModelRequest, ModelResponse, ModelSpec, ProviderError, ResponseFormat,
    StreamEvent,
};
use futures::future::BoxFuture;
use futures::stream::Stream;
use tokio_util::sync::CancellationToken;

/// 归一化流句柄。
pub type EventStream = Pin<Box<dyn Stream<Item = StreamEvent> + Send + 'static>>;

/// 一次调用的上下文。用 struct 而非裸 token，后续加字段不破坏适配器签名。
#[derive(Debug, Clone, Default)]
pub struct CallContext {
    /// 观测用；最终会落到 `ProviderError::request_id` 与 trace。
    pub request_id: Option<String>,
    /// 取消令牌：UI 点「停止」时触发。
    pub cancel: CancellationToken,
}

impl CallContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }
}

/// 供应商适配器契约。
///
/// - `chat` 与 `stream` 都必须实现（见 A2 §6 落地清单）。
/// - `stream` **同步返回**流句柄：鉴权/参数错误要在发出请求前就暴露，
///   而不是等消费者第一次 poll。
/// - 厂商特有参数走 `ModelRequest::additional_params`，不得污染公共字段。
/// - `Clone` 是硬要求：适配器内部用 `Arc` 持有 client，clone 只是引用计数，
///   类型擦除（[`ErasedProvider`]）需要 owned future，见下方实现。
pub trait Provider: Send + Sync + Clone + 'static {
    fn id(&self) -> &'static str;
    fn list_models(&self) -> Vec<ModelSpec>;
    fn chat(
        &self,
        req: ModelRequest,
        ctx: CallContext,
    ) -> impl Future<Output = Result<ModelResponse, ProviderError>> + Send;
    fn stream(&self, req: ModelRequest, ctx: CallContext) -> Result<EventStream, ProviderError>;
    /// 可选：本地估算或走供应商接口。
    fn count_tokens(&self, _req: &ModelRequest) -> Option<u32> {
        None
    }
}

/// 类型擦除视图：`Provider` 因 RPITIT 不是 object-safe，路由层需要 `dyn`。
pub trait ErasedProvider: Send + Sync + 'static {
    fn id(&self) -> &'static str;
    fn list_models(&self) -> Vec<ModelSpec>;
    fn chat_boxed(
        &self,
        req: ModelRequest,
        ctx: CallContext,
    ) -> BoxFuture<'static, Result<ModelResponse, ProviderError>>;
    fn stream(&self, req: ModelRequest, ctx: CallContext) -> Result<EventStream, ProviderError>;
}

impl<P: Provider> ErasedProvider for P {
    fn id(&self) -> &'static str {
        Provider::id(self)
    }

    fn list_models(&self) -> Vec<ModelSpec> {
        Provider::list_models(self)
    }

    fn chat_boxed(
        &self,
        req: ModelRequest,
        ctx: CallContext,
    ) -> BoxFuture<'static, Result<ModelResponse, ProviderError>> {
        let this = self.clone();
        Box::pin(async move { this.chat(req, ctx).await })
    }

    fn stream(&self, req: ModelRequest, ctx: CallContext) -> Result<EventStream, ProviderError> {
        Provider::stream(self, req, ctx)
    }
}

/// 运行时注册表。Core 只与它打交道，不认识任何具体供应商。
#[derive(Default)]
pub struct ProviderRegistry {
    providers: HashMap<&'static str, Arc<dyn ErasedProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<P: Provider>(&mut self, provider: P) -> &mut Self {
        self.providers.insert(provider.id(), Arc::new(provider));
        self
    }

    pub fn get(&self, provider_id: &str) -> Option<Arc<dyn ErasedProvider>> {
        self.providers.get(provider_id).cloned()
    }

    pub fn ids(&self) -> Vec<&'static str> {
        let mut ids: Vec<&'static str> = self.providers.keys().copied().collect();
        ids.sort_unstable();
        ids
    }

    /// 所有已注册供应商的模型清单（路由层的输入），按 provider + id 排序。
    pub fn models(&self) -> Vec<ModelSpec> {
        let mut all: Vec<ModelSpec> = self
            .providers
            .values()
            .flat_map(|provider| provider.list_models())
            .collect();
        all.sort_by(|a, b| {
            (a.provider.as_str(), a.id.as_str()).cmp(&(b.provider.as_str(), b.id.as_str()))
        });
        all
    }

    pub fn find_model(&self, model_id: &str) -> Option<ModelSpec> {
        self.providers
            .values()
            .flat_map(|provider| provider.list_models())
            .find(|spec| spec.id == model_id)
    }
}

/// 从请求推导出所需的模型能力。
///
/// 路由层在发请求**之前**用它做 fast-fail：凡是模型声明不支持的能力，
/// 都不要把请求打到供应商那边去（见 A2 §1）。
pub fn required_capabilities(req: &ModelRequest) -> Vec<Capability> {
    let mut caps = vec![Capability::Text];

    if !req.tools.is_empty() {
        caps.push(Capability::Tools);
    }
    match &req.response_format {
        Some(ResponseFormat::JsonObject) => caps.push(Capability::JsonMode),
        Some(ResponseFormat::JsonSchema { .. }) => caps.push(Capability::JsonSchema),
        _ => {}
    };
    if has_part(req, |part| matches!(part, ContentPart::Image { .. })) {
        caps.push(Capability::Vision);
    }
    if has_part(req, |part| matches!(part, ContentPart::File { .. })) {
        caps.push(Capability::File);
    }

    caps
}

fn has_part(req: &ModelRequest, pred: impl Fn(&ContentPart) -> bool) -> bool {
    req.messages
        .iter()
        .flat_map(|message| message.content.iter())
        .any(pred)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_request() -> ModelRequest {
        ModelRequest::new("m", vec![agent_common::Message::user("hi")])
    }

    #[test]
    fn plain_request_only_needs_text() {
        assert_eq!(
            required_capabilities(&base_request()),
            vec![Capability::Text]
        );
    }

    #[test]
    fn json_object_and_json_schema_are_distinct_capabilities() {
        let mut object = base_request();
        object.response_format = Some(ResponseFormat::JsonObject);
        assert!(required_capabilities(&object).contains(&Capability::JsonMode));
        assert!(!required_capabilities(&object).contains(&Capability::JsonSchema));

        let mut schema = base_request();
        schema.response_format = Some(ResponseFormat::JsonSchema {
            name: "out".into(),
            schema: serde_json::json!({ "type": "object" }),
        });
        assert!(required_capabilities(&schema).contains(&Capability::JsonSchema));
    }

    #[test]
    fn tools_and_vision_are_detected() {
        let mut req = base_request();
        req.tools = vec![agent_common::ToolDefinition {
            name: "t".into(),
            description: "d".into(),
            parameters: serde_json::json!({ "type": "object" }),
        }];
        req.messages.push(agent_common::Message {
            role: agent_common::Role::User,
            content: vec![ContentPart::Image {
                url: "https://example.com/a.png".into(),
                mime_type: Some("image/png".into()),
            }],
            tool_call_id: None,
            tool_calls: Vec::new(),
        });

        let caps = required_capabilities(&req);
        assert!(caps.contains(&Capability::Tools));
        assert!(caps.contains(&Capability::Vision));
    }

    #[test]
    fn unregistered_provider_is_none() {
        let registry = ProviderRegistry::new();
        assert!(registry.get("nope").is_none());
        assert!(registry.ids().is_empty());
    }
}
