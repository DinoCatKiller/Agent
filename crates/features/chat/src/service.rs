//! 轮次编排：把一次用户输入驱动到自然结束——可能经过多次模型调用与工具执行。
//!
//! 对外是**事件流**（`run_round` 返回 `Stream<LoopEvent>`，流式优先）：CLI、未来 UI、
//! 测试都是消费者。内部两条铁律：
//!
//! - **只在 `End` 后入史**：增量先攒在 [`TurnDraft`] 里，
//!   取消 / 出错时丢弃 draft 即回滚（`README` 的「状态回滚」）；
//! - **工具错误喂回模型**：作为 `tool_result` 文本回传，模型自救；反复失败由
//!   `max_model_turns` 兜底，不做流中重试（重试属 `A6`）。

use std::sync::Arc;

use agent_common::{
    DeltaKind, ErrorCategory, FinishReason, ModelRequest, ProviderError, StreamEvent, Usage,
};
use agent_providers::{CallContext, ErasedProvider, EventStream};
use futures::{Stream, StreamExt};
use tokio_util::sync::CancellationToken;

use crate::chat::{Chat, TurnDraft};
use crate::context::trim_to_fit;
use crate::tools::ToolSet;

/// 一轮对话怎么收场。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundStop {
    /// 模型自然结束（`Stop` / `Length` / `Truncated`…，`A2` §4）。
    Finished(FinishReason),
    /// 达到最大模型轮次（防工具乒乓死循环）。
    MaxTurns(u32),
}

/// 编排层事件：UI 与测试消费的单位。
#[derive(Debug, Clone, PartialEq)]
pub enum LoopEvent {
    /// 模型增量（Text / Thinking 都透传；thinking 只显示不进历史）。
    Delta {
        kind: DeltaKind,
        text: String,
    },
    /// 一次模型调用结束（收到 `End` 并已入史）。
    ModelTurnEnded {
        finish_reason: FinishReason,
        usage: Usage,
    },
    ToolStarted {
        id: String,
        name: String,
    },
    /// `ok = false` 表示执行失败（错误文案已喂回模型）。
    ToolFinished {
        id: String,
        ok: bool,
    },
    /// 整轮结束：`usage` 为本轮各次调用之和（会话累计在 `Chat::total_usage`）。
    RoundEnded {
        stop: RoundStop,
        turns: u32,
        round_usage: Usage,
    },
    /// 用户取消（Ctrl-C）。未提交的增量已回滚，之后不再有任何事件。
    Cancelled,
    /// 终止性错误。之后不再有任何事件（`A2` §4）。
    Error {
        error: ProviderError,
    },
}

/// 循环与裁剪的运行参数。
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// 一轮用户输入内的最大模型调用次数（默认 8）。
    pub max_model_turns: u32,
    /// 裁剪目标：历史估算 ≤ `context_window × keep_ratio`（默认 0.8，给输出留余量）。
    pub context_keep_ratio: f64,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            max_model_turns: 8,
            context_keep_ratio: 0.8,
        }
    }
}

/// 会话编排服务。持有一个供应商与一组工具；状态由 [`Chat`] 外置携带。
pub struct ChatService {
    provider: Arc<dyn ErasedProvider>,
    tools: ToolSet,
    config: ServiceConfig,
}

impl ChatService {
    pub fn new(provider: Arc<dyn ErasedProvider>, tools: ToolSet) -> Self {
        Self {
            provider,
            tools,
            config: ServiceConfig::default(),
        }
    }

    pub fn with_config(mut self, config: ServiceConfig) -> Self {
        self.config = config;
        self
    }

    pub fn provider(&self) -> &Arc<dyn ErasedProvider> {
        &self.provider
    }

    pub fn tools(&self) -> &ToolSet {
        &self.tools
    }

    pub fn config(&self) -> &ServiceConfig {
        &self.config
    }

