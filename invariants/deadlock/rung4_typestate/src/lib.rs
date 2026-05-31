//! Rung 4: lock order (AccountsTable, Account, AuditLog) enforced by the type
//! system; acquiring out of order is a type error. Built on the lock_ordering
//! crate: levels are marker types and impl_transitive_lock_order supplies the
//! transitive closure, so AccountsTable before AuditLog is provable for free.

use lock_ordering::relation::{LockAfter, LockBefore};
use lock_ordering::impl_transitive_lock_order;

// BOILERPLATE-START rung4_hierarchy
pub enum AccountsTable {}
pub enum Account {}
pub enum AuditLog {}

impl LockAfter<AccountsTable> for Account {}
impl_transitive_lock_order!(AccountsTable => Account);
impl LockAfter<Account> for AuditLog {}
impl_transitive_lock_order!(Account => AuditLog);
// BOILERPLATE-END rung4_hierarchy

/// Compiles iff `A` may be held while acquiring `B`.
pub fn may_acquire<A, B>() where A: LockBefore<B> {}

/// In-order acquisition compiles; the transitive edge is proven for free.
pub fn legal() {
    may_acquire::<AccountsTable, Account>();
    may_acquire::<Account, AuditLog>();
    may_acquire::<AccountsTable, AuditLog>();
}

/// Out of order acquisition does not compile.
/// ```compile_fail,E0277
/// use rung4_typestate::{may_acquire, AuditLog, AccountsTable};
/// // acquire AccountsTable while holding AuditLog: E0277, trait bound unsatisfied
/// may_acquire::<AuditLog, AccountsTable>();
/// ```
#[allow(dead_code)]
fn _doc_anchor() {}

// the relation traits prove order but acquire nothing; the runtime bench needs
// real machinery, so the levels are wired to concrete mutexes via LockLevel and
// MutexLockLevel and rooted at Unlocked.
use lock_ordering::lock::MutexLockLevel;
use lock_ordering::{LockLevel, LockedAt, MutualExclusion, Unlocked};
use std::sync::Mutex;

// BOILERPLATE-START rung4_runtime
impl LockLevel for AccountsTable {
    type Method = MutualExclusion;
}
impl LockLevel for Account {
    type Method = MutualExclusion;
}
impl LockLevel for AuditLog {
    type Method = MutualExclusion;
}

impl MutexLockLevel for AccountsTable {
    type Mutex = Mutex<u64>;
}
impl MutexLockLevel for Account {
    type Mutex = Mutex<u64>;
}
impl MutexLockLevel for AuditLog {
    type Mutex = Mutex<u64>;
}

// root at Unlocked so a fresh LockedAt::new can acquire the shallowest level
// first. only the concrete root edge is needed; a transitive macro on the
// upstream Unlocked type would collide under coherence (E0119).
impl LockAfter<Unlocked> for AccountsTable {}
// BOILERPLATE-END rung4_runtime

// BOILERPLATE-START rung4_state
pub struct Bank {
    pub accounts_table: Mutex<u64>,
    pub account: Mutex<u64>,
    pub audit_log: Mutex<u64>,
}

impl Bank {
    pub fn new() -> Self {
        Bank {
            accounts_table: Mutex::new(0),
            account: Mutex::new(0),
            audit_log: Mutex::new(0),
        }
    }
}
// BOILERPLATE-END rung4_state

impl Default for Bank {
    fn default() -> Self {
        Self::new()
    }
}

// acquire all three locks in declared order and bump each counter; LockedAt
// threading is the only thing the call site pays
// BOILERPLATE-START rung4_acquire
pub fn hot_path(bank: &Bank) -> u64 {
    let mut root = LockedAt::new();
    let (mut at_held, mut g_at) = root.with_lock::<AccountsTable>(&bank.accounts_table).unwrap();
    *g_at += 1;
    let (mut acct_held, mut g_acct) = at_held.with_lock::<Account>(&bank.account).unwrap();
    *g_acct += 1;
    let mut g_audit = acct_held.lock::<AuditLog>(&bank.audit_log).unwrap();
    *g_audit += 1;
    *g_at + *g_acct + *g_audit
}
// BOILERPLATE-END rung4_acquire

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hot_path_acquires_in_order() {
        let bank = Bank::new();
        assert_eq!(hot_path(&bank), 3);
        assert_eq!(hot_path(&bank), 6);
    }
}
