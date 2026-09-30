# M1 — 文档规范（归属规则 + 任务路由）

| | |
|---|---|
| ID | `M1` |
| 类型 | 规格 |
| 状态 | 稳定 |
| 更新 | 2026-09-29 |
| 何时读 | 新建 / 搬移文档；开新任务、不确定该读哪些文档 |
| 规模 | ~0.9k token |

## TL;DR

- 本文件是 [`AGENTS.md`](../../AGENTS.md) `§1`（文档归属）与 `§3`（任务路由）的**全文落点**，从常驻上下文外移，**按需读**。
- 归属：**机制**进 `kernel`（`README.md` + `docs/`），**语义**进 `features`；跨 crate 进根 `docs/`（判据 `D6`）。
- 路由：按「任务 → 加载清单」表，一次拿齐该读的文档。

## §1 文档归属规则（新增文档放哪）

| 影响范围 | 放哪 |
|---------|------|
| **机制**（怎么发 HTTP / 怎么连库） | `crates/kernel/<crate>/README.md`（门面）+ `docs/*.md`（细节）。**kernel 不许出现业务名词**（`D6`） |
| **语义**（业务：对话 / 会话 / 设置） | `crates/features/<功能>/`：README + `src/{service,repo,ui,领域类型}.rs` + `tests/`（`D6` §3） |
| 影响多个 crate / 全项目 | 根 `docs/*.md` |
| 决策（只增不改） | `docs/decisions/NNNN-<slug>.md` |
| 长期计划（尚无归属 crate） | `docs/plans/*.md` |
| 当前阶段状态 · **概览** | 根 [`STATUS.md`](../../STATUS.md) |
| 当前阶段状态 · **某 crate 的细节** | `crates/<crate>/status.md`（按需建立，随任务生灭） |
| 未决问题 | 根 [`OPEN-QUESTIONS.md`](../../OPEN-QUESTIONS.md) |
| 里程碑与排期（**不含状态**） | [`docs/roadmap.md`](../roadmap.md) |
| 模板 | [`docs/_template.md`](../_template.md)、[`docs/decisions/_template.md`](../decisions/_template.md)、[`crates/kernel/providers/docs/providers/_template.md`](../../../crates/kernel/providers/docs/providers/_template.md) |

## §2 任务路由（任务 → 加载清单）

| 任务 | 加载 |
|------|------|
| 现在做什么 / 下一步 | `S1` → 命中任务后进它所链接的 `crates/<crate>/status.md` |
| 整体排期 / 完成标准 | `RM` |
| 一眼看懂整体架构 / 当前进度 | `A1-V`（图）；事实仍以 `A1` + `RM` + `S1` 为准 |
| 等用户拍板的事 | `S2` |
| 新增 / 修改供应商 | `A2` → `PT` → 对应 `P*` → `X3` |
| 写传输 / SSE / 重试 | `A4`（⬜ 先建）→ `Q1` → `A2` |
| 改契约类型 | 对应 crate 的 README → `A2` / `A3` |
| 新增功能 / 新建 feature | `D5` + `D6` §1 判据 → 该功能的 `crates/features/<功能>/README.md` |
| 「这段代码该放哪」 | `D6` §1 判据（机制 → kernel / 语义 → features） |
| 新建模块 / 改目录结构 | `A1` §5 → `D5` / `D6` → `G1` |
| 决定 UI 技术 | `D3` → `X4`（GPUI 细节 → `X2`） |
| 样式怎么组织 | `D4` → `X5` |
| 「自研还是找现成轮子」 | `G1` §3 → `X1`/`X2`/`X3` |
| 长期排期 / L3 | `RM` → `GUI` |
| 推翻某个决定 | 对应 `D*` → 写**新** ADR（旧的不改） |

## 相关

- 入口 / 地图 → [`AGENTS.md`](../../AGENTS.md)
- 落点判据 → `D6`
- 文档模板 → `T0`
