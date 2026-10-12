//! UI 无关的命令通道与事件流（`D3`：接口按「事件流 + 命令通道」设计）。
//!
//! 宿主（`app` 的 TUI / GPUI）与 worker 之间**只走这两个枚举**，渲染端不碰编排：
//! - UI → worker：[`Cmd`]
//! - worker → UI：[`UiEvent`]
//!
//! 放在本切片而不是宿主，是为了让两种渲染端消费同一套契约（一期 TUI 与二期 GPUI 并存期间
//! 尤其重要）。改这里等于改宿主契约，两个渲染端都要跟。

use crate::chat::Chat;
use crate::repo::SessionMeta;
use crate::service::LoopEvent;

/// UI → worker 的命令通道。
#[derive(Debug)]
pub enum Cmd {
    /// 提交一行用户输入，开一轮。
    Submit(String),
    /// 切换会话（worker 侧 `Chat` 会被整体替换）。
    Load { id: String, title: String },
    /// 新建空会话。
    New,
    /// 切换生效的 provider + model（运行中切换；生成中会被拒绝）。
    ///
    /// 按**组合**切而不是只切模型：密钥按 provider 分别存
    /// （`R1`：钥匙串账号 `<provider>:api_key`），换 provider 必须连带重建 provider 实例
    /// 以重新取密钥。
    SelectModel { provider: String, model: String },
}

/// worker → UI 的事件通道。
#[derive(Debug)]
pub enum UiEvent {
    /// 会话列表整体刷新（启动、每次落盘后）。
    Sessions(Vec<SessionMeta>),
    /// 会话已加载（worker 侧 `Chat` 已替换）。
    SessionLoaded { title: String, chat: Chat },
    /// 新会话（worker 侧已重置）。
    Cleared,
    /// 编排事件原样转发（`A5` 的 `LoopEvent`）。
    Loop(LoopEvent),
    /// 生效的 provider / model 变了（启动时发一次，之后每次切换都发）。
    ///
    /// **worker 是唯一真相源**：宿主只跟着更新状态栏与选择器，不自己猜 ——
    /// 否则切换失败（缺密钥、供应商未知）时界面会显示成成功的。
    ModelChanged { provider: String, model: String },
    /// `run_round` 同步 fast-fail。
    Rejected(String),
    /// 杂项提示（生成中不可切换、落盘失败…）。
    Notice(String),
}
