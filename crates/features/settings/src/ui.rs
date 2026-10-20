//! GPUI 设置面板（`gui` feature）。
//!
//! 语义按 `A` 方案：**保存只写盘，下次启动生效**（`S1` 切片 5 拍板）。界面因此只做三件事：
//! 读出来显示、校验后写回、把「需要重启」讲清楚。
//!
//! 两条安全线（`R1` / 硬约束 5）：
//! - 已存的密钥**不回显**，界面只显示「来源」；输入框里是本次要写入的新值。
//! - 读不动配置文件时（`load` 报错）**禁止保存** —— 否则一按保存就把用户手写的内容覆盖掉。

use std::rc::Rc;
use std::sync::Arc;

use agent_config::{Preferences, SecretString};
use agent_ui_kit::UiTheme;
use agent_ui_kit::component::button::{Button, ButtonVariants as _};
use agent_ui_kit::component::input::{Input, InputEvent, InputState};
use agent_ui_kit::component::switch::Switch;
use agent_ui_kit::component::{Icon, IconName};
use agent_ui_kit::kit::prelude::FluentBuilder as _;
use agent_ui_kit::kit::*;

use crate::service::{KeySource, SettingsService};
use crate::settings::Draft;

/// 面板内的分区。
///
/// 默认「通用」；从对话页那个胶囊里的「管理模型」进来时直接落在「模型」
/// （宿主调 [`SettingsView::show_models`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    General,
    Models,
    Connect,
}

/// 「刷新模型」回调（`C2d`）：宿主接到它后把 `Cmd::RefreshModels` 发回 worker。
///
/// 提成别名只是为了让字段与注入点的签名不撞 clippy 的 `type_complexity`；语义见字段注释。
pub type RefreshModelsCallback = Rc<dyn Fn(&str, &mut App)>;

/// 面板状态。
pub struct SettingsView {
    service: Arc<SettingsService>,
    /// 当前分区。
    page: Page,
    provider: Entity<InputState>,
    model: Entity<InputState>,
    base_url: Entity<InputState>,
    proxy: Entity<InputState>,
    timeout: Entity<InputState>,
    /// **每个供应商一行密钥**：密钥在钥匙串里本来就是按 provider 存的
    /// （账号 `<provider>:api_key`，`R1`），所以界面也按 provider 分行。
    keys: Vec<ProviderKeyRow>,
    /// 可配密钥 / 可当默认供应商的条目（宿主给的）。
    ///
    /// 为什么用 chip 选「默认供应商」：它**真的生效**（`R1` §1.1），
    /// 手滑写出未知名字会让 `gui`（没有命令行参数的启动方式）直接起不来。
    providers: Vec<ProviderRow>,
    /// 全部候选模型（宿主编排给的）——「模型」页要看到**全量**才能把关掉的再打开。
    models: Vec<(String, String)>,
    /// 「模型」页的搜索框。
    model_search: Entity<InputState>,
    /// 可连接的内置目录条目（宿主给的，`D6`）——「连接提供商」页列它。
    catalog: Vec<CatalogRow>,
    /// 「连接提供商」页的搜索框。
    connect_search: Entity<InputState>,
    /// 当前选中、准备连接的目录条目 id。
    connect_selected: Option<String>,
    connect_base_url: Entity<InputState>,
    connect_model: Entity<InputState>,
    connect_key: Entity<InputState>,
    /// 连接成功后通知宿主刷新模型选择器 —— 跨功能的线只由宿主接（`D6`）。
    ///
    /// 建本视图时对话页还没建出来，所以先给空回调，宿主随后用 [`Self::set_on_connected`] 注入。
    on_connected: Rc<dyn Fn(&mut App)>,
    /// 「刷新模型」回调（`C2d`）：宿主把它转成 `Cmd::RefreshModels` 发回 worker。
    ///
    /// 与 [`Self::on_connected`] 同一条规矩：先给空回调，宿主用
    /// [`Self::set_on_refresh_models`] 注入 —— `features/*` 不碰命令通道（`D6`）。
    on_refresh_models: RefreshModelsCallback,
    /// 偏好快照（哪些模型被关掉了）。开关是**受控**的：点一下 → 落盘 → 更新这里。
    hidden: Preferences,
    /// `Some` = 配置文件读不动（坏 TOML 等）：此时**禁止保存**。
    load_error: Option<String>,
    status: Option<Status>,
    config_file: String,
    preferences_file: String,
    /// 订阅必须持有，drop 即取消订阅。
    _search_sub: Subscription,
    _connect_sub: Subscription,
}

