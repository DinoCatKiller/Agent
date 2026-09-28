# X5 · TS/Web 样式资产移植到 GPUI 的可行性

| | |
|---|---|
| ID | `X5` |
| 类型 | 调研（一次性消费，会过时） |
| 状态 | ✅ 完成 |
| 更新 | 2026-09-28 |
| 何时读 | 想复用 Web 样式/设计系统、讨论样式层建设时 |
| 规模 | ~2.5k token |

## TL;DR

- **能复用**：设计令牌（配色/间距/圆角/字号/阴影）、设计语言与交互范式、组件的**结构设计**。
- **不能复用**：CSS 运行时语义（选择器/层叠/伪类/媒体查询/@keyframes/变量作用域），以及现成 React 组件代码与 Tailwind 插件生态。
- **一条机制差异**：CSS 是**运行时组合**，GPUI 是**编译期组合** → 正确做法是「可变量全部下沉到 token 层（可热加载），只把结构留在编译期」。
- 先例：`gpui-component` 的 UI 设计本身即从 shadcn/ui 移植（**重写**，不是跑 CSS）。
- 决策 → [`D4`](../30-decisions/0004-styling-layer.md)；CSS 兼容层已列入后期计划 → [`P0`](../50-plans/gui-phase2.md) 的 L3。

## 1. 三层可移植性

| 层 | 内容 | 可移植性 | 说明 |
|----|------|---------|------|
| **L1 设计令牌** | 配色、间距刻度、圆角、字号/行高、字重、阴影、透明度、层级 | ✅ **完全可移植，收益最大** | `tailwind.config` / CSS 变量 / 设计工具导出 → JSON → codegen 成 Rust `Theme`。GPUI 的 `Theme` 是 `Global`，**可运行时切换**，token 层能热加载、不用重编译 |
| **L2 布局与工具类** | `flex` / `gap` / `p-*` / `rounded-*` / `text-*` / `border-*` / `overflow-*` | ✅ **大部分可移植** | GPUI 用 `taffy` 做布局，**Flexbox 语义与 Web 一致**；工具类可近乎 1:1 映射成 Rust 方法链（宏或代码生成） |
| **L3 CSS 运行时语义** | 选择器、层叠、继承、`:hover`/`:focus`、媒体查询、`@keyframes`、CSS 变量作用域、`backdrop-filter`、`mix-blend-mode` | ❌ **一期不可移植**（列入后期计划） | GPUI 里没有「选择器」与层叠概念。要支持它 = 自己实现一个简化 CSS 引擎 → 成本远超收益，挪到 P0 的 L3 |

## 2. 必须接受的机制差异

| 差异 | 后果 | 缓解 |
|------|------|------|
| CSS 运行时组合 vs GPUI 编译期组合 | 改样式要重编译；没有 Web 的热更新 | 可变量全部抽到 token/theme 层（`Global`，可热加载）；只把结构留在编译期 |
| 无 `:hover` 等伪类 | `hover:bg-x` 不能照搬 | 映射为显式状态 API（`.hover(\|s\| s.bg(x))`），宏自动展开 |
| 无媒体查询 | 响应式要手写 | `responsive(window_width, …)` helper，按窗口宽度手动分支 |
| 无 DevTools 样式面板 | 调样式体验差 | **Zed Inspector**（`dev::ToggleInspector`）内部有 `STYLE_METHODS` 反射表，支持 Rust/JSON **双向**样式编辑 —— 可基于它做「实时调参 + 导出 token」 |
| 动画 API 较基础 | 组件库动画被社区评价为「偏基础」（有人因此自建组件库） | `with_animation` + easing 覆盖简单过渡；Web 级动效需自己补 |

## 3. 推荐落地架构（三级）

```
design/tokens.json       ← 跨语言唯一事实来源（tailwind.config / 设计工具导出）
      │ codegen（build.rs 或 cargo xtask）
      ▼
crates/agent-ui-theme    ← Rust Theme(Global) + 类型化 token 访问器；支持运行时热加载
      │
      ▼
crates/agent-ui-kit      ← 自建组件层（建在 gpui-base 之上）+ tw! 宏（常用子集）
      │
      ▼
crates/agent-ui          ← 业务界面（聊天、会话列表、设置）
```

要点：

- **token 是唯一跨语言接口**：设计师 / TS 前端改 `tokens.json`，Rust 侧重新 codegen 即可。
- **组件「抄结构不抄代码」**：对标 shadcn/ui 的结构、交互与状态语义，用 Rust builder + `gpui-base` 重写；这部分工作量无法省。
- **`tw!` 宏先做子集**：覆盖 flex/gap/spacing/rounded/typography/border/color + 状态变体即可。

## 4. 诚实评估：能复用多少

| 能复用 | 不能复用 |
|--------|---------|
| 设计令牌（配色/间距/圆角/字号/阴影） | 现成的 React 组件代码 |
| 设计语言与交互范式（shadcn 那套语义色/状态） | `.css` 文件、Tailwind 插件生态 |
| 组件的**结构设计**与状态模型 | 第三方动效库 / 图表库 / 富文本库 |
| 设计工具的导出产物 | 浏览器 DevTools 式调样式工作流 |

→ 「用 TS 生态样式构建界面」的准确含义是：**以 Web 侧打磨好的设计系统为源头，在 Rust 侧 codegen + 自建组件层**，而不是把 Tailwind 搬过来跑。人力问题会**部分**缓解（设计/样式协作在 JSON 与组件 story 上），组件实现仍是 Rust 工作量。

## 5. 若坚持「真·TS 写 UI」

| 路线 | 样式真相 |
|------|---------|
| GPUIX（React reconciler + napi 桥 → GPUI 直接绘制，无 WebView） | React 负责组件与 Diff，但**样式仍落到 GPUI 能表达的子集**（Rust 侧 RetainedTree 只带元素/样式/事件标记）—— **不是 CSS** |
| Tauri（Rust 后端 + Web 前端） | **真 CSS / 真 Tailwind / 真现成组件**，代价是背 WebView |

## 6. 相关

- 决策 → [`D4`](../30-decisions/0004-styling-layer.md)
- 后期计划（L3） → [`P0`](../50-plans/gui-phase2.md)
- 未决项 → [`S2`](../10-now/open-questions.md) Q2
