//! Column: a legitimate program rung 4 CANNOT express.
//!
//! Type-level lock ordering assigns one static *level* per lock type. The
//! classic two-account transfer locks two elements of the *same* collection,
//! chosen at run time (`account[i]` and `account[j]`). Both elements share the
//! one level `AccountLevel`, so acquiring the second while holding the first
//! asks the solver to prove `AccountLevel: LockBefore<AccountLevel>` — a
//! self-edge that does not (and must not) exist. The program is legitimate (the
//! real-world fix is "always lock the lower account id first," a perfectly sound
//! dynamic order) but static levels have no way to say "i before j when i < j."
//!
//! That is the rigidity cost: rung 4 rejects a correct program because the
//! safe order is *data-dependent*, not *type-dependent*.

use lock_ordering::lock::MutexLockLevel;
use lock_ordering::relation::LockAfter;
use lock_ordering::{LockLevel, LockedAt, MutualExclusion, Unlocked};
use std::sync::Mutex;

/// One level for every account — there is no per-element type to distinguish
/// `account[i]` from `account[j]`.
pub enum AccountLevel {}
impl LockLevel for AccountLevel {
    type Method = MutualExclusion;
}
impl MutexLockLevel for AccountLevel {
    type Mutex = Mutex<u64>;
}
impl LockAfter<Unlocked> for AccountLevel {}

/// A table of accounts indexed at run time.
pub fn accounts() -> Vec<Mutex<u64>> {
    (0..4).map(Mutex::new).collect()
}

/// Locking ONE account is fine — a single level, one acquisition.
pub fn balance_of(table: &[Mutex<u64>], i: usize) -> u64 {
    let mut root = LockedAt::new();
    let guard = root.lock::<AccountLevel>(&table[i]).unwrap();
    *guard
}

/// The legitimate transfer rung 4 rejects: hold `account[i]` and `account[j]`
/// at once. Because both are `AccountLevel`, the second acquisition demands
/// `AccountLevel: LockBefore<AccountLevel>`, which is unprovable — E0277.
///
/// ```compile_fail
/// use harness::rigidity::legit_program_rejected::{accounts, AccountLevel};
/// use lock_ordering::LockedAt;
/// let table = accounts();
/// let mut root = LockedAt::new();
/// let (mut held_i, _gi) = root.with_lock::<AccountLevel>(&table[0]).unwrap();
/// // second account, SAME level: needs `AccountLevel: LockBefore<AccountLevel>`
/// let _gj = held_i.lock::<AccountLevel>(&table[1]).unwrap();
/// ```
#[allow(dead_code)]
fn _rejected_transfer_anchor() {}

#[cfg(test)]
mod tests {
    use super::*;

    /// The single-lock case the static level CAN express, for contrast.
    #[test]
    fn single_account_lock_is_expressible() {
        let table = accounts();
        assert_eq!(balance_of(&table, 2), 2);
    }
}
