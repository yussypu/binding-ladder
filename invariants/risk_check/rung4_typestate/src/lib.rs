//! Rung 4 for the second invariant: plain Rust typestate, no lock_ordering and no
//! transitive graph (hence no compile time blowup). UncheckedOrder has no submit;
//! CheckedOrder has submit but private fields, so the only way to get one is
//! RiskCheck::approve, which runs the check.

// BOILERPLATE-START risk_check_rung4
pub struct UncheckedOrder {
    pub symbol: String,
    pub qty: u64,
}

impl UncheckedOrder {
    pub fn new(symbol: &str, qty: u64) -> Self {
        UncheckedOrder { symbol: symbol.to_string(), qty }
    }
}

// private fields, so the only way to build one outside this module is approve
pub struct CheckedOrder {
    symbol: String,
    qty: u64,
}

impl CheckedOrder {
    // only a CheckedOrder can submit, so reaching here proves a check passed
    pub fn submit(self) -> Receipt {
        Receipt { symbol: self.symbol, qty: self.qty }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Receipt {
    pub symbol: String,
    pub qty: u64,
}

// the sole bridge from UncheckedOrder to the submittable CheckedOrder
pub struct RiskCheck {
    pub max_qty: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Rejected;

impl RiskCheck {
    pub fn approve(&self, order: UncheckedOrder) -> Result<CheckedOrder, Rejected> {
        if order.qty <= self.max_qty {
            Ok(CheckedOrder { symbol: order.symbol, qty: order.qty })
        } else {
            Err(Rejected)
        }
    }
}
// BOILERPLATE-END risk_check_rung4

/// UncheckedOrder has no submit, so submitting without a check does not compile:
/// ```compile_fail,E0599
/// use risk_check_rung4_typestate::UncheckedOrder;
/// let order = UncheckedOrder::new("AAPL", 100);
/// order.submit(); // E0599: no method named `submit` found for `UncheckedOrder`
/// ```
///
/// And a CheckedOrder cannot be forged to dodge the gate:
/// ```compile_fail,E0451
/// use risk_check_rung4_typestate::CheckedOrder;
/// let forged = CheckedOrder { symbol: "AAPL".into(), qty: 100 }; // E0451: private fields
/// forged.submit();
/// ```
#[allow(dead_code)]
fn _doc_anchor() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_order_submits() {
        let gate = RiskCheck { max_qty: 1000 };
        let checked = gate.approve(UncheckedOrder::new("AAPL", 100)).expect("within limit");
        let receipt = checked.submit();
        assert_eq!(receipt, Receipt { symbol: "AAPL".to_string(), qty: 100 });
    }

    // a failing check yields no CheckedOrder, so there is nothing to submit
    #[test]
    fn rejected_order_cannot_be_submitted() {
        let gate = RiskCheck { max_qty: 1000 };
        // CheckedOrder is neither Debug nor PartialEq, so match rather than assert_eq
        assert!(matches!(gate.approve(UncheckedOrder::new("AAPL", 5000)), Err(Rejected)));
    }
}
