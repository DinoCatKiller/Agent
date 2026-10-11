# S1 · 当前阶段（概览）

| | |
|---|---|
| ID | `S1` |
| 类型 | 状态 · **概览** |
| 更新 | 2026-10-11 |
| 何时读 | **每次开工第一眼**，5 秒确认「现在做什么」 |
| 规模 | ~0.5k token |

> **本文件只写当前阶段的任务。** 任务完成 → 清空并重写；历史交给 git，长期排期见 `RM`。
> 任务跨多个 crate 时：**这里只给概览**，细节写在相关 crate 的 `status.md` 里，并从下表链过去。

## 当前任务

**补规格文档**（四项中的最后一项）｜ 状态：🚧 进行中

一句话：把地图上仍为 ⬜ 的三份规格补齐 —— **先补文档**，代码落地随各自触发条件。

| ID | 路径 | 要写什么 |
|----|------|---------|
| `R3` | `crates/kernel/telemetry/docs/observability.md` | 日志 / trace / 计费；**是否同期新建 `kernel/telemetry` crate** 一并拍板 |
| `Q2` | `docs/eval.md` | 评测集与 prompt 回归：怎么建、跑什么、判据 |
| `Q3` | `docs/security.md` | 密钥、审计、数据合规边界（含 MCP server 可信度，`D8` 遗留） |

四项进度：`A6` ✅ → `R1` ✅ → `定名落地` ✅ → **本项 🚧**

## 阻塞

无。

## 上一阶段

- **定名落地（2026-10-11）**：只改用户可见那一层 —— 可执行文件名 `agent-app` → **`codingrocket`**（`[[bin]] name` + clap 显示名）；crate 包名维持 `agent-*`（内部技术名），`cargo run -p agent-app -- …` 不受影响。
- **`R1` 完成（2026-10-11）**：新建 `kernel/config`（TOML 配置文件 + 系统钥匙串）；`app` 接入 `--config` 与密钥三级优先级。踩到坑：`keyring` 3.x 的 default features 为空，须按平台显式开 backend，否则静默退化为 mock。
- **`A6` 完成（2026-10-11）**：候选内退避重试接进降级链（`Router::with_retry`），流式窗口仍是「首事件之前」；默认 `RetryPolicy::none()`。

## 相关

- 里程碑与排期定义 → `RM` ｜ 未决问题 → `S2` ｜ 地图 → [`AGENTS.md`](AGENTS.md) §2
