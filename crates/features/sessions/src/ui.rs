//! `sessions` 功能的界面：会话侧栏（列表 + 选择）。
//!
//! 与 `chat` 的 [`agent_chat::ui`] 同一分工：状态纯数据（可离线测），渲染端无业务判断。
//! 元类型 [`SessionMeta`] 与 [`Theme`] 来自 `agent-chat`（feature 间依赖，`D5`）——
//! 本切片不碰数据库，数据由宿主（app）从 `SessionRepo` 取来喂给 [`SessionPane::set_sessions`]。
//!
//! 鼠标命中（一期 TUI）：[`SessionPane::render`] 回传 ratatui 实际使用的首项偏移，
//! 宿主据此把鼠标行换算成会话索引并调 [`SessionPane::select`]——本切片只提供换算所需的
//! 两个钩子，不解析鼠标事件（事件在 `app` 的事件泵里）。

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, List, ListItem, ListState},
};

use agent_chat::repo::SessionMeta;
use agent_chat::ui::Theme;

/// 会话侧栏状态：列表 + 选中下标（纯数据，无 ratatui 状态）。
#[derive(Debug, Default)]
pub struct SessionPane {
    items: Vec<SessionMeta>,
    selected: usize,
    focused: bool,
}

impl SessionPane {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    /// 整体替换列表（worker 每次落盘后刷新）；保持选中项尽量不动。
    pub fn set_sessions(&mut self, items: Vec<SessionMeta>) {
        self.selected = self.selected.min(items.len().saturating_sub(1));
        self.items = items;
    }

    pub fn next(&mut self) {
        if self.selected + 1 < self.items.len() {
            self.selected += 1;
        }
    }

    pub fn previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    /// 按索引选中（鼠标命中用）；越界收敛到最后一条，空列表保持 0。
    pub fn select(&mut self, index: usize) {
        self.selected = match self.items.len() {
            0 => 0,
            len => index.min(len - 1),
        };
    }

    pub fn selected(&self) -> Option<&SessionMeta> {
        self.items.get(self.selected)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 渲染列表，返回 ratatui 实际使用的**首项偏移**（`0` = 从第一条画起）。
    ///
    /// 列表长于可视区时偏移非零，宿主做鼠标命中测试需要它（见模块头注释）。
    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme) -> usize {
        let border = if self.focused {
            theme.border_focus
        } else {
            theme.border
        };
        let title = if self.items.is_empty() {
            " 会话（空，直接输入即建） ".to_string()
        } else {
            format!(" 会话（{}） ", self.items.len())
        };
        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|meta| {
                ListItem::new(Span::styled(
                    meta.title.clone(),
                    Style::new().fg(theme.status_fg),
                ))
            })
            .collect();
        let list = List::new(items)
            .block(
                Block::bordered()
                    .title(title)
                    .border_style(Style::new().fg(border)),
            )
            .highlight_style(
                Style::new()
                    .fg(theme.border_focus)
                    .bg(theme.status_bg)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");
        let mut state = ListState::default();
        if !self.items.is_empty() {
            state.select(Some(self.selected));
        }
        f.render_stateful_widget(list, area, &mut state);
        state.offset()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(id: &str, title: &str) -> SessionMeta {
        SessionMeta {
            id: id.into(),
            title: title.into(),
            model: "m".into(),
            created_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn navigation_clamps_at_edges() {
        let mut pane = SessionPane::new();
        assert!(pane.selected().is_none(), "空列表无选中");

        pane.set_sessions(vec![meta("a", "A"), meta("b", "B"), meta("c", "C")]);
        assert_eq!(pane.selected().unwrap().id, "a", "默认选第一条");
        pane.previous();
        assert_eq!(pane.selected().unwrap().id, "a", "到顶不再上移");
        pane.next();
        pane.next();
        assert_eq!(pane.selected().unwrap().id, "c");
        pane.next();
        assert_eq!(pane.selected().unwrap().id, "c", "到底不再下移");
    }

    #[test]
    fn refresh_keeps_selection_when_possible() {
        let mut pane = SessionPane::new();
        pane.set_sessions(vec![meta("a", "A"), meta("b", "B")]);
        pane.next();
        assert_eq!(pane.selected().unwrap().id, "b");

        // 列表缩短到 1 条：选中收敛到最后一条
        pane.set_sessions(vec![meta("a", "A")]);
        assert_eq!(pane.selected().unwrap().id, "a");

        // 清空：无选中，再恢复时回到第一条
        pane.set_sessions(vec![]);
        assert!(pane.is_empty());
        assert!(pane.selected().is_none());
        pane.set_sessions(vec![meta("a", "A")]);
        assert_eq!(pane.selected().unwrap().id, "a");
    }

    #[test]
    fn select_by_index_clamps() {
        let mut pane = SessionPane::new();
        pane.select(3);
        assert!(pane.selected().is_none(), "空列表选中仍为空");

        pane.set_sessions(vec![meta("a", "A"), meta("b", "B"), meta("c", "C")]);
        pane.select(1);
        assert_eq!(pane.selected().unwrap().id, "b");
        pane.select(99);
        assert_eq!(pane.selected().unwrap().id, "c", "越界收敛到最后一条");
    }
}
