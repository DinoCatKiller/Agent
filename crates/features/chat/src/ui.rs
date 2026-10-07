//! `chat` 功能的界面（`D5`：界面跟功能走；`D3`：接口 = UI 无关的事件流 + 命令通道）。
//!
//! 两半分离，为二期换 GPUI 留位（届时只换渲染端）：
//!
//! - **状态机**：[`ChatUi`] 纯数据——吃 [`LoopEvent`]（worker 转发）与宿主转发的按键意图，
//!   不含 ratatui 类型，可脱离终端单测（`Q1`）；
//! - **渲染端**：把 [`ChatUi`] 画进 ratatui [`Frame`]，不含业务判断。
//!
//! [`Theme`] 从 `design/tokens.json` 读（`D4` 一期：令牌是唯一来源，编译期内嵌、启动时解析，
//! 解析失败回退内置默认盘）。渲染层不做节流——事件频率由宿主控制，全量重绘交给 ratatui 差分。

use std::collections::HashMap;

use ratatui::{
    Frame,
    layout::{Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Wrap},
};
use serde::Deserialize;

use agent_common::{DeltaKind, Role, Usage};

use crate::chat::Chat;
use crate::service::{LoopEvent, RoundStop};

// —— 设计令牌（`D4` 一期） ——

const TOKENS_JSON: &str = include_str!("../../../../design/tokens.json");

#[derive(Deserialize)]
struct TokensFile {
    color: ColorTokens,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ColorTokens {
    bg: String,
    fg: String,
    dim: String,
    accent: String,
    border: String,
    border_focus: String,
    user: String,
    thinking: String,
    tool: String,
    ok: String,
    err: String,
    status_bg: String,
    status_fg: String,
}

fn hex_color(s: &str) -> Option<Color> {
    let s = s.trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    u32::from_str_radix(s, 16)
        .ok()
        .map(|v| Color::Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

impl From<&ColorTokens> for Theme {
    fn from(t: &ColorTokens) -> Self {
        let pick = |s: &str, fallback: Color| hex_color(s).unwrap_or(fallback);
        let d = Theme::default();
        Self {
            bg: pick(&t.bg, d.bg),
            fg: pick(&t.fg, d.fg),
            dim: pick(&t.dim, d.dim),
            accent: pick(&t.accent, d.accent),
            border: pick(&t.border, d.border),
            border_focus: pick(&t.border_focus, d.border_focus),
            user: pick(&t.user, d.user),
            thinking: pick(&t.thinking, d.thinking),
            tool: pick(&t.tool, d.tool),
            ok: pick(&t.ok, d.ok),
            err: pick(&t.err, d.err),
            status_bg: pick(&t.status_bg, d.status_bg),
            status_fg: pick(&t.status_fg, d.status_fg),
        }
    }
}

/// 界面配色（来自 `design/tokens.json`，`D4`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub dim: Color,
    pub accent: Color,
    pub border: Color,
    pub border_focus: Color,
    pub user: Color,
    pub thinking: Color,
    pub tool: Color,
    pub ok: Color,
    pub err: Color,
    pub status_bg: Color,
    pub status_fg: Color,
}

impl Default for Theme {
    fn default() -> Self {
        // 与 design/tokens.json 一致的兜底盘（令牌文件损坏 / 缺键时使用）。
        let hex = |s: &str| hex_color(s).unwrap_or(Color::Reset);
        Self {
            bg: hex("#1b1e28"),
            fg: hex("#d5d9e0"),
            dim: hex("#6b7280"),
            accent: hex("#7aa2f7"),
            border: hex("#3b4252"),
            border_focus: hex("#7aa2f7"),
            user: hex("#9ece6a"),
            thinking: hex("#bb9af7"),
            tool: hex("#e0af68"),
            ok: hex("#9ece6a"),
            err: hex("#f7768e"),
            status_bg: hex("#24283b"),
            status_fg: hex("#a9b1d6"),
        }
    }
}

impl Theme {
    /// 编译期内嵌 `design/tokens.json` 并解析；失败回退默认盘（不 panic）。
    pub fn load() -> Self {
        serde_json::from_str::<TokensFile>(TOKENS_JSON)
            .map(|t| Self::from(&t.color))
            .unwrap_or_default()
    }
}

// —— 状态机（无 ratatui 类型） ——

/// 工具调用的显示状态；`None` = 未知（历史回放）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolStatus {
    Running,
    Ok,
    Failed,
}

/// 转录条目：显示单位。直播时由 [`LoopEvent`] 攒出，历史回放时由 [`Chat`] 消息映射而来。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    User(String),
    Assistant(String),
    /// 思考增量（只显示不进历史，`A3`）——重启回放后消失是预期行为。
    Thinking(String),
    Tool {
        name: String,
        status: Option<ToolStatus>,
    },
    /// 工具结果（仅历史回放；直播时结果不展示，只更新 [`Entry::Tool`] 状态）。
    ToolResult(String),
    /// 系统注记：轮次统计、取消、错误、拒绝。
    Note(String),
}

