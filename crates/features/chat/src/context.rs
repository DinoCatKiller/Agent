//! 上下文裁剪（规格：`A7`）。
//!
//! 超窗时按**交换块**从最老处整块丢弃：一个块 = 一条 `User` 消息**及其后直到下一条
//! `User` 之前的全部内容**（Assistant 回复、它发起的 `tool_calls`、配套的 Tool 结果）。
//! 这样保证两条不变量：
//!
//! 1. **tool 配对完整**：assistant 的工具调用与它的结果同生共死，不会留孤儿
//!    （孤儿会被两家供应商 400，`P1` / `P2`）；
//! 2. **当前轮不动**：最后一条 User 消息及其之后的内容是本轮上下文，永不裁剪。
//!
//! 裁剪后若历史以 Assistant 起头（如本就缺 User 的历史被削了头），按同规则继续删到
//! User 起头为止（Anthropic 强制 user 起头，OpenAI 也更稳）。估算不准（≈4 字符/token，
//! 适配器各自实现）没关系：目标是**尽力不超窗**，裁不干净时由供应商报
//! `ContextOverflow`（`A2` §5），上层自会看到。

use agent_common::{Message, Role};

/// 把历史裁到窗口的 `keep_ratio` 以内（默认 0.8，给输出留余量）。
///
/// `estimate` 返回全量消息的近似 token 数。逐块从最老处删，直到达标或无块可删（尽力而为）。
pub fn trim_to_fit(
    messages: &mut Vec<Message>,
    window: u32,
    keep_ratio: f64,
    estimate: impl Fn(&[Message]) -> u32,
) {
    let target = (f64::from(window) * keep_ratio.clamp(0.1, 1.0)) as u32;
    if estimate(messages) <= target {
        return;
    }
    while estimate(messages) > target {
        let Some(last_user) = messages.iter().rposition(|m| m.role == Role::User) else {
            break; // 没有 User 消息（不该发生），不动
        };
        let head_start = usize::from(messages.first().is_some_and(|m| m.role == Role::System));
        if head_start >= last_user {
            break; // 只剩当前轮：尽力而为，交给供应商报 ContextOverflow
        }
        if messages[head_start].role != Role::User {
            break; // 头部不是交换块起点，交给下方不变量修复
        }
        // 整块：User 起，到下一条 User 之前（永不越过当前轮起点）。
        let mut end = head_start + 1;
        while end < last_user && messages[end].role != Role::User {
            end += 1;
        }
        messages.drain(head_start..end);
    }
    // 不变量：首条非 system 消息必须是 User（Anthropic 要求 user 起头）。
    while messages
        .get(usize::from(
            messages.first().is_some_and(|m| m.role == Role::System),
        ))
        .is_some_and(|m| m.role == Role::Assistant)
    {
        let head_start = usize::from(messages.first().is_some_and(|m| m.role == Role::System));
        let mut end = head_start + 1;
        while end < messages.len() && messages[end].role != Role::User {
            end += 1;
        }
        messages.drain(head_start..end);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_common::{Message, Role};

    /// 固定估算：每条消息 10 token（便于构造恰好超窗的场景）。
    fn each_10(msgs: &[Message]) -> u32 {
        msgs.len() as u32 * 10
    }

    fn assistant_with_tool(id: &str) -> Message {
        let mut msg = Message::assistant("");
        msg.tool_calls = vec![agent_common::ToolCall {
            id: id.into(),
            name: "t".into(),
            arguments: serde_json::json!({}),
            provider_id: None,
        }];
        msg
    }

    #[test]
    fn under_target_is_untouched() {
        let mut msgs = vec![Message::system("s"), Message::user("q")];
        trim_to_fit(&mut msgs, 1000, 0.8, each_10);
        assert_eq!(msgs.len(), 2);
    }

    #[test]
    fn drops_oldest_exchanges_until_fit() {
        let mut msgs = vec![
            Message::system("s"),
            Message::user("q1"),
            Message::assistant("a1"),
            Message::user("q2"),
            Message::assistant("a2"),
            Message::user("current"),
        ];
        // 目标 25（= window 50 × 0.5）：6 条 60 → 删 q1+a1（剩 40）→ 删 q2+a2（剩 20 ≤ 25）。
        // 语义是「删到达标」：上一轮完整交换也让路，只保 system + 当前轮。
        trim_to_fit(&mut msgs, 50, 0.5, each_10);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, Role::System, "system 永不裁剪");
        assert_eq!(msgs[1], Message::user("current"));
    }

    #[test]
    fn can_stop_mid_history_when_target_reached() {
        let mut msgs = vec![
            Message::system("s"),
            Message::user("q1"),
            Message::assistant("a1"),
            Message::user("q2"),
            Message::assistant("a2"),
            Message::user("current"),
            Message::assistant("a3"),
        ];
        // 目标 45：70 → 删 q1+a1（60 > 45）→ 删 q2+a2（40 ≤ 45）停。
        // 剩 system + current + a3（当前轮完整保留，尽管 a2 已被删）。
        trim_to_fit(&mut msgs, 90, 0.5, each_10);
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[1], Message::user("current"));
        assert_eq!(msgs[2], Message::assistant("a3"));
    }

    #[test]
    fn tool_pair_is_dropped_as_one_block() {
        let mut msgs = vec![
            Message::system("s"),
            Message::user("q1"),
            assistant_with_tool("c1"),
            Message::tool_result("c1", "r1"),
            Message::tool_result("c1x", "r2"), // 同一块的后续 tool 结果
            Message::user("current"),
        ];
        trim_to_fit(&mut msgs, 50, 0.5, each_10); // 6 条 60 > 25 → 删 q1 起的整个交换块
        assert_eq!(msgs.len(), 2, "assistant 与其 tool 结果必须同块删除");
        assert_eq!(msgs[0].role, Role::System);
        assert_eq!(msgs[1], Message::user("current"));
        assert!(
            msgs.iter().all(|m| m.role != Role::Tool),
            "不留孤儿 tool 结果"
        );
    }

    #[test]
    fn never_touches_current_round_even_if_still_over() {
        let mut msgs = vec![
            Message::system("s"),
            Message::user("current"),
            assistant_with_tool("c1"),
            Message::tool_result("c1", "r1"),
        ];
        trim_to_fit(&mut msgs, 10, 0.5, each_10); // 无论如何都超：尾部不可动
        assert_eq!(msgs.len(), 4, "当前轮（user+assistant+tool）永不裁剪");
    }

    #[test]
    fn assistant_first_after_trim_is_dropped() {
        // 历史本就以 assistant 起头（无 user 头）：先删 q1 块，再按不变量删 assistant 头。
        let mut msgs = vec![
            Message::system("s"),
            Message::user("q1"),
            Message::assistant("orphan-lead"),
            Message::user("current"),
        ];
        trim_to_fit(&mut msgs, 40, 0.5, each_10); // 4 条 40 > 20 → 删 q1+orphan-lead → 20 ≤ 20 停
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, Role::System);
        assert_eq!(msgs[1], Message::user("current"));
    }
}
