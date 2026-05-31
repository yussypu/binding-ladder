#!/usr/bin/env python3
"""Generate the rung-1/2/5 BASELINE probe: N lock types, NO type-level ordering.

The compile-time jewel is specific to rung 4: the trait-solver work of proving a
transitive lock-order graph. Rungs 1, 2, and 5 still have N locks — they just do
not encode the order in the type system (rung 1/2 put it in a comment; rung 5
removes the second lock entirely). So the honest baseline for "what does the
build cost as lock count grows, WITHOUT the typestate machinery" is: declare the
same N marker types and do nothing with the trait solver.

Mirrors gen_levels.py's shape (same N `pub enum L{i}`) minus the `LockAfter`
graph and the `impl_transitive_lock_order!` calls — so the only difference timed
against gen_levels.py is the trait-graph cost itself.
"""
import sys


def emit(n: int) -> str:
    out = ["// BASELINE: N lock types, order is convention (a comment), not types.",
           "// LOCK ORDER: L0 < L1 < ... (unenforced — this is rungs 1/2/5)"]
    out += [f"pub enum L{i} {{}}" for i in range(n)]
    out += ["", "pub fn exercise() {}"]
    return "\n".join(out) + "\n"


if __name__ == "__main__":
    sys.stdout.write(emit(int(sys.argv[1])))
