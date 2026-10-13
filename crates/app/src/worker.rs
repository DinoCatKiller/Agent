//! worker：UI 无关的会话宿主（一期 TUI 与二期 GPUI **共用同一份**）。
//!
//! 职责（`D6`：组装归 `app`）：唯一拥有 `Chat` 与 `SessionRepo`——跑 `run_round`、
//! 把 [`LoopEvent`](agent_chat::LoopEvent) 转发给 UI、轮末落盘并刷新会话列表。
//! UI 永不直接碰 SQLite；两个渲染端只通过 [`Cmd`] / [`UiEvent`] 与它对话。
//!
//! 抽出来的理由很直接：命令通道与事件的**语义**（何时拒绝、何时落盘、如何回滚）
//! 只能有一份，否则 TUI 与 GPUI 的行为会慢慢漂开。

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use agent_chat::channel::{Cmd, UiEvent};
use agent_chat::{Chat, ChatService, SessionMeta, SessionRepo, ToolSet};
use agent_providers::ErasedProvider;
use agent_sessions::session::derive_title;
use agent_store::Store;
use futures::StreamExt;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::setup::{self, ChatArgs};

/// 取消令牌的共享槽位。
///
/// worker 在轮内不接收命令（见下），所以「取消」不能走命令通道：UI 直接把令牌塞进槽位
/// 并 `cancel()`，worker 轮末清空。两个渲染端都按这个约定办。
pub type CancelSlot = Arc<Mutex<Option<CancellationToken>>>;

/// 打开会话库（连接与迁移机制来自 `agent-store`）。
pub fn open_repo(path: &str) -> Result<SessionRepo, Box<dyn std::error::Error>> {
    let store = Store::open(path)?;
    Ok(SessionRepo::open(store)?)
}

/// 起一个取消令牌槽位。
pub fn cancel_slot() -> CancelSlot {
    Arc::new(Mutex::new(None))
}

/// 生效的 provider + model（用户可随时切，见 [`Cmd::SelectModel`]）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub provider: String,
    pub model: String,
}

impl Selection {
    pub fn new(provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
        }
    }

    /// `openai-compatible / gpt-4o` —— 状态栏与提示用。
    pub fn label(&self) -> String {
        format!("{} / {}", self.provider, self.model)
    }
}

/// 重建 provider 的上下文（切换 provider / model 时用）。
///
/// 一个必须讲清楚的细节：**CLI 的 `--api-key` / `--base-url` 只对启动时那个条目有效**。
/// 换到别家还带着它，等于把 A 家的密钥与端点送给 B 家 —— `R1` 的优先级里 CLI 最高，
/// 会盖过钥匙串里 B 家正确的那把。所以只在条目相同时沿用。
/// （`setup::declared_for` 也是同一条规则，两处必须一致。）
pub struct ProviderFactory {
    base_url: Option<String>,
    api_key: Option<String>,
    /// CLI 参数属于哪个条目（启动时那个）。
    cli_provider: String,
    config: Option<String>,
    tools: ToolSet,
}

impl ProviderFactory {
    pub fn new(cli: &ChatArgs, tools: ToolSet) -> Self {
        Self {
            base_url: cli.base_url.clone(),
            api_key: cli.api_key.clone(),
            cli_provider: cli.provider.clone(),
            config: cli.config.clone(),
            tools,
        }
    }

    /// 工具集（每次重建 `ChatService` 都要一份）。
    pub fn tools(&self) -> ToolSet {
        self.tools.clone()
    }

    /// 为某个选择构造 provider。`None` = 未知供应商（`build_provider` 已打印原因）。
    pub fn build(&self, selection: &Selection) -> Option<Arc<dyn ErasedProvider>> {
        let args = ChatArgs {
            provider: selection.provider.clone(),
            model: selection.model.clone(),
            // 切换后的组合只有这一个候选：降级链是 CLI 的概念（`R2`），界面不参与
            fallbacks: Vec::new(),
            // CLI 的 `--base-url` / `--api-key` 都只属于启动时那个条目（见本结构体的文档）
            base_url: if selection.provider == self.cli_provider {
                self.base_url.clone()
            } else {
                None
            },
            api_key: if selection.provider == self.cli_provider {
                self.api_key.clone()
            } else {
                None
            },
            config: self.config.clone(),
        };
        setup::build_provider(&args)
    }
}

