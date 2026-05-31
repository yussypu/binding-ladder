//! Rung 4 — unrepresentable, for the second invariant: "an order cannot be
//! submitted without a passing risk check."
//!
//! Typestate, the plain-Rust way (no `lock_ordering` needed here — this
//! invariant has no transitive graph, which is exactly why it lacks the
//! compile-time-blowup jewel and makes the cleaner, smaller second data point):
//!
//!   * `UncheckedOrder` has NO `submit` method.
//!   * `CheckedOrder` has `submit` — and its fields are private, so the ONLY way
//!     to obtain one is `RiskCheck::approve`, which performs the check.
//!
//! "Submit without a passing risk check" is therefore not a rule you must
//! remember — it is a sentence the type system will not let you write.

// BOILERPLATE-START risk_check_rung4 (caller-authored: the two states + the gate)
/// An order that has not yet passed risk. Note what is absent: no `submit`.
pub struct UncheckedOrder {
    pub symbol: String,
    pub qty: u64,
}

impl UncheckedOrder {
    pub fn new(symbol: &str, qty: u64) -> Self {
        UncheckedOrder { symbol: symbol.to_string(), qty }
    }
}

/// An order that has passed risk. Fields are private: outside this module there
/// is no way to build one except by going through [`RiskCheck::approve`].
pub struct CheckedOrder {
    symbol: String,
    qty: u64,
}

impl CheckedOrder {
    /// Only a `CheckedOrder` can be submitted. Reaching this method at all is
    /// proof a risk check passed.
    pub fn submit(self) -> Receipt {
        Receipt { symbol: self.symbol, qty: self.qty }
    }
}

/// Confirmation that an order reached the venue.
#[derive(Debug, PartialEq, Eq)]
pub struct Receipt {
    pub symbol: String,
    pub qty: u64,
}

/// The risk gate. `approve` is the sole bridge from `UncheckedOrder` to the
/// submittable `CheckedOrder`.
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

/// Submitting without a passing risk check is unrepresentable.
///
/// `UncheckedOrder` has no `submit`, and `CheckedOrder`'s fields are private so
/// it cannot be forged — the only path to `submit` runs through `approve`:
/// ```compile_fail,E0599
/// use risk_check_rung4_typestate::UncheckedOrder;
/// let order = UncheckedOrder::new("AAPL", 100);
/// order.submit(); // E0599: no method named `submit` found for `UncheckedOrder`
/// ```
///
/// You also cannot fabricate a `CheckedOrder` to dodge the gate:
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

    /// The happy path: approve yields a `CheckedOrder`, which can submit.
    #[test]
    fn checked_order_submits() {
        let gate = RiskCheck { max_qty: 1000 };
        let checked = gate.approve(UncheckedOrder::new("AAPL", 100)).expect("within limit");
        let receipt = checked.submit();
        assert_eq!(receipt, Receipt { symbol: "AAPL".to_string(), qty: 100 });
    }

    /// A failing check yields no `CheckedOrder` at all, so there is nothing to
    /// submit — rejection is not a flag to remember, it is the absence of the
    /// capability.
    #[test]
    fn rejected_order_cannot_be_submitted() {
        let gate = RiskCheck { max_qty: 1000 };
        // `CheckedOrder` is deliberately neither `Debug` nor `PartialEq` (it is a
        // capability, not data), so match rather than `assert_eq` on the Ok side.
        assert!(matches!(gate.approve(UncheckedOrder::new("AAPL", 5000)), Err(Rejected)));
    }
}
