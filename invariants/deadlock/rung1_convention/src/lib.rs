//! Rung 1: convention. The lock order lives in a comment and nothing enforces it.
//! The rule is to always acquire in the order AccountsTable, Account, AuditLog,
//! because out of order risks deadlock.
//!
//! This is the weakest rung. The rule is good, it is just unenforced. A tired
//! person at 2am writes the locks in the other order and nothing stops them. The
//! point of this crate is the test below, which breaks the rule with two threads
//! acquiring in opposite order and produces a real deadlock.

use std::sync::Mutex;

// BOILERPLATE-START rung1_task
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

    // The convention can be broken, and when it is the deadlock fires. Two
    // threads acquire the same two locks in opposite order. A barrier makes each
    // grab its first lock before either reaches for its second, so the cycle is
    // forced rather than flaky.
    //
    // Deadlocked threads never return, so we cannot join them. The main thread
    // detects the hang with a timed channel: no completion signal means the
    // deadlock happened. Without the timeout a real hang would block the runner
    // forever.
    #[test]
    fn deadlock_actually_fires_under_opposite_order() {
        let a = Arc::new(Mutex::new(0u64));
        let b = Arc::new(Mutex::new(0u64));
        let gate = Arc::new(Barrier::new(2));
        let (tx, rx) = mpsc::channel::<()>();

        // thread 1 follows the convention: a then b
        {
            let (a, b, gate, tx) = (a.clone(), b.clone(), gate.clone(), tx.clone());
            thread::spawn(move || {
                let _ga = a.lock().unwrap();
                gate.wait();
                let _gb = b.lock().unwrap();
                let _ = tx.send(());
            });
        }
        // thread 2 violates it: b then a
        {
            let (a, b, gate, tx) = (a.clone(), b.clone(), gate.clone(), tx.clone());
            thread::spawn(move || {
                let _gb = b.lock().unwrap();
                gate.wait();
                let _ga = a.lock().unwrap();
                let _ = tx.send(());
            });
        }

        // Neither thread can send: 1 holds a wants b, 2 holds b wants a. A
        // timeout here is the demonstrated deadlock.
        match rx.recv_timeout(Duration::from_secs(3)) {
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            other => panic!("expected a deadlock (timeout), but threads progressed: {other:?}"),
        }
        // The workers are left blocked on purpose and reaped at process exit.
    }

    // The in order hot path never deadlocks. The convention, when followed, is
    // correct; that was never the problem.
    #[test]
    fn in_order_hot_path_is_fine() {
        let bank = Bank::new();
        assert_eq!(hot_path(&bank), 3);
        assert_eq!(hot_path(&bank), 6);
    }
}
