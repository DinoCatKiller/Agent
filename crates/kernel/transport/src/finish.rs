//! 收尾语义（**机制**，对应 `A2` §4）。
//!
//! 本模块把「SSE 帧流」管成一条**有终局概念**的流，强制三分语义：
//!
//! 1. **传输错误** → 上抛 [`TransportError`] 后立即停止，之后不再产出任何事件；
//! 2. **可恢复坏帧** → [`Frame::Skip`]：跳过该帧、流继续，不打断整条流；
//! 3. **EOF 无终止记录（截断）** → 调 [`FramePolicy::on_truncated`] 补一个信号，
//!    **绝不允许静默当成功**。
//!
//! ## 为什么是「策略」而不是写死
//!
//! 「哪一帧算终止记录」（OpenAI 的 `[DONE]`、Anthropic 的 `message_stop`、
//! OpenAI Responses 的 `response.completed`）、「坏帧长什么样」（`data` 不是合法
//! JSON）、「截断该补 `End { Truncated }` 还是 `Error`」——全是**供应商语义**，
//! 按 `D6` §1 只能落在 `kernel/providers` 的适配器里。
//!
//! 因此传输层只提供**控制流机制**：什么时候转发、什么时候跳过、什么时候收尾，
//! 由适配器实现的 [`FramePolicy`] 决定「判决」，本模块负责「执行判决」。
//!
//! ## 典型装配
//!
//! ```text
//! bytes_stream → guard_idle → parse_sse → finalize(适配器的 FramePolicy)
//! ```
//!
//! 不想分两步时可直接用 [`finalize_bytes`]（等价于 `parse_sse` + [`finalize`]）。
//!
//! ## 与 SSE 解耦
//!
//! 输入帧类型 `F` 是**泛型**的（见 [`FramePolicy`] / [`FinalizedStream`]）：机制本身
//! 不知道「帧」是 SSE、NDJSON 还是 WebSocket 消息，也不认识任何厂商语义。
//! 本文件里唯一提到 SSE 的地方是便捷装配函数 [`finalize_bytes`]——它只是把
//! `parse_sse` 与 [`finalize`] 串起来，让适配器少写一行；不用它同样成立。

use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures::Stream;

use crate::http::TransportError;
use crate::sse::{SseEvent, SseStream, parse_sse};

/// 单帧的判决结果。由适配器在 [`FramePolicy::classify`] 里给出。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame<T> {
    /// 正常帧：产出 0..n 个下游事件，流继续。
    Emit(Vec<T>),
    /// 可恢复坏帧：跳过该帧，**不打断**整条流（`A2` §4-2）。
    ///
    /// 行为等价于 `Emit(vec![])`，但语义显式——便于观测与排查。
    Skip,
    /// 终止记录：产出 0..n 个末帧后，流**正常结束**，不再向上游取数据。
    ///
    /// 为什么是**多个**：契约要求 `End` 之前必须有一次 `Usage`（`A2` §4）。供应商
    /// 可能把 usage 与终止原因放在同一条终止记录里（如 OpenAI Responses 的
    /// `response.completed`）；供应商不给 usage 时，适配器也要在此补一次
    /// `Usage::default()`（`Q1` 用例 11）。一帧只能产出一个事件的话，这两件事
    /// 无法同时满足。
    Terminal(Vec<T>),
}

impl<T> Frame<T> {
    /// 最常见情形：一帧 → 一个事件。
    pub fn one(event: T) -> Self {
        Self::Emit(vec![event])
    }

    /// 一帧 → 多个事件（如 Anthropic `message_start` 一次给出 `Start` 与 `Usage`）。
    pub fn many(events: impl IntoIterator<Item = T>) -> Self {
        Self::Emit(events.into_iter().collect())
    }

    /// 不需要额外末帧的终止记录。
    pub fn end() -> Self {
        Self::Terminal(Vec::new())
    }
}

/// 收尾判决：由适配器实现，把「一帧」翻译成「归一化事件 + 终局判定」。
///
/// `F` 是**输入帧类型**（典型为 [`SseEvent`]，但机制不限于 SSE——见模块文档）。
/// 实现者**不得**在此读网络或做 IO：它只是纯函数式的翻译 + 判定。
pub trait FramePolicy<F> {
    /// 下游看到的事件类型（典型为 `agent_common::StreamEvent`）。
    type Output;

