//! 传输层「一次调用」的端到端装配测试（断网可测，`G-3`）。
//!
//! 覆盖 `Q1` §3 里**属于传输层**的用例：
//! - 用例 2：流式文本的事件顺序与拼接；
//! - 用例 5：截断（EOF 无终止记录）不得静默当成功；
//! - 用例 6：坏帧跳过、流继续；
//! - 用例 11：无 usage 时仍补一次 `Usage::default()`。
//!
//! 其余用例（1 / 3 / 4 / 7–10 / 12）属适配器，见 `M3`。
//!
//! 管线与适配器将要使用的**完全一致**：
//! `bytes → guard_idle → parse_sse → finalize`（最外层可再套 `guard` 管取消）。
//!
//! `TestPolicy` 是**最小测试替身，不是适配器**：真正的厂商字段映射在
//! `agent-providers`（`A2` §2）。它只认识 OpenAI 兼容的 `chat.completions` 形状。

use agent_common::{DeltaKind, FinishReason, StreamEvent, Usage};
use agent_transport::{
    Frame, FramePolicy, HttpClient, HttpConfig, SseEvent, TransportError, finalize, guard,
    parse_sse,
};
use bytes::Bytes;
use futures::{StreamExt, stream};
use serde_json::Value;
use tokio_util::sync::CancellationToken;

const TEXT_BASIC: &[u8] = include_bytes!("fixtures/text_basic.sse");
const TRUNCATED: &[u8] = include_bytes!("fixtures/truncated_no_terminal.sse");
const BAD_FRAME: &[u8] = include_bytes!("fixtures/bad_frame_in_middle.sse");
const NO_USAGE: &[u8] = include_bytes!("fixtures/no_usage.sse");

/// 最小测试判决（**不是适配器**）。
struct TestPolicy {
    started: bool,
    usage_seen: bool,
    finish_reason: FinishReason,
}

impl TestPolicy {
    fn new() -> Self {
        Self {
            started: false,
            usage_seen: false,
            finish_reason: FinishReason::Other,
        }
    }

    /// 补齐「`End` 之前必须有一次 `Usage`」（`A2` §4）——供应商没给就补默认值。
    fn usage_if_missing(&mut self, out: &mut Vec<StreamEvent>) {
        if !self.usage_seen {
            self.usage_seen = true;
            out.push(StreamEvent::Usage {
                usage: Usage::default(),
            });
        }
    }
}

impl FramePolicy<SseEvent> for TestPolicy {
    type Output = StreamEvent;

