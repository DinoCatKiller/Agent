# R5 · 内置供应商目录

| | |
|---|---|
| ID | `R5` |
| 类型 | 规格（归属 `agent-providers`） |
| 状态 | ✅ 已落地（2026-10-13 `C1a` / `C1b`，2026-10-16 `C1c` 接上界面） |
| 更新 | 2026-10-16 |
| 何时读 | 加供应商、改端点、讨论「用户为什么还要填适配器名」时 |
| 规模 | ~1.5k token |

## TL;DR

- **厂商 ≠ 适配器**：适配器是实现（`openai-compatible` / `anthropic`），厂商是
  「适配器 + 默认端点 + 密钥变量名」的一组预设。**用户只填厂商名**（`ollama` / `openrouter`），
  适配器由目录给出。
- 表在 `crates/kernel/providers/src/catalog.rs`（`PRESETS`，编译期常量，不联网、不读文件）。
- 解析统一入口 `catalog::resolve(id, Declared)`：一次算清 **适配器 / 端点 / 密钥变量名 / 要不要密钥**，
  `build_provider`、模型清单、设置页三处共用 —— 不会出现「界面列得出来，但连不上」。
- **不猜**：目录里没有的 id 又没写 `adapter` → 报错，且文案要指路到目录里的名字。

## 1. 表里有什么

```rust
pub struct ProviderPreset {
    pub id: &'static str,             // 用户写的名字：default_provider / providers.<id>
    pub name: &'static str,           // 展示名（设置页那一行、chip）
    pub adapter: &'static str,        // 用哪个适配器实现
    pub base_url: &'static str,       // 默认端点
    pub api_key_env: Option<&'static str>,  // 约定环境变量名（None = 没有，只看钥匙串）
    pub key_required: bool,           // 通常要密钥吗（**只是界面提示**）
    pub docs: &'static str,           // 文档来源（核对端点用）
}
```

当前 12 条：`openai-compatible`、`anthropic`、`ollama`、`lmstudio`、`vllm`、
`openrouter`、`deepseek`、`groq`、`xai`、`moonshot`、`zhipu`、`siliconflow`。

**两个通用适配器 id 本身也在表里**（条目 id == 适配器 id）—— 于是老配置里那句
`default_provider = "openai-compatible"` 只是「id 命中目录」的一种情况，向后兼容不需要额外分支。

### 端点的可信度

每条都带 `docs`，值取自各家文档的 **OpenAI 兼容**地址。**目录写错不会编译失败**，
只会表现为"连不上" —— 接入某家之前按 `docs` 核对一次（`X2` 的口径：不信记录，信实测）。

## 2. `resolve` 的三条规则

| 项 | 顺序 |
|----|------|
| adapter | 配置里显式写的 `adapter` > 目录推断（目录 id / 适配器 id）> **报错** |
| base_url | `--base-url` > `providers.<id>.base_url` > 顶层 `base_url` > 目录的 `base_url` > 适配器官方端点 |
| api_key_env | **目录里有的条目以目录为准**（`None` 就是"没有约定名"）；只有目录里没有的自定义条目才借它所适配器的名字 |

第三行是修过的 bug（2026-10-13）：原先写成 `条目 > 适配器` 的简单回退，
导致 `ollama` 的密钥去读 `OPENAI_API_KEY` —— 等于把你的 OpenAI 密钥发给 ollama.com。
触发它的正是"裸敲 `codingrocket gui`"那条路径（提示文案里写着变量名）。

### `custom_base` 的含义

`base_url != 该适配器的官方端点` 为真时，我们**不列官方内置清单**（`gpt-4o` 对 vLLM 没有意义），
也**不做能力 fast-fail** —— `models` 只由配置里为该条目声明的 id（`models` / `model` /
顶层 `default_model` / 本次选中）组成，且未知 id 一律是 `capabilities_known = false` 的保守占位，
不拦能力。其余交给服务端报错 —— 这就是 `R2` 的「不拦清单」。于是 `ollama`（`localhost:11434`）
与 `openai-compatible`（`api.openai.com`）行为天然不同，不需要额外的「本地/云端」维度。

## 3. 与 `D7` 的关系

`D7` 决定「Ollama 零特化、不设本地节点、不接原生 API」。本目录**不推翻它**：

- 没有新增注册表节点类型，没有「本地 vs 云端」维度进入路由或校验；
- `key_required` / `is_local` **只是界面提示位**（本地服务不该催用户填密钥），
  不参与任何查找或校验；
- `ollama` 只是「`openai-compatible` + 某个端点」的名字层糖。

## 4. 自定义条目

目录里没有的 id 必须自己写 `adapter`：

```toml
[providers.my-vllm]
adapter = "openai-compatible"
base_url = "https://my-server.example/v1"
model = "qwen3-32b"
```

密钥变量名会回退到该适配器的约定名（`adapter = "anthropic"` → `ANTHROPIC_API_KEY`）；
`key_required` 按「端点是本地地址吗」猜（`localhost` / `127.0.0.1` / `[::1]` / `0.0.0.0` → 不需要）。

## 5. 已知取舍

- **`provider.id()` 仍返回适配器 id**（`ollama` 的 provider 实例 id 是 `openai-compatible`）：
  它是 `&'static str`，而条目 id 是运行期字符串。只影响路由/观测的标签，不参与查找。
- 设置页只列「已在用的条目」（两个适配器 ∪ 配置里连接过的 ∪ 当前生效的），
  不列整张目录 —— 整张目录由「连接提供商」页列（`C1c`，2026-10-16 落地）。
- **目录 → 界面的映射在 `app`**（`setup::catalog_rows`，`D6`）：`features/settings` 只认
  `CatalogRow` 这个结构，不认识 `PRESETS`。连接动作 = `SettingsService::connect_provider`
  写**一个** `[providers.<id>]` 条目（先读全量再写回，其余内容原样保留）+ `save_key` 写钥匙串；
  写成功后宿主重算候选并推给对话页（`ChatView::set_candidates`），**不用重启**。
- 目录写进 `config.toml` 的 `providers.<id>` 时用了 `deny_unknown_fields` 的老规矩：
  `adapter` 字段是**新键**，旧版本二进制读到会解析失败（`R1` §2 的单向兼容取舍，
  与 `models.toml` 相反 —— 那个选择容忍未知键）。

## 相关

- 配置与密钥优先级 → `R1` §1｜模型清单与路由 → `R2`
- 适配器契约与两家差异 → `A2` / `P1` / `P2`
- Ollama 与本地模型的定位 → `D7`｜分层判据（机制 vs 语义） → `D6` §1
- 门面 → `crates/kernel/providers/README.md`