/// 设置页里的一个供应商条目 —— **宿主构造**（`features/*` 不认识供应商目录，`D6`）。
///
/// `app` 从 `agent_providers::catalog`（预设厂商）+ 配置文件里已连接的条目拼出来。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRow {
    /// 条目 id：钥匙串账号、`default_provider` 用它（`<id>:api_key`，`R1`）。
    pub id: String,
    /// 展示名（目录里的 `name`，自定义条目就是 id）。
    pub name: String,
    /// 约定环境变量名。`None` = 没有约定名（本地服务 / 自定义厂商）→ 只看钥匙串。
    pub env: Option<String>,
    /// 是否必须有密钥 —— 只用于提示文案：本地服务不该催用户去填。
    pub key_required: bool,
}

/// 「连接提供商」页要展示的一条内置目录条目 —— **宿主构造**（`D6`）。
///
/// `app` 从 `agent_providers::catalog::PRESETS` 映射出来；本切片不认识供应商目录。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogRow {
    /// 条目 id，写进 `[providers.<id>]` 与钥匙串账号。
    pub id: String,
    pub name: String,
    /// 用哪个适配器（`openai-compatible` / `anthropic`）—— 由目录决定，用户不用管。
    pub adapter: String,
    /// 内置端点（预填到表单，可改）。
    pub base_url: String,
    /// 约定环境变量名（`None` = 本地服务或不带约定名）。
    pub env: Option<String>,
    pub key_required: bool,
}

/// 一个供应商的密钥行。
struct ProviderKeyRow {
    row: ProviderRow,
    /// 本次要写入的新密钥（明文只活在输入框与那一次调用里；**已存值不回显**）。
    input: Entity<InputState>,
    /// 生效来源：环境变量 > 钥匙串 > 未配置（`R1`）。
    source: KeySource,
}

struct Status {
    message: String,
    error: bool,
}

