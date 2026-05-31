//! Rung 4 — unrepresentable. Lock order enforced by the type system.
//!
//! Invariant: locks are always acquired in the global order
//!     AccountsTable  <  Account  <  AuditLog
//! On rung 1 this lives in a wiki and is violated at 2am. Here, acquiring them
//! out of order is a *type error*: the wrong program cannot be written.
//!
//! Built on the `lock_ordering` crate (akonradi, Fuchsia-team lineage). We do
//! not reimplement it — we measure it (see ../../../harness). Levels are marker
//! types; `impl_transitive_lock_order!` supplies the transitive closure so that
//! e.g. AccountsTable < AuditLog is provable without writing it by hand.

use lock_ordering::relation::{LockAfter, LockBefore};
use lock_ordering::impl_transitive_lock_order;

// BOILERPLATE-START rung4_hierarchy (caller-authored: the declared lock order)
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

/// In-order acquisition: legal, compiles. Transitive edge proven for free.
pub fn legal() {
    may_acquire::<AccountsTable, Account>();
    may_acquire::<Account, AuditLog>();
    may_acquire::<AccountsTable, AuditLog>(); // transitive
}

/// Out-of-order acquisition is unrepresentable.
/// ```compile_fail,E0277
/// use rung4_typestate::{may_acquire, AuditLog, AccountsTable};
/// // acquire AccountsTable while holding AuditLog: E0277, trait bound unsatisfied
/// may_acquire::<AuditLog, AccountsTable>();
/// ```
#[allow(dead_code)]
fn _doc_anchor() {}

// ---------------------------------------------------------------------------
// Runtime acquisition path (additive; the verified items above are unchanged).
//
// The relation traits prove order at compile time but acquire nothing. To
// MEASURE rung-4's runtime cost (harness/runtime_bench) we need the real
// acquisition machinery: `lock_ordering::LockedAt` threaded through nested
// `with_lock`/`lock` calls. The level types must be wired to concrete mutexes
// via `LockLevel` + `MutexLockLevel`, and rooted at `Unlocked`. This is exactly
// the per-caller setup the boilerplate column quantifies — it is not free.
// ---------------------------------------------------------------------------
use lock_ordering::lock::MutexLockLevel;
use lock_ordering::{LockLevel, LockedAt, MutualExclusion, Unlocked};
use std::sync::Mutex;

// BOILERPLATE-START rung4_runtime (caller-authored: level<->mutex wiring + Unlocked root)
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

// Root the hierarchy at `Unlocked` so a fresh `LockedAt::new()` may acquire the
// shallowest level first. The existing `AccountsTable => Account => AuditLog`
// transitive impls carry this edge down the chain automatically, so we add only
// the concrete root edge here. (A transitive macro on the upstream `Unlocked`
// type would collide under coherence — E0119 — since the compiler cannot assume
// `lock_ordering` won't add `LockAfter<Unlocked> for Unlocked` upstream.)
impl LockAfter<Unlocked> for AccountsTable {}
// BOILERPLATE-END rung4_runtime

/// The three counters the hot path touches, one behind each ordered lock.
// BOILERPLATE-START rung4_state (caller-authored: the locked state, == rung 1's)
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

/// The measured hot path: acquire all three locks *in declared order* and bump
/// each counter. Order is enforced by the type system — `LockedAt` threading is
/// the only thing acquisition sites pay. (See runtime_bench: this compiles down
/// to the same code as the rung-1 plain-Mutex version.)
// BOILERPLATE-START rung4_acquire (caller-authored: ordered acquisition via LockedAt)
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

    /// In-order acquisition through the real `LockedAt` path runs and returns.
    #[test]
    fn hot_path_acquires_in_order() {
        let bank = Bank::new();
        assert_eq!(hot_path(&bank), 3);
        assert_eq!(hot_path(&bank), 6);
    }
}
