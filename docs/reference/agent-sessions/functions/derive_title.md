---
id: derive_title
title: derive_title
---

# Function: derive_title()

```rust
pub fn derive_title(input: &str) -> String
```

Defined in: [`crates/features/sessions/src/session.rs:7`](../../../../crates/features/sessions/src/session.rs#L7)

从首条用户输入生成会话标题：取第一行，截到 24 字符（超出加省略号），空白回退「新会话」。

## Parameters

### input

`&str`

## Returns

`String`

