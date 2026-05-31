//! Rung 4 still allows releasing locks out of acquisition order.
//!
//! Type level lock ordering constrains acquisition: you cannot acquire B while
//! holding A unless A < B. It says nothing about release. The guards it hands
//! back are ordinary MutexGuards, RAII values you may drop in any order. A
//! discipline that assumes strict LIFO release (common in hierarchical locking,
//! and required by some hand over hand traversals) gets no help here.
//!
//! Demonstrated on plain std::sync::Mutex guards, the same type LockedAt::lock
//! returns. Acquire A, B, C, then release A first. It compiles and runs.

use std::sync::Mutex;

// Acquire three locks in order, then release them out of order. Returns the
// release order to prove it ran.
pub fn release_out_of_acquisition_order() -> Vec<&'static str> {
    let a = Mutex::new(0u64);
    let b = Mutex::new(0u64);
    let c = Mutex::new(0u64);

    // acquisition order: A, B, C
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();
    let gc = c.lock().unwrap();
    let _ = (&*ga, &*gb, &*gc);

    let mut released = Vec::new();
    // release A first, while still holding the inner two; strict LIFO forbids
    // this, the type system does not
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

    #[test]
    fn still_allows_drop_order() {
        let released = release_out_of_acquisition_order();
        assert_eq!(released, vec!["A", "C", "B"]);
        assert_ne!(released, vec!["C", "B", "A"], "this would be the LIFO order");
    }
}
