//! Rung 1: convention, for the risk check invariant.
//!
//! The rule is: call risk_check and confirm it passed before submit. One Order
//! type. submit exists unconditionally; risk_check is a separate call the author
//! is supposed to make first. Nothing enforces the ordering, since submit never
//! looks at whether a check ran. Same invariant as the rung 4 crate next door,
//! resting on willpower instead of the type system. The test below breaks it.

#[derive(Debug, PartialEq, Eq)]
pub struct Receipt {
    pub symbol: String,
    pub qty: u64,
}

// BOILERPLATE-START risk_check_rung1
pub struct Order {
    pub symbol: String,
    pub qty: u64,
    // set by risk_check; submit does not consult it, which is the hole
    pub checked: bool,
}

impl Order {
    pub fn new(symbol: &str, qty: u64) -> Self {
        Order { symbol: symbol.to_string(), qty, checked: false }
    }

    // Runs the check and records the result on the order. The convention is that
    // you call this and honor the result before submitting. Nothing makes you.
    pub fn risk_check(&mut self, max_qty: u64) -> bool {
        self.checked = self.qty <= max_qty;
        self.checked
    }

    // Does not verify self.checked. A caller who forgets risk_check, or ignores a
    // false result, submits anyway.
    pub fn submit(self) -> Receipt {
        Receipt { symbol: self.symbol, qty: self.qty }
    }
}
// BOILERPLATE-END risk_check_rung1

#[cfg(test)]
mod tests {
    use super::*;

    // submit without a check compiles and runs: build an over limit order, never
    // call risk_check, submit it straight through. The rung 4 crate rejects the
    // same code at compile time.
    #[test]
    fn submit_without_check_compiles_and_runs() {
        let order = Order::new("AAPL", 1_000_000);
        assert!(!order.checked, "no risk check was run");
        let receipt = order.submit();
        assert_eq!(receipt, Receipt { symbol: "AAPL".to_string(), qty: 1_000_000 });
    }

    // Even when the check runs and fails, submit ignores it. The result is only
    // advisory, so the convention is doubly unenforced.
    #[test]
    fn failing_check_does_not_block_submit() {
        let mut order = Order::new("AAPL", 1_000_000);
        let passed = order.risk_check(1000);
        assert!(!passed, "check correctly reports over limit");
        let _receipt = order.submit();
    }
}
