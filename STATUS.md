# S1 · 当前阶段（概览）

| | |
|---|---|
| ID | `S1` |
| 类型 | 状态 · **概览** |
| 更新 | 2026-10-25 |
| 何时读 | **每次开工第一眼**，5 秒确认「现在做什么」 |
| 规模 | ~0.5k token |

> **本文件只写当前阶段的任务。** 任务完成 → 清空并重写；历史交给 git，长期排期见 `RM`。
> 任务跨多个 crate 时：**这里只给概览**，细节写在相关 crate 的 `status.md` 里，并从下表链过去。

## 当前任务

**设置保存后立即生效（B 方案）** ｜ 状态：✅ 完成（2026-10-25）

把「保存只写盘、重启后生效」改为「保存落盘 → worker 用**当前组合**重建 provider」
（重读磁盘：密钥 / 端点 / 代理 / 流空闲超时），复用 `Cmd::SelectModel` 的重建通路。

关键发现：worker 轮内不处理命令（`worker.rs` 的 `CancelSlot` 注释），生成中发来的命令
本来就**排队、轮末生效**——「生成中不许保存」的旧决策随之消解：保存总是允许，
生效自动延迟到轮末（与切模型等其它命令的排队语义一致），不需要禁用保存按钮、
不需要 pending 标志、不需要把忙闲信号穿到设置页。

| # | 切片 | 落点 | 状态 |
|---|------|------|------|
| 1 | 契约：`Cmd::ApplySettings` + `UiEvent::SettingsApplied` / `SettingsApplyFailed` | `features/chat` `channel.rs` | ✅ 2026-10-25 |
| 2 | worker 处理 `ApplySettings`：`factory.build(&current)` 重建（失败旧连接保留）；**故意不做 busy 检查**——排队即延迟 | `app/worker.rs` | ✅ 2026-10-25 |
| 3 | 设置页保存收尾发起生效（`ApplySettingsCallback` 回调）+ 三处「重启后生效」文案改掉 | `features/settings` | ✅ 2026-10-25 |
| 4 | 宿主接线：注入回调、`on_connected` 补发（重连当前条目立即生效）、事件桥回灌（候选重算抽成 `recompute_choices` 共用） | `app/gui.rs` | ✅ 2026-10-25 |
| 5 | 端到端测试 ×3：重建后下一轮可用 / 生成中排队轮末生效 / 失败旧连接可用 | `app/tui.rs` tests | ✅ 2026-10-25 |

顺带修一个真 bug：`ChatUi.generating` 从未被置 `true`（只有轮末各终点清 `false`）——
以 `push_user`（发 `Cmd::Submit` 前的既有调用）为轮首锚点置位。GUI 的生成中拦截
（切会话 / 新建 / 切模型三处守卫）与 TUI「退出前等本轮收尾」由此**恢复设计意图**。

本轮仍不做：`GUI` §3 的 L3 CSS 兼容层。

## 待办池（本轮之后）

| 候选 | 一句话 | 备注 |
|------|--------|------|
| `GUI` §3 · L3 CSS 层 | CSS 运行时语义子集，吃现成样式资产 | 体量大，四级递进、每级有停止条件 |
| `G-5` 打包分发 | MSI（`cargo-wix`）+ 体积核对 | 卡在 GPUI 体积，先量再决定（`S2` Q6） |
| `M7` MCP 客户端 | 自研最小 stdio 客户端，列工具 + 调工具 | `RM` 最后一个里程碑，曾按「使用频率不高」暂缓 |
| `G-4` 交互补齐 | 快捷键（Actions）、命令面板、多窗口 | `GUI` §2 唯一未动的一项 |

## 阻塞

无。

> **两件待人工**：① `tmp/gpui-spike` 的中文 IME 实测（`X2` §7 最后一条）；
> ② GUI 窗口的观感（Ely 重构 + 本轮改动后）—— 截屏在远程会话下取不到真实桌面，只能肉眼看。
> 另：本地有未推送的提交（`gh` token 失效后一直没推，`gh auth login -h github.com` 即可恢复）。

## 上一阶段

- **模型选择器改造（对齐 opencode 的交互，2026-10-13 → 10-21）**：切片 `A` → `C1c` + `C2a`/`C2b`/`C2c`/`C2d`
  全部落地——胶囊 + 搜索弹层、「管理模型」页 + 独立偏好文件 `models.toml`、内置供应商目录（`R5`）、
  条目化配置（用户只填厂商名）、「连接提供商」页、双适配器联网拉模型清单
  （`RefreshableModels`：OpenAI `GET /models` + Anthropic `GET /v1/models?limit=1000`，
  并集缓存、失败不清空）、worker 通道 + 逐条目刷新按钮。
- **Ely 组件库重构（2026-10-21）**：三界面组件层换成 Ely GPUI Component；`MessageList`
  虚拟列表解决 `G-3`（长会话性能）。
- **GPUI 界面走查（2026-10-15）**：色阶分层、消息层级、对比度修复；顺带修「SVG 资产源未注册
  → 图标全空白」与「`api_key_env` 把 Ollama 的密钥错发给 OpenAI」两个真问题。
- **运行中切模型 + 多份密钥（2026-10-12）**：`Cmd::SelectModel` / `UiEvent::ModelChanged` 按
  (provider, model) 组合切，worker 持 `ProviderFactory` 重建；GPUI 模型选择器；每供应商一行密钥。
- **二期 GUI 启动轮（2026-10-12）**：`D3` 方案 A 落地——探针、`kernel/ui-kit`、`features/chat`
  的 GPUI 渲染端（通道契约上移到切片）、`app` 的 `gui` 子命令、设置界面三件套。

## 相关

- 里程碑与排期定义 → `RM` ｜ 未决问题 → `S2` ｜ 地图 → [`AGENTS.md`](AGENTS.md) §2
