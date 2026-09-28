//! SSE（Server-Sent Events）帧解析（**机制**，对应 `A4` / `X1` §2）。
//!
//! 本模块只做「字节 → 通用 SSE 事件」，**不认识任何厂商语义**：
//! OpenAI 的 `data: [DONE]`、Anthropic 的 `event: message_stop` 等，一律由
//! `kernel/providers` 的适配器在本模块产出的 [`SseEvent`] 之上解释（`D6` §1/§4）。
//!
//! ## 选型说明（`A4`）
//!
//! 评估过 `eventsource-stream` 与 `sse-rs`，最终**自研**：
//! - `eventsource-stream` 面向浏览器 `EventSource`（自带自动重连语义），与本层需要的
//!   「纯解析 + 我们自己的收尾/取消」不贴合；
//! - `sse-rs`（`sse-core`）零 I/O 状态机方向契合，但首个版本很新，仍要额外包装；
//! - 自研约 200 行即可覆盖，且能**完全离线**、用原始字节做单测（`Q1` / G-3）。
//!
//! ## 收尾语义（与 `A2` §4 衔接）
//!
//! 本层保证：**EOF 处未以空行收尾的不完整事件不会被派发**（见 [`SseDecoder::finish`]），
//! 截断因此不会被误当成功——具体的 `End { Truncated }` / `Error` 由上层（第 4 步）决定。

use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures::Stream;

use crate::http::TransportError;

/// UTF-8 BOM。
const BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// 解析 `retry:` 字段：仅接受纯数字（SSE 规范行为）。
fn parse_retry(value: &str) -> Option<u64> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

/// 一条已解析的 SSE 事件（通用字段，不含厂商语义）。
///
/// 字段对应 SSE 规范：`event` / `data` / `id` / `retry`。
/// 多行 `data:` 按规范以 `\n` 连接后放入 [`SseEvent::data`]。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SseEvent {
    /// 命名事件类型（如 Anthropic 的 `message_start`，或 OpenAI Responses 的
    /// `response.completed`）。未出现 `event:` 字段时为 `None`（data-only 流）。
    pub event: Option<String>,
    /// 数据负载。多个 `data:` 行以 `\n` 连接。
    pub data: String,
    /// 最近一次 `id:`（忽略含 `\0` 的非法值）。
    pub id: Option<String>,
    /// 最近一次 `retry:`，单位毫秒。
    pub retry: Option<u64>,
}

impl SseEvent {
    /// OpenAI / DeepSeek Chat 流以 `data: [DONE]` 表示正常结束。
    pub fn is_done(&self) -> bool {
        self.data.trim() == "[DONE]"
    }
}

/// 增量 SSE 解码器：喂入原始字节，吐出**完整**事件。
///
/// 在**字节层**切行（先找 `\n` / `\r` 终止符，再整行按 UTF-8 解码），
/// 因此天然免疫「多字节字符被切在 chunk 边界」的问题。
#[derive(Debug, Default)]
pub struct SseDecoder {
    /// 尚未成行的残留字节。
    buf: Vec<u8>,
    /// 当前事件的 `event:` 字段。
    event: Option<String>,
    /// 当前事件的 data 缓冲（多行以 `\n` 连接）。
    data: String,
    /// 最近一次 `id:`（跨事件保留，符合规范语义）。
    id: Option<String>,
    /// 最近一次 `retry:`（跨事件保留）。
    retry: Option<u64>,
    /// 是否已判定过起始 BOM。
    started: bool,
}

