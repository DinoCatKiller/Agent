# 传输层 fixtures

**范围**：这些是**传输层机制**用的原始 SSE 字节（`Q1` §2「原始字节优先」）。
用来验证「一次调用的管线」——`guard_idle → parse_sse → finalize`——在四种形状下的行为：

| 文件 | 形状 | 覆盖 |
|------|------|------|
| `text_basic.sse` | 正常收尾（有 `[DONE]`、有 usage） | 事件顺序 `Start → Delta* → Usage → End`（`Q1` 用例 2） |
| `truncated_no_terminal.sse` | **EOF 无终止记录** | 截断必须被发现（`Q1` 用例 5） |
| `bad_frame_in_middle.sse` | 中间一条非法 JSON | 跳过该帧、流继续（`Q1` 用例 6） |
| `no_usage.sse` | 有终止记录但全程无 usage | 收尾必须补一次 `Usage::default()`（`Q1` 用例 11） |

**不属于这里**：适配器的契约 fixtures（`request.json` / `wire_request.json` /
`expected_events.jsonl` / `meta.json`）按 `Q1` §2 的目录约定放在
`crates/kernel/providers/tests/fixtures/`，**由 M3 建立**（见 `S1`）。

## 两条注意

1. 每条事件必须**以空行收尾**——空行才是 SSE 的分帧依据。去掉最后一行的空行，
   截断会发生在**解析层**（不完整事件被丢弃），那是与
   `truncated_no_terminal.sse` 不同的另一种截断。
2. 这些是**合成**字节（形状贴近 OpenAI 兼容的 `chat.completions` 流），
   不是真实抓包；逐供应商的真实报文由 `P*` 与 M3 的 fixtures 承担。
