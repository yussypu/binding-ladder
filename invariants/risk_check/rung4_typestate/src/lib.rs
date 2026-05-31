//! Rung 4: unrepresentable, for the second invariant. An order cannot be
//! submitted without a passing risk check.
//!
//! Plain Rust typestate, no lock_ordering needed. This invariant has no
//! transitive graph, which is why it lacks the compile time blowup and makes the
//! smaller second data point. UncheckedOrder has no submit method. CheckedOrder
//! has submit, and its fields are private, so the only way to get one is through
//! RiskCheck::approve, which runs the check. Submit without a passing check is
//! therefore not a rule to remember, it is a sentence the type system rejects.

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

// Fields are private, so outside this module the only way to build one is
// RiskCheck::approve.
pub struct CheckedOrder {
    symbol: String,
    qty: u64,
}

impl CheckedOrder {
    // Only a CheckedOrder can submit, so reaching this proves a check passed.
    pub fn submit(self) -> Receipt {
        Receipt { symbol: self.symbol, qty: self.qty }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Receipt {
    pub symbol: String,
    pub qty: u64,
}

// The sole bridge from UncheckedOrder to the submittable CheckedOrder.
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

/// Submitting without a passing risk check does not compile. UncheckedOrder has
/// no submit, and CheckedOrder cannot be forged because its fields are private:
/// ```compile_fail,E0599
/// use risk_check_rung4_typestate::UncheckedOrder;
/// let order = UncheckedOrder::new("AAPL", 100);
/// order.submit(); // E0599: no method named `submit` found for `UncheckedOrder`
/// ```
///
/// And a CheckedOrder cannot be fabricated to dodge the gate:
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

    // A failing check yields no CheckedOrder, so there is nothing to submit.
    #[test]
    fn rejected_order_cannot_be_submitted() {
        let gate = RiskCheck { max_qty: 1000 };
        // CheckedOrder is deliberately neither Debug nor PartialEq, so match
        // rather than assert_eq on the Ok side.
        assert!(matches!(gate.approve(UncheckedOrder::new("AAPL", 5000)), Err(Rejected)));
    }
}
