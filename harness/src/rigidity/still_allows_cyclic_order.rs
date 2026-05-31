//! Rung 4 still allows a cyclic, unsound order if you declare it.
//!
//! Type level lock ordering enforces consistency with the order you declare, not
//! that the order is deadlock free. lock_ordering has no cycle check: if you
//! write both Connection after Mempool and Mempool after Connection, it believes
//! you. Now both acquisition orders type check, so the type system waves through
//! the exact M then C / C then M inversion that deadlocks at runtime. The
//! guarantee is that you acquired locks consistently with your declarations, and
//! a cyclic declaration makes every order consistent.

use lock_ordering::relation::{LockAfter, LockBefore};

pub enum Mempool {}
pub enum Connection {}

// A declared cycle. Each line is reasonable alone; together they are a 2 cycle.
// LockBefore is blanket implemented from LockAfter, so no transitive closure is
// needed and nothing overflows. It just compiles.
impl LockAfter<Mempool> for Connection {}
impl LockAfter<Connection> for Mempool {}

fn assert_before<A, B>()
where
    A: LockBefore<B>,
{
}

// Both directions prove. A program acquiring Mempool then Connection in one path
// and Connection then Mempool in another type checks at both sites, and rung 4
// does not catch the deadlock because you told it the cycle was legal.
pub fn both_orders_are_provable() {
    assert_before::<Mempool, Connection>();
    assert_before::<Connection, Mempool>();
}

#[cfg(test)]
mod tests {
    use super::*;

    // That this compiles and runs is the demonstration: the type system enforced
    // a cyclic order without complaint.
    #[test]
    fn still_allows_cyclic_order() {
        both_orders_are_provable();
    }
}