impl SettingsView {
    /// `providers` / `models` 都由宿主给（`features/*` 不认识供应商名单，`D6`）。
    ///
    /// `models` 是**全量**候选（含被关掉的）：关掉的也得列出来，否则用户没法再打开。
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        service: Arc<SettingsService>,
        providers: Vec<ProviderRow>,
        models: Vec<(String, String)>,
        catalog: Vec<CatalogRow>,
        on_connected: Rc<dyn Fn(&mut App)>,
    ) -> Self {
        let (draft, load_error) = match service.load() {
            Ok(settings) => (Draft::from_settings(&settings), None),
            Err(error) => (Draft::default(), Some(error.to_string())),
        };

        // 逐个显式建输入框，别用「捕获 `window` 的闭包」批量建 —— 那样会和下面循环里的闭包
        // 同时要求 `*window` 的唯一访问权（E0524）。
        let provider_input =
            cx.new(|cx| InputState::new(window, cx).default_value(draft.default_provider.clone()));
        let model_input =
            cx.new(|cx| InputState::new(window, cx).default_value(draft.default_model.clone()));
        let base_url_input =
            cx.new(|cx| InputState::new(window, cx).default_value(draft.base_url.clone()));
        let proxy_input =
            cx.new(|cx| InputState::new(window, cx).default_value(draft.proxy.clone()));
        let timeout_input = cx.new(|cx| {
            InputState::new(window, cx).default_value(draft.stream_idle_timeout_secs.clone())
        });

        let provider_options = providers.clone();
        let mut keys = Vec::new();
        for row in providers {
            let source = service
                .key_source(&row.id, row.env.as_deref())
                .unwrap_or(KeySource::None);
            let key_input = cx.new(|cx| {
                InputState::new(window, cx)
                    .masked(true)
                    .placeholder("粘贴新密钥（已存的不回显）")
            });
            keys.push(ProviderKeyRow {
                row,
                input: key_input,
                source,
            });
        }

        // 「模型」页的搜索框 + 其订阅（内容一变就重渲染 → 重新过滤）。
        let model_search = cx.new(|cx| InputState::new(window, cx).placeholder("搜索模型…"));
        let search_sub = cx.subscribe_in(
            &model_search,
            window,
            |_this, _state, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            },
        );

        // 「连接提供商」页的搜索框与表单输入框：和上面一样，渲染期不新造 entity。
        let connect_search = cx.new(|cx| InputState::new(window, cx).placeholder("搜索厂商…"));
        let connect_sub = cx.subscribe_in(
            &connect_search,
            window,
            |_this, _state, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            },
        );
        let connect_base_url = cx.new(|cx| InputState::new(window, cx));
        let connect_model = cx.new(|cx| InputState::new(window, cx));
        let connect_key = cx.new(|cx| {
            InputState::new(window, cx)
                .masked(true)
                .placeholder("粘贴密钥（存系统钥匙串，不回显）")
        });

        let config_file = service.config_file().display().to_string();
        let preferences_file = service.preferences_file().display().to_string();
        // 偏好**降级不报错**（`agent_config::prefs` 的口径）：坏了就当没关过任何模型。
        let hidden = service.preferences();

        Self {
            service,
            page: Page::General,
            provider: provider_input,
            model: model_input,
            base_url: base_url_input,
            proxy: proxy_input,
            timeout: timeout_input,
            keys,
            providers: provider_options,
            models,
            model_search,
            catalog,
            connect_search,
            connect_selected: None,
            connect_base_url,
            connect_model,
            connect_key,
            on_connected,
            on_refresh_models: Rc::new(|_, _| {}),
            hidden,
            load_error,
            status: None,
            config_file,
            preferences_file,
            _search_sub: search_sub,
            _connect_sub: connect_sub,
        }
    }

    /// 切到「模型」页（宿主在对话页点「管理模型」时调）。
    pub fn show_models(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.page = Page::Models;
        self.status = None;
        // 重新读一次：别的地方（手改文件）动过偏好，这里要跟上。
        self.hidden = self.service.preferences();
        self.model_search
            .update(cx, |state, cx| state.set_value("", window, cx));
        cx.notify();
    }

    /// 切回「通用」页。
    fn show_general(&mut self, cx: &mut Context<Self>) {
        self.page = Page::General;
        cx.notify();
    }

    /// 切到「连接提供商」页。
    fn show_connect(&mut self, cx: &mut Context<Self>) {
        self.page = Page::Connect;
        self.connect_selected = None;
        self.status = None;
        cx.notify();
    }

    /// 连接成功后由宿主注入回调（建视图时对话页还没建出来，只能后接）。
    pub fn set_on_connected(&mut self, on_connected: Rc<dyn Fn(&mut App)>) {
        self.on_connected = on_connected;
    }

    /// 由宿主注入「刷新模型」回调（`C2d`）：本视图只发起，拉取与写回都在 worker 侧。
    pub fn set_on_refresh_models(&mut self, on_refresh_models: RefreshModelsCallback) {
        self.on_refresh_models = on_refresh_models;
    }

    /// 宿主把刷新后的**全量**候选灌回来（`C2d`，收 `ModelsRefreshed` 后）。
    ///
    /// 传全量而非可见集：这一页要看到所有模型才能把关掉的再打开。
    pub fn set_models(&mut self, models: Vec<(String, String)>) {
        self.models = models;
    }

    /// 刷新结果的提示（成功 / 失败都由宿主转达，`C2d`）。
    pub fn set_status(&mut self, message: String, error: bool) {
        self.status = Some(Status { message, error });
    }

    /// 点「刷新模型」：先把状态置成「进行中」，再把命令交给宿主（`on_refresh_models`）。
    ///
    /// 结果不由本视图猜 —— worker 拉完发 `ModelsRefreshed` / `ModelsRefreshFailed`，
    /// 宿主回灌 [`Self::set_models`] / [`Self::set_status`]。
    fn request_refresh(&mut self, provider: String, cx: &mut Context<Self>) {
        self.status = Some(Status {
            message: format!("正在刷新 {provider} 的模型清单…"),
            error: false,
        });
        (self.on_refresh_models.clone())(&provider, cx);
        cx.notify();
    }

    /// 选中一条目录条目：预填内置端点，清空上一轮的模型与密钥输入。
    fn select_connect_row(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(row) = self.catalog.iter().find(|row| row.id == id).cloned() else {
            return;
        };
        self.connect_selected = Some(id);
        self.connect_base_url
            .update(cx, |state, cx| state.set_value(row.base_url, window, cx));
        self.connect_model
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.connect_key
            .update(cx, |state, cx| state.set_value("", window, cx));
        cx.notify();
    }

    /// 连接：写 `[providers.<id>]`（+ 密钥进钥匙串），然后通知宿主刷新模型选择器。
    ///
    /// 与 [`Self::save`] 一致：只增改**这一个**条目，用户手写的其它内容原样保留。
    fn connect(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.connect_selected.clone() else {
            return;
        };
        let Some(row) = self.catalog.iter().find(|row| row.id == id).cloned() else {
            return;
        };

        let base_url = self.connect_base_url.read(cx).value().trim().to_string();
        let model = self.connect_model.read(cx).value().trim().to_string();
        let key = self.connect_key.read(cx).value().trim().to_string();
        let base = (!base_url.is_empty()).then_some(base_url.as_str());
        let mdl = (!model.is_empty()).then_some(model.as_str());

        if let Err(error) = self.service.connect_provider(&id, &row.adapter, base, mdl) {
            self.status = Some(Status {
                message: format!("写配置失败：{error}"),
                error: true,
            });
            cx.notify();
            return;
        }

        let mut note = String::new();
        let mut key_error = None;
        if !key.is_empty() {
            match self.service.save_key(&id, &SecretString::from(key)) {
                Ok(()) => note = "，密钥已存入钥匙串".to_string(),
                Err(error) => {
                    // 配置条目**已经**写进去了（上面成功）——不能因为密钥失败就整体当「没连上」：
                    // 否则配置里已有该条目、界面却看不到，用户会以为要重启。
                    note = "，但密钥保存失败".to_string();
                    key_error = Some(error);
                }
            }
        }

        // 无论如何都通知宿主重算候选（`D6`：worker / 宿主才是候选的真相源）。
        self.remember_connected(&row, window, cx);
        self.connect_selected = None;
        self.connect_key
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.status = Some(match key_error {
            Some(error) => Status {
                message: format!("已连接 {}{note}：{error}。可稍后重试密钥。", row.name),
                error: true,
            },
            None => Status {
                message: format!("已连接 {}{note}，已加入模型选择器", row.name),
                error: false,
            },
        });
        (self.on_connected.clone())(cx);
        cx.notify();
    }

    /// 把刚连接的条目就地补进界面（供应商芯片 / 密钥行无需重启即可见）。
    fn remember_connected(
        &mut self,
        row: &CatalogRow,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.providers.iter().any(|item| item.id == row.id) {
            self.refresh_key_sources();
            return;
        }
        let provider = ProviderRow {
            id: row.id.clone(),
            name: row.name.clone(),
            env: row.env.clone(),
            key_required: row.key_required,
        };
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .masked(true)
                .placeholder("粘贴新密钥（已存的不回显）")
        });
        self.providers.push(provider.clone());
        self.keys.push(ProviderKeyRow {
            row: provider,
            input,
            source: KeySource::None,
        });
        self.refresh_key_sources();
    }

    /// 开关一个模型是否出现在选择器里。**立即落盘**（偏好文件很小，且不存在覆盖用户内容的风险）。
    ///
    /// 界面**不做乐观更新**：落盘成功才改本地快照，失败就保持原样并报错 ——
    /// 否则"开关弹回去了但文件里已经写进去"这种状态最难查。
    fn toggle_model(
        &mut self,
        provider: String,
        model: String,
        visible: bool,
        cx: &mut Context<Self>,
    ) {
        match self.service.set_model_hidden(&provider, &model, !visible) {
            Ok(()) => {
                self.hidden = self.service.preferences();
                self.status = Some(Status {
                    message: if visible {
                        format!("已显示 {model}")
                    } else {
                        format!("已隐藏 {model}（对话页的模型菜单里不再出现）")
                    },
                    error: false,
                });
            }
            Err(error) => {
                self.status = Some(Status {
                    message: format!("写偏好失败：{error}"),
                    error: true,
                });
            }
        }
        cx.notify();
    }

    /// 用 chip 选默认供应商（写进输入框，保存时随其余配置一起落盘）。
    fn pick_provider(&mut self, provider: String, window: &mut Window, cx: &mut Context<Self>) {
        self.provider
            .update(cx, |state, cx| state.set_value(provider, window, cx));
        cx.notify();
    }

    /// 重新从磁盘读（切到本页、或用户手改过文件之后）。
    pub fn reload(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.status = None;
        let settings = match self.service.load() {
            Ok(settings) => {
                self.load_error = None;
                settings
            }
            Err(error) => {
                self.load_error = Some(error.to_string());
                return;
            }
        };
        let draft = Draft::from_settings(&settings);
        let pairs: [(Entity<InputState>, String); 5] = [
            (self.provider.clone(), draft.default_provider),
            (self.model.clone(), draft.default_model),
            (self.base_url.clone(), draft.base_url),
            (self.proxy.clone(), draft.proxy),
            (self.timeout.clone(), draft.stream_idle_timeout_secs),
        ];
        for (state, value) in pairs {
            state.update(cx, |state, cx| state.set_value(value, window, cx));
        }
        self.refresh_key_sources();
        cx.notify();
    }

    /// 保存：校验 → 写配置 → （填了新密钥才）写钥匙串。
    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(error) = &self.load_error {
            self.status = Some(Status {
                message: format!("配置文件读不动，先修好再保存：{error}"),
                error: true,
            });
            cx.notify();
            return;
        }

        let draft = Draft {
            default_provider: self.provider.read(cx).value().to_string(),
            default_model: self.model.read(cx).value().to_string(),
            base_url: self.base_url.read(cx).value().to_string(),
            proxy: self.proxy.read(cx).value().to_string(),
            stream_idle_timeout_secs: self.timeout.read(cx).value().to_string(),
        };

        let settings = match draft.to_settings() {
            Ok(settings) => settings,
            Err(errors) => {
                let list = errors
                    .iter()
                    .map(|error| error.to_string())
                    .collect::<Vec<_>>()
                    .join("；");
                self.status = Some(Status {
                    message: format!("有 {list}"),
                    error: true,
                });
                cx.notify();
                return;
            }
        };

        if let Err(error) = self.service.save(&settings) {
            self.status = Some(Status {
                message: format!("写配置失败：{error}"),
                error: true,
            });
            cx.notify();
            return;
        }

        // 逐供应商写密钥：**只写填了值的那几行**（空 = 不动它现在那把）
        let mut key_note = String::new();
        for index in 0..self.keys.len() {
            let (provider, state) = {
                let row = &self.keys[index];
                (row.row.id.clone(), row.input.clone())
            };
            let raw = state.read(cx).value().trim().to_string();
            if raw.is_empty() {
                continue;
            }
            match self.service.save_key(&provider, &SecretString::from(raw)) {
                Ok(()) => {
                    key_note = "，密钥已存入系统钥匙串".to_string();
                    state.update(cx, |state, cx| state.set_value("", window, cx));
                }
                Err(error) => {
                    self.status = Some(Status {
                        message: format!("保存 {provider} 的密钥失败：{error}"),
                        error: true,
                    });
                    cx.notify();
                    return;
                }
            }
        }

        self.refresh_key_sources();
        self.status = Some(Status {
            message: format!("已保存{key_note}（**重启后生效**）"),
            error: false,
        });
        cx.notify();
    }

    /// 清掉某个供应商在钥匙串里的密钥（环境变量那份不受影响，且优先级更高）。
    fn clear_key(&mut self, provider: String, cx: &mut Context<Self>) {
        match self.service.delete_key(&provider) {
            Ok(()) => {
                self.status = Some(Status {
                    message: format!("已清除 {provider} 的钥匙串密钥"),
                    error: false,
                });
                self.refresh_key_sources();
            }
            Err(error) => {
                self.status = Some(Status {
                    message: format!("清除 {provider} 失败：{error}"),
                    error: true,
                });
            }
        }
        cx.notify();
    }

    /// 刷新每行的密钥来源（保存 / 清除 / 重新载入后都要跟一下）。
    fn refresh_key_sources(&mut self) {
        for row in &mut self.keys {
            row.source = self
                .service
                .key_source(&row.row.id, row.row.env.as_deref())
                .unwrap_or(KeySource::None);
        }
    }
}

