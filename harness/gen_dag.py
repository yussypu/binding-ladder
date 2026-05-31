#!/usr/bin/env python3
"""Shallow wide lock hierarchy: W independent chains of depth D.

gen_levels.py measures the worst case, a single total order of N levels. Real
hierarchies are shallow and wide (depth around 4, many locks), not chains of 128.
This builds the realistic shape with the same mechanism (impl_transitive_lock_order,
single parent transitive edges), so the only thing that changes versus
gen_levels.py is the topology. Total lock count is N = D * W, the longest
transitive chain is D, and width W is the number of independent chains.

A forest of chains is the cleanest wide hierarchy lock_ordering can express,
since its transitive macro models single parent chains. Holding N constant and
lowering D isolates whether the cost and the 128 cliff are driven by depth or by
total lock count. recursion_limit scales with D only, so a shallow forest never
nears the cliff.
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
