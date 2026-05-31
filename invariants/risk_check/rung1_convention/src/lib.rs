//! Rung 1 — convention, for the risk-check invariant.
//!
//! ```text
//! // RULE: always call `risk_check()` and confirm it passed before `submit()`.
//! ```
//!
//! One `Order` type. `submit()` exists unconditionally; `risk_check()` is a
//! separate call the author is *supposed* to make first. Nothing enforces the
//! ordering — `submit` does not look at whether a check ran. This is the same
//! invariant as the rung-4 crate next door, resting on willpower instead of the
//! type system. The test below demonstrates the rule breaking.

/// Confirmation that an order reached the venue.
#[derive(Debug, PartialEq, Eq)]
pub struct Receipt {
    pub symbol: String,
    pub qty: u64,
}

// BOILERPLATE-START risk_check_rung1 (caller-authored: one type, the order is a convention)
pub struct Order {
    pub symbol: String,
    pub qty: u64,
    /// Set by `risk_check`. `submit` does not consult it — that is the hole.
    pub checked: bool,
}

impl Order {
    pub fn new(symbol: &str, qty: u64) -> Self {
        Order { symbol: symbol.to_string(), qty, checked: false }
    }

    /// Run the risk check. Returns whether it passed; also records it on the
    /// order. The CONVENTION is that you call this and honor the result before
    /// submitting — but nothing makes you.
    pub fn risk_check(&mut self, max_qty: u64) -> bool {
        self.checked = self.qty <= max_qty;
        self.checked
    }

    /// Submit the order. Note: does NOT verify `self.checked`. A tired caller
    /// who forgets `risk_check` (or ignores its `false`) submits anyway.
    pub fn submit(self) -> Receipt {
        Receipt { symbol: self.symbol, qty: self.qty }
    }
}
// BOILERPLATE-END risk_check_rung1

#[cfg(test)]
mod tests {
    use super::*;

    /// THE POINT OF RUNG 1: submit-without-check compiles AND runs. We build an
    /// over-limit order, never call `risk_check`, and submit it straight to the
    /// venue. The rule said "check first"; nothing enforced it; the bad order
    /// goes through. Compare the rung-4 crate, where this same code does not
    /// compile.
    #[test]
    fn submit_without_check_compiles_and_runs() {
        let order = Order::new("AAPL", 1_000_000); // wildly over any limit
        assert!(!order.checked, "no risk check was run");
        let receipt = order.submit(); // nothing stops this
        assert_eq!(receipt, Receipt { symbol: "AAPL".to_string(), qty: 1_000_000 });
    }

    /// Even when the check is run and FAILS, `submit` ignores it — the result
    /// is advisory. The convention is doubly unenforced.
    #[test]
    fn failing_check_does_not_block_submit() {
        let mut order = Order::new("AAPL", 1_000_000);
        let passed = order.risk_check(1000);
        assert!(!passed, "check correctly reports over-limit");
        let _receipt = order.submit(); // submitted anyway
    }
}
