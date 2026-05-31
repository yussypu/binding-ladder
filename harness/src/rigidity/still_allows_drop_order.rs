//! Column: rung 4 still allows releasing locks out of acquisition order.
//!
//! Type-level lock ordering constrains *acquisition*: you cannot acquire B
//! while holding A unless A < B. It says nothing about *release*. The guards it
//! hands back are ordinary `MutexGuard`s — RAII values you may `drop` in any
//! order you like. A discipline that assumes strict LIFO release (common in
//! hierarchical locking, and required by some lock-coupling / hand-over-hand
//! traversals to stay correct) gets no help from rung 4 here.
//!
//! We demonstrate on plain `std::sync::Mutex` guards — the exact type
//! `lock_ordering` returns from `LockedAt::lock`. Acquire A→B→C, then release A
//! first (non-LIFO). It compiles and runs; nothing in the type system objects.

use std::sync::Mutex;

/// Acquire three locks in order, then release them out of acquisition order.
/// Returns the order in which the locks were *released*, to prove it ran.
pub fn release_out_of_acquisition_order() -> Vec<&'static str> {
    let a = Mutex::new(0u64);
    let b = Mutex::new(0u64);
    let c = Mutex::new(0u64);

    // Acquisition order: A, then B, then C (a legal A < B < C order).
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();
    let gc = c.lock().unwrap();
    let _ = (&*ga, &*gb, &*gc);

    let mut released = Vec::new();
    // Release order: A first — i.e. drop the OUTERMOST lock while still holding
    // the inner two. Strict-LIFO disciplines forbid this; the type system does
    // not. (Then C, then B — fully scrambled, just to make the point.)
    drop(ga);
    released.push("A");
    drop(gc);
    released.push("C");
    drop(gb);
    released.push("B");
    released
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fact that this compiles and the locks come back in non-LIFO order is
    /// the demonstration: acquisition order is enforced, release order is not.
    #[test]
    fn still_allows_drop_order() {
        let released = release_out_of_acquisition_order();
        assert_eq!(released, vec!["A", "C", "B"]);
        // Acquired A,B,C; released A,C,B — out of acquisition order, no error.
        assert_ne!(released, vec!["C", "B", "A"], "this would be the LIFO order");
    }
}
