//! 轮次编排集成测试（`Q1` §1：Core 的循环用假 provider，断网可测）。
//!
//! 假 provider 按脚本逐次吐 `StreamEvent`，并记录收到的 `ModelRequest`，
//! 用来断言：工具结果回填形态、裁剪后的请求内容、以及「只在 End 后入史」。

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use agent_chat::{
    Chat, ChatService, LoopEvent, RoundStop, ServiceConfig, Tool, ToolOutput, ToolSet,
};
use agent_common::{
    Capability, DeltaKind, ErrorCategory, FinishReason, Message, ModelRequest, ModelResponse,
    ModelSpec, ProviderError, StreamEvent, ToolCall, ToolDefinition, Usage,
};
use agent_providers::{CallContext, EventStream, Provider};
use async_stream::stream;
use futures::{Stream, StreamExt};
use serde_json::{Value, json};

// —— 假 provider：按脚本回放 ——

#[derive(Debug, Clone)]
struct ScriptedTurn {
    text_parts: Vec<String>,
    tool_calls: Vec<ToolCall>,
    finish: FinishReason,
    usage: Usage,
    /// Some(msg) 时吐 `Error` 事件并终止（不吐 `End`），模拟模型侧失败。
    error: Option<String>,
}

impl ScriptedTurn {
    fn text(parts: &[&str], usage: (u32, u32)) -> Self {
        Self {
            text_parts: parts.iter().map(|s| s.to_string()).collect(),
            tool_calls: Vec::new(),
            finish: FinishReason::Stop,
            usage: Usage {
                input_tokens: usage.0,
                output_tokens: usage.1,
            },
            error: None,
        }
    }

    fn tool_calls(calls: Vec<ToolCall>, usage: (u32, u32)) -> Self {
        Self {
            text_parts: Vec::new(),
            tool_calls: calls,
            finish: FinishReason::ToolCalls,
            usage: Usage {
                input_tokens: usage.0,
                output_tokens: usage.1,
            },
            error: None,
        }
    }
}

#[derive(Clone)]
struct FakeProvider {
    id: &'static str,
    turns: Arc<Mutex<VecDeque<ScriptedTurn>>>,
    requests: Arc<Mutex<Vec<ModelRequest>>>,
    context_window: u32,
}

impl FakeProvider {
    fn requests(&self) -> Vec<ModelRequest> {
        self.requests.lock().unwrap().clone()
    }
}

impl Provider for FakeProvider {
    fn id(&self) -> &'static str {
        self.id
    }

    fn list_models(&self) -> Vec<ModelSpec> {
        vec![ModelSpec {
            id: "fake-model".into(),
            provider: self.id.into(),
            context_window: self.context_window,
            max_output: 4096,
            capabilities: vec![Capability::Text, Capability::Tools],
            pricing: None,
            deprecated: false,
        }]
    }

    async fn chat(
        &self,
        _req: ModelRequest,
        _ctx: CallContext,
    ) -> Result<ModelResponse, ProviderError> {
        Err(ProviderError::new(self.id, ErrorCategory::Unknown).with_message("fake: chat 未实现"))
    }

    fn stream(&self, req: ModelRequest, _ctx: CallContext) -> Result<EventStream, ProviderError> {
        self.requests.lock().unwrap().push(req);
        let turn = self
            .turns
            .lock()
            .unwrap()
            .pop_front()
            .expect("脚本用尽：测试脚本轮数与循环轮数不符");
        let provider_id = self.id;
        let out = stream! {
            yield StreamEvent::Start { response_id: "fake-1".into(), model: "fake-model".into() };
            if let Some(message) = turn.error {
                yield StreamEvent::Error {
                    error: ProviderError::new(provider_id, ErrorCategory::Server).with_message(message),
                };
                return;
            }
            for part in turn.text_parts {
                yield StreamEvent::Delta { kind: DeltaKind::Text, text: part };
            }
            for call in turn.tool_calls {
                yield StreamEvent::ToolCall { call };
            }
            yield StreamEvent::Usage { usage: turn.usage };
            yield StreamEvent::End { finish_reason: turn.finish };
        };
        Ok(Box::pin(out))
    }

    /// 固定估算：每条消息 10 token（供裁剪断言，`A7`）。
    fn count_tokens(&self, req: &ModelRequest) -> Option<u32> {
        Some(req.messages.len() as u32 * 10)
    }
}

// —— 假工具：静态返回 / 固定失败 ——

struct StaticTool {
    name: &'static str,
    output: ToolOutput,
}