    /// 判定并翻译一帧，产出 0..n 个事件。
    fn classify(&mut self, frame: F) -> Frame<Self::Output>;

    /// 流在**未见终止记录**的情况下 EOF 时调用的兜底，产出 0..n 个事件。
    ///
    /// 返回的事件里**必须**包含表达「**截断**」的那一个（例如
    /// `End { finish_reason: Truncated }` 或 `Error { category: Truncated, .. }`），
    /// **不得**只给出正常结束语义——这正是 `A2` §4-3「禁止静默当成功」的落点。
    fn on_truncated(&mut self) -> Vec<Self::Output>;
}

/// 给帧流套上三分收尾语义的适配器，由 [`finalize`] / [`finalize_bytes`] 构造。
///
/// - `S`：上游帧流 ｜ `F`：帧类型（与 SSE 无关，见模块文档） ｜ `P`：判决策略
pub struct FinalizedStream<S, F, P: FramePolicy<F>> {
    inner: S,
    policy: P,
    /// 已判决、待下游取走的事件。
    ready: VecDeque<P::Output>,
    /// 是否已见到终止记录。
    terminal_seen: bool,
    /// 本流是否已终结（错误 / 终止 / 截断兜底之后）；此后一律 `None`。
    done: bool,
}

impl<S, F, P: FramePolicy<F>> FinalizedStream<S, F, P> {
    /// 用 `policy` 包装帧流 `inner`。
    pub fn new(inner: S, policy: P) -> Self {
        Self {
            inner,
            policy,
            ready: VecDeque::new(),
            terminal_seen: false,
            done: false,
        }
    }

    /// 已判决的判决对象（供适配器在收尾后读取累积状态，如拼接中的工具参数）。
    pub fn policy(&self) -> &P {
        &self.policy
    }

    /// 已判决的判决对象（可变）。
    pub fn policy_mut(&mut self) -> &mut P {
        &mut self.policy
    }

    /// 是否已见过终止记录（收尾后可用于区分「正常结束」与「截断」）。
    pub fn terminal_seen(&self) -> bool {
        self.terminal_seen
    }
}

