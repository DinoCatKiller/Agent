//! 端到端测试（`Q1` §1）：`agent-chat` 编排 + **真实适配器与 HTTP 栈**（reqwest → wiremock）。
//!
//! 与 `features/chat/tests`（假 provider）互补：这里验证装配层——OpenAI 兼容适配器
//! 的 SSE 流经完整管线喂进 `ChatService`，工具循环在真实网络上收口。

use std::sync::Arc;

use agent_chat::{Chat, ChatService, LoopEvent, RoundStop, Tool, ToolOutput, ToolSet};
use agent_common::{FinishReason, ToolDefinition};
use agent_providers::{ErasedProvider, OpenAiCompatible, OpenAiConfig};
use agent_routing::Router;
use futures::StreamExt;
use serde_json::{Value, json};
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

struct Add;

impl Tool for Add {
    fn def(&self) -> ToolDefinition {
        ToolDefinition {
            name: "add".into(),
            description: "两数相加".into(),
            parameters: json!({
                "type": "object",
                "properties": {"a": {"type": "number"}, "b": {"type": "number"}},
                "required": ["a", "b"]
            }),
        }
    }

    fn run(&self, arguments: Value) -> futures::future::BoxFuture<'static, ToolOutput> {
        Box::pin(async move {
            let (Some(a), Some(b)) = (
                arguments.get("a").and_then(|v| v.as_f64()),
                arguments.get("b").and_then(|v| v.as_f64()),
            ) else {
                return Err("参数需要数字 a 与 b".into());
            };
            Ok(json!({ "sum": a + b }))
        })
    }
}

/// 构造一帧 OpenAI 流式 chunk。`delta` 是**对象**（裸字符串会被 json! 当成字符串值）。
fn chunk(id: &str, delta: Value, finish: Option<&str>) -> String {
    format!(
        "data: {}\n\n",
        json!({
            "id": id,
            "object": "chat.completion.chunk",
            "model": "fake-model",
            "choices": [{"index": 0, "delta": delta, "finish_reason": finish}]
        })
    )
}

fn raw(delta: &str) -> Value {
    serde_json::from_str(delta).expect("delta is valid json")
}

#[tokio::test]
async fn tool_loop_completes_over_real_http_stack() {
    let server = MockServer::start().await;
    // 第 1 次调用：模型要工具（arguments 分两片下发，验证拼装）。
    let tool_turn = format!(
        "{}{}{}{}",
        chunk(
            "c1",
            raw(
                r#"{"role":"assistant","tool_calls":[{"index":0,"id":"call_1","type":"function","function":{"name":"add","arguments":"{\"a\":"}}]}"#
            ),
            None
        ),
        chunk(
            "c1",
            raw(r#"{"tool_calls":[{"index":0,"function":{"arguments":"1,\"b\":2}"}}]}"#),
            None
        ),
        chunk(
            "c1",
            raw(r#"{"tool_calls":[{"index":0,"function":{"arguments":""}}]}"#),
            Some("tool_calls")
        ),
        "data: {\"id\":\"c1\",\"object\":\"chat.completion.chunk\",\"model\":\"fake-model\",\"choices\":[],\"usage\":{\"prompt_tokens\":21,\"completion_tokens\":6}}\n\ndata: [DONE]\n\n"
    );
    // 第 2 次调用：拿到结果后给出最终回答。
    let final_turn = format!(
        "{}{}{}{}",
        chunk("c2", raw(r#"{"role":"assistant","content":""}"#), None),
        chunk("c2", raw(r#"{"content":"3"}"#), None),
        chunk("c2", raw(r#"{}"#), Some("stop")),
        "data: {\"id\":\"c2\",\"object\":\"chat.completion.chunk\",\"model\":\"fake-model\",\"choices\":[],\"usage\":{\"prompt_tokens\":30,\"completion_tokens\":1}}\n\ndata: [DONE]\n\n"
    );
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(tool_turn, "text/event-stream"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(final_turn, "text/event-stream"))
        .mount(&server)
        .await;

    let provider = OpenAiCompatible::new(OpenAiConfig {
        id: "openai-compatible",
        base_url: format!("{}/v1", server.uri()),
        api_key: Some("test-key".into()),
        http: Default::default(),
        models: Vec::new(), // 自定义 base_url（模拟 vLLM）：不拦截，交服务端报错
    })
    .expect("provider builds");
    let service = ChatService::new(Arc::new(provider), ToolSet::new().with(Arc::new(Add)));
    let mut chat = Chat::new("fake-model");

    let round = service
        .run_round(&mut chat, "1+2=?", Default::default())
        .expect("round starts");
    let mut tool_ok = None;
    let mut final_text = String::new();
    {
        tokio::pin!(round);
        while let Some(event) = round.next().await {
            match event {
                LoopEvent::ToolFinished { ok, .. } => tool_ok = Some(ok),
                LoopEvent::Delta { text, .. } => final_text.push_str(&text),
                LoopEvent::Error { error } => panic!("端到端不应出错: {error}"),
                _ => {}
            }
        }
    }

    assert_eq!(tool_ok, Some(true), "add 工具执行成功");
    assert_eq!(final_text, "3");
    assert_eq!(
        chat.messages.len(),
        4,
        "user + assistant(tool) + tool + assistant"
    );
    assert_eq!(
        chat.messages[2].text(),
        json!({"sum": 3.0}).to_string(),
        "工具结果已回填"
    );
    assert_eq!(chat.total_usage.total(), 58, "两轮 usage 累计");
}

// —— M6：路由降级（`R2`）——

#[tokio::test]
async fn router_falls_back_to_next_model_on_rate_limit() {
    let server = MockServer::start().await;
    // m1 → 429；m2 → 正常流（按请求体里的 model 字段区分路由）
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(body_string_contains(r#""model":"m1""#))
        .respond_with(ResponseTemplate::new(429).set_body_raw(
            r#"{"error":{"message":"slow down","type":"rate_limit_error"}}"#,
            "application/json",
        ))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(body_string_contains(r#""model":"m2""#))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            format!(
                "{}{}{}",
                chunk("c9", raw(r#"{"role":"assistant","content":""}"#), None),
                chunk("c9", raw(r#"{"content":"备用模型顶上"}"#), None),
                chunk("c9", raw(r#"{}"#), Some("stop")),
            ),
            "text/event-stream",
        ))
        .mount(&server)
        .await;

    let provider = OpenAiCompatible::new(OpenAiConfig {
        id: "openai-compatible",
        base_url: format!("{}/v1", server.uri()),
        api_key: Some("test-key".into()),
        http: Default::default(),
        models: Vec::new(),
    })
    .expect("provider builds");
    let router = Router::new()
        .with_erased(Arc::new(provider) as Arc<dyn ErasedProvider>)
        .with_route(
            "m1",
            [("openai-compatible", "m1"), ("openai-compatible", "m2")],
        );
    let service = ChatService::new(Arc::new(router), ToolSet::new());
    let mut chat = Chat::new("m1");

    let events = {
        let round = service
            .run_round(&mut chat, "问一句", Default::default())
            .unwrap();
        tokio::pin!(round);
        let mut events = Vec::new();
        while let Some(event) = round.next().await {
            events.push(event);
        }
        events
    };

    assert!(
        !events.iter().any(|e| matches!(e, LoopEvent::Error { .. })),
        "429 被透明降级，消费者看不到失败候选的错误"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, LoopEvent::Delta { text, .. } if text == "备用模型顶上"))
    );
    assert!(matches!(
        events.last(),
        Some(LoopEvent::RoundEnded {
            stop: RoundStop::Finished(FinishReason::Stop),
            ..
        })
    ));
}