// —— 渲染 ——

impl SettingsView {
    /// 「模型」页：搜索 + 按已连接条目分组（每组一个「刷新」按钮，`C2d`）+ 每个模型一个开关
    /// （控制它是否出现在对话页的选择器里）。
    ///
    /// 关掉是**加法语义**：偏好文件里只记被隐藏的那些，新模型默认可见。
    /// 「刷新」联网拉取该端点的模型清单并写回配置 —— 结果由宿主回灌（`on_refresh_models` + `set_models`）。
    fn models_page(&self, cx: &mut Context<Self>, fg: Hsla, dim: Hsla, err: Hsla) -> AnyElement {
        let query = self.model_search.read(cx).value().trim().to_lowercase();
        // 按**已连接条目**分组（不是按候选清单）：刚连上、还没拉到模型时也要能点「刷新」。
        let mut rows = div().flex().flex_col().gap_1();
        let mut shown = 0usize;
        for provider in &self.providers {
            let provider_hit = query.is_empty()
                || provider.id.to_lowercase().contains(&query)
                || provider.name.to_lowercase().contains(&query);
            // 该条目的候选：去重 + 按查询过滤（条目名命中时整组放行）。
            let mut models: Vec<String> = Vec::new();
            for (p, model) in &self.models {
                if p != &provider.id {
                    continue;
                }
                if !provider_hit && !model.to_lowercase().contains(&query) {
                    continue;
                }
                if !models.iter().any(|it| it == model) {
                    models.push(model.clone());
                }
            }
            if !query.is_empty() && !provider_hit && models.is_empty() {
                continue;
            }
            shown += 1;

            // 组头：条目名 + 「刷新」按钮（联网拉这个端点的模型清单，C2d）。
            let header_provider = provider.id.clone();
            rows = rows.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .pt_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(dim)
                            .child(format!("{} / {}", provider.name, provider.id)),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new(ElementId::Name(
                            format!("refresh-models::{}", provider.id).into(),
                        ))
                        .ghost()
                        .compact()
                        .icon(IconName::RefreshCw)
                        .label("刷新")
                        .tooltip("联网拉取该条目的模型清单（写回配置；失败不动现有清单）")
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.request_refresh(header_provider.clone(), cx)
                        })),
                    ),
            );

            if models.is_empty() {
                rows = rows.child(
                    div()
                        .text_xs()
                        .text_color(dim)
                        .child("（还没有已知模型：点「刷新」联网拉取，或连接时手填）"),
                );
            }
            for model in models {
                let visible = !self.hidden.is_hidden(&provider.id, &model);
                let (row_provider, row_model) = (provider.id.clone(), model.clone());
                rows = rows.child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_3()
                        .child(div().flex_1().truncate().child(model.clone()))
                        .child(
                            Switch::new(ElementId::Name(
                                format!("model-switch::{}::{}", provider.id, model).into(),
                            ))
                            .checked(visible)
                            .on_change(cx.listener(
                                move |this, wanted: &bool, _, cx| {
                                    this.toggle_model(
                                        row_provider.clone(),
                                        row_model.clone(),
                                        *wanted,
                                        cx,
                                    )
                                },
                            )),
                        ),
                );
            }
        }
        if shown == 0 {
            rows = rows.child(div().py_2().text_color(dim).child("没有匹配的模型"));
        }

        let hidden_count: usize = self.hidden.hidden_models.values().map(Vec::len).sum();

        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .size_full()
            .max_w(px(560.))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::new("models-back")
                            .ghost()
                            .compact()
                            .icon(IconName::ChevronLeft)
                            .label("设置")
                            .on_click(cx.listener(|this, _, _, cx| this.show_general(cx))),
                    )
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(fg)
                            .child("管理模型"),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("connect-provider")
                            .outline()
                            .icon(IconName::Plus)
                            .label("连接提供商")
                            .tooltip("从内置目录挑一个厂商，填密钥即可")
                            .on_click(cx.listener(|this, _, _, cx| this.show_connect(cx))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .text_xs()
                    .text_color(dim)
                    .child("自定义模型选择器中显示的模型。")
                    .child("「刷新」联网拉取该端点的模型清单（新模型默认显示）。")
                    .child(format!(
                        "偏好文件：{}（已隐藏 {hidden_count} 个）",
                        self.preferences_file
                    )),
            )
            .child(
                Input::new(&self.model_search)
                    .prefix(Icon::new(IconName::Search).text_color(dim))
                    .cleanable(true),
            )
            .child(rows)
            .children(self.status.as_ref().map(|status| {
                div()
                    .text_xs()
                    .text_color(if status.error { err } else { fg })
                    .child(status.message.clone())
            }))
            .into_any_element()
    }

    /// 「连接提供商」页：从内置目录里搜、选中、填密钥，一次写一个条目。
    ///
    /// 只写非敏感项 + 钥匙串（`R1`）；写成功后调 [`SettingsView::connect`] 里的宿主回调
    /// 刷新模型选择器，**不用重启**。
    #[allow(clippy::too_many_arguments)]
    fn connect_page(
        &self,
        cx: &mut Context<Self>,
        fg: Hsla,
        dim: Hsla,
        err: Hsla,
        accent: Hsla,
        secondary: Hsla,
    ) -> AnyElement {
        let query = self.connect_search.read(cx).value().trim().to_lowercase();
        let mut list = div().flex().flex_col().gap_1();
        for row in &self.catalog {
            let hit = query.is_empty()
                || row.id.to_lowercase().contains(&query)
                || row.name.to_lowercase().contains(&query);
            if !hit {
                continue;
            }
            let connected = self.providers.iter().any(|item| item.id == row.id);
            let selected = self.connect_selected.as_deref() == Some(row.id.as_str());
            let id = row.id.clone();
            let meta = format!("{} · {}", row.adapter, row.base_url);
            list = list.child(
                div()
                    .id(ElementId::Name(format!("catalog-{}", row.id).into()))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .cursor_pointer()
                    .when(selected, |item| item.bg(secondary))
                    .hover(|item| item.bg(secondary))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.select_connect_row(id.clone(), window, cx)
                    }))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w(px(0.))
                            .child(div().text_color(fg).child(row.name.clone()))
                            .child(div().text_xs().text_color(dim).truncate().child(meta)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(if connected { accent } else { dim })
                            .child(if connected { "已连接" } else { "未连接" }),
                    ),
            );
        }
        if self.catalog.is_empty() {
            list = list.child(div().py_2().text_color(dim).child("目录为空"));
        }

        let form = self
            .connect_selected
            .as_ref()
            .and_then(|id| self.catalog.iter().find(|row| &row.id == id).cloned())
            .map(|row| {
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(dim)
                            .child(format!("连接 {}（适配器 {}）", row.name, row.adapter)),
                    )
                    .child(field(
                        "Base URL（留空用内置默认）",
                        &self.connect_base_url,
                        dim,
                    ))
                    .child(field(
                        "模型（可选，自定义端点时填）",
                        &self.connect_model,
                        dim,
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(dim)
                                    .child(connect_key_hint(&row)),
                            )
                            .child(Input::new(&self.connect_key)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .child(Button::new("connect-do").primary().label("连接").on_click(
                                cx.listener(|this, _, window, cx| this.connect(window, cx)),
                            ))
                            .child(
                                Button::new("connect-cancel")
                                    .ghost()
                                    .label("取消")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.connect_selected = None;
                                        cx.notify();
                                    })),
                            ),
                    )
            });

        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .size_full()
            .max_w(px(560.))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::new("connect-back")
                            .ghost()
                            .compact()
                            .icon(IconName::ChevronLeft)
                            .label("设置")
                            .on_click(cx.listener(|this, _, _, cx| this.show_general(cx))),
                    )
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(fg)
                            .child("连接提供商"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .text_xs()
                    .text_color(dim)
                    .child("从内置目录里挑一个厂商，填密钥即可；适配器由目录决定，你不用管。")
                    .child(format!("配置文件：{}", self.config_file)),
            )
            .child(
                Input::new(&self.connect_search)
                    .prefix(Icon::new(IconName::Search).text_color(dim))
                    .cleanable(true),
            )
            .child(list)
            .children(form)
            .children(self.status.as_ref().map(|status| {
                div()
                    .text_xs()
                    .text_color(if status.error { err } else { fg })
                    .child(status.message.clone())
            }))
            .into_any_element()
    }
}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = UiTheme::global(cx);
        let (fg, dim, err, border, accent, secondary) = (
            theme.colors.foreground,
            theme.colors.muted_foreground,
            theme.colors.destructive,
            theme.colors.border,
            theme.colors.accent,
            theme.colors.secondary,
        );

        let mut form = div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .max_w(px(560.))
            .child(self.provider_field(cx, dim, accent, border))
            .child(field("默认模型", &self.model, dim))
            .child(field("Base URL（留空用内置默认）", &self.base_url, dim))
            .child(field("代理（http/https/socks5）", &self.proxy, dim))
            .child(field("流式空闲超时（秒）", &self.timeout, dim));

        // 密钥：**每个供应商一行**（钥匙串里本来就是按 provider 存的）。只显示来源，不回显已存值
        form = form.child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .text_xs()
                .text_color(dim)
                .child("API Key（每行单独保存；留空 = 不动它现在那把）"),
        );
        for index in 0..self.keys.len() {
            let (provider, name, env, required, state, source) = {
                let row = &self.keys[index];
                (
                    row.row.id.clone(),
                    row.row.name.clone(),
                    row.row.env.clone(),
                    row.row.key_required,
                    row.input.clone(),
                    row.source,
                )
            };
            form = form.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    // 第一行：展示名 + id + 清除。hint 一句话太长，独占第三行。
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .child(div().text_sm().text_color(fg).child(name))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(dim)
                                    .child(format!("({provider})")),
                            )
                            .child(div().flex_1())
                            .child(
                                Button::new(ElementId::Name(
                                    format!("clear-key-{provider}").into(),
                                ))
                                .ghost()
                                .compact()
                                .label("清除")
                                .on_click(cx.listener(
                                    move |this, _, _, cx| this.clear_key(provider.clone(), cx),
                                )),
                            ),
                    )
                    .child(Input::new(&state))
                    .child(div().text_xs().text_color(dim).child(key_hint(
                        source,
                        env.as_deref(),
                        required,
                    ))),
            );
        }

        form = form.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    Button::new("save-settings")
                        .label("保存")
                        .on_click(cx.listener(|this, _, window, cx| this.save(window, cx))),
                )
                .child(
                    Button::new("reload-settings")
                        .label("重新载入")
                        .on_click(cx.listener(|this, _, window, cx| this.reload(window, cx))),
                )
                .child(
                    // 模型分区也放个入口：不然它只能在对话页的胶囊弹层里找到
                    Button::new("open-models")
                        .ghost()
                        .icon(IconName::Settings2)
                        .label("管理模型")
                        .on_click(cx.listener(|this, _, window, cx| this.show_models(window, cx))),
                )
                .child(
                    Button::new("connect-provider-general")
                        .ghost()
                        .icon(IconName::Plus)
                        .label("连接提供商")
                        .on_click(cx.listener(|this, _, _, cx| this.show_connect(cx))),
                ),
        );

        if let Some(status) = &self.status {
            form = form.child(
                div()
                    .text_xs()
                    .text_color(if status.error { err } else { fg })
                    .child(status.message.clone()),
            );
        }
        if let Some(error) = &self.load_error {
            form = form.child(
                div()
                    .text_xs()
                    .text_color(err)
                    .child(format!("配置文件读不动：{error}")),
            );
        }

        match self.page {
            Page::Models => self.models_page(cx, fg, dim, err),
            Page::Connect => self.connect_page(cx, fg, dim, err, accent, secondary),
            Page::General => div()
                .flex()
                .flex_col()
                .gap_3()
                .size_full()
                .p_4()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(fg)
                        .child("设置"),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .text_xs()
                        .text_color(dim)
                        .child(format!("配置文件：{}", self.config_file))
                        .child("保存只写盘，重启后生效"),
                )
                .child(form)
                .into_any_element(),
        }
    }
}

