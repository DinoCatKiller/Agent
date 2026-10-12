# agent-settings · 功能切片：设置

| | |
|---|---|
| ID | —（功能切片，非索引文档） |
| 状态 | ✅ **A 语义落地**（2026-10-12）：设置模型 + 校验 + 读写编排 + 密钥录入 + GPUI 面板（`gui` feature）；⏳ 运行中热切换（B 方案）未做 |
| 边界 | **语义**：有哪些设置项、怎么校验、怎么呈现；密钥的录入体验 |
| 依赖 | `agent-config`（**机制**：文件与钥匙串的读写）｜`agent-ui-kit`（仅 `gui` feature，设置面板） |
| 不允许 | 直接读写钥匙串 / 配置文件；依赖 `app`；被 `kernel/*` 依赖 |
| 何时读 | 加设置项、改密钥录入流程、改默认模型 / 代理 / 主题时 |

## 这个切片是什么

`agent-config` 提供**机制**（配置从哪读、密钥往哪存），本切片提供**该有哪些设置项、
怎么校验、怎么呈现**。两者之间只有一条缝：`AppConfig`（文件格式）↔ `Settings`（产品语义）。

| 文件 | 职责 |
|------|------|
| [`src/settings.rs`](src/settings.rs) | `Settings`（5 个设置项）+ `Draft`（界面草稿）+ `validate()` |
| [`src/service.rs`](src/service.rs) | `SettingsService`：读取 / 写回 / 密钥增删查；`SecretBackend` 抽象 |
| [`src/ui.rs`](src/ui.rs) | GPUI 设置面板（**仅 `gui` feature**） |
| [`tests/settings.rs`](tests/settings.rs) | 6 例：校验、归一化、保住用户手写内容、坏文件不覆盖、密钥不落文件 |

## 四个必须知道的约束

1. **生效时机 = 下次启动**（`A` 方案，`S1` 切片 5 拍板）。界面只写盘，不做运行中热切换；
   界面上明说「重启后生效」。B 方案（保存后立即重建 provider）是给 `app` 留的接口，**尚未实现**；
   C 方案（轮内切换）已明确不做。
   > 这些设置项**真的有人读了**：`default_provider` / `default_model` / `providers.<p>.model`
   > 由 `app::setup::resolve` 消费（`R1` §1.1 的解析顺序）。此前它们只写不读 —— 因为 CLI 曾把
   > provider/model 设成必填，默认值永远轮不到，界面改了也不起作用（2026-10-12 修）。
2. **存的是「下一份」配置，不是「它的替换物」**：`save` 先读全量、只改本切片管的 5 个字段、
   再写回——`[providers.*]` 等本界面不管的内容原样保留（有专门用例守着）。
3. **读不动就不许写**。`app` 启动时读到坏配置可以退回默认继续跑；但设置界面**必须报错并禁用保存**，
   否则用户一按保存就把那个「读不动的文件」连同他手写的内容一起覆盖了。
4. **密钥只有一个去处**：`SecretString` → 钥匙串。明文不进配置文件（有用例断言）、不进日志、
   不进错误、不回显（界面只显示「来源：环境变量 / 钥匙串 / 未配置」）。
5. **密钥按供应商分行**：钥匙串账号本来就是 `<provider>:api_key`（`R1`），所以界面也一个供应商
   一行（`ProviderKeyRow`），保存时只写**填了值的那几行**，其余不动。供应商名单由宿主给
   （`app::setup::provider_ids`）——`features/*` 不认识供应商名单。
   `Draft` 里**不含密钥**：它是「一次性输入」，不是设置项。

## 用法

```rust
let service = SettingsService::discover()?;      // 平台标准目录 + 系统钥匙串
let settings = service.load()?;                  // 读当前设置
let draft = Draft::from_settings(&settings);
// ……界面改 draft……
let next = draft.to_settings()?;                 // 校验 + 归一化（空串 = 不设置该顶）
service.save(&next)?;                            // 写回（保住 [providers.*]）
service.save_key("anthropic", &SecretString::from(key))?;   // 密钥进钥匙串
```

测试**绝不碰真实钥匙串**：用 `SettingsService::with_parts(paths, Arc::new(MemoryBackend::new()))`。

## 它不能做什么

- ❌ **保存后立刻生效**（B 方案）。当前必须重启；`ChatService` 在 worker 里是一次性构造的。
- ❌ 设置运行中的对话时热切换模型 / 端点。
- ❌ 无钥匙串服务时的加密文件降级（`R1` 已识别该路径，未实现；别在这里自创一套）。
- ❌ 主题编辑：`design/tokens.json` 是设计资产（`D4`），不在设置界面里改。

## 相关

- 机制与硬约定 → `R1`（[`crates/kernel/config`](../../kernel/config/README.md) ｜ [docs](../../kernel/config/docs/config-and-secrets.md)）
- 组织决策 → `D5`、`D6` ｜ UI 形态与样式 → `D3`、`D4` ｜ 当前阶段 → `S1`
