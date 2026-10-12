//! `gui` 子命令：二期图形界面（`D3` 方案 A，需 `--features gui` 构建）。
//!
//! 线程模型（`X2` §5）：
//!
//! ```text
//! 主线程      GPUI 事件循环（`application().run`）—— 绝不 block_on
//! 后台线程池   tokio runtime：worker 跑在这里
//! 桥           tokio mpsc → GPUI 侧 `cx.spawn` 里 `recv().await` → `view.update`
//! ```
//!
//! 与 TUI 的关系：**同一份 worker、同一份 `ChatUi` 状态机、同一份 `Cmd`/`UiEvent`**，
//! 差别只在渲染端（`features/chat/src/gpui.rs` vs `ui.rs`）。

use agent_chat::{ChatService, ChatView, Cmd, StatusInfo, UiEvent};
use agent_ui_kit::kit::*;
use tokio::sync::mpsc;

use crate::setup::{self, ChatArgs};
use crate::worker;

/// `gui` 子命令参数。
#[derive(Debug, Clone)]
pub struct GuiArgs {
    pub provider: String,
    pub model: String,
    /// 降级候选（`--model a,b` 的 b…，`R2`）。
    pub fallbacks: Vec<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    /// 配置文件路径（`R1`）。
    pub config: Option<String>,
    /// SQLite 路径（缺省 `./agent-sessions.db`）。
    pub db: Option<String>,
}

/// 起窗口并跑消息循环，返回进程退出码。
///
/// 同步函数：GPUI 的 `application().run` 自己占住主线程，不需要（也不能）`block_on`。
pub fn run(args: GuiArgs) -> i32 {
    let chat_args = ChatArgs {
        provider: args.provider.clone(),
        model: args.model.clone(),
        fallbacks: args.fallbacks.clone(),
        base_url: args.base_url.clone(),
        api_key: args.api_key.clone(),
        config: args.config.clone(),
    };
    let Some(provider) = setup::build_provider(&chat_args) else {
        return 2;
    };
    let db_path = args
        .db
        .clone()
        .unwrap_or_else(|| "agent-sessions.db".into());
    let repo = match worker::open_repo(&db_path) {
        Ok(repo) => repo,
        Err(error) => {
            eprintln!("打开会话库失败（{db_path}）：{error}");
            return 2;
        }
    };

    let model = args.model.clone();
    let service = ChatService::new(provider, setup::demo_tools());

    // 后台 runtime：worker 跑在这里，UI 线程只收事件。
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("创建 tokio 运行时失败：{error}");
            return 2;
        }
    };

    let cancel_slot = worker::cancel_slot();
    let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>(32);
    let (ui_tx, mut ui_rx) = mpsc::unbounded_channel::<UiEvent>();
    runtime.spawn(worker::run_worker(
        model,
        service,
        repo,
        cancel_slot.clone(),
        cmd_rx,
        ui_tx,
    ));

    let status = StatusInfo {
        provider: args.provider.clone(),
        model: args.model.clone(),
    };

    application().run(move |cx| {
        // 组件库 + 设计令牌主题（必须在开窗之前）
        agent_ui_kit::init(cx);
        open_window(window_options(), cx, |window, cx| {
            let view = cx.new(|cx| {
                ChatView::new(
                    window,
                    cx,
                    status.clone(),
                    cmd_tx.clone(),
                    cancel_slot.clone(),
                )
            });
            // 桥：worker 的事件流 → 视图。`rx.recv().await` 在 GPUI 的执行器里轮询
            // （`X2` §8 已验证过这个形态）。
            view.update(cx, |_view, cx| {
                cx.spawn(async move |this, cx| {
                    while let Some(event) = ui_rx.recv().await {
                        if this.update(cx, |view, cx| view.apply(event, cx)).is_err() {
                            break; // 视图已销毁
                        }
                    }
                })
                .detach();
            });
            view
        })
        .expect("open window");
    });

    // 走到这里说明窗口已关闭：runtime 随本函数返回而 drop，worker 一并收尾。
    let _ = runtime;
    0
}

/// 窗口参数：给个像样的初始尺寸与标题（`WindowBounds` 在屏幕坐标里）。
fn window_options() -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(80.), px(80.)),
            size: size(px(1100.), px(760.)),
        })),
        titlebar: Some(TitlebarOptions {
            title: Some("CodingRocket".into()),
            ..Default::default()
        }),
        ..Default::default()
    }
}
