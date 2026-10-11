//! token 成本计算（`R3` §3）。
//!
//! 纯函数、无 IO：价格来自 `ModelSpec::pricing`（`A2` §1），没有定价就是
//! `None` —— **不猜**（宁可显示「未知」也不要给一个编出来的数字）。

use agent_common::{Pricing, Usage};

/// 每百万 token 的计价单位（`Pricing` 的字段语义）。
const PER_MTOK: f64 = 1_000_000.0;

/// 一次调用的成本，与 `Pricing::currency` 同币种。
///
/// 返回 `None` 的两种情况：**模型没有定价信息**。
pub fn cost_of(usage: &Usage, pricing: Option<&Pricing>) -> Option<f64> {
    let pricing = pricing?;
    let input = f64::from(usage.input_tokens) / PER_MTOK * pricing.input_per_mtok;
    let output = f64::from(usage.output_tokens) / PER_MTOK * pricing.output_per_mtok;
    Some(input + output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pricing() -> Pricing {
        Pricing {
            input_per_mtok: 3.0,
            output_per_mtok: 15.0,
            currency: "USD".into(),
        }
    }

    #[test]
    fn computes_from_both_directions() {
        let usage = Usage {
            input_tokens: 1_000_000,
            output_tokens: 2_000_000,
        };
        let cost = cost_of(&usage, Some(&pricing())).expect("priced");
        // 1M×3.0 + 2M×15.0 = 3.0 + 30.0
        assert!((cost - 33.0).abs() < f64::EPSILON, "实际 {cost}");
    }

    #[test]
    fn scales_below_one_million() {
        let usage = Usage {
            input_tokens: 500_000,
            output_tokens: 0,
        };
        let cost = cost_of(&usage, Some(&pricing())).expect("priced");
        assert!((cost - 1.5).abs() < f64::EPSILON, "实际 {cost}");
    }

    #[test]
    fn unknown_pricing_is_none_not_zero() {
        let usage = Usage {
            input_tokens: 1_000,
            output_tokens: 1_000,
        };
        assert!(
            cost_of(&usage, None).is_none(),
            "没有定价时必须给 None，不能假装是 0"
        );
    }

    #[test]
    fn zero_usage_costs_zero_when_priced() {
        let cost = cost_of(&Usage::default(), Some(&pricing())).expect("priced");
        assert_eq!(cost, 0.0);
    }
}