impl SseDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// 喂入一块字节，把新解析出的完整事件追加到 `out`。
    pub fn feed(&mut self, chunk: &[u8], out: &mut Vec<SseEvent>) {
        self.buf.extend_from_slice(chunk);
        self.strip_bom();
        self.drain_lines(out);
    }

    /// 流结束。处理末尾残行，但**不派发**未以空行收尾的不完整事件。
    ///
    /// 依据 SSE 规范：文件在事件中间结束时，该不完整事件应被丢弃
    /// （这正是 `A2` §4 的「截断不得当成功」在本层的落点）。
    pub fn finish(&mut self, out: &mut Vec<SseEvent>) {
        // 残留恰好是一个孤立 `\r`：它是完整的行终止符，等价于一个空行 → 派发。
        if self.buf.len() == 1 && self.buf[0] == b'\r' {
            self.buf.clear();
            self.dispatch(out);
            return;
        }
        // 其它残行按规范处理，但随即丢弃待派发事件。
        if !self.buf.is_empty() {
            let line = std::mem::take(&mut self.buf);
            self.process_line(&line, out);
        }
        self.reset_pending();
    }

    /// 首个 chunk 判定并去掉起始 BOM（BOM 允许跨 chunk 到达）。
    fn strip_bom(&mut self) {
        if self.started {
            return;
        }
        if self.buf.len() >= BOM.len() {
            self.started = true;
            if self.buf.starts_with(&BOM) {
                self.buf.drain(..BOM.len());
            }
        } else if !BOM.starts_with(self.buf.as_slice()) {
            // 已有字节不可能是 BOM 前缀 → 确定无 BOM。
            self.started = true;
        }
    }

    /// 把缓冲里所有已终止的行取出并处理。
    fn drain_lines(&mut self, out: &mut Vec<SseEvent>) {
        loop {
            let Some(pos) = self.buf.iter().position(|&b| b == b'\n' || b == b'\r') else {
                return;
            };
            // 末尾孤立的 `\r` 可能是跨 chunk 的 CRLF 前半，先等后续字节。
            if self.buf[pos] == b'\r' && pos + 1 == self.buf.len() {
                return;
            }
            let consumed = if self.buf[pos] == b'\r' && self.buf.get(pos + 1) == Some(&b'\n') {
                pos + 2
            } else {
                pos + 1
            };
            let raw: Vec<u8> = self.buf.drain(..consumed).collect();
            self.process_line(&raw[..pos], out);
        }
    }

    /// 处理一行（不含终止符）。
    fn process_line(&mut self, line: &[u8], out: &mut Vec<SseEvent>) {
        if line.is_empty() {
            self.dispatch(out);
            return;
        }

        let text = String::from_utf8_lossy(line);
        if text.starts_with(':') {
            // 注释 / keep-alive，直接忽略。
            return;
        }

        let (field, value) = match text.split_once(':') {
            Some((field, rest)) => (field, rest.strip_prefix(' ').unwrap_or(rest)),
            None => (text.as_ref(), ""),
        };

        match field {
            "event" => self.event = Some(value.to_string()),
            "data" => {
                if !self.data.is_empty() {
                    self.data.push('\n');
                }
                self.data.push_str(value);
            }
            "id" => {
                if !value.contains('\0') {
                    self.id = Some(value.to_string());
                }
            }
            "retry" => self.retry = parse_retry(value).or(self.retry),
            // 未知字段按规范忽略。
            _ => {}
        }
    }

    /// 空行触发的派发：仅当 data 非空（符合规范）。
    fn dispatch(&mut self, out: &mut Vec<SseEvent>) {
        if !self.data.is_empty() {
            out.push(SseEvent {
                event: self.event.take(),
                data: std::mem::take(&mut self.data),
                id: self.id.clone(),
                retry: self.retry,
            });
        }
        self.reset_pending();
    }

    /// 丢弃当前事件的待派发状态（保留跨事件保留的 `id` / `retry`）。
    fn reset_pending(&mut self) {
        self.event = None;
        self.data.clear();
    }
}

/// 把字节流适配成 [`SseEvent`] 流。
///
/// 输入流若出错，则**原样上抛**该 [`TransportError`] 并结束（传输错误语义，`A2` §4）。
/// 输入流正常结束（EOF）时按 [`SseDecoder::finish`] 收尾，不补发不完整事件。
pub struct SseStream<S> {
    inner: S,
    decoder: SseDecoder,
    ready: VecDeque<SseEvent>,
    finished: bool,
}

impl<S> SseStream<S>
where
    S: Stream<Item = Result<Bytes, TransportError>> + Unpin,
{
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            decoder: SseDecoder::new(),
            ready: VecDeque::new(),
            finished: false,
        }
    }
}