impl SettingsView {
    /// 「默认供应商」：已知的用 chip 点选，也能手输（自定义适配器）。
    ///
    /// 为什么要 chip：`default_provider` 现在**真的生效**（`R1` §1.1），
    /// 手滑写成未知名字会让 `gui`（没有命令行参数的启动方式）直接起不来。
    fn provider_field(
        &self,
        cx: &mut Context<Self>,
        dim: Hsla,
        accent: Hsla,
        border: Hsla,
    ) -> AnyElement {
        let current = self.provider.read(cx).value().trim().to_string();
        let mut chips = div().flex().flex_row().flex_wrap().items_center().gap_1();
        for row in self.providers.clone() {
            let selected = row.id == current;
            let label = row.id.clone();
            let shown = row.name.clone();
            chips = chips.child(
                div()
                    .id(ElementId::Name(format!("provider-{}", row.id).into()))
                    .px_2()
                    .py_0p5()
                    .rounded_md()
                    .cursor_pointer()
                    .text_xs()
                    .border_1()
                    .border_color(if selected { accent } else { border })
                    .text_color(if selected { accent } else { dim })
                    .hover(|d| d.text_color(accent))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.pick_provider(label.clone(), window, cx)
                    }))
                    // chip 上写**展示名**，点下去写进输入框的是 id ——
                    // 用户不用记 `openai-compatible` 这种内部名字。
                    .child(shown),
            );
        }

        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_xs().text_color(dim).child("默认供应商"))
            .child(chips)
            .child(Input::new(&self.provider))
            .into_any_element()
    }
}