/// 对话 UI 状态机。
///
/// 生命周期由宿主驱动：submit 前 [`ChatUi::push_user`] → worker 转发 [`LoopEvent`] 给
/// [`ChatUi::on_event`]；切会话时 [`ChatUi::load_chat`] 整体替换。
///
/// 回滚语义与编排层一致（`README` 铁律 1）：[`LoopEvent::ModelTurnEnded`] 是提交边界，
/// [`LoopEvent::Cancelled`] / [`LoopEvent::Error`] 把条目截回边界（未提交增量即丢弃）。
#[derive(Debug, Default)]
pub struct ChatUi {
    entries: Vec<Entry>,
    /// 已提交（入史）的条目数——回滚截断点。
    committed: usize,
    /// 开启中的增量块在 `entries` 里的下标（每次模型轮结束置空，下轮另起新块）。
    open_text: Option<usize>,
    open_thinking: Option<usize>,
    /// 工具行下标，按 call id 索引（更新 Running → Ok/Failed 用）。
    tool_rows: HashMap<String, usize>,
    generating: bool,
    input: String,
    /// 光标在输入里的**字符**位置（不是字节）。
    cursor: usize,
    /// `None` = 跟随底部；`Some(offset)` = 用户上滚的行数。
    scroll: Option<u16>,
    total_usage: Usage,
    notice: Option<String>,
}

impl ChatUi {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn generating(&self) -> bool {
        self.generating
    }

