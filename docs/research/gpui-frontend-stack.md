# X2 · GPUI 前端技术路线调研

| | |
|---|---|
| ID | `X2` |
| 类型 | 调研（一次性消费，会过时） |
| 状态 | ✅ 完成 |
| 更新 | 2026-09-27 |
| 何时读 | 动 UI、升 GPUI、遇到平台 / IME / 渲染问题时 |
| 规模 | ~2k token |

## 目的

确认「用 GPUI 画前端」的可行性、版本现状、平台支持、组件生态与关键风险。

## 1. 框架现状

| 项 | 结论 |
|----|------|
| 定位 | 混合即时/保留模式的 GPU 加速 Rust UI 框架，Zed 编辑器的底层 UI 框架 |
| 最新版本 | `gpui` **0.2.2**（2026-08-15 发布，Apache-2.0） |
| 稳定性 | **pre-1.0**，官方明说版本间常有破坏性变更，API 以 Zed 源码为准 |
| 渲染栈 | `blade-graphics` + `naga`；布局 `taffy 0.9`；文本整形 `cosmic-text`（Linux）/ DirectWrite（Windows）/ `font-kit`（macOS） |
| 生态定位 | 官方定位是「Zed 的 UI 层」，通用组件能力依赖社区补齐 |

## 2. 平台支持（Windows 是本项目的关键路径）

| 平台 | 窗口后端 | 文本 | 需要的 feature |
|------|---------|------|---------------|
| Windows | Win32 | DirectWrite | **无需任何 feature**（`font-kit` 在 Windows 无效） |
| macOS | Metal + cocoa | `font-kit` | `font-kit` |
| Linux / FreeBSD | Wayland / X11 | `cosmic-text` | `wayland` 或 `x11`（至少一个） |

- 构建要求：最新 stable Rust；Windows 需 Windows 10 SDK ≥ **10.0.20348.0**（Zed Windows 构建文档）。
- **文档冲突已确认**：`docs.rs` 的 Getting Started 仍写「需要 macOS 或 Linux」，而 `crates/gpui/README.md` 明确 Windows 无需 feature。实际依据以 **Zed 1.0（2026-04-29）已提供 Windows 稳定构建** 为准 —— Windows 可用，官方文档滞后。

## 3. 三种使用层级（选型时的拆分依据）

| 层级 | 载体 | 用途 |
|------|------|------|
| 状态与通信 | `Entity<T>`（`App` 拥有，类 `Rc` 访问） | 跨组件共享状态，如会话、模型列表 |
| 声明式 UI | 实现 `Render` 的 `Entity`（View） | 每帧重建 element 树，类 Tailwind API |
| 命令式 UI | 实现 `Element` | 大列表高效渲染、自定义编辑器布局 |

配套：`Actions`（快捷键→逻辑操作）、`executor`（与平台事件循环集成的异步执行器）、`#[gpui::test]` + `TestAppContext`（可做 UI 单测）、`inspector_reflection`（调试反射）。

最小启动骨架：

```rust
use gpui::*;

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        // 创建窗口 + 注册根视图
    });
}
```

## 4. 组件与富内容：`gpui-kit`（含 `gpui-component`）

| 项 | 结论 |
|----|------|
| 版本 | `gpui-component` **0.6.6**（Apache-2.0），上层整合包 `gpui-kit` 0.6.x |
| 依赖策略 | 只依赖 `gpui-kit = "0.6"` 一个 crate；它会**固定匹配的 GPUI 版本**并重导出 |
| 分层 | `gpui-kit` = `gpui-base`（无样式行为基座）+ `gpui-component`（60+ 样式化组件）；另有 `gpui-shell`（Rust 承载的 JS 运行时） |
| 组件能力 | 表单/导航/浮层/反馈/布局；数据表格（虚拟滚动、固定列、列宽可调、十万行级）；虚拟列表（支持不等高）；代码编辑器（Tree-sitter 高亮 + LSP 诊断/补全/悬停）；Dock 布局（可拖拽、可序列化） |
| 富内容 | **原生 Markdown 与 HTML 渲染**、语法高亮、内置图表 —— 聊天消息渲染直接可用 |
| 生产验证 | `Longbridge Pro` 桌面商业应用自研即用 |
| 初始化约束 | 必须 `gpui_kit::init(cx)`；窗口根节点必须是 `Root` |

初始化范式：

```rust
fn main() {
    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);
        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| HelloWorld);
                cx.new(|cx| Root::new(view, window, cx))
            }).expect("Failed to open window");
        }).detach();
    });
}
```

## 5. GPUI ↔ tokio 桥接

结论：**自研**。

| 方案 | 版本/状态 | 评价 |
|------|----------|------|
| GPUI 自带 `executor` | 随 `gpui` 发布 | GPUI 已有与平台事件循环集成的 async executor，跨 runtime 调用的桥接层本身很薄 |
| `gpui-tokio-bridge` | 0.1.0（2026-01） | 第三方个人 crate，仅一个版本，API 面积极小，长期维护存疑 |
| `guic-gpui-tokio` | 0.3.1 | 属于 GPUI 的 fork 生态（`guic-gpui`），与官方 `gpui` 生态分叉 |
| Zed 内部 `gpui_tokio` | 未发布 | 不在 crates.io，只能抄思路 |

自研形态（约 50 行）：应用启动时创建全局 `tokio::runtime::Runtime`（多线程），暴露 `spawn_tokio(cx, fut)`；tokio 侧完成后用 `cx.spawn` / `WeakEntity::update` 把结果送回 GPUI 上下文。要点：

- **UI 线程绝不 `block_on`**。
- 流式场景不要每个 delta 都 `notify`：按 ~16ms（一帧）批量刷新，避免渲染风暴。
- 取消：`AbortSignal` 语义用 `tokio_util::sync::CancellationToken` + `Drop` 兜底。

## 6. 关键风险与缓解

| 风险 | 缓解 |
|------|------|
| pre-1.0 破坏性变更 | 固定 `gpui-kit` 版本；GPUI API 只在 `agent-ui` crate 内部出现，Core 不感知 |
| 组件库版本绑定 GPUI 版本 | 通过 `gpui-kit` 单依赖引入，不直接写 `gpui` 到业务 crate |
| 长会话消息列表性能 | 用 `gpui-component` 虚拟列表；单条消息内部用 Element 级实现避免整树重建 |
| 流式 Markdown 渲染开销 | 增量只重渲染最后一条消息；代码块高亮做延迟（流结束后再高亮） |
| 平台差异（IME、字体） | Windows 优先实测 IME；中文输入在聊天输入框属高频路径，早期就验 |

## 7. 待办

- [ ] Windows 环境实测：`gpui-kit` hello world 与中文 IME 输入
- [ ] 实测流式 Markdown 增量渲染帧率
- [ ] 确认 `gpui-kit` 与目标 Rust 版本（≥ 1.95）匹配

## 相关

- 上游：`G1`
- 下游：`D1`
