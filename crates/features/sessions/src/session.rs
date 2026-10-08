//! 会话语义：会话「是什么」的最小规则，先落标题生成（`README` 布局的 `session.rs`）。
//!
//! 触发点在 `chat` 落盘时（worker 首次保存会话调用 [`derive_title`]），
//! 生成逻辑住本切片、编排侧只调用（`README` 第一个任务的切分约定）。

/// 从首条用户输入生成会话标题：取第一行，截到 24 字符（超出加省略号），空白回退「新会话」。
pub fn derive_title(input: &str) -> String {
    const MAX: usize = 24;
    let first = input.lines().next().unwrap_or("").trim();
    if first.is_empty() {
        return "新会话".into();
    }
    let mut title: String = first.chars().take(MAX).collect();
    if first.chars().count() > MAX {
        title.push('…');
    }
    title
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn takes_first_line_and_truncates() {
        assert_eq!(derive_title("帮我写个函数\n顺便看看测试"), "帮我写个函数");
        let long = "这是一条非常非常非常非常非常非常非常非常非常长的标题";
        assert!(long.chars().count() > 24);
        assert_eq!(derive_title(long).chars().count(), 25, "24 字 + 省略号");
        assert!(derive_title(long).ends_with('…'));
    }

    #[test]
    fn blank_falls_back() {
        assert_eq!(derive_title(""), "新会话");
        assert_eq!(derive_title("   \n x"), "新会话");
    }
}