    pub fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }

    pub fn total_usage(&self) -> Usage {
        self.total_usage
    }

    /// 用户提交一行（宿主在 submit 时调用；编排层随后也会把它写进历史）。
    /// 同时推进提交边界：取消只丢弃模型输出，不丢用户输入（与 REPL 语义一致）。
    pub fn push_user(&mut self, text: &str) {
        self.entries.push(Entry::User(text.to_string()));
        self.committed = self.entries.len();
    }

    /// 消费一个编排事件（直播）。
    pub fn on_event(&mut self, event: LoopEvent) {
        match event {
            LoopEvent::Delta { kind, text } => match kind {
                DeltaKind::Text => match self.open_text {
                    Some(i) => {
                        if let Some(Entry::Assistant(t)) = self.entries.get_mut(i) {
                            t.push_str(&text);
                        }
                    }
                    None => {
                        self.entries.push(Entry::Assistant(text));
                        self.open_text = Some(self.entries.len() - 1);
                    }
                },
                DeltaKind::Thinking => match self.open_thinking {
                    Some(i) => {
                        if let Some(Entry::Thinking(t)) = self.entries.get_mut(i) {
                            t.push_str(&text);
                        }
                    }
                    None => {
                        self.entries.push(Entry::Thinking(text));
                        self.open_thinking = Some(self.entries.len() - 1);
                    }
                },
                DeltaKind::ToolArgs => {}
            },
            LoopEvent::ModelTurnEnded { usage, .. } => {
                self.total_usage.input_tokens += usage.input_tokens;
                self.total_usage.output_tokens += usage.output_tokens;
                self.open_text = None;
                self.open_thinking = None;
                self.committed = self.entries.len();
                self.notice = None;
            }
            LoopEvent::ToolStarted { id, name } => {
                self.entries.push(Entry::Tool {
                    name,
                    status: Some(ToolStatus::Running),
                });
                self.tool_rows.insert(id, self.entries.len() - 1);
            }
            LoopEvent::ToolFinished { id, ok } => {
                if let Some(&i) = self.tool_rows.get(&id)
                    && let Some(Entry::Tool { status, .. }) = self.entries.get_mut(i)
                {
                    *status = Some(if ok {
                        ToolStatus::Ok
                    } else {
                        ToolStatus::Failed
                    });
                }
            }
            LoopEvent::RoundEnded {
                stop,
                turns,
                round_usage,
            } => {
                let note = match stop {
                    RoundStop::Finished(reason) => format!(
                        "本轮 {turns} 次模型调用结束（{reason:?}），tokens {}/{}",
                        round_usage.input_tokens, round_usage.output_tokens
                    ),
                    RoundStop::MaxTurns(n) => format!("达到最大模型轮次（{n}），工具循环停止"),
                };
                self.entries.push(Entry::Note(note));
                self.close_turn();
            }
            LoopEvent::Cancelled => {
                self.rollback();
                self.entries
                    .push(Entry::Note("已取消：未提交的输出已丢弃".into()));
                self.close_turn();
            }
            LoopEvent::Error { error } => {
                self.rollback();
                self.entries.push(Entry::Note(format!("错误：{error}")));
                self.close_turn();
            }
        }
    }

    /// `run_round` 同步 fast-fail（能力协商 / 校验拒绝）。用户输入已入史，提示后可继续。
    pub fn rejected(&mut self, message: String) {
        self.entries.push(Entry::Note(format!("拒绝：{message}")));
        self.close_turn();
    }

    /// 用已落盘的会话整体替换显示（切会话 / 启动恢复）。
    pub fn load_chat(&mut self, chat: &Chat) {
        *self = Self::default();
        for msg in &chat.messages {
            match &msg.role {
                Role::User => {
                    self.entries.push(Entry::User(msg.text()));
                }
                Role::Assistant => {
                    let text = msg.text();
                    if !text.is_empty() {
                        self.entries.push(Entry::Assistant(text));
                    }
                    for call in &msg.tool_calls {
                        self.entries.push(Entry::Tool {
                            name: call.name.clone(),
                            status: None,
                        });
                    }
                }
                Role::Tool => {
                    self.entries.push(Entry::ToolResult(msg.text()));
                }
                Role::System => {}
            }
        }
        self.total_usage = chat.total_usage;
        self.committed = self.entries.len();
    }

    /// 新会话：清空显示（worker 侧同步重置 `Chat`）。
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn set_notice(&mut self, message: String) {
        self.notice = Some(message);
    }

    fn close_turn(&mut self) {
        self.generating = false;
        self.open_text = None;
        self.open_thinking = None;
        self.committed = self.entries.len();
    }

    fn rollback(&mut self) {
        self.entries.truncate(self.committed);
        self.tool_rows.retain(|_, &mut i| i < self.committed);
        self.open_text = None;
        self.open_thinking = None;
        self.generating = false;
    }

    // —— 输入编辑（Focus::Input 时由宿主转发按键） ——

    pub fn input(&self) -> &str {
        &self.input
    }

    /// 取走输入（发送时调用）。
    pub fn take_input(&mut self) -> String {
        self.cursor = 0;
        std::mem::take(&mut self.input)
    }

    pub fn insert(&mut self, c: char) {
        let byte = self.cursor_byte();
        self.input.insert(byte, c);
        self.cursor += 1;
    }

    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.cursor -= 1;
        let byte = self.cursor_byte();
        self.input.remove(byte);
    }

    pub fn delete(&mut self) {
        let byte = self.cursor_byte();
        if byte < self.input.len() {
            self.input.remove(byte);
        }
    }

    pub fn left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn right(&mut self) {
        let max = self.input.chars().count();
        self.cursor = (self.cursor + 1).min(max);
    }

    pub fn home(&mut self) {
        self.cursor = 0;
    }

    pub fn end(&mut self) {
        self.cursor = self.input.chars().count();
    }

    fn cursor_byte(&self) -> usize {
        self.input
            .char_indices()
            .nth(self.cursor)
            .map(|(b, _)| b)
            .unwrap_or(self.input.len())
    }

    // —— 滚动 ——

    pub fn scroll_up(&mut self, lines: u16) {
        self.scroll = Some(self.scroll.unwrap_or(0).saturating_add(lines));
    }

    /// 下滚到底后回到跟随模式。
    pub fn scroll_down(&mut self, lines: u16) {
        if let Some(offset) = self.scroll {
            self.scroll = offset.checked_sub(lines);
        }
    }
}