impl<S> Stream for SseStream<S>
where
    S: Stream<Item = Result<Bytes, TransportError>> + Unpin,
{
    type Item = Result<SseEvent, TransportError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            if let Some(event) = this.ready.pop_front() {
                return Poll::Ready(Some(Ok(event)));
            }
            if this.finished {
                return Poll::Ready(None);
            }
            match Pin::new(&mut this.inner).poll_next(cx) {
                Poll::Ready(Some(Ok(chunk))) => {
                    let mut out = Vec::new();
                    this.decoder.feed(&chunk, &mut out);
                    this.ready.extend(out);
                }
                Poll::Ready(Some(Err(err))) => {
                    this.finished = true;
                    return Poll::Ready(Some(Err(err)));
                }
                Poll::Ready(None) => {
                    this.finished = true;
                    let mut out = Vec::new();
                    this.decoder.finish(&mut out);
                    this.ready.extend(out);
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

/// 便捷入口：`bytes_stream` → [`SseEvent`] 流。
///
/// 典型管线：
/// `resp.bytes_stream().map(TransportError::from)` →（可选）`HttpClient::guard_idle`
/// → `parse_sse`。
pub fn parse_sse<S>(stream: S) -> SseStream<S>
where
    S: Stream<Item = Result<Bytes, TransportError>> + Unpin,
{
    SseStream::new(stream)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 用原始字节解码并收尾，返回全部事件。
    fn decode(chunks: &[&[u8]]) -> Vec<SseEvent> {
        let mut decoder = SseDecoder::new();
        let mut out = Vec::new();
        for chunk in chunks {
            decoder.feed(chunk, &mut out);
        }
        decoder.finish(&mut out);
        out
    }

    #[test]
    fn splits_events_on_blank_line() {
        let events = decode(&[b"data: a\n\ndata: b\n\n"]);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].data, "a");
        assert_eq!(events[1].data, "b");
    }

    #[test]
    fn joins_multiple_data_lines() {
        let events = decode(&[b"data: line1\ndata: line2\n\n"]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "line1\nline2");
    }

    #[test]
    fn supports_crlf_and_lone_cr() {
        assert_eq!(decode(&[b"data: a\r\n\r\n"])[0].data, "a");
        assert_eq!(decode(&[b"data: a\r\r"])[0].data, "a");
    }

    #[test]
    fn ignores_comment_and_keepalive_lines() {
        let events = decode(&[b": keep-alive\n\ndata: x\n\n"]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "x");
    }

    #[test]
    fn parses_event_id_and_retry() {
        let events = decode(&[b"event: ping\nid: 7\nretry: 1000\ndata: {}\n\n"]);
        let event = &events[0];
        assert_eq!(event.event.as_deref(), Some("ping"));
        assert_eq!(event.id.as_deref(), Some("7"));
        assert_eq!(event.retry, Some(1000));
        assert_eq!(event.data, "{}");
    }

    #[test]
    fn strips_leading_bom() {
        let events = decode(&[b"\xEF\xBB\xBFdata: a\n\n"]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "a");
    }

    #[test]
    fn reassembles_lines_split_across_chunks() {
        let events = decode(&[b"da", b"ta: hi\n", b"\n"]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "hi");
    }

    #[test]
    fn reassembles_crlf_split_across_chunks() {
        let events = decode(&[b"data: a\r", b"\n\r\n"]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "a");
    }

    #[test]
    fn multibyte_split_across_chunks_is_decoded_intact() {
        // 「数据」= E6 95 B0 E6 8D AE，切在中间。
        let events = decode(&[
            &[b'd', b'a', b't', b'a', b':', b' ', 0xE6, 0x95],
            &[0xB0, b'\n', b'\n'],
        ]);
        assert_eq!(events[0].data, "数");
    }

    #[test]
    fn incomplete_event_at_eof_is_not_dispatched() {
        // 截断：没有空行收尾 → 不派发（A2 §4 不得当成功）。
        assert!(decode(&[b"data: partial\n"]).is_empty());
        assert!(decode(&[b"data: partial"]).is_empty());
    }

    #[test]
    fn detects_done_sentinel() {
        let events = decode(&[b"data: [DONE]\n\n"]);
        assert_eq!(events.len(), 1);
        assert!(events[0].is_done());
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let events = decode(&[b"foo: bar\ndata: x\n\n"]);
        assert_eq!(events[0].data, "x");
    }

    #[tokio::test]
    async fn stream_adapter_yields_events_then_ends() {
        use futures::StreamExt;

        let chunks: Vec<Result<Bytes, TransportError>> = vec![
            Ok(Bytes::from_static(b"data: hel")),
            Ok(Bytes::from_static(b"lo\n\n")),
        ];
        let mut stream = parse_sse(futures::stream::iter(chunks));

        let mut got = Vec::new();
        while let Some(item) = stream.next().await {
            got.push(item.expect("no error"));
        }
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].data, "hello");
    }

    #[tokio::test]
    async fn stream_adapter_propagates_transport_error_and_stops() {
        use futures::StreamExt;

        let chunks: Vec<Result<Bytes, TransportError>> = vec![
            Ok(Bytes::from_static(b"data: a\n\n")),
            Err(TransportError::Network("boom".into())),
        ];
        let mut stream = parse_sse(futures::stream::iter(chunks));

        assert!(stream.next().await.expect("first").is_ok());
        assert!(matches!(
            stream.next().await.expect("second"),
            Err(TransportError::Network(_))
        ));
        assert!(stream.next().await.is_none(), "error 之后必须结束");
    }
}
