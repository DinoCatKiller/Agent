# A7 · 上下文与 Token

| | |
|---|---|
| ID | `A7` |
| 类型 | 规格（归属 `agent-chat`） |
| 状态 | ✅ 已落地（M4：估算 + 交换块裁剪；摘要压缩二期） |
| 更新 | 2026-09-30 |
| 何时读 | 改裁剪策略、接真实 token 计数、做成本显示前 |
| 规模 | ~1.2k token |

## TL;DR

- **历史是事实，请求是视图**：裁剪只作用于发出去的请求（`Chat.messages` 永不删改），M5 存档即完整历史。
- 超窗按**交换块**从最老处整块丢弃：一个块 = 一条 User + 其后直到下一条 User 的全部内容（assistant、tool_calls、tool 结果同生共死）。
- 三条不变量：tool 配对完整（不留孤儿，两家厂商都会 400）；当前轮（最后一条 User 起）永不裁剪；裁后首条非 system 必须是 User。
- token 数是**估算**（≈4 字符/token，`ErasedProvider::count_tokens`）；裁不干净由供应商报 `ContextOverflow`（`A2` §5），上层自会看到。
- 用量语义：`ModelTurnEnded.usage` = 单次调用；`RoundEnded.round_usage` = 本轮合计；`Chat.total_usage` = 会话累计。

## 1. 估算

- `Provider::count_tokens` 是同步签名（`A2` §2），只做本地估算；Anthropic 有真实计数端点（`P2` §6），接入随 `A2` §7 未决项。
- 估算输入含工具定义占位（tool schema 也占窗口）；多模态内容按文本部分估（图片低估——显式声明，不做像素折算）。

## 2. 裁剪（`trim_to_fit`）

```
目标 = context_window × keep_ratio（默认 0.8，给输出留余量）
while 估算 > 目标 且 头部存在可删交换块:
    从最老的交换块起整块删除
再修不变量: 首条非 system 若是 Assistant → 继续删到 User 起头
```

- `keep_ratio` 防的是「裁到 100% 窗口」——输出与函数调用开销都在同一窗口里。
- 估算恒为 0（`count_tokens = None` 或模型不在清单）时跳过裁剪，等价于尽力而为的降级。
- 逐块删除到达标即停，不做「保留最近 N 轮」启发式（效果调优随 `Q2` 评测）。

## 3. 落点

- 裁剪发生在 `ChatService::start_turn`：每次模型调用前（不只首轮——工具循环中历史会增长）。
- `Chat::request` 负责运行期校验（`role=Tool` 缺 `tool_call_id` → `InvalidRequest`，`A3` §5）。

## 4. 未做（显式声明）

- **摘要压缩**（超长会话折叠成摘要消息）：二期，触发条件与成本另定（`GUI` 二期计划）。
- **精确计费**：`ModelSpec.pricing` 已留位，成本展示随 `R3`（可观测性与成本）。
- **缓存命中计费**（Anthropic `cache_read_input_tokens` 等）：随 `R4`。

## 落地清单

- [x] `Chat.total_usage` 会话累计（M4）
- [x] 交换块裁剪 + 三不变量（单测 6 例，M4）
- [x] 集成到 `start_turn`（超窗场景集成测试，M4）
- [ ] 摘要压缩（二期）
- [ ] 精确 token 计数接入（`A2` §7 未决）

## 相关

- 契约：`A2` §2（count_tokens）、§5（ContextOverflow）｜ 消息：`A3` ｜ 持久化：M5 `kernel/store`（方案 → `S2` Q8）