// —— 渲染端（ratatui，无业务判断） ——

/// 状态栏的宿主信息。
pub struct StatusInfo<'a> {
    pub provider: &'a str,
    pub model: &'a str,
    /// 当前会话标题；`None` 表示新会话（首次落盘前）。
    pub session: Option<&'a str>,
}

pub fn render_transcript(f: &mut Frame, ui: &ChatUi, area: Rect, theme: &Theme) {
    let block = Block::bordered()
        .title(" 对话 ")
        .border_style(Style::new().fg(theme.border));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let cols = inner.width.max(1);
    let mut lines: Vec<Line> = Vec::with_capacity(ui.entries.len());
    let mut total_height: usize = 0;
    for entry in &ui.entries {
        let (line, prefix_w, text) = entry_line(entry, theme);
        total_height += wrapped_height(&text, prefix_w, cols);
        lines.push(line);
    }

    let offset = match ui.scroll {
        None => total_height.saturating_sub(inner.height as usize),
        Some(o) => (o as usize).min(total_height.saturating_sub(1)),
    } as u16;

    let para = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((offset, 0));
    f.render_widget(para, inner);
}

pub fn render_input(f: &mut Frame, ui: &ChatUi, area: Rect, focused: bool, theme: &Theme) {
    let color = if focused { theme.border_focus } else { theme.border };
    let title = if focused {
        " 输入（Enter 发送） "
    } else {
        " 输入 "
    };
    let block = Block::bordered()
        .title(title)
        .border_style(Style::new().fg(color));
    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(
        Paragraph::new(ui.input.as_str()).style(Style::new().fg(theme.fg)),
        inner,
    );
    if focused {
        let byte = ui
            .input
            .char_indices()
            .nth(ui.cursor)
            .map(|(b, _)| b)
            .unwrap_or(ui.input.len());
        let x = inner.x + display_width(&ui.input[..byte]) as u16;
        f.set_cursor_position(Position::new(x, inner.y));
    }
}

pub fn render_status(f: &mut Frame, ui: &ChatUi, info: &StatusInfo, area: Rect, theme: &Theme) {
    let bg = Style::new().bg(theme.status_bg);
    let mut spans = vec![
        Span::styled(
            format!(" {} · {} ", info.provider, info.model),
            bg.fg(theme.accent),
        ),
        Span::styled(
            format!(" {} ", info.session.unwrap_or("新会话")),
            bg.fg(theme.status_fg),
        ),
        Span::styled(
            format!(
                " tokens {}/{} ",
                ui.total_usage.input_tokens, ui.total_usage.output_tokens
            ),
            bg.fg(theme.dim),
        ),
    ];
    if let Some(notice) = ui.notice() {
        spans.push(Span::styled(format!(" ｜ {notice} "), bg.fg(theme.err)));
    }
    spans.push(Span::styled(
        " Tab 焦点 · Ctrl-C 取消/退出 · Ctrl-Q 退出 ",
        bg.fg(theme.dim),
    ));
    f.render_widget(Line::from(spans).style(bg), area);
}