impl Tool for StaticTool {
    fn def(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name.into(),
            description: "static test tool".into(),
            parameters: json!({"type": "object"}),
        }
    }

    fn run(&self, _arguments: Value) -> futures::future::BoxFuture<'static, ToolOutput> {
        let output = self.output.clone();
        Box::pin(async move { output })
    }
}

fn tool_call(id: &str, name: &str, args: Value) -> ToolCall {
    ToolCall {
        id: id.into(),
        name: name.into(),
        arguments: args,
        provider_id: None,
    }
}

fn scripted_service(turns: Vec<ScriptedTurn>) -> (ChatService, FakeProvider) {
    scripted_service_with(turns, 1_000_000, ServiceConfig::default())
}

fn scripted_service_with(
    turns: Vec<ScriptedTurn>,
    context_window: u32,
    config: ServiceConfig,
) -> (ChatService, FakeProvider) {
    let provider = FakeProvider {
        id: "fake",
        turns: Arc::new(Mutex::new(turns.into())),
        requests: Arc::new(Mutex::new(Vec::new())),
        context_window,
    };
    let tools = ToolSet::new()
        .with(Arc::new(StaticTool {
            name: "weather",
            output: Ok(json!({"temp_c": 21})),
        }))
        .with(Arc::new(StaticTool {
            name: "boom",
            output: Err("炸了".into()),
        }));
    (
        ChatService::new(Arc::new(provider.clone()), tools).with_config(config),
        provider,
    )
}

fn user_text(chat: &Chat) -> String {
    chat.messages.last().map(|m| m.text()).unwrap_or_default()
}

async fn collect(stream: impl Stream<Item = LoopEvent>) -> Vec<LoopEvent> {
    stream.collect::<Vec<_>>().await
}

// —— 用例 ——

#[tokio::test]
async fn single_text_round() {
    let (service, _fake) = scripted_service(vec![ScriptedTurn::text(&["你好", "！"], (3, 5))]);
    let mut chat = Chat::new("fake-model");

    let events = collect(
        service
            .run_round(&mut chat, "hi", Default::default())
            .unwrap(),
    )
    .await;

    assert_eq!(
        events,
        vec![
            LoopEvent::Delta {
                kind: DeltaKind::Text,
                text: "你好".into()
            },
            LoopEvent::Delta {
                kind: DeltaKind::Text,
                text: "！".into()
            },
            LoopEvent::ModelTurnEnded {
                finish_reason: FinishReason::Stop,
                usage: Usage {
                    input_tokens: 3,
                    output_tokens: 5
                },
            },
            LoopEvent::RoundEnded {
                stop: RoundStop::Finished(FinishReason::Stop),
                turns: 1,
                round_usage: Usage {
                    input_tokens: 3,
                    output_tokens: 5
                },
            },
        ],
        "事件顺序：Delta* → ModelTurnEnded → RoundEnded"
    );
    assert_eq!(chat.messages.len(), 2);
    assert_eq!(chat.messages[1].text(), "你好！");
    assert_eq!(chat.total_usage.total(), 8);
}

