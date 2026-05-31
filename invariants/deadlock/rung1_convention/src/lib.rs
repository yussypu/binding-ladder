//! Rung 1 — convention. The lock order lives in a comment and nothing enforces it.
//!
//! ```text
//! // LOCK ORDER: AccountsTable < Account < AuditLog
//! //   Always acquire in this order. Acquiring out of order risks deadlock.
//! //   -- the wiki, the style guide, the code review nit. Pure willpower.
//! ```
//!
//! This is the weakest rung: the rule is *good*, it is just *unenforced*. A
//! tired person at 2am writes the locks in the other order and nothing stops
//! them. The point of this crate is the test below: it DEMONSTRATES the rule
//! can break — two threads, opposite acquisition order, an actual deadlock.

use std::sync::Mutex;

/// Same three-counter task as every other rung, expressed at rung 1: plain
/// mutexes, ordering held only by the programmer following the comment above.
// BOILERPLATE-START rung1_task (caller-authored: the whole mechanism is a comment)
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

/// The hot path, following the convention (AccountsTable < Account < AuditLog).
/// Nothing but this comment and the author's memory keeps the order correct.
pub fn hot_path(bank: &Bank) -> u64 {
    // LOCK ORDER: AccountsTable < Account < AuditLog
    let mut g_at = bank.accounts_table.lock().unwrap();
    *g_at += 1;
    let mut g_acct = bank.account.lock().unwrap();
    *g_acct += 1;
    let mut g_audit = bank.audit_log.lock().unwrap();
    *g_audit += 1;
    *g_at + *g_acct + *g_audit
}
// BOILERPLATE-END rung1_task

impl Default for Bank {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    /// THE POINT OF RUNG 1: the convention can be broken, and when it is, the
    /// deadlock actually fires. Two threads acquire the same two locks in
    /// OPPOSITE order. A barrier guarantees each grabs its first lock before
    /// either reaches for its second, so the cycle is forced, not flaky.
    ///
    /// We cannot `join` deadlocked threads (they never return), so the main
    /// thread detects the hang with a timed channel: if no completion signal
    /// arrives, the deadlock is confirmed. A real hang would block the test
    /// runner forever — the watchdog is what makes "demonstrate a deadlock"
    /// into a test that terminates.
    #[test]
    fn deadlock_actually_fires_under_opposite_order() {
        let a = Arc::new(Mutex::new(0u64)); // stands in for AccountsTable
        let b = Arc::new(Mutex::new(0u64)); // stands in for Account
        let gate = Arc::new(Barrier::new(2));
        let (tx, rx) = mpsc::channel::<()>();

        // Thread 1 respects the convention: a (lower) then b (higher).
        {
            let (a, b, gate, tx) = (a.clone(), b.clone(), gate.clone(), tx.clone());
            thread::spawn(move || {
                let _ga = a.lock().unwrap();
                gate.wait(); // both first-locks are now held
                let _gb = b.lock().unwrap();
                let _ = tx.send(());
            });
        }
        // Thread 2 VIOLATES it: b then a. This is the 2am mistake.
        {
            let (a, b, gate, tx) = (a.clone(), b.clone(), gate.clone(), tx.clone());
            thread::spawn(move || {
                let _gb = b.lock().unwrap();
                gate.wait();
                let _ga = a.lock().unwrap();
                let _ = tx.send(());
            });
        }

        // Neither thread can ever send: thread 1 holds a wants b, thread 2 holds
        // b wants a. recv_timeout returning Timeout IS the demonstrated deadlock.
        match rx.recv_timeout(Duration::from_secs(3)) {
            Err(mpsc::RecvTimeoutError::Timeout) => { /* deadlock confirmed */ }
            other => panic!("expected a deadlock (timeout), but threads progressed: {other:?}"),
        }
        // The two worker threads are intentionally left blocked; they are reaped
        // at process exit. This is the cost of rung 1: the bug is real.
    }

    /// Sanity: the in-order hot path itself never deadlocks (the convention,
    /// when actually followed, is correct — that was never the problem).
    #[test]
    fn in_order_hot_path_is_fine() {
        let bank = Bank::new();
        assert_eq!(hot_path(&bank), 3);
        assert_eq!(hot_path(&bank), 6);
    }
}
