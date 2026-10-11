//! 可观测性与成本（规格：`R3`）。
//!
//! **机制层**（`D6` §1）：只负责「日志怎么初始化」「一次调用花了多少钱」。
//! **不含**「在哪儿打点」——打点散在各自的 `kernel/*` 与 `features/*` 里；
//! 也不做远程上报（本地优先，`R3` §4）。

mod cost;

pub use cost::cost_of;

/// 初始化日志（`R3` §1）：级别由 `RUST_LOG` 控制，缺省 `info`；输出到 **stderr**。
///
/// 幂等：已装过 subscriber 时直接返回（测试或多入口重复调用不会炸）。
pub fn init_from_env() {
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}
