# agent-routing · 模型路由与降级链

| | |
|---|---|
| 状态 | ✅ M6 落地：注册表查询、能力协商、失败降级链 |
| 边界 | **机制**：模型 → 候选链 → 能力协商 → 失败换下一家。**不含**对话语义、工具编排；Router 本身实现 `Provider` 契约 |
| 上游 | `agent-common`（契约）、`agent-providers`（`Provider` / `ErasedProvider` / `required_capabilities`） |
| 下游 | `app`（构造 Router 注入 `ChatService`）；`features/chat` **不依赖**本 crate——Router 以 `Arc<dyn ErasedProvider>` 形态被注入 |
| 何时读 | 加路由策略、改降级规则、讨论模型选型前 |

## 这个 crate 是什么

模型世界的「配电箱」：[`Router`](src/lib.rs) 实现 `Provider`（自动获得 `ErasedProvider`），
编排层把它当一个普通供应商用。请求进来时：解析候选链 → 能力协商剔除 → 依次尝试；
可降级失败（429 / 5xx / 超时 / 断网 / Auth）静默换下一家，请求本身有错（InvalidRequest 等）立即失败。

## 核心内容

| 项 | 说明 |
|----|------|
| `Router::with_provider` / `with_erased` | 登记供应商；**注册顺序 = 隐式路由优先级** |
| `Router::with_route(target, [(provider, model), …])` | 显式兜底链（主模型挂了换备模型 / 跨供应商别名） |
| `list_models()` | 各家清单并集 + 别名合成（裁剪查询窗口用，`A7`） |
| 降级策略 | `R2` §3：`retryable \|\| Auth` 换下一家；InvalidRequest / ContextOverflow / ContentFilter 立即失败 |
| 流式窗口 | `R2` §4：只偷看首事件；`Start` 之后不换流；对消费者透明 |

## 三个必须知道的设计决定

1. **透明降级**：被换掉的候选不产生任何事件，耗尽才浮出最后一个错误——UI 不会看到「先报错再成功」的鬼影。
2. **Auth 也降级**：密钥/权限是单家供应商的配置问题，换一家可能就好；但 InvalidRequest 不降——请求本身有错换谁都没用。
3. **`features/chat` 不依赖本 crate**：Router 在 `app` 构造后以 `Arc<dyn ErasedProvider>` 注入 `ChatService`，契约边界（`G-1`）不被打破。

## 相关

- 规格：`R2`（模型注册表与路由）｜ 契约：`A2` ｜ 组织决策：`D6`
- 用法：`crates/app`（`--model a,b` 兜底链）｜ 概览 → `S1` ｜ 地图 → [`AGENTS.md`](../../../AGENTS.md) §2
