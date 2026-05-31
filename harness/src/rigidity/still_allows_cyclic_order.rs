//! Rung 4 still allows a cyclic, unsound order if you declare it. lock_ordering
//! has no cycle check: write both Connection after Mempool and Mempool after
//! Connection and both acquisition orders type check, so the M-then-C / C-then-M
//! inversion that deadlocks at runtime passes. It enforces consistency with what
//! you declared, and a cyclic declaration makes every order consistent.

use lock_ordering::relation::{LockAfter, LockBefore};

pub enum Mempool {}
pub enum Connection {}

// a declared 2-cycle. LockBefore is blanket-impl'd from LockAfter, so no
// transitive closure is needed and nothing overflows; it just compiles.
impl LockAfter<Mempool> for Connection {}
impl LockAfter<Connection> for Mempool {}

fn assert_before<A, B>()
where
    A: LockBefore<B>,
{
}

// both directions prove
pub fn both_orders_are_provable() {
    assert_before::<Mempool, Connection>();
    assert_before::<Connection, Mempool>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn still_allows_cyclic_order() {
        both_orders_are_provable();
    }
}
