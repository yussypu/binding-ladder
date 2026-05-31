#!/usr/bin/env python3
"""Generate a probe crate: N lock levels in a total order, built on `lock_ordering`.

Emits the rung-4 (typestate) deadlock-freedom hierarchy at scale N, used by the
compile-time harness to measure the cost of declaring the order.

Findings baked in from the validation spike (see SPIKE.md):
  * The transitive-closure chain hits Rust's default trait recursion limit (128)
    at ~128 levels: E0275 'overflow evaluating the requirement'. We emit an
    explicit #![recursion_limit] scaled to N so large-N proofs actually complete.
  * The cost is in DECLARING the hierarchy, not using it: number of proof sites
    is free (the solver caches within a crate). One deep proof suffices to force
    full closure resolution.
"""
import sys

def emit(n: int) -> str:
    out = [f'#![recursion_limit = "{max(256, n * 4)}"]',
           "use lock_ordering::relation::{LockAfter, LockBefore};",
           "use lock_ordering::impl_transitive_lock_order;",
           ""]
    out += [f"pub enum L{i} {{}}" for i in range(n)]
    out.append("")
    for i in range(1, n):
        out.append(f"impl LockAfter<L{i-1}> for L{i} {{}}")
        out.append(f"impl_transitive_lock_order!(L{i-1} => L{i});")
    out += ["",
            "fn assert_before<A, B>() where A: LockBefore<B> {}",
            "// One root->deepest proof forces full transitive-closure resolution.",
            f"pub fn exercise() {{ assert_before::<L0, L{n-1}>(); }}"]
    return "\n".join(out) + "\n"

if __name__ == "__main__":
    sys.stdout.write(emit(int(sys.argv[1])))
