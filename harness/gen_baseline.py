#!/usr/bin/env python3
"""Baseline probe for rungs 1, 2, 5: N lock types, no type level ordering.

The compile time cost is specific to rung 4, the trait solver work of proving a
transitive lock order graph. Rungs 1, 2, 5 still have N locks, they just do not
encode the order in types (1 and 2 put it in a comment, 5 removes the second lock
entirely). So the baseline for what the build costs as lock count grows without
the typestate machinery is N marker types and nothing for the solver to do.

Same shape as gen_levels.py (N enums) minus the LockAfter graph and the
impl_transitive_lock_order calls, so the only difference timed against
gen_levels.py is the trait graph cost.
"""
import sys


def emit(n: int) -> str:
    out = ["// baseline: N lock types, order is convention not types",
           "// lock order L0 < L1 < ... is unenforced, this is rungs 1, 2, 5"]
    out += [f"pub enum L{i} {{}}" for i in range(n)]
    out += ["", "pub fn exercise() {}"]
    return "\n".join(out) + "\n"


if __name__ == "__main__":
    sys.stdout.write(emit(int(sys.argv[1])))
