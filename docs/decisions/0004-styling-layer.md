# ADR-004：样式层建设路线

| | |
|---|---|
| ID | `D4` |
| 状态 | **提案（待确认）** |
| 日期 | 2026-09-28 |
| 依据 | `X5` |
| 相关 | `P0`（L3 后期计划）、`D3` |

## 背景

希望「用 TS/Web 生态的设计资产构建出好看的界面」，同时保留 GPUI 的渲染性能。需要明确：复用 Web 样式的哪一层、用什么机制、代价是什么。

调研结论（X5）：**设计令牌与布局可移植，CSS 运行时不移植**（选择器/层叠/伪类/媒体查询/@keyframes 在 GPUI 无对应物）。

## 备选方案

| 方案 | 说明 | 评价 |
|------|------|------|
| A：把 CSS 引擎移植到 GPUI | 在 GPUI 上实现选择器 + 层叠 + 变量作用域 | ❌ 一期不做。成本≈自研简化版 style 系统，收益低于成本 → 挪到 **L3 后期计划**（P0） |
| B：**Token 桥 + 工具类子集 + 自建组件层** | 设计令牌做成 JSON 唯一来源 → codegen 成 Rust Theme；工具类用宏覆盖常用子集；组件对标 shadcn 结构重写 | ✅ 采纳。一次投入小、收益集中在「设计资产复用 + 主题可热切换」 |
| C：GPUIX / Tauri 用真 CSS | 放弃纯 GPUI 渲染 | 保留为逃生通道：若 B 的观感上限不够，二期评估（P0） |

## 决定

采纳 **方案 B**：

1. `design/tokens.json` 作为**跨语言唯一事实来源**（可从 `tailwind.config` / 设计工具导出）；codegen 成 Rust `Theme`（GPUI `Global`，**支持运行时热加载，不需重编译**）。
2. 建 `agent-ui-kit` 于 `gpui-base`（无样式行为基座）之上：组件**对标 shadcn/ui 的结构与交互重写**，不抄代码。
3. `tw!("flex gap-2 p-4 rounded-lg bg-surface")` 宏覆盖常用 utility **子集**（flex/gap/spacing/rounded/typography/border/color + 状态变体），不追求 Tailwind 全量。
4. 状态样式由宏展开为 GPUI 显式状态 API（`hover:bg-x` → `.hover(|s| s.bg(x))`）；响应式用 `responsive(window_width, …)` helper 手动分支。
5. **L3（CSS 运行时兼容）列入后期计划**，见 P0；触发条件与停止条件都写在那里。

## 影响

- **正面**：设计资产（配色/间距/圆角/字号/阴影）可复用；主题可运行时切换；UI 与 Core 依然完全解耦；设计/样式协作可以发生在 `tokens.json` 与组件 story 上。
- **代价**：无 CSS 热更新（结构改动需重编译）；现成 React 组件与第三方样式库**不可复用**，组件实现是纯 Rust 工作量；`tw!` 宏本身是一个需要维护的子系统。
- **前提**：D3（UI 形态）确定后才动手；若 D3 选 TUI 或 Tauri，本 ADR 需重写对应部分。

## 相关

- 未决项 → `S2` Q2