    fn classify(&mut self, frame: SseEvent) -> Frame<StreamEvent> {
        if frame.is_done() {
            let mut trailing = Vec::new();
            self.usage_if_missing(&mut trailing);
            trailing.push(StreamEvent::End {
                finish_reason: self.finish_reason,
            });
            return Frame::Terminal(trailing);
        }

        let Ok(value) = serde_json::from_str::<Value>(&frame.data) else {
            // Q1 用例 6：坏帧 → 跳过该帧，不打断整条流。
            return Frame::Skip;
        };

        let mut events = Vec::new();
        if !self.started {
            self.started = true;
            events.push(StreamEvent::Start {
                response_id: value
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                model: value
                    .get("model")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        let choice = &value["choices"][0];
        if let Some(text) = choice["delta"]["content"].as_str()
            && !text.is_empty()
        {
            events.push(StreamEvent::Delta {
                kind: DeltaKind::Text,
                text: text.to_string(),
            });
        }
        if let Some(usage) = value.get("usage").filter(|usage| !usage.is_null()) {
            self.usage_seen = true;
            events.push(StreamEvent::Usage {
                usage: Usage {
                    input_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                    output_tokens: usage["completion_tokens"].as_u64().unwrap_or(0) as u32,
                },
            });
        }
        if let Some(reason) = choice["finish_reason"].as_str() {
            self.finish_reason = match reason {
                "stop" => FinishReason::Stop,
                "length" => FinishReason::Length,
                "tool_calls" => FinishReason::ToolCalls,
                "content_filter" => FinishReason::ContentFilter,
                _ => FinishReason::Other,
            };
        }

        Frame::many(events)
    }

    fn on_truncated(&mut self) -> Vec<StreamEvent> {
        let mut trailing = Vec::new();
        // 截断也要先把该给的 Usage 给掉（Q1 用例 11 + 用例 5 同时成立）。
        self.usage_if_missing(&mut trailing);
        trailing.push(StreamEvent::End {
            finish_reason: FinishReason::Truncated,
        });
        trailing
    }
}

/// 把原始字节按固定大小切块，模拟真实网络分片（含把终止符切在中间）。
///
/// 包成 `Result` 是因为真实管线里这一步由
/// `bytes_stream().map(TransportError::from)` 完成（`guard_idle` 要求
/// 已归一化的流）。
fn chunked(raw: &'static [u8], size: usize) -> Vec<Result<Bytes, TransportError>> {
    raw.chunks(size)
        .map(|chunk| Ok(Bytes::copy_from_slice(chunk)))
        .collect()
}

/// 跑一遍真实管线：`bytes → guard_idle → parse_sse → finalize`。
async fn run(raw: &'static [u8], chunk_size: usize) -> Vec<Result<StreamEvent, TransportError>> {
    let client = HttpClient::new(HttpConfig::default()).expect("client builds");
    let bytes = client.guard_idle(stream::iter(chunked(raw, chunk_size)));
    finalize(parse_sse(bytes), TestPolicy::new())
        .collect()
        .await
}

/// 同上，但要求全流无传输错误（本文件里的 fixtures 都不该出错）。
async fn run_ok(raw: &'static [u8], chunk_size: usize) -> Vec<StreamEvent> {
    run(raw, chunk_size)
        .await
        .into_iter()
        .map(|item| item.expect("fixtures 不应产生传输错误"))
        .collect()
}

/// 拼接全部文本增量。
fn text_of(events: &[StreamEvent]) -> String {
    events
        .iter()
        .filter_map(|event| match event {
            StreamEvent::Delta {
                kind: DeltaKind::Text,
                text,
            } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

/// 契约：`End` 之前必须有一次 `Usage`（`A2` §4）。
fn assert_usage_before_end(events: &[StreamEvent]) {
    let usage = events
        .iter()
        .position(|event| matches!(event, StreamEvent::Usage { .. }))
        .expect("End 之前必须有一次 Usage");
    let end = events
        .iter()
        .position(|event| matches!(event, StreamEvent::End { .. }))
        .expect("必须有 End");
    assert!(usage < end, "Usage(#{usage}) 必须在 End(#{end}) 之前");
}

#[tokio::test]
async fn q1_case_2_streaming_text_order_and_concatenation() {
    let expected = vec![
        StreamEvent::Start {
            response_id: "chatcmpl-t1".into(),
            model: "test-model".into(),
        },
        StreamEvent::Delta {
            kind: DeltaKind::Text,
            text: "Hello".into(),
        },
        StreamEvent::Delta {
            kind: DeltaKind::Text,
            text: ", world".into(),
        },
        StreamEvent::Usage {
            usage: Usage {
                input_tokens: 9,
                output_tokens: 3,
            },
        },
        StreamEvent::End {
            finish_reason: FinishReason::Stop,
        },
    ];

    // 逐字节切到整块喂：事件序列必须**完全一致**（分片边界不该影响语义）。
    for size in [1, 3, 7, 64, TEXT_BASIC.len()] {
        let events = run_ok(TEXT_BASIC, size).await;
        assert_eq!(events, expected, "chunk size = {size}");
        assert_eq!(text_of(&events), "Hello, world");
        assert_usage_before_end(&events);
    }
}

#[tokio::test]
async fn q1_case_5_truncated_is_not_silent_success() {
    let events = run_ok(TRUNCATED, 11).await;

    assert_eq!(text_of(&events), "Hello, wor", "截断前的增量仍要送达");
    assert!(
        events.iter().any(|event| matches!(
            event,
            StreamEvent::End {
                finish_reason: FinishReason::Truncated
            }
        )),
        "截断必须被发现，实际 {events:?}"
    );
    assert!(
        !events.iter().any(|event| matches!(
            event,
            StreamEvent::End {
                finish_reason: FinishReason::Stop
            }
        )),
        "截断不得被当成正常结束"
    );
    assert_usage_before_end(&events);
}

#[tokio::test]
async fn q1_case_6_bad_frame_is_skipped_and_stream_continues() {
    let events = run_ok(BAD_FRAME, 5).await;

    // 中间那条非法 JSON 被跳过，前后两条增量照常拼接，流仍正常收尾。
    assert_eq!(text_of(&events), "ac");
    assert_eq!(
        events.last(),
        Some(&StreamEvent::End {
            finish_reason: FinishReason::Stop
        })
    );
    assert_usage_before_end(&events);
}

#[tokio::test]
async fn q1_case_11_missing_usage_is_synthesized_before_end() {
    let events = run_ok(NO_USAGE, 9).await;

    assert_eq!(text_of(&events), "Hi");
    assert_usage_before_end(&events);

    let usage = events
        .iter()
        .find(|event| matches!(event, StreamEvent::Usage { .. }))
        .expect("必须补一次 Usage");
    assert_eq!(
        usage,
        &StreamEvent::Usage {
            usage: Usage::default()
        },
        "供应商没给 usage 时补默认值"
    );
    assert_eq!(
        events.last(),
        Some(&StreamEvent::End {
            finish_reason: FinishReason::Length
        })
    );
}

#[tokio::test]
async fn cancel_mid_stream_stops_the_pipeline() {
    let token = CancellationToken::new();
    let client = HttpClient::new(HttpConfig::default()).expect("client builds");
    let bytes = client.guard_idle(stream::iter(chunked(TEXT_BASIC, 7)));
    let mut stream = guard(finalize(parse_sse(bytes), TestPolicy::new()), token.clone());

    assert!(matches!(
        stream.next().await,
        Some(Ok(StreamEvent::Start { .. }))
    ));

    token.cancel();

    assert!(matches!(
        stream.next().await,
        Some(Err(TransportError::Cancelled))
    ));
    assert!(stream.next().await.is_none(), "取消之后必须结束");
}