/// worker 主循环。宿主自己决定怎么起它：
/// TUI 走 `tokio::spawn`，GPUI 走后台 runtime 的 `runtime.spawn`。
///
/// `initial_provider` 由宿主先构造并校验（未知供应商能在开窗口之前退出），
/// 之后的切换一律走 `factory` 重建 —— 因为换 provider 意味着换密钥。
pub async fn run_worker(
    selection: Selection,
    initial_provider: Arc<dyn ErasedProvider>,
    factory: ProviderFactory,
    repo: SessionRepo,
    cancel_slot: CancelSlot,
    mut cmd_rx: mpsc::Receiver<Cmd>,
    ui_tx: mpsc::UnboundedSender<UiEvent>,
) {
    let mut current = selection.clone();
    let mut service = ChatService::new(initial_provider, factory.tools());
    let mut chat = Chat::new(current.model.clone());
    // (session_id, title)；首次落盘时创建。
    let mut session: Option<(String, String)> = None;
    // 是否有一轮进行中（New / Load / Submit 互斥的依据）——以 cancel_slot 是否持有令牌为准。
    let busy = || cancel_slot.lock().expect("cancel slot").is_some();
    // 先播报一次生效组合：宿主的状态栏与选择器**以 worker 为准**，不自己猜
    let _ = ui_tx.send(UiEvent::ModelChanged {
        provider: current.provider.clone(),
        model: current.model.clone(),
    });
    let _ = ui_tx.send(UiEvent::Sessions(repo_list(&repo).await));

    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            Cmd::New => {
                if busy() {
                    let _ = ui_tx.send(UiEvent::Notice("生成中，先取消再新建".into()));
                    continue;
                }
                chat = Chat::new(chat.model.clone());
                session = None;
                let _ = ui_tx.send(UiEvent::Cleared);
            }
            Cmd::Load { id, title } => {
                if busy() {
                    let _ = ui_tx.send(UiEvent::Notice("生成中，先取消再切换".into()));
                    continue;
                }
                let repo2 = repo.clone();
                let id2 = id.clone();
                let loaded = match tokio::task::spawn_blocking(move || repo2.load(&id2)).await {
                    Ok(Ok(Some(chat))) => chat,
                    Ok(Ok(None)) => {
                        let _ = ui_tx.send(UiEvent::Notice("会话不存在或已被删除".into()));
                        continue;
                    }
                    Ok(Err(error)) => {
                        let _ = ui_tx.send(UiEvent::Notice(format!("读取会话失败：{error}")));
                        continue;
                    }
                    Err(_) => {
                        let _ = ui_tx.send(UiEvent::Notice("读取会话任务失败".into()));
                        continue;
                    }
                };
                chat = loaded;
                session = Some((id, title.clone()));
                // 会话自带模型：跟着它走，并让界面显示真相（provider 是宿主级选择，不随会话变）
                current.model = chat.model.clone();
                let _ = ui_tx.send(UiEvent::ModelChanged {
                    provider: current.provider.clone(),
                    model: current.model.clone(),
                });
                let _ = ui_tx.send(UiEvent::SessionLoaded {
                    title,
                    chat: chat.clone(),
                });
            }
            Cmd::Submit(text) => {
                if busy() {
                    let _ = ui_tx.send(UiEvent::Notice("生成中…".into()));
                    continue;
                }
                let existing = session
                    .as_ref()
                    .map(|(id, title)| (id.clone(), title.clone()));
                let (id, title) = match existing {
                    Some(pair) => pair,
                    None => {
                        let id = new_session_id();
                        let title = derive_title(&text);
                        session = Some((id.clone(), title.clone()));
                        (id, title)
                    }
                };

                let token = CancellationToken::new();
                *cancel_slot.lock().expect("cancel slot") = Some(token.clone());

                match service.run_round(&mut chat, text, token) {
                    Err(error) => {
                        // fast-fail：用户输入已入史（见 run_round），提示后可继续。
                        let _ = ui_tx.send(UiEvent::Rejected(error.to_string()));
                    }
                    Ok(round) => {
                        tokio::pin!(round);
                        while let Some(event) = round.next().await {
                            if ui_tx.send(UiEvent::Loop(event)).is_err() {
                                break; // UI 已退出
                            }
                        }
                    }
                }
                *cancel_slot.lock().expect("cancel slot") = None;

                // 轮末落盘（取消 / 错误也存：用户输入已在史中）。
                let (repo2, chat2, id2, title2) = (repo.clone(), chat.clone(), id, title);
                let now = now_ms();
                let saved =
                    tokio::task::spawn_blocking(move || repo2.save(&id2, &title2, &chat2, now))
                        .await;
                match saved {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        let _ = ui_tx.send(UiEvent::Notice(format!("保存会话失败：{error}")));
                    }
                    Err(_) => {
                        let _ = ui_tx.send(UiEvent::Notice("保存会话任务失败".into()));
                    }
                }
                let _ = ui_tx.send(UiEvent::Sessions(repo_list(&repo).await));
            }
            Cmd::SelectModel { provider, model } => {
                let next = Selection::new(provider, model);
                if next == current {
                    continue;
                }
                if busy() {
                    let _ = ui_tx.send(UiEvent::Notice("生成中，先取消再切模型".into()));
                    continue;
                }
                match factory.build(&next) {
                    Some(provider) => {
                        // 同一会话换模型：历史保留，**下一轮**用新模型
                        // （`Chat::request` 读的就是 `chat.model`）。
                        service = ChatService::new(provider, factory.tools());
                        chat.model = next.model.clone();
                        current = next;
                        let _ = ui_tx.send(UiEvent::ModelChanged {
                            provider: current.provider.clone(),
                            model: current.model.clone(),
                        });
                    }
                    None => {
                        // 构造失败的原因 `build_provider` 已打印（缺密钥 / 未知供应商）
                        let _ = ui_tx.send(UiEvent::Notice(format!(
                            "切到 {} 失败：供应商未知或构造 provider 失败",
                            next.label()
                        )));
                    }
                }
            }
        }
    }
}

async fn repo_list(repo: &SessionRepo) -> Vec<SessionMeta> {
    let repo2 = repo.clone();
    tokio::task::spawn_blocking(move || repo2.list().unwrap_or_default())
        .await
        .unwrap_or_default()
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 会话 id：纳秒时间戳的十六进制（单用户本地场景足够唯一；正式方案随 `R1`）。
fn new_session_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("s{nanos:024x}")
}