#[tokio::test]
async fn multi_turn_tool_loop_backfills_results() {
    let (service, fake) = scripted_service(vec![
        ScriptedTurn::tool_calls(
            vec![tool_call("call_1", "weather", json!({"city": "上海"}))],
            (10, 5),
        ),
        ScriptedTurn::text(&["上海 21 度"], (20, 8)),
    ]);
    let mut chat = Chat::new("fake-model");

    let events = collect(
        service
            .run_round(&mut chat, "天气如何", Default::default())
            .unwrap(),
    )
    .await;

    assert_eq!(events.len(), 6);
    assert!(matches!(
        &events[0],
        LoopEvent::ModelTurnEnded {
            finish_reason: FinishReason::ToolCalls,
            ..
        }
    ));
    assert!(
        matches!(&events[1], LoopEvent::ToolStarted { id, name } if id == "call_1" && name == "weather")
    );
    assert!(matches!(&events[2], LoopEvent::ToolFinished { id, ok: true } if id == "call_1"));
    assert!(matches!(&events[3], LoopEvent::Delta { text, .. } if text == "上海 21 度"));
    assert!(matches!(
        &events[4],
        LoopEvent::ModelTurnEnded {
            finish_reason: FinishReason::Stop,
            ..
        }
    ));
    assert!(matches!(
        &events[5],
        LoopEvent::RoundEnded {
            stop: RoundStop::Finished(FinishReason::Stop),
            turns: 2,
            ..
        }
    ));
    // 第二次请求的历史：user → assistant(tool_calls) → tool 结果
    let second = &fake.requests()[1];
    assert_eq!(second.messages.len(), 3);
    assert_eq!(second.messages[1].tool_calls[0].id, "call_1");
    assert_eq!(second.messages[2].role, agent_common::Role::Tool);
    assert_eq!(second.messages[2].tool_call_id.as_deref(), Some("call_1"));
    assert_eq!(second.messages[2].text(), r#"{"temp_c":21}"#);
    // 会话终态
    assert_eq!(
        chat.messages.len(),
        4,
        "user + assistant + tool + assistant"
    );
    assert_eq!(user_text(&chat), "上海 21 度");
    assert_eq!(chat.total_usage.total(), 43, "两轮 usage 累加进会话");
}

#[tokio::test]
async fn unknown_tool_error_is_fed_back_to_model() {
    let (service, fake) = scripted_service(vec![
        ScriptedTurn::tool_calls(vec![tool_call("call_1", "no_such_tool", json!({}))], (5, 2)),
        ScriptedTurn::text(&["抱歉，没有这个工具"], (4, 3)),
    ]);
    let mut chat = Chat::new("fake-model");

    let events = collect(
        service
            .run_round(&mut chat, "q", Default::default())
            .unwrap(),
    )
    .await;

    assert_eq!(events.len(), 6);
    assert!(matches!(
        &events[2],
        LoopEvent::ToolFinished { ok: false, .. }
    ));
    let second = &fake.requests()[1];
    assert_eq!(
        second.messages[2].text(),
        "error: unknown tool: no_such_tool"
    );
    assert_eq!(user_text(&chat), "抱歉，没有这个工具");
}

#[tokio::test]
async fn tool_execution_error_is_fed_back_to_model() {
    let (service, fake) = scripted_service(vec![
        ScriptedTurn::tool_calls(vec![tool_call("call_1", "boom", json!({}))], (5, 2)),
        ScriptedTurn::text(&["工具坏了，抱歉"], (4, 3)),
    ]);
    let mut chat = Chat::new("fake-model");

    let events = collect(
        service
            .run_round(&mut chat, "q", Default::default())
            .unwrap(),
    )
    .await;

    assert_eq!(events.len(), 6);
    assert!(matches!(&events[2], LoopEvent::ToolFinished { id, ok: false } if id == "call_1"));
    assert_eq!(fake.requests()[1].messages[2].text(), "error: 炸了");
}

#[tokio::test]
async fn model_error_rolls_back_uncommitted_output() {
    let (service, _fake) = scripted_service(vec![ScriptedTurn {
        text_parts: Vec::new(),
        tool_calls: Vec::new(),
        finish: FinishReason::Stop,
        usage: Usage::default(),
        error: Some("模型炸了".into()),
    }]);
    let mut chat = Chat::new("fake-model");

    let events = collect(
        service
            .run_round(&mut chat, "q", Default::default())
            .unwrap(),
    )
    .await;

    assert_eq!(events.len(), 1);
    assert!(
        matches!(&events[0], LoopEvent::Error { error } if error.category == ErrorCategory::Server)
    );
    assert_eq!(chat.messages.len(), 1, "只有用户输入入史，模型输出全部回滚");
    assert_eq!(chat.messages[0].text(), "q");
}

#[tokio::test]
async fn truncated_output_is_committed_and_reported() {
    let (service, _fake) = scripted_service(vec![ScriptedTurn {
        text_parts: vec!["半截输出".into()],
        tool_calls: Vec::new(),
        finish: FinishReason::Truncated,
        usage: Usage {
            input_tokens: 1,
            output_tokens: 2,
        },
        error: None,
    }]);
    let mut chat = Chat::new("fake-model");

    let events = collect(
        service
            .run_round(&mut chat, "q", Default::default())
            .unwrap(),
    )
    .await;

    assert_eq!(events.len(), 3);
    assert!(matches!(&events[0], LoopEvent::Delta { text, .. } if text == "半截输出"));
    assert!(matches!(
        &events[1],
        LoopEvent::ModelTurnEnded {
            finish_reason: FinishReason::Truncated,
            ..
        }
    ));
    assert!(matches!(
        &events[2],
        LoopEvent::RoundEnded {
            stop: RoundStop::Finished(FinishReason::Truncated),
            turns: 1,
            ..
        }
    ));
    assert_eq!(chat.messages.len(), 2, "截断的增量是用户看到的，入史");
    assert_eq!(chat.messages[1].text(), "半截输出");
}

#[tokio::test]
async fn max_turns_stops_tool_ping_pong() {
    let (service, fake) = scripted_service_with(
        vec![
            ScriptedTurn::tool_calls(vec![tool_call("c1", "weather", json!({}))], (1, 1)),
            ScriptedTurn::tool_calls(vec![tool_call("c2", "weather", json!({}))], (2, 2)),
            ScriptedTurn::tool_calls(vec![tool_call("c3", "weather", json!({}))], (3, 3)),
        ],
        1_000_000,
        ServiceConfig {
            max_model_turns: 2,
            ..ServiceConfig::default()
        },
    );
    let mut chat = Chat::new("fake-model");

    let events = collect(
        service
            .run_round(&mut chat, "q", Default::default())
            .unwrap(),
    )
    .await;

    assert!(matches!(
        events.last(),
        Some(LoopEvent::RoundEnded {
            stop: RoundStop::MaxTurns(2),
            turns: 2,
            ..
        })
    ));
    assert_eq!(fake.requests().len(), 2, "达到上限后不再发起新的模型调用");
    assert!(
        matches!(events.last(), Some(LoopEvent::RoundEnded { .. })),
        "max_turns 收尾前不执行最后一轮的工具（执行了也没人消费结果）"
    );
}

#[tokio::test]
async fn cancellation_before_first_event_rolls_back() {
    let (service, fake) = scripted_service(vec![ScriptedTurn::text(&["不会到达"], (1, 1))]);
    let mut chat = Chat::new("fake-model");
    let cancel = tokio_util::sync::CancellationToken::new();
    cancel.cancel();

    let events = collect(service.run_round(&mut chat, "q", cancel).unwrap()).await;

    assert_eq!(events, vec![LoopEvent::Cancelled]);
    assert_eq!(chat.messages.len(), 1, "模型输出未入史");
    assert_eq!(
        fake.requests().len(),
        1,
        "请求已发起（取消是流阶段的第一判据）"
    );
}

#[tokio::test]
async fn context_is_trimmed_before_request_when_over_window() {
    let (service, fake) = scripted_service_with(
        vec![ScriptedTurn::text(&["答"], (1, 1))],
        50, // 目标 = 50 × 0.5 = 25（每条消息估 10 token）
        ServiceConfig {
            context_keep_ratio: 0.5,
            ..ServiceConfig::default()
        },
    );
    let mut chat = Chat::new("fake-model");
    chat.push_user("q0");
    chat.messages.push(Message::assistant("a0")); // 旧交换：30 token > 25

    let events = collect(
        service
            .run_round(&mut chat, "q1", Default::default())
            .unwrap(),
    )
    .await;

    assert!(
        matches!(
            events.last(),
            Some(LoopEvent::RoundEnded {
                stop: RoundStop::Finished(FinishReason::Stop),
                ..
            })
        ),
        "裁剪不影响正常收尾"
    );
    let first = &fake.requests()[0];
    assert_eq!(first.messages.len(), 1, "超窗的旧交换被整块裁掉");
    assert_eq!(first.messages[0].text(), "q1", "当前轮保留");
    // 会话历史本身不动：裁剪只作用于请求（A7：历史是事实，裁剪是视图）
    assert_eq!(chat.messages.len(), 4);
}

// —— M6：并行执行（A5 §2）——

/// 延时工具：暂停时钟下做确定性时序断言。
struct DelayTool {
    name: &'static str,
    delay: std::time::Duration,
    output: ToolOutput,
}

impl Tool for DelayTool {
    fn def(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name.into(),
            description: "delay test tool".into(),
            parameters: json!({"type": "object"}),
        }
    }

    fn run(&self, _arguments: Value) -> futures::future::BoxFuture<'static, ToolOutput> {
        let output = self.output.clone();
        let delay = self.delay;
        Box::pin(async move {
            tokio::time::sleep(delay).await;
            output
        })
    }
}