impl<S, F, P> Stream for FinalizedStream<S, F, P>
where
    S: Stream<Item = Result<F, TransportError>> + Unpin,
    P: FramePolicy<F> + Unpin,
    P::Output: Unpin,
{
    type Item = Result<P::Output, TransportError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            // 先把已判决的事件吐干净。
            if let Some(output) = this.ready.pop_front() {
                return Poll::Ready(Some(Ok(output)));
            }
            if this.done {
                return Poll::Ready(None);
            }

            match Pin::new(&mut this.inner).poll_next(cx) {
                Poll::Ready(Some(Ok(frame))) => match this.policy.classify(frame) {
                    Frame::Emit(events) => this.ready.extend(events),
                    Frame::Skip => {
                        // 可恢复坏帧：跳过继续（A2 §4-2）。
                        tracing::debug!(target: "agent_transport", "跳过可恢复坏帧");
                    }
                    Frame::Terminal(events) => {
                        // 终止记录：正常收尾，此后不再向上游取数据（A2 §4）。
                        this.terminal_seen = true;
                        this.done = true;
                        this.ready.extend(events);
                    }
                },
                Poll::Ready(Some(Err(err))) => {
                    // 传输错误：上抛后停止（A2 §4-1），且**不补**截断信号。
                    this.done = true;
                    return Poll::Ready(Some(Err(err)));
                }
                Poll::Ready(None) => {
                    // EOF 且未见终止记录 → 截断兜底（A2 §4-3）。
                    this.done = true;
                    if !this.terminal_seen {
                        let events = this.policy.on_truncated();
                        this.ready.extend(events);
                    }
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

/// 把**任意帧流**交给 `policy` 做三分收尾（见模块文档）。
///
/// 帧类型 `F` 由调用方决定：机制不假设它是 SSE。
pub fn finalize<S, F, P>(stream: S, policy: P) -> FinalizedStream<S, F, P>
where
    S: Stream<Item = Result<F, TransportError>> + Unpin,
    P: FramePolicy<F> + Unpin,
{
    FinalizedStream::new(stream, policy)
}

/// **本模块唯一提到 SSE 的地方**：SSE 装配便捷入口（`parse_sse` + [`finalize`]）。
///
/// 只做「解析 + 收尾」；空闲超时由 [`crate::http::HttpClient::guard_idle`] 在更上游套好。
/// 不使用本函数时，直接写 `finalize(parse_sse(bytes), policy)` 完全等价。
pub fn finalize_bytes<S, P>(bytes: S, policy: P) -> FinalizedStream<SseStream<S>, SseEvent, P>
where
    S: Stream<Item = Result<Bytes, TransportError>> + Unpin,
    P: FramePolicy<SseEvent> + Unpin,
{
    finalize(parse_sse(bytes), policy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{StreamExt, stream};

    /// 测试输出：能区分普通帧与「截断兜底」。
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Out {
        Item(String),
        End,
        Truncated,
    }

    /// 测试判决：`done` / `stop` 是终止记录，`bad` 是可恢复坏帧，其余是普通帧。
    struct FakePolicy;

    impl FramePolicy<SseEvent> for FakePolicy {
        type Output = Out;

        fn classify(&mut self, frame: SseEvent) -> Frame<Out> {
            match frame.data.trim() {
                // 带末帧的终止记录。
                "done" => Frame::Terminal(vec![Out::End]),
                // 不带末帧的终止记录。
                "stop" => Frame::end(),
                // 可恢复坏帧：跳过继续。
                "bad" => Frame::Skip,
                other => Frame::one(Out::Item(other.to_string())),
            }
        }

        fn on_truncated(&mut self) -> Vec<Out> {
            vec![Out::Truncated]
        }
    }

    /// 构造一个正常 SSE 帧。
    fn frame(data: &str) -> Result<SseEvent, TransportError> {
        Ok(SseEvent {
            data: data.to_string(),
            ..SseEvent::default()
        })
    }

    /// 跑帧流，把错误字符串化以便断言。
    async fn run(frames: Vec<Result<SseEvent, TransportError>>) -> Vec<Result<Out, String>> {
        finalize(stream::iter(frames), FakePolicy)
            .map(|item| item.map_err(|e| e.to_string()))
            .collect()
            .await
    }

    /// 跑完整字节管线（parse_sse → finalize）。
    async fn run_bytes(chunks: Vec<&'static [u8]>) -> Vec<Result<Out, String>> {
        let bytes: Vec<Result<Bytes, TransportError>> = chunks
            .into_iter()
            .map(|c| Ok(Bytes::from_static(c)))
            .collect();
        finalize_bytes(stream::iter(bytes), FakePolicy)
            .map(|item| item.map_err(|e| e.to_string()))
            .collect()
            .await
    }

    fn item(text: &str) -> Result<Out, String> {
        Ok(Out::Item(text.to_string()))
    }

    #[tokio::test]
    async fn forwards_frames_until_terminal_then_ends() {
        let got = run(vec![frame("a"), frame("b"), frame("done")]).await;
        assert_eq!(got, vec![item("a"), item("b"), Ok(Out::End)]);
    }

    #[tokio::test]
    async fn skips_recoverable_bad_frame_and_continues() {
        // 坏帧夹在中间：跳过它，流照常走完。
        let got = run(vec![frame("a"), frame("bad"), frame("b"), frame("done")]).await;
        assert_eq!(got, vec![item("a"), item("b"), Ok(Out::End)]);
    }

    #[tokio::test]
    async fn eof_without_terminal_is_reported_as_truncated() {
        // 截断：不得静默当成功（A2 §4-3）。
        let got = run(vec![frame("a"), frame("b")]).await;
        assert_eq!(got, vec![item("a"), item("b"), Ok(Out::Truncated)]);
    }

    #[tokio::test]
    async fn empty_stream_is_truncated_not_silent_success() {
        // 零字节响应也必须被判为截断，而不是「干净结束」。
        let got = run(Vec::new()).await;
        assert_eq!(got, vec![Ok(Out::Truncated)]);
    }

    #[tokio::test]
    async fn transport_error_stops_stream_without_truncation_signal() {
        let got = run(vec![
            frame("a"),
            Err(TransportError::Network("boom".into())),
            frame("done"),
        ])
        .await;

        assert_eq!(got.len(), 2, "错误之后必须停止，不再转发任何帧");
        assert_eq!(got[0], item("a"));
        assert!(
            matches!(&got[1], Err(message) if message.contains("boom")),
            "第二个必须是传输错误，实际 {:?}",
            got[1]
        );
    }

    #[tokio::test]
    async fn frames_after_terminal_are_ignored() {
        let got = run(vec![frame("a"), frame("done"), frame("late")]).await;
        assert_eq!(got, vec![item("a"), Ok(Out::End)]);
    }

    #[tokio::test]
    async fn terminal_without_payload_ends_cleanly() {
        let got = run(vec![frame("stop")]).await;
        assert!(got.is_empty(), "无载荷终止记录不应产出事件，也不应判截断");
    }

    #[tokio::test]
    async fn bytes_pipeline_reports_truncation() {
        // 真实字节：有正常帧但没有终止记录 → 截断。
        let got = run_bytes(vec![b"data: a\n\n"]).await;
        assert_eq!(got, vec![item("a"), Ok(Out::Truncated)]);
    }

    #[tokio::test]
    async fn bytes_pipeline_skips_bad_frame_and_finishes_normally() {
        let got = run_bytes(vec![b"data: a\n\ndata: bad\n\ndata: done\n\n"]).await;
        assert_eq!(got, vec![item("a"), Ok(Out::End)]);
    }

    #[tokio::test]
    async fn bytes_pipeline_reassembles_split_terminator() {
        // 终止记录被切在 chunk 边界，仍必须被识别（不误判为截断）。
        let got = run_bytes(vec![b"data: a\n\ndata: do", b"ne\n\n"]).await;
        assert_eq!(got, vec![item("a"), Ok(Out::End)]);
    }

    /// 与 SSE 无关的帧类型：`0` 是终止记录。
    struct NumberPolicy;

    impl FramePolicy<u32> for NumberPolicy {
        type Output = u32;

        fn classify(&mut self, frame: u32) -> Frame<u32> {
            if frame == 0 {
                Frame::end()
            } else {
                Frame::one(frame)
            }
        }

        fn on_truncated(&mut self) -> Vec<u32> {
            // 用最大值代表「截断兜底」。
            vec![u32::MAX]
        }
    }

    async fn run_numbers(frames: Vec<Result<u32, TransportError>>) -> Vec<u32> {
        finalize(stream::iter(frames), NumberPolicy)
            .map(|item| item.expect("no error"))
            .collect()
            .await
    }

    #[tokio::test]
    async fn mechanism_is_frame_type_agnostic() {
        // 收尾机制不依赖 SSE：换成 u32 帧，三分语义照样成立。
        // 无终止记录 → 补兜底。
        assert_eq!(run_numbers(vec![Ok(1), Ok(2)]).await, vec![1, 2, u32::MAX]);
        // 见终止记录 → 正常结束，且其后的帧被丢弃。
        assert_eq!(run_numbers(vec![Ok(1), Ok(0), Ok(9)]).await, vec![1]);
    }

    /// 一帧可以产出**多个**事件，终止记录也可以一次带出多个末帧。
    struct PairPolicy;

    impl FramePolicy<SseEvent> for PairPolicy {
        type Output = Out;

        fn classify(&mut self, frame: SseEvent) -> Frame<Out> {
            match frame.data.trim() {
                // 终止记录：一次补齐「Usage + End」（Q1 用例 11）。
                "done" => Frame::Terminal(vec![Out::Item("usage".into()), Out::End]),
                other => Frame::many([Out::Item(other.to_string()), Out::Item("extra".into())]),
            }
        }

        fn on_truncated(&mut self) -> Vec<Out> {
            // 截断兜底同样可以补齐「Usage + 截断标记」。
            vec![Out::Item("usage".into()), Out::Truncated]
        }
    }

    #[tokio::test]
    async fn frames_can_emit_multiple_events_including_at_termination() {
        let got = finalize(stream::iter(vec![frame("a"), frame("done")]), PairPolicy)
            .map(|item| item.map_err(|e| e.to_string()))
            .collect::<Vec<_>>()
            .await;
        assert_eq!(
            got,
            vec![item("a"), item("extra"), item("usage"), Ok(Out::End)]
        );

        // 截断时也能一次补齐，而不是二选一。
        let got = finalize(stream::iter(vec![frame("a")]), PairPolicy)
            .map(|item| item.map_err(|e| e.to_string()))
            .collect::<Vec<_>>()
            .await;
        assert_eq!(
            got,
            vec![item("a"), item("extra"), item("usage"), Ok(Out::Truncated)]
        );
    }
}
