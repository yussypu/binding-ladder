//! Rung 2: runtime detection with parking_lot's deadlock detector.
//!
//! Still convention at compile time; the wrong program compiles. The difference
//! from rung 1 is that at runtime, if a lock cycle forms, check_deadlock walks
//! the wait graph and reports it. That only fires when a test happens to drive
//! the threads into the cycle, so it catches the bug probabilistically and after
//! the fact. The per site code matches rung 1 with parking_lot::Mutex in place
//! of std::sync::Mutex; the only extra setup is wiring the detector once.

use parking_lot::Mutex;

// BOILERPLATE-START rung2_task
pub struct Bank {
    pub accounts_table: Mutex<u64>,
    pub account: Mutex<u64>,
    pub audit_log: Mutex<u64>,
}

impl Bank {
    pub fn new() -> Self {
        Bank { accounts_table: Mutex::new(0), account: Mutex::new(0), audit_log: Mutex::new(0) }
    }
}

pub fn hot_path(bank: &Bank) -> u64 {
    // lock order: accounts_table, account, audit_log
    let mut g_at = bank.accounts_table.lock();
    *g_at += 1;
    let mut g_acct = bank.account.lock();
    *g_acct += 1;
    let mut g_audit = bank.audit_log.lock();
    *g_audit += 1;
    *g_at + *g_acct + *g_audit
}
// BOILERPLATE-END rung2_task

impl Default for Bank {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::deadlock;
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::{Duration, Instant};

    // Force the same opposite order cycle rung 1 only suffers, then confirm the
    // detector reports it. It does not prevent the deadlock, it observes one once
    // the threads are already frozen, and only because the test drove them there.
    #[test]
    fn detector_flags_opposite_order_deadlock() {
        let a = Arc::new(Mutex::new(0u64));
        let b = Arc::new(Mutex::new(0u64));
        let gate = Arc::new(Barrier::new(2));

        {
            let (a, b, gate) = (a.clone(), b.clone(), gate.clone());
            thread::spawn(move || {
                let _ga = a.lock();
                gate.wait();
                let _gb = b.lock();
                let _ = (&_ga, &_gb);
            });
        }
        {
            let (a, b, gate) = (a.clone(), b.clone(), gate.clone());
            thread::spawn(move || {
                let _gb = b.lock();
                gate.wait();
                let _ga = a.lock();
                let _ = (&_gb, &_ga);
            });
        }

        let deadline = Instant::now() + Duration::from_secs(5);
        let mut cycles = Vec::new();
        while Instant::now() < deadline {
            cycles = deadlock::check_deadlock();
            if !cycles.is_empty() {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }

        assert!(
            !cycles.is_empty(),
            "parking_lot deadlock detector did not flag the planted cycle"
        );
        assert_eq!(cycles.len(), 1, "expected a single deadlock cycle");
        assert_eq!(cycles[0].len(), 2, "expected both workers in the cycle");
    }

    #[test]
    fn in_order_hot_path_is_fine() {
        let bank = Bank::new();
        assert_eq!(hot_path(&bank), 3);
        assert_eq!(hot_path(&bank), 6);
    }
}
