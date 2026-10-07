//! 二进制入口：组装依赖、承载 CLI 命令。**不含**业务逻辑（都在 `features/*` 里）。
//!
//! - `self-check`：契约层冒烟（M1）。
//! - `chat`：对话 REPL（M4）——`agent-chat` 的装配与事件消费参考实现。

mod repl;

use repl::ChatArgs;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("self-check") => self_check(),
        Some("chat") => match parse_chat_args(&args[1..]) {
            Ok(chat_args) => {
                let code = tokio::runtime::Runtime::new()
                    .expect("tokio runtime")
                    .block_on(repl::run(chat_args));
                std::process::exit(code);
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!(
                    "用法: agent-app chat --provider <openai-compatible|anthropic> --model <id> [--base-url URL] [--api-key KEY]"
                );
                std::process::exit(2);
            }
        },
        Some(other) => {
            eprintln!("未知命令: {other}");
            eprintln!("可用命令: self-check | chat");
            std::process::exit(2);
        }
        None => {
            println!("agent-app {}", env!("CARGO_PKG_VERSION"));
            println!("可用命令: self-check | chat");
            println!("里程碑与下一步: S1");
        }
    }
}

fn parse_chat_args(rest: &[String]) -> Result<ChatArgs, String> {
    let mut provider = None;
    let mut model = None;
    let mut base_url = None;
    let mut api_key = None;
    let mut iter = rest.iter();
    while let Some(flag) = iter.next() {
        let value = iter.next().ok_or_else(|| format!("缺少 {flag} 的值"))?;
        match flag.as_str() {
            "--provider" => provider = Some(value.clone()),
            "--model" => model = Some(value.clone()),
            "--base-url" => base_url = Some(value.clone()),
            "--api-key" => api_key = Some(value.clone()),
            other => return Err(format!("未知参数: {other}")),
        }
    }
    Ok(ChatArgs {
        provider: provider.ok_or("缺少 --provider")?,
        model: model.ok_or("缺少 --model")?,
        base_url,
        api_key,
    })
}

use agent_common::{
    Capability, DeltaKind, ErrorCategory, FinishReason, Message, ModelRequest, ModelSpec, Pricing,
    ProviderError, ResponseFormat, StreamEvent, ToolCall, ToolChoice, ToolDefinition, Usage,
};
use agent_providers::{ProviderRegistry, required_capabilities};
use serde_json::json;

fn self_check() {
    println!("== 1. 消息模型 ==");
    let messages = vec![
        Message::system("你是一个简洁的助手。"),
        Message::user("用一句话说明 SSE 流式的好处。"),
    ];
    println!("用户消息文本: {}", messages[1].text());

    println!("\n== 2. 请求（工具 + 结构化输出）==");
    let mut req = ModelRequest::new("gpt-4o-mini", messages);
    req.temperature = Some(0.3);
    req.max_tokens = Some(256);
    req.response_format = Some(ResponseFormat::JsonObject);
    req.tools = vec![ToolDefinition {
        name: "get_weather".into(),
        description: "查询某城市当前天气".into(),
        parameters: json!({
            "type": "object",
            "properties": { "city": { "type": "string" } },
            "required": ["city"],
            "additionalProperties": false
        }),
    }];
    req.tool_choice = Some(ToolChoice::Auto);
    println!("{}", serde_json::to_string_pretty(&req).unwrap());

    println!("\n== 3. 能力协商（路由阶段 fast-fail）==");
    let spec = ModelSpec {
        id: "gpt-4o-mini".into(),
        provider: "openai-compatible".into(),
        context_window: 128_000,
        max_output: 16_384,
        capabilities: vec![Capability::Text, Capability::Tools, Capability::JsonMode],
        pricing: Some(Pricing {
            input_per_mtok: 0.15,
            output_per_mtok: 0.60,
            currency: "USD".into(),
        }),
        deprecated: false,
    };
    let required = required_capabilities(&req);
    let missing = spec.missing(&required);
    println!("请求需要: {required:?}");
    println!("模型支持: {:?}", spec.capabilities);
    println!(
        "缺失能力: {missing:?} -> {}",
        if missing.is_empty() {
            "通过"
        } else {
            "拒绝"
        }
    );

    // 该模型只有 JsonMode（合法 JSON），不支持严格 schema 输出 → 必须被拒绝
    let mut strict = req.clone();
    strict.response_format = Some(ResponseFormat::JsonSchema {
        name: "weather".into(),
        schema: json!({ "type": "object" }),
    });
    let missing_strict = spec.missing(&required_capabilities(&strict));
    assert_eq!(
        missing_strict,
        vec![Capability::JsonSchema],
        "只有 JsonMode 的模型不应被允许做 JsonSchema 输出"
    );
    println!("严格 schema 输出: 已拒绝（缺失 {missing_strict:?}）");

    println!("\n== 4. 流式事件序列（UI 将来消费的就是这个）==");
    let events = vec![
        StreamEvent::Start {
            response_id: "resp_1".into(),
            model: spec.id.clone(),
        },
        StreamEvent::Delta {
            kind: DeltaKind::Text,
            text: "SSE 让".into(),
        },
        StreamEvent::Delta {
            kind: DeltaKind::Text,
            text: "首字更早到达".into(),
        },
        StreamEvent::ToolCall {
            call: ToolCall {
                id: "call_1".into(),
                name: "get_weather".into(),
                arguments: json!({ "city": "北京" }),
                provider_id: None,
            },
        },
        StreamEvent::Usage {
            usage: Usage {
                input_tokens: 42,
                output_tokens: 17,
            },
        },
        StreamEvent::End {
            finish_reason: FinishReason::ToolCalls,
        },
    ];
    for event in &events {
        println!("{}", serde_json::to_string(event).unwrap());
    }

    let mut text = String::new();
    let mut usage = Usage::default();
    for event in events {
        match event {
            StreamEvent::Delta {
                kind: DeltaKind::Text,
                text: chunk,
            } => text.push_str(&chunk),
            StreamEvent::Usage { usage: u } => usage = u,
            _ => {}
        }
    }
    println!("无状态拼接结果: {text}（{} token）", usage.total());

    println!("\n== 5. 工具结果回填 ==");
    let tool_message = Message::tool_result("call_1", r#"{"temp_c":21}"#);
    println!("{}", serde_json::to_string(&tool_message).unwrap());

    println!("\n== 6. 错误分类与重试判定 ==");
    for (category, status) in [
        (ErrorCategory::RateLimit, 429_u16),
        (ErrorCategory::Auth, 401),
        (ErrorCategory::ContextOverflow, 400),
    ] {
        let error = ProviderError::new("openai-compatible", category)
            .with_status(status)
            .with_message("示例错误");
        println!("{error} | retryable={}", error.retryable);
    }

    println!("\n== 7. 供应商注册表 ==");
    let registry = ProviderRegistry::new();
    println!("已注册供应商: {:?}（M3 起登记首个适配器）", registry.ids());

    println!("\n[ok] 契约层自检通过");
}
