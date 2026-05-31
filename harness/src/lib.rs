//! Measurement harness for the binding ladder.
//!
//! This lib hosts the **rigidity** suite — the legitimate programs rung 4
//! cannot express and the unsound things it still allows. These are table
//! columns, not footnotes (spec §3): each is a test or a `compile_fail` example
//! that demonstrates the named behavior.
//!
//! The other columns live alongside:
//!   * compile-time  -> `compile_time_bench.py` (+ `gen_levels.py`), reused as-is
//!   * runtime        -> the `runtime_bench` binary (`src/runtime_bench.rs`)
//!   * boilerplate    -> `boilerplate.py` (counts BOILERPLATE-marked caller code)

pub mod rigidity;
