#!/usr/bin/env python3
"""Generate a probe crate: N lock levels in a total order, on lock_ordering.

Emits the rung 4 typestate deadlock free hierarchy at scale N, used by the
compile harness to measure the cost of declaring the order. Two findings from the
spike are baked in (see SPIKE.md):

The transitive closure chain hits Rust's default trait recursion limit of 128 at
about 128 levels (E0275, overflow evaluating the requirement), so we emit an
explicit recursion_limit scaled to N so large N proofs complete.

The cost is in declaring the hierarchy, not using it. The number of proof sites
is free since the solver caches within a crate, and one deep proof forces full
closure resolution.
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
            "// one deep proof forces full transitive closure resolution",
            f"pub fn exercise() {{ assert_before::<L0, L{n-1}>(); }}"]
    return "\n".join(out) + "\n"

if __name__ == "__main__":
    sys.stdout.write(emit(int(sys.argv[1])))
