//! Rigidity suite for rung 4: a legitimate program it rejects, plus two unsound
//! things it still allows (a declared cycle, out-of-order release). Each is a
//! runnable test or compile_fail example.

pub mod legit_program_rejected;
pub mod still_allows_cyclic_order;
pub mod still_allows_drop_order;
