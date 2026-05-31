#!/usr/bin/env python3
"""Generate a SHALLOW, WIDE lock hierarchy: W independent chains of depth D.

The chain generator (gen_levels.py) measures the worst case — a single total
order of N levels. Real lock hierarchies are shallow and wide (depth ~4, many
locks), not chains of 128. This generator builds the realistic shape using the
EXACT same mechanism (`impl_transitive_lock_order!`, single-parent transitive
edges) so the only thing that changes versus gen_levels.py is the TOPOLOGY:

  * total lock count N = D * W
  * longest transitive chain (closure depth) = D
  * width W = number of independent chains

A forest of chains is the cleanest expressible wide hierarchy: lock_ordering's
transitive macro models single-parent chains, so each of the W chains uses it
identically to gen_levels.py. By holding N constant and lowering D (deepening
into widening), we isolate whether the super-linear cost and the 128 cliff are
driven by DEPTH (closure length) or by total lock count.

recursion_limit scales with D only — a shallow forest never approaches the cliff.
"""
import sys


def emit(depth: int, width: int) -> str:
    out = [f'#![recursion_limit = "{max(256, depth * 4)}"]',
           "use lock_ordering::relation::{LockAfter, LockBefore};",
           "use lock_ordering::impl_transitive_lock_order;",
           ""]
    # W independent chains, each L{c}_0 < L{c}_1 < ... < L{c}_{depth-1}.
    for c in range(width):
        out += [f"pub enum L{c}_{i} {{}}" for i in range(depth)]
    out.append("")
    for c in range(width):
        for i in range(1, depth):
            out.append(f"impl LockAfter<L{c}_{i-1}> for L{c}_{i} {{}}")
            out.append(f"impl_transitive_lock_order!(L{c}_{i-1} => L{c}_{i});")
    out += ["",
            "fn assert_before<A, B>() where A: LockBefore<B> {}",
            "// One root->deepest proof per chain forces each closure to resolve.",
            "pub fn exercise() {"]
    for c in range(width):
        out.append(f"    assert_before::<L{c}_0, L{c}_{depth-1}>();")
    out.append("}")
    return "\n".join(out) + "\n"


if __name__ == "__main__":
    # args: depth width
    sys.stdout.write(emit(int(sys.argv[1]), int(sys.argv[2])))
