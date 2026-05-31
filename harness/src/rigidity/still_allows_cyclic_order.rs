//! Column: rung 4 still allows a cyclic (unsound) order — if you declare it.
//!
//! Type-level lock ordering enforces consistency with the order *you declare*,
//! not that the order is actually deadlock-free. `lock_ordering` has no cycle
//! check: if you write both `Connection after Mempool` and `Mempool after
//! Connection`, it believes you. Now BOTH acquisition orders type-check — which
//! is to say the type system will happily wave through the exact M→C / C→M
//! inversion that deadlocks at run time. Rung 4's guarantee is "you acquired
//! locks in an order consistent with your declarations," and a cyclic
//! declaration makes every order consistent.

use lock_ordering::relation::{LockAfter, LockBefore};

pub enum Mempool {}
pub enum Connection {}

// A DECLARED CYCLE. Each line is individually reasonable; together they are a
// 2-cycle. `LockBefore` is blanket-implemented from `LockAfter`, so no
// transitive closure is needed and nothing overflows — it just compiles.
impl LockAfter<Mempool> for Connection {} // Mempool < Connection
impl LockAfter<Connection> for Mempool {} // Connection < Mempool  (the unsound edge)

/// Compiles iff `A` may be held while acquiring `B`.
fn assert_before<A, B>()
where
    A: LockBefore<B>,
{
}

/// Both directions prove. A program that acquires Mempool-then-Connection in
/// one path and Connection-then-Mempool in another would type-check at both
/// sites — rung 4 would not catch the deadlock, because you told it the cycle
/// was legal.
pub fn both_orders_are_provable() {
    assert_before::<Mempool, Connection>();
    assert_before::<Connection, Mempool>();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fact that this compiles AND runs is the demonstration: the type
    /// system enforced a cyclic order without complaint.
    #[test]
    fn still_allows_cyclic_order() {
        both_orders_are_provable();
    }
}