/// 永不完成的工具：取消路径专用（无计时器，避免暂停时钟自动推进的竞态）。
struct Hang;

impl Tool for Hang {
    fn def(&self) -> ToolDefinition {
        ToolDefinition {
            name: "hang".into(),
            description: "hangs forever".into(),
            parameters: json!({"type": "object"}),
        }
    }

    fn run(&self, _arguments: Value) -> futures::future::BoxFuture<'static, ToolOutput> {
        Box::pin(futures::future::pending())
    }
}

#[tokio::test(start_paused = true)]
async fn tools_run_in_parallel_and_results_keep_call_order() {
    let provider = FakeProvider {
        id: "fake",
        turns: Arc::new(Mutex::new(
            vec![
                ScriptedTurn::tool_calls(
                    vec![
                        tool_call("c1", "slow", json!({})),
                        tool_call("c2", "fast", json!({})),
                    ],
                    (5, 2),
                ),
                ScriptedTurn::text(&["两件都办妥"], (6, 3)),
            ]
            .into(),
        )),
        requests: Arc::new(Mutex::new(Vec::new())),
        context_window: 1_000_000,
    };
    let tools = ToolSet::new()
        .with(Arc::new(DelayTool {
            name: "slow",
            delay: std::time::Duration::from_millis(100),
            output: Ok(json!({"n": 1})),
        }))
        .with(Arc::new(DelayTool {
            name: "fast",
            delay: std::time::Duration::from_millis(50),
            output: Ok(json!({"n": 2})),
        }));
    let service = ChatService::new(Arc::new(provider.clone()), tools);
    let mut chat = Chat::new("fake-model");

    let start = tokio::time::Instant::now();
    let events = collect(
        service
            .run_round(&mut chat, "并行", Default::default())
            .unwrap(),
    )
    .await;
    let elapsed = start.elapsed();

    assert_eq!(
        elapsed,
        std::time::Duration::from_millis(100),
        "暂停时钟下两者都完成于 max(100, 50)，而非顺序执行的 sum(150)"
    );
    let starts: Vec<String> = events
        .iter()
        .filter_map(|e| match e {
            LoopEvent::ToolStarted { id, .. } => Some(id.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(starts, vec!["c1", "c2"], "Started 按调用序全发");
    let finishes: Vec<String> = events
        .iter()
        .filter_map(|e| match e {
            LoopEvent::ToolFinished { id, .. } => Some(id.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(finishes, vec!["c2", "c1"], "Finished 按完成序（50ms 先回）");
    // 历史回填按**调用顺序**（结果序 = 调用序，P2：Anthropic 合并 tool_result 块）
    let second = &provider.requests()[1];
    assert_eq!(second.messages[2].tool_call_id.as_deref(), Some("c1"));
    assert_eq!(second.messages[2].text(), r#"{"n":1}"#);
    assert_eq!(second.messages[3].tool_call_id.as_deref(), Some("c2"));
    assert_eq!(second.messages[3].text(), r#"{"n":2}"#);
    assert_eq!(user_text(&chat), "两件都办妥");
}

#[tokio::test]
async fn cancellation_during_tools_discards_all_results() {
    let provider = FakeProvider {
        id: "fake",
        turns: Arc::new(Mutex::new(
            vec![ScriptedTurn::tool_calls(
                vec![
                    tool_call("c1", "hang", json!({})),
                    tool_call("c2", "boom", json!({})),
                ],
                (5, 2),
            )]
            .into(),
        )),
        requests: Arc::new(Mutex::new(Vec::new())),
        context_window: 1_000_000,
    };
    let tools = ToolSet::new()
        .with(Arc::new(Hang))
        .with(Arc::new(StaticTool {
            name: "boom",
            output: Err("x".into()),
        }));
    let service = ChatService::new(Arc::new(provider.clone()), tools);
    let mut chat = Chat::new("fake-model");
    let cancel = tokio_util::sync::CancellationToken::new();

    let mut cancelled = false;
    {
        let round = service.run_round(&mut chat, "q", cancel.clone()).unwrap();
        tokio::pin!(round);
        while let Some(event) = round.next().await {
            match event {
                LoopEvent::ToolStarted { id, .. } if id == "c1" => cancel.cancel(),
                LoopEvent::Cancelled => {
                    cancelled = true;
                    break;
                }
                LoopEvent::ToolFinished { id, .. } if id == "c1" => {
                    panic!("hang 永不完成，不应有 c1 的 Finished")
                }
                _ => {}
            }
        }
    }
    assert!(cancelled, "工具执行中取消应立即收轮");
    assert_eq!(
        chat.messages.len(),
        2,
        "user + assistant(tool_calls)——assistant 轮已提交，但没有任何工具结果回填"
    );
    assert!(
        !chat
            .messages
            .iter()
            .any(|m| m.role == agent_common::Role::Tool),
        "已执行完的结果也不回填（A5 取消语义）"
    );
}