fn entry_line(entry: &Entry, theme: &Theme) -> (Line<'static>, usize, String) {
    match entry {
        Entry::User(t) => (
            prefixed("你", theme.user, t, Style::new().fg(theme.fg)),
            5,
            t.clone(),
        ),
        Entry::Assistant(t) => (
            prefixed("AI", theme.accent, t, Style::new().fg(theme.fg)),
            5,
            t.clone(),
        ),
        Entry::Thinking(t) => (
            prefixed(
                "思考",
                theme.thinking,
                t,
                Style::new().fg(theme.dim).add_modifier(Modifier::ITALIC),
            ),
            7,
            t.clone(),
        ),
        Entry::Tool { name, status } => {
            let (suffix, color) = match status {
                None => ("", theme.tool),
                Some(ToolStatus::Running) => (" …", theme.tool),
                Some(ToolStatus::Ok) => (" ✓", theme.ok),
                Some(ToolStatus::Failed) => (" ✗", theme.err),
            };
            (
                Line::from(vec![
                    Span::styled("◆ ".to_string(), Style::new().fg(theme.tool)),
                    Span::styled(
                        format!("工具 {name}{suffix}"),
                        Style::new().fg(color).add_modifier(Modifier::BOLD),
                    ),
                ]),
                0,
                format!("工具 {name}"),
            )
        }
        Entry::ToolResult(t) => {
            let text = format!("  └ {}", truncate(t, 160));
            (
                Line::from(Span::styled(text.clone(), Style::new().fg(theme.dim))),
                0,
                text,
            )
        }
        Entry::Note(t) => {
            let text = format!("—— {t} ——");
            (
                Line::from(Span::styled(
                    text.clone(),
                    Style::new().fg(theme.dim).add_modifier(Modifier::ITALIC),
                )),
                0,
                text,
            )
        }
    }
}

fn prefixed(prefix: &str, color: Color, text: &str, body: Style) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("[{prefix}] "),
            Style::new().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(text.to_string(), body),
    ])
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max_chars).collect();
        format!("{cut}…")
    }
}

/// 估算文本在 `cols` 列内占的行数（首行另有 `prefix_w` 前缀）。
/// 宽字符按 2 列估；这只是滚动跟随的近似，误差不破坏正确性（最坏情况偏移一两行）。
fn wrapped_height(text: &str, prefix_w: usize, cols: u16) -> usize {
    let cols = (cols.max(1)) as usize;
    let mut total = 0usize;
    for (i, seg) in text.split('\n').enumerate() {
        let w = display_width(seg) + if i == 0 { prefix_w } else { 0 };
        total += w.div_ceil(cols).max(1);
    }
    total.max(1)
}

fn display_width(s: &str) -> usize {
    s.chars().map(char_width).sum()
}

