//! Rung 1: convention. The lock order (AccountsTable, Account, AuditLog) lives in
//! a comment and nothing enforces it. The test below breaks the rule with two
//! threads acquiring in opposite order and produces a real deadlock.

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

    // two threads acquire the same locks in opposite order; the barrier forces
    // the cycle rather than leaving it flaky. deadlocked threads never return, so
    // we detect the hang with a timed channel: no signal means it deadlocked.
    #[test]
    fn deadlock_actually_fires_under_opposite_order() {
        let a = Arc::new(Mutex::new(0u64));
        let b = Arc::new(Mutex::new(0u64));
        let gate = Arc::new(Barrier::new(2));
        let (tx, rx) = mpsc::channel::<()>();

        // thread 1: a then b
        {
            let (a, b, gate, tx) = (a.clone(), b.clone(), gate.clone(), tx.clone());
            thread::spawn(move || {
                let _ga = a.lock().unwrap();
                gate.wait();
                let _gb = b.lock().unwrap();
                let _ = tx.send(());
            });
        }
        // thread 2: b then a
        {
            let (a, b, gate, tx) = (a.clone(), b.clone(), gate.clone(), tx.clone());
            thread::spawn(move || {
                let _gb = b.lock().unwrap();
                gate.wait();
                let _ga = a.lock().unwrap();
                let _ = tx.send(());
            });
        }

        // 1 holds a wants b, 2 holds b wants a: a timeout here is the deadlock
        match rx.recv_timeout(Duration::from_secs(3)) {
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            other => panic!("expected a deadlock (timeout), but threads progressed: {other:?}"),
        }
        // workers left blocked on purpose, reaped at process exit
    }

    #[test]
    fn in_order_hot_path_is_fine() {
        let bank = Bank::new();
        assert_eq!(hot_path(&bank), 3);
        assert_eq!(hot_path(&bank), 6);
    }
}