    /// 发起一轮对话。用户输入先入史，然后循环「模型 →（工具 → 回填 →）模型」直到自然结束。
    ///
    /// 同步阶段（返回 `Err` 即 fast-fail，`A2` §2 的精神）：入史、请求校验、裁剪、
    /// 首次 `provider.stream`（能力协商失败在这里就暴露，而不是等第一次 poll）。
    pub fn run_round<'a>(
        &'a self,
        chat: &'a mut Chat,
        input: impl Into<String>,
        cancel: CancellationToken,
    ) -> Result<impl Stream<Item = LoopEvent> + 'a, ProviderError> {
        chat.push_user(input);
        let (_, first) = self.start_turn(chat, &cancel)?;

        let out = async_stream::stream! {
            let mut events = first;
            let mut turns: u32 = 0;
            let mut round_usage = Usage::default();
            loop {
                turns += 1;
                let mut draft = TurnDraft::default();
                // —— 消费一次模型调用 ——
                loop {
                    tokio::select! {
                        biased;
                        _ = cancel.cancelled() => {
                            // draft 丢弃即回滚：本轮增量不入史。
                            yield LoopEvent::Cancelled;
                            return;
                        }
                        item = events.next() => {
                            let event = match item {
                                Some(event) => event,
                                // 防御：传输层保证 End/Error 二选一后才结束（A4 §2），到这里说明契约被破坏。
                                None => {
                                    yield LoopEvent::Error {
                                        error: ProviderError::new(self.provider.id(), ErrorCategory::Truncated)
                                            .with_message("provider stream ended without End event"),
                                    };
                                    return;
                                }
                            };
                            match &event {
                                StreamEvent::Delta { kind, text } => {
                                    draft.apply(&event);
                                    yield LoopEvent::Delta { kind: *kind, text: text.clone() };
                                }
                                StreamEvent::Error { error } => {
                                    yield LoopEvent::Error { error: error.clone() };
                                    return; // 未提交 → 自动回滚
                                }
                                StreamEvent::End { .. } => {
                                    draft.apply(&event);
                                    break;
                                }
                                _ => draft.apply(&event), // Start / ToolCall / Usage
                            }
                        }
                    }
                }
                let Some(finish_reason) = draft.finish_reason() else {
                    // unreachable：break 只发生在 End 分支；防御性兜底，不当成功。
                    yield LoopEvent::Error {
                        error: ProviderError::new(self.provider.id(), ErrorCategory::Truncated)
                            .with_message("model turn ended without End event"),
                    };
                    return;
                };
                let turn_usage = draft.usage();
                round_usage.input_tokens += turn_usage.input_tokens;
                round_usage.output_tokens += turn_usage.output_tokens;
                let calls: Vec<_> = draft.tool_calls().to_vec();
                chat.commit_turn(draft);
                yield LoopEvent::ModelTurnEnded { finish_reason, usage: turn_usage };

                if finish_reason != FinishReason::ToolCalls {
                    yield LoopEvent::RoundEnded {
                        stop: RoundStop::Finished(finish_reason),
                        turns,
                        round_usage,
                    };
                    return;
                }
                if turns >= self.config.max_model_turns {
                    yield LoopEvent::RoundEnded {
                        stop: RoundStop::MaxTurns(turns),
                        turns,
                        round_usage,
                    };
                    return;
                }

                // —— 顺序执行工具（M6 再并行化，A5）——
                for call in calls {
                    yield LoopEvent::ToolStarted { id: call.id.clone(), name: call.name.clone() };
                    let output = match self.tools.get(&call.name) {
                        Some(tool) => {
                            let running = tool.run(call.arguments.clone());
                            tokio::select! {
                                biased;
                                _ = cancel.cancelled() => {
                                    yield LoopEvent::Cancelled;
                                    return;
                                }
                                output = running => output,
                            }
                        }
                        // 未知工具也喂回：模型看得到错误，可改道或道歉（A5 错误语义）。
                        None => Err(format!("unknown tool: {}", call.name)),
                    };
                    let ok = output.is_ok();
                    let content = match output {
                        Ok(value) => value.to_string(),
                        Err(message) => format!("error: {message}"),
                    };
                    chat.push_tool_result(&call.id, content);
                    yield LoopEvent::ToolFinished { id: call.id.clone(), ok };
                }

                // —— 下一次模型调用 ——
                match self.start_turn(chat, &cancel) {
                    Ok((_, next)) => events = next,
                    Err(error) => {
                        yield LoopEvent::Error { error };
                        return;
                    }
                }
            }
        };
        Ok(out)
    }

    /// 构造请求（校验 + 裁剪）并发起一次流式调用。
    fn start_turn(
        &self,
        chat: &Chat,
        cancel: &CancellationToken,
    ) -> Result<(ModelRequest, EventStream), ProviderError> {
        let mut req = chat.request(&self.tools.definitions())?;
        self.trim_context(&mut req);
        let ctx = CallContext {
            request_id: None,
            cancel: cancel.clone(),
        };
        let stream = self.provider.stream(req.clone(), ctx)?;
        Ok((req, stream))
    }

    /// 超窗时裁剪（`A7`）。估算或窗口不可得（`count_tokens = None` / 模型不在清单）就跳过。
    fn trim_context(&self, req: &mut ModelRequest) {
        let Some(window) = self
            .provider
            .list_models()
            .iter()
            .find(|m| m.id == req.model)
            .map(|m| m.context_window)
        else {
            return;
        };
        let tools = req.tools.clone();
        let estimate = |msgs: &[agent_common::Message]| -> u32 {
            let mut probe = ModelRequest::new(req.model.clone(), msgs.to_vec());
            probe.tools = tools.clone();
            self.provider.count_tokens(&probe).unwrap_or(0)
        };
        // count_tokens 不可得时估算恒为 0 → trim_to_fit 直接返回，等价于跳过。
        trim_to_fit(
            &mut req.messages,
            window,
            self.config.context_keep_ratio,
            estimate,
        );
    }
}
