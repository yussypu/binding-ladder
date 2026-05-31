//! Rung 4 constrains acquisition order, not release. The guards it hands back are
//! ordinary MutexGuards, droppable in any order, so a discipline assuming strict
//! LIFO release gets no help. Shown on plain std::sync::Mutex guards (the same
//! type LockedAt::lock returns): acquire A, B, C, release A first.

use std::sync::Mutex;

// acquire A, B, C in order, release out of order; returns the release order
pub fn release_out_of_acquisition_order() -> Vec<&'static str> {
    let a = Mutex::new(0u64);
    let b = Mutex::new(0u64);
    let c = Mutex::new(0u64);

    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();
    let gc = c.lock().unwrap();
    let _ = (&*ga, &*gb, &*gc);

    let mut released = Vec::new();
    // release A first, still holding the inner two; strict LIFO forbids this
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
