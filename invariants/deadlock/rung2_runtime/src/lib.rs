//! Rung 2 — runtime detection. `parking_lot`'s deadlock detector.
//!
//! Still convention at compile time — the wrong program compiles fine. The
//! difference from rung 1 is that *at run time*, if a lock cycle actually forms,
//! the detector can see it: `parking_lot::deadlock::check_deadlock()` walks the
//! wait-for graph and reports cycles. This catches the bug at TEST time — but
//! only if a test happens to drive the threads into the cycle. It is a net under
//! the trapeze, not a wall: probabilistic, observational, after-the-fact.
//!
//! The per-call-site code is identical to rung 1 (just `parking_lot::Mutex`
//! instead of `std::sync::Mutex`); the only added boilerplate is wiring up the
//! detector once, globally. That asymmetry is itself a finding for the table.

use parking_lot::Mutex;

/// Same three-counter task, now on `parking_lot::Mutex` so the detector can see
/// the wait-for edges. Ordering is still pure convention.
// BOILERPLATE-START rung2_task (caller-authored: identical to rung 1, parking_lot mutexes)
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

/// The hot path, following the convention. Same as rung 1 — see runtime_bench.
pub fn hot_path(bank: &Bank) -> u64 {
    // LOCK ORDER: AccountsTable < Account < AuditLog
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

    /// THE POINT OF RUNG 2: the same opposite-order deadlock that rung 1 only
    /// suffers, rung 2 can FLAG. We force the cycle (barrier, opposite order)
    /// and then poll `check_deadlock()`; it returns the cycle of threads. We
    /// assert it found exactly the deadlock we planted.
    ///
    /// Note what this is NOT: it does not prevent the deadlock, it observes one
    /// after it has already frozen those threads. And it only fires because
    /// this test deliberately drives the threads into the cycle — in production
    /// the detector sees nothing until the unlucky interleaving happens to occur.
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
                let _gb = b.lock(); // blocks forever
                let _ = (&_ga, &_gb);
            });
        }
        {
            let (a, b, gate) = (a.clone(), b.clone(), gate.clone());
            thread::spawn(move || {
                let _gb = b.lock();
                gate.wait();
                let _ga = a.lock(); // blocks forever
                let _ = (&_gb, &_ga);
            });
        }

        // Poll the detector until it reports the cycle (or we time out).
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
        // Exactly one cycle, and it involves both deadlocked worker threads.
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