/// 一行「标签 + 输入框」。
fn field(label: &str, state: &Entity<InputState>, dim: Hsla) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_xs().text_color(dim).child(label.to_string()))
        .child(Input::new(state))
        .into_any_element()
}

/// 「连接提供商」页的密钥提示：连接前没有「来源」，只讲**该不该填**。
fn connect_key_hint(row: &CatalogRow) -> String {
    match (row.key_required, row.env.as_deref()) {
        (false, _) => "密钥：通常不需要（本地服务），留空即可".to_string(),
        (true, Some(env)) => format!("密钥：可填环境变量 {env}，或在此粘贴（存系统钥匙串）"),
        (true, None) => "密钥：在此粘贴（存系统钥匙串）".to_string(),
    }
}

/// 密钥来源提示。`env` / `required` 用来把"该不该填"讲清楚 ——
/// 本地服务不该催用户填密钥，有约定变量名的则应当告诉他变量叫什么。
fn key_hint(source: KeySource, env: Option<&str>, required: bool) -> String {
    match source {
        KeySource::Env => format!(
            "密钥来源：环境变量 {}（优先级更高，会盖过钥匙串）",
            env.unwrap_or("(未知)")
        ),
        KeySource::Keyring => "密钥来源：系统钥匙串（已配置，不回显）".to_string(),
        KeySource::None if !required => {
            format!(
                "密钥来源：未配置（{} 通常不需要密钥，留空即可）",
                "本地服务"
            )
        }
        KeySource::None => match env {
            Some(env) => format!("密钥来源：未配置（可设环境变量 {env}，或在这里粘贴）"),
            None => "密钥来源：未配置（没有约定环境变量名，只能存钥匙串）".to_string(),
        },
    }
}
