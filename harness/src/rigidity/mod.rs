//! Rigidity suite for rung 4 (type level lock ordering). Three demonstrations:
//!   legit_program_rejected     a legitimate program static levels cannot express
//!                              (runtime indexed account[i]/account[j])
//!   still_allows_cyclic_order  declare a cyclic order, the type system enforces it
//!   still_allows_drop_order    guards release out of acquisition order
//!
//! Rung 4 is not more correct for free. It buys acquisition order consistency
//! with what you declared and pays in expressiveness, rejecting legitimate
//! dynamic programs while still leaving real holes. These are the honesty ledger
//! items, demonstrated rather than asserted.

pub mod legit_program_rejected;
pub mod still_allows_cyclic_order;
pub mod still_allows_drop_order;