fn char_width(c: char) -> usize {
    let v = c as u32;
    let wide = matches!(v,
        0x1100..=0x115F | 0x2E80..=0xA4CF | 0xAC00..=0xD7A3
        | 0xF900..=0xFAFF | 0xFE30..=0xFE4F | 0xFF00..=0xFF60
        | 0xFFE0..=0xFFE6 | 0x1F300..=0x1FAFF | 0x20000..=0x3FFFD);
    if wide {
        2
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_common::{FinishReason, ProviderError, ErrorCategory};

    fn error_event() -> LoopEvent {
        LoopEvent::Error {
            error: ProviderError::new("test", ErrorCategory::Truncated).with_message("断了"),
        }
    }

    #[test]
    fn deltas_build_entries_and_usage() {
        let mut ui = ChatUi::new();
        ui.push_user("你好");
        ui.on_event(LoopEvent::Delta { kind: DeltaKind::Thinking, text: "想一下".into() });
        ui.on_event(LoopEvent::Delta { kind: DeltaKind::Text, text: "回复".into() });
        ui.on_event(LoopEvent::Delta { kind: DeltaKind::Text, text: "A".into() });
        ui.on_event(LoopEvent::ModelTurnEnded {
            finish_reason: FinishReason::Stop,
            usage: Usage { input_tokens: 3, output_tokens: 7 },
        });

        assert_eq!(ui.entries[0], Entry::User("你好".into()));
        assert_eq!(ui.entries[1], Entry::Thinking("想一下".into()));
        assert_eq!(ui.entries[2], Entry::Assistant("回复A".into()));
        assert_eq!(ui.total_usage().total(), 10);
        assert!(ui.notice().is_none());
    }

    #[test]
    fn cancel_rolls_back_uncommitted_keeps_user_input() {
        let mut ui = ChatUi::new();
        ui.push_user("写首诗");
        ui.on_event(LoopEvent::Delta { kind: DeltaKind::Text, text: "春眠不".into() });
        ui.on_event(LoopEvent::Cancelled);

        assert!(!ui.generating());
        assert_eq!(ui.entries[0], Entry::User("写首诗".into()), "用户输入不入回滚");
        assert_eq!(ui.entries.len(), 2, "未提交增量被丢弃，只余注记");
        assert!(matches!(ui.entries[1], Entry::Note(_)));
    }

    #[test]
    fn error_keeps_committed_turns_rolls_back_current() {
        let mut ui = ChatUi::new();
        ui.push_user("先算一次");
        ui.on_event(LoopEvent::Delta { kind: DeltaKind::Text, text: "第一轮回答".into() });
        ui.on_event(LoopEvent::ModelTurnEnded {
            finish_reason: FinishReason::ToolCalls,
            usage: Usage::default(),
        });
        ui.on_event(LoopEvent::ToolStarted { id: "c1".into(), name: "echo".into() });
        ui.on_event(LoopEvent::ToolFinished { id: "c1".into(), ok: true });
        ui.on_event(LoopEvent::Delta { kind: DeltaKind::Text, text: "第二轮断".into() });
        ui.on_event(error_event());

        assert_eq!(
            ui.entries,
            vec![
                Entry::User("先算一次".into()),
                Entry::Assistant("第一轮回答".into()),
                // Note 含 ProviderError 的 Display（provider + 分类前缀）
                Entry::Note("错误：test Truncated: 断了".into()),
            ],
            "已提交轮保留；未提交轮（含工具行）回滚"
        );
    }

    #[test]
    fn tool_status_transitions() {
        let mut ui = ChatUi::new();
        ui.on_event(LoopEvent::ToolStarted { id: "c1".into(), name: "echo".into() });
        ui.on_event(LoopEvent::ToolStarted { id: "c2".into(), name: "time".into() });
        ui.on_event(LoopEvent::ToolFinished { id: "c1".into(), ok: true });
        ui.on_event(LoopEvent::ToolFinished { id: "c2".into(), ok: false });

        assert_eq!(
            ui.entries[0],
            Entry::Tool { name: "echo".into(), status: Some(ToolStatus::Ok) }
        );
        assert_eq!(
            ui.entries[1],
            Entry::Tool { name: "time".into(), status: Some(ToolStatus::Failed) }
        );
    }

    #[test]
    fn round_ends_with_note_and_stops_generating() {
        let mut ui = ChatUi::new();
        ui.push_user("hi");
        ui.on_event(LoopEvent::Delta { kind: DeltaKind::Text, text: "答".into() });
        ui.on_event(LoopEvent::RoundEnded {
            stop: RoundStop::Finished(FinishReason::Stop),
            turns: 1,
            round_usage: Usage { input_tokens: 1, output_tokens: 2 },
        });

        assert!(!ui.generating());
        assert!(matches!(&ui.entries[2], Entry::Note(t) if t.contains("1 次")));
    }

    #[test]
    fn load_chat_replays_history() {
        let mut chat = Chat::new("m1");
        chat.push_user("问");
        // 第一轮：文本 + 工具调用 → 提交 → 结果回填（与编排层的真实顺序一致）
        let mut draft = crate::chat::TurnDraft::default();
        draft.apply(&agent_common::StreamEvent::Delta {
            kind: DeltaKind::Text,
            text: "我来查".into(),
        });
        draft.apply(&agent_common::StreamEvent::ToolCall {
            call: agent_common::ToolCall {
                id: "c1".into(),
                name: "echo".into(),
                arguments: serde_json::json!({}),
                provider_id: None,
            },
        });
        draft.apply(&agent_common::StreamEvent::End {
            finish_reason: FinishReason::ToolCalls,
        });
        chat.commit_turn(draft);
        chat.push_tool_result("c1", r#"{"echo":{}}"#);
        // 第二轮：自然结束
        let mut draft = crate::chat::TurnDraft::default();
        draft.apply(&agent_common::StreamEvent::Delta {
            kind: DeltaKind::Text,
            text: "结果是空对象".into(),
        });
        draft.apply(&agent_common::StreamEvent::End {
            finish_reason: FinishReason::Stop,
        });
        chat.commit_turn(draft);

        let mut ui = ChatUi::new();
        ui.load_chat(&chat);
        assert_eq!(
            ui.entries,
            vec![
                Entry::User("问".into()),
                Entry::Assistant("我来查".into()),
                Entry::Tool { name: "echo".into(), status: None },
                Entry::ToolResult(r#"{"echo":{}}"#.into()),
                Entry::Assistant("结果是空对象".into()),
            ]
        );
        assert_eq!(ui.entries.len(), ui.committed, "回放全部视为已提交");
    }

    #[test]
    fn input_editing_with_wide_chars() {
        let mut ui = ChatUi::new();
        ui.insert('你');
        ui.insert('好');
        assert_eq!(ui.input(), "你好");
        assert_eq!(ui.cursor, 2);
        ui.left();
        ui.insert('啊');
        assert_eq!(ui.input(), "你啊好");
        ui.backspace();
        assert_eq!(ui.input(), "你好");
        ui.end();
        ui.take_input();
        assert_eq!(ui.input(), "");
        assert_eq!(ui.cursor, 0);
    }

    #[test]
    fn rejected_and_notice_show_in_status() {
        let mut ui = ChatUi::new();
        ui.push_user("hi");
        ui.rejected("模型缺 JsonSchema 能力".into());
        assert!(matches!(&ui.entries[1], Entry::Note(t) if t.contains("拒绝")));
        ui.set_notice("保存会话失败".into());
        assert_eq!(ui.notice(), Some("保存会话失败"));
    }

    #[test]
    fn scroll_manual_then_follow() {
        let mut ui = ChatUi::new();
        assert_eq!(ui.scroll, None, "默认跟随底部");
        ui.scroll_up(5);
        assert_eq!(ui.scroll, Some(5));
        ui.scroll_down(2);
        assert_eq!(ui.scroll, Some(3));
        ui.scroll_down(10);
        assert_eq!(ui.scroll, None, "滚到底回到跟随");
        ui.scroll_down(1);
        assert_eq!(ui.scroll, None);
    }

    #[test]
    fn theme_tokens_parse() {
        let theme = Theme::load();
        assert_eq!(theme.accent, hex_color("#7aa2f7").unwrap());
        assert_eq!(theme.user, hex_color("#9ece6a").unwrap());
    }
}
