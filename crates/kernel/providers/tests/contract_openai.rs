//! OpenAI 兼容适配器契约测试（`Q1` §3 的 12 条用例，断网可测，G-3）。
//!
//! - 流式用例：读 fixture 的**原始 SSE 字节** → `parse_sse` + `OpenAiFramePolicy` 纯解析，
//!   比对 `expected_events.jsonl`（不经过 HTTP）。
//! - 错误映射：读 `errors/*.json` 错误体 → `map_http_error` 断言类别与可重试。
//! - 用例 4 / 12：请求体映射与能力 fast-fail，纯函数直接断言。
//! - 端到端（wiremock）：`chat` 全链路 + 401 映射，本地假服务器。

use std::fs;
use std::path::PathBuf;

use agent_common::{
    Capability, ContentPart, ErrorCategory, FinishReason, Message, ModelRequest, ModelResponse,
    ModelSpec, Role, StreamEvent, Usage,
};
use agent_providers::{
    OpenAiCompatible, OpenAiConfig, OpenAiFramePolicy, Provider, RefreshableModels, map_http_error,
    openai_default_models, parse_chat_response,
};
use agent_transport::{TransportError, finalize, parse_sse};
use bytes::Bytes;
use futures::StreamExt;
use serde_json::Value;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/openai-compatible")
}

fn read_fixture(dir: &str, file: &str) -> Vec<u8> {
    fs::read(fixture_root().join(dir).join(file))
        .unwrap_or_else(|e| panic!("fixture missing: {dir}/{file}: {e}"))
}

fn read_json(dir: &str, file: &str) -> Value {
    serde_json::from_slice(&read_fixture(dir, file))
        .unwrap_or_else(|e| panic!("fixture not valid json: {dir}/{file}: {e}"))
}

fn expected_events(dir: &str) -> Vec<StreamEvent> {
    let raw = fs::read_to_string(fixture_root().join(dir).join("expected_events.jsonl"))
        .unwrap_or_else(|e| panic!("expected_events.jsonl missing for {dir}: {e}"));
    raw.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).unwrap_or_else(|e| panic!("bad event line: {e}")))
        .collect()
}

/// 把 fixture 的原始 SSE 字节跑过「parse_sse → finalize」管线。
async fn run_sse(dir: &str) -> Vec<StreamEvent> {
    let bytes: Vec<Result<Bytes, TransportError>> = vec![Ok(Bytes::copy_from_slice(
        &read_fixture(dir, "wire_response.sse"),
    ))];
    finalize(
        parse_sse(futures::stream::iter(bytes)),
        OpenAiFramePolicy::new(),
    )
    .map(|item| item.expect("no transport error in fixture replay"))
    .collect()
    .await
}

fn provider_with(models: Vec<ModelSpec>, base_url: String) -> OpenAiCompatible {
    OpenAiCompatible::new(OpenAiConfig {
        id: "test",
        base_url,
        api_key: None,
        http: Default::default(),
        models,
    })
    .expect("provider builds")
}

fn text_model() -> ModelSpec {
    ModelSpec {
        id: "text-only".into(),
        provider: "test".into(),
        context_window: 128_000,
        max_output: 16_384,
        capabilities: vec![Capability::Text],
        pricing: None,
        deprecated: false,
        capabilities_known: true,
    }
}

fn chat_request() -> ModelRequest {
    ModelRequest::new("gpt-4o", vec![Message::user("hi")])
}

// 用例 1：非流式基础对话
#[test]
fn case01_chat_basic() {
    let body = read_json(".", "chat_basic.json");
    let resp = parse_chat_response("openai", body).expect("parse chat response");
    assert_eq!(resp.text, "Hello! How can I help you today?");
    assert_eq!(resp.finish_reason, FinishReason::Stop);
    assert_eq!(resp.usage.input_tokens, 11);
    assert_eq!(resp.usage.output_tokens, 5);
    assert_eq!(resp.model, "gpt-4o");
}

