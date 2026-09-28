# D6 · kernel（机制）与 features（语义）的判据

| | |
|---|---|
| ID | `D6` |
| 类型 | 决策（**只增不改**） |
| 状态 | **已接受** |
| 更新 | 2026-09-28 |
| 何时读 | 新建 crate、决定代码放哪、质疑「这算功能还是基础设施」时 |
| 规模 | ~1.6k token |

## 背景

`D5` 决定「代码按功能组织」，把目录分成 `common / infra / features / app`。执行后暴露两个问题：

1. **`infra/*` 仍是按层切的**（transport / providers / routing / store / config / telemetry），名字叫"基础设施"，语义模糊 —— 它既容易被理解为"功能之外的杂物间"，就容易被用来塞任何东西。
2. **`features/*` 只有空壳**：界面与数据访问仍被预设在外面的 crate（原 `agent-ui`、`agent-store`）。等于**付了垂直切片的成本，拿到的是层切**。

本 ADR 给出一条可执行的判据，并修订 `D5` 的命名与 feature 内部布局。

## 决定

### 1. 判据：机制 vs 语义

> 问一句：**这是"怎么发 HTTP / 怎么连库 / 怎么读钥匙串"（机制），还是"对话怎么进行、会话怎么命名"（语义）？**
> 机制 → `crates/kernel/*`；语义 → `crates/features/<功能>/`。

两条硬约束：

- **kernel 里不允许出现业务名词**（对话、会话、设置项…）。出现即说明放错位置。
- **features 里不允许出现连接池、TLS、SSE 帧解析的实现**。需要就用 kernel 的接口。

### 2. 命名：`infra/` → `kernel/`

`crates/infra/*` 改名为 `crates/kernel/*`。语义从"基础设施（杂物间）"收紧为**"机制内核（明确不许放业务）"**。crate 划分与包名不变。

### 3. features 内部布局（垂直切片）

```
crates/features/<功能>/
  README.md        门面：边界 / 对外接口 / 待办
  src/
    lib.rs         模块声明 + 对外导出
    service.rs     业务编排（本功能的主流程）
    repo.rs        本功能自己的数据访问（调用 kernel/store 的连接与事务机制）
    ui.rs          本功能的界面（等 UI 选型 `D3` 定了再建）
    <domain>.rs    本功能的领域类型（如 chat 的 Chat、sessions 的 Session）
  tests/           本功能的测试与 fixtures（含端到端：本功能 → kernel 的假实现）
  docs/            本功能的规格（按需，如 `A5` / `A7`）
```

**按需创建，不预建空文件**：`ui.rs` 等 `D3`；`repo.rs` 等真需要持久化时。

### 4. 边界示例（避免切错或过度切分）

| 代码 | 归属 | 为什么 |
|------|------|--------|
| SSE 帧解析、字节 → 事件 | `kernel/transport` | 机制：与"对话"无关，任何流式功能都用得上 |
| 流式增量累积成一条消息 | `features/chat` | 语义：这是一次对话的产出 |
| SQLite 连接池、迁移、事务 | `kernel/store` | 机制 |
| "本会话最近 20 条消息"这类查询 | `features/sessions` 的 `repo.rs` | 语义 |
| 会话标题自动生成 | `features/sessions`；由 `chat` 落盘时调用 | feature 之间允许依赖，但不得成环 |
| keyring 读写、配置文件解析 | `kernel/config` | 机制 |
| "哪些设置项、怎么校验、怎么呈现" | `features/settings` | 语义 |

### 5. 对 D5 的修订（D5 原文不改，以此表为准）

| D5 原文 | D6 修订 |
|---------|---------|
| `crates/infra/*` | → `crates/kernel/*`，语义收紧为"机制，不含业务" |
| feature 内含 service / ui / tests | 明确为 `service.rs` / `repo.rs` / `ui.rs` / 领域类型 / `tests/`；**`repo.rs` 在 feature 内**（只有连接与事务在 kernel） |
| 未给"什么算功能"的判据 | → 本文 §1 |

## 影响

- **已执行**：目录改名（`git mv`，历史保留），全仓库引用同步（残留 `infra` = 0），`cargo test` 全绿。
- 新建 crate 的落点从此**只能**用 §1 判据决定。
- `A1` §5、`AGENTS.md` §1、`RM` 的承接 crate 列、架构图均已按本 ADR 更新。

## 已知风险

| 风险 | 对策 |
|------|------|
| **kernel 变胖**：有人把"跨功能共享的业务逻辑"塞进去 | kernel 的改动只要出现业务名词就驳回；共享业务逻辑宁可放某个 feature 并让另一个 feature 依赖它 |
| **切太细**：feature 数量增长后 crate 边界成本上升 | 新 feature 只在"有独立界面或独立生命周期"时才切 |

## 未决项

→ `S2`：`features/chat` 的 `repo.rs` 是否现在就需要（取决于 `kernel/store` 何时落地，建议 M5）
