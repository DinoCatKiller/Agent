# AGENTS.md — Agent 入口

> 常驻上下文，**必须保持精简**。这里只有「协议 + 路由 + 硬约束」，不含知识。
> 全量文档地图在 [`docs/00-start-here.md`](docs/00-start-here.md) —— 那是唯一的地图。

## 0. 读取协议（三步）

1. 读本文件（≈1k token，只有协议）。
2. 读 `docs/00-start-here.md` §3，命中与任务相关的 **1–3 行**。
3. **只读命中那几篇**，其它文档默认不读。

| 预算项 | 上限 |
|--------|------|
| 常驻 | 本文件，其余默认不读 |
| 单任务 | 3 篇 / ≈6k token |
| 单篇 | 先读 `TL;DR` 段，不够再往下 |
| 超预算 | 先把结论写进 `docs/10-now/status.md`，再继续 |

命中 `⬜` 条目（文件不存在）：复制 `docs/90-templates/doc.md` 建骨架 → 写完回填 `00-start-here.md` 状态。**不要预建空文件。**

## 1. 路由（任务 → 加载清单）

| 任务 | 加载 |
|------|------|
| 现在做什么 / 下一步 | `docs/10-now/status.md` |
| 等用户拍板的事 | `docs/10-now/open-questions.md` |
| 新增/修改模型供应商 | `20-spec/provider-contract.md` → `20-spec/providers/*` → `40-research/llm-abstraction.md` |
| 新建模块 / 改目录结构 | `20-spec/architecture.md` → `20-spec/goals-and-routes.md` |
| 写 UI / 决定 UI 技术 | `30-decisions/0003-ui-delivery-form.md` → `40-research/ui-delivery-options.md`（GPUI 细节 → `40-research/gpui-frontend-stack.md`） |
| 样式怎么组织 / 复用 Web 样式 | `30-decisions/0004-styling-layer.md` → `40-research/styling-portability.md` |
| 「自研还是找现成轮子」 | `20-spec/goals-and-routes.md` §3 → `40-research/*` |
| 长期排期 / L3 | `50-plans/gui-phase2.md` |
| 推翻某个决定 | 对应 `30-decisions/*` → 写**新** ADR（旧的不改） |

## 2. 硬约束（始终生效）

1. **技术栈已定**：Rust（edition 2024）+ tokio 后端。UI 形态未定前**不要写 UI 代码**；不要引入 Electron/Tauri 等替代方案。
2. 供应商差异**不得泄漏**到 `agent-core`：核心只依赖 `20-spec/provider-contract.md` 的契约。
3. **一个结论只有一个家**：其它文档只许链接、不许复制内容。
4. 未决问题只写在 `docs/10-now/open-questions.md`；ADR **只增不改**。
5. 密钥 / 真实 endpoint 不入库；对外接口流式优先；UI 线程禁止阻塞。
6. 文档与代码同批次提交；改了行为没改文档视为未完成。
7. 目录语义：`10-now` 做什么 → `20-spec` 怎么做 → `30-decisions` 为什么 → `40-research` 凭什么 → `50-plans` 以后做什么。
