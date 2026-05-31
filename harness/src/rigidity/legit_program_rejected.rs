//! A legitimate program rung 4 cannot express: a two-account transfer locks
//! account[i] and account[j] from one collection. Both are AccountLevel, so the
//! second acquisition needs AccountLevel: LockBefore<AccountLevel>, a self edge
//! that does not exist. The safe order (lower id first) is data dependent, not
//! type dependent.

use lock_ordering::lock::MutexLockLevel;
use lock_ordering::relation::LockAfter;
use lock_ordering::{LockLevel, LockedAt, MutualExclusion, Unlocked};
use std::sync::Mutex;

// one level for every account; nothing distinguishes account[i] from account[j]
pub enum AccountLevel {}
impl LockLevel for AccountLevel {
    type Method = MutualExclusion;
}
impl MutexLockLevel for AccountLevel {
    type Mutex = Mutex<u64>;
}
impl LockAfter<Unlocked> for AccountLevel {}

pub fn accounts() -> Vec<Mutex<u64>> {
    (0..4).map(Mutex::new).collect()
}

// locking one account is fine: a single level, one acquisition
pub fn balance_of(table: &[Mutex<u64>], i: usize) -> u64 {
    let mut root = LockedAt::new();
    let guard = root.lock::<AccountLevel>(&table[i]).unwrap();
    *guard
}

/// Holding account[i] and account[j] at once does not compile: the second needs
/// AccountLevel: LockBefore<AccountLevel>, which is unprovable.
/// ```compile_fail,E0277
/// use harness::rigidity::legit_program_rejected::{accounts, AccountLevel};
/// use lock_ordering::LockedAt;
/// let table = accounts();
/// let mut root = LockedAt::new();
/// let (mut held_i, _gi) = root.with_lock::<AccountLevel>(&table[0]).unwrap();
/// // second account, same level: needs `AccountLevel: LockBefore<AccountLevel>`
/// let _gj = held_i.lock::<AccountLevel>(&table[1]).unwrap();
/// ```
#[allow(dead_code)]
fn _rejected_transfer_anchor() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_account_lock_is_expressible() {
        let table = accounts();
        assert_eq!(balance_of(&table, 2), 2);
    }
}