// 用例 2：流式文本，事件顺序 Start → Delta* → Usage → End，拼接等于完整文本
#[tokio::test]
async fn case02_stream_text_basic() {
    let events = run_sse("stream_text_basic").await;
    assert_eq!(events, expected_events("stream_text_basic"));
    let joined: String = events
        .iter()
        .filter_map(|e| match e {
            StreamEvent::Delta { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(joined, "Hello, world");
    assert!(matches!(events.first(), Some(StreamEvent::Start { .. })));
    assert!(matches!(
        events.last(),
        Some(StreamEvent::End {
            finish_reason: FinishReason::Stop
        })
    ));
}

// 用例 3：流式工具参数分片 → 完整可解析 JSON
#[tokio::test]
async fn case03_tool_call_split() {
    let events = run_sse("tool_call_split").await;
    assert_eq!(events, expected_events("tool_call_split"));
    let call = events
        .iter()
        .find_map(|e| match e {
            StreamEvent::ToolCall { call } => Some(call),
            _ => None,
        })
        .expect("tool call emitted");
    assert_eq!(call.name, "weather");
    assert_eq!(call.arguments, serde_json::json!({"city": "Shanghai"}));
}

// 用例 4：工具结果回填 → 映射为 role=tool + tool_call_id
#[test]
fn case04_tool_result_mapping() {
    let req: ModelRequest =
        serde_json::from_value(read_json("request_tool_result", "request.json"))
            .expect("valid ModelRequest fixture");
    let provider = provider_with(openai_default_models(), "https://example.com/v1".into());
    let body = provider
        .build_request_body(&req, false)
        .expect("request body");
    let messages = body["messages"].as_array().expect("messages array");
    let tool_msg = messages
        .iter()
        .find(|m| m["role"] == "tool")
        .expect("role=tool message");
    assert_eq!(tool_msg["tool_call_id"], "call_1");
    assert_eq!(tool_msg["content"], "sunny");
    let assistant = messages
        .iter()
        .find(|m| m["role"] == "assistant")
        .expect("assistant message");
    assert_eq!(assistant["tool_calls"][0]["id"], "call_1");
    assert_eq!(assistant["tool_calls"][0]["function"]["name"], "weather");
}

// 用例 5：截断（EOF 无终止记录）→ End { Truncated }，不得当成功
#[tokio::test]
async fn case05_truncated() {
    let events = run_sse("truncated").await;
    assert_eq!(events, expected_events("truncated"));
    assert!(matches!(
        events.last(),
        Some(StreamEvent::End {
            finish_reason: FinishReason::Truncated
        })
    ));
}

// 用例 6：坏帧（中间非法 JSON）→ 跳过、流继续、正常结束
#[tokio::test]
async fn case06_bad_frame() {
    let events = run_sse("bad_frame").await;
    assert_eq!(events, expected_events("bad_frame"));
    let joined: String = events
        .iter()
        .filter_map(|e| match e {
            StreamEvent::Delta { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(joined, "beforeafter", "坏帧被跳过，前后文本都保留");
    assert!(matches!(
        events.last(),
        Some(StreamEvent::End {
            finish_reason: FinishReason::Stop
        })
    ));
}

// 用例 7：429 → RateLimit + retryable
#[test]
fn case07_rate_limit() {
    let raw = read_json("errors", "429.json");
    let err = map_http_error("openai", 429, raw);
    assert_eq!(err.category, ErrorCategory::RateLimit);
    assert!(err.retryable);
}

// 用例 8：401 → Auth + 不可重试
#[test]
fn case08_auth() {
    let raw = read_json("errors", "401.json");
    let err = map_http_error("openai", 401, raw);
    assert_eq!(err.category, ErrorCategory::Auth);
    assert!(!err.retryable);
    assert_eq!(err.status, Some(401));
}

// 用例 9：5xx → Server + retryable
#[test]
fn case09_server_error() {
    let raw = read_json("errors", "500.json");
    let err = map_http_error("openai", 500, raw);
    assert_eq!(err.category, ErrorCategory::Server);
    assert!(err.retryable);
}

// 用例 10：上下文超限 → ContextOverflow（供上层裁剪重试）
#[test]
fn case10_context_overflow() {
    let raw = read_json("errors", "400_context.json");
    let err = map_http_error("openai", 400, raw);
    assert_eq!(err.category, ErrorCategory::ContextOverflow);
    assert!(!err.retryable, "裁剪重试由上层决定，默认不重试");
}

// 用例 11：无 usage → 仍以 Usage::default() 发一次
#[tokio::test]
async fn case11_no_usage_still_emits_default() {
    let events = run_sse("no_usage").await;
    let usages: Vec<Usage> = events
        .iter()
        .filter_map(|e| match e {
            StreamEvent::Usage { usage } => Some(*usage),
            _ => None,
        })
        .collect();
    assert_eq!(usages.len(), 1, "必须恰好发一次 Usage");
    assert_eq!(
        usages[0],
        Usage::default(),
        "供应商未回 usage 时用默认值保持契约"
    );
}

// 用例 12：能力不符 → 路由阶段拒绝，根本不产生请求
#[test]
fn case12_capability_mismatch_is_rejected_before_request() {
    let provider = provider_with(vec![text_model()], "https://example.com/v1".into());
    let mut req = chat_request();
    req.model = "text-only".into();
    req.messages.push(Message {
        role: Role::User,
        content: vec![ContentPart::Image {
            url: "https://example.com/a.png".into(),
            mime_type: Some("image/png".into()),
        }],
        tool_call_id: None,
        tool_calls: Vec::new(),
    });
    let err = match provider.stream(req, Default::default()) {
        Err(err) => err,
        Ok(_) => panic!("vision 打到纯文本模型必须在发请求前 fast-fail"),
    };
    assert_eq!(err.category, ErrorCategory::InvalidRequest);
    assert!(!err.retryable);
}

// 用例 13（C2 / `R1`）：未知模型的**保守规格不拦能力** —— 带工具的请求放行
// （`P1`：别误杀本地模型；`capabilities_known = false` 只免「能力」检查，不免「在不在清单」）
#[test]
fn case13_conservative_spec_does_not_block_tool_requests() {
    let unknown = agent_providers::conservative_spec("test", "local-llama");
    assert!(
        !unknown.capabilities_known,
        "conservative_spec 不是权威规格"
    );
    let provider = provider_with(vec![unknown], "https://example.com/v1".into());
    let mut req = chat_request();
    req.model = "local-llama".into();
    req.tools = vec![agent_common::ToolDefinition {
        name: "now".into(),
        description: "current time".into(),
        parameters: serde_json::json!({"type": "object"}),
    }];
    assert!(
        provider.stream(req, Default::default()).is_ok(),
        "保守规格（capabilities_known=false）不得在本地拦截 Tools 请求"
    );
}

// 用例 14（C2）：id **不在清单里**仍然 fast-fail —— 保守规格只免「能力」检查
#[test]
fn case14_unlisted_model_still_rejected() {
    let provider = provider_with(
        vec![agent_providers::conservative_spec("test", "local-llama")],
        "https://example.com/v1".into(),
    );
    let mut req = chat_request();
    req.model = "not-declared".into();
    let err = match provider.stream(req, Default::default()) {
        Err(err) => err,
        Ok(_) => panic!("未在清单里的模型仍必须 fast-fail"),
    };
    assert_eq!(err.category, ErrorCategory::InvalidRequest);
}

// —— wiremock 端到端（本地假服务器，断网可测）——

mod e2e {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn chat_roundtrip_via_mock_http() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw(read_fixture(".", "chat_basic.json"), "application/json"),
            )
            .mount(&server)
            .await;

        let provider = provider_with(openai_default_models(), format!("{}/v1", server.uri()));
        let resp: ModelResponse = provider
            .chat(chat_request(), Default::default())
            .await
            .expect("chat succeeds");
        assert_eq!(resp.text, "Hello! How can I help you today?");
    }

    #[tokio::test]
    async fn http_401_maps_to_auth_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(401)
                    .set_body_raw(read_fixture("errors", "401.json"), "application/json"),
            )
            .mount(&server)
            .await;

        let provider = provider_with(openai_default_models(), format!("{}/v1", server.uri()));
        let err = provider
            .chat(chat_request(), Default::default())
            .await
            .expect_err("401 must be an error");
        assert_eq!(err.category, ErrorCategory::Auth);
        assert!(!err.retryable);
        assert!(err.raw.is_some(), "原始错误体保留用于排错");
    }

    // —— C2a：联网拉取模型清单（`GET /models` 端到端）——

    #[tokio::test]
    async fn fetch_models_merges_dictionary_unknowns() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"object":"list","data":[
                    {"id":"gpt-4o","object":"model"},
                    {"id":"custom-local-model","object":"model"}
                ]}"#,
                "application/json",
            ))
            .mount(&server)
            .await;

        let provider = provider_with(Vec::new(), format!("{}/v1", server.uri()));
        let fetched = provider.fetch_models().await.expect("fetch succeeds");
        let gpt4o = fetched
            .iter()
            .find(|m| m.id == "gpt-4o")
            .expect("字典命中的模型被保留");
        assert_eq!(gpt4o.context_window, 128_000, "命中字典沿用官方规格");
        assert!(gpt4o.capabilities.contains(&Capability::Vision));
        assert_eq!(gpt4o.provider, "test", "归属当前适配器");
        let unknown = fetched
            .iter()
            .find(|m| m.id == "custom-local-model")
            .expect("未知模型不被丢弃");
        assert_eq!(unknown.context_window, 128_000, "未知模型回落保守默认");
        assert_eq!(unknown.max_output, 4096);
        assert_eq!(unknown.capabilities, vec![Capability::Text]);
    }

    #[tokio::test]
    async fn fetch_models_error_keeps_cache() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(
                ResponseTemplate::new(500)
                    .set_body_raw(read_fixture("errors", "500.json"), "application/json"),
            )
            .mount(&server)
            .await;

        let provider = provider_with(vec![text_model()], format!("{}/v1", server.uri()));
        let err = provider.fetch_models().await.expect_err("500 必须失败");
        assert_eq!(err.category, ErrorCategory::Server);
        // C2a 契约：拉取失败不清空现有缓存
        assert_eq!(provider.list_models().len(), 1);
        assert_eq!(provider.list_models()[0].id, "text-only");
    }

    // C2（`R2`）：200 但 `data` 缺失 / 为空 —— 按失败处理，**不清空缓存**
    #[tokio::test]
    async fn refresh_models_empty_data_keeps_cache() {
        for body in [
            r#"{"object":"list","data":[]}"#,
            r#"{"object":"list"}"#,
            r#"{"data":[{"object":"model"}]}"#,
        ] {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/v1/models"))
                .respond_with(ResponseTemplate::new(200).set_body_raw(body, "application/json"))
                .mount(&server)
                .await;

            let provider = provider_with(vec![text_model()], format!("{}/v1", server.uri()));
            assert!(
                provider.refresh_models().await.is_err(),
                "空 / 畸形清单必须按失败处理：{body}"
            );
            assert_eq!(provider.list_models().len(), 1, "缓存不能被清空：{body}");
            assert_eq!(provider.list_models()[0].id, "text-only");
        }
    }

    // C2（`R2`）：刷新与已有缓存**取并集** —— 端点没回的模型（用户手写 / 当前选中）不被丢掉
    #[tokio::test]
    async fn refresh_models_unions_with_existing_cache() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"object":"list","data":[{"id":"gpt-4o","object":"model"}]}"#,
                "application/json",
            ))
            .mount(&server)
            .await;

        let provider = provider_with(vec![text_model()], format!("{}/v1", server.uri()));
        let merged = provider.refresh_models().await.expect("refresh succeeds");
        let ids: Vec<&str> = merged.iter().map(|m| m.id.as_str()).collect();
        assert!(ids.contains(&"gpt-4o"), "拉到的模型进入清单：{ids:?}");
        assert!(
            ids.contains(&"text-only"),
            "已有缓存（当前选中 / 手写）不被覆盖丢掉：{ids:?}"
        );
        assert_eq!(
            provider.list_models().len(),
            merged.len(),
            "缓存与返回值一致"
        );
    }
}
