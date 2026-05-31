//! Rigidity suite for rung 4 (type-level lock ordering).
//!
//! Three demonstrations, each a column in the cost table:
//!   * `legit_program_rejected` — a legitimate program static levels CANNOT
//!     express (runtime-indexed `account[i]`/`account[j]`).
//!   * `still_allows_cyclic_order` — declare a cyclic order; the type system
//!     enforces the unsound order you declared.
//!   * `still_allows_drop_order` — guards release out of acquisition order.
//!
//! The point of the suite: rung 4 is not "more correct for free." It buys
//! acquisition-order consistency *with what you declared* and pays in
//! expressiveness (it rejects legitimate dynamic programs) while still leaving
//! real holes (declared cycles, release order). These are the §6 honesty-ledger
//! items, now demonstrated rather than asserted.

pub mod legit_program_rejected;
pub mod still_allows_cyclic_order;
pub mod still_allows_drop_order;
