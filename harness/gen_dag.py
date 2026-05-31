#!/usr/bin/env python3
# forest of W independent chains of depth D, same macro mechanism as gen_levels.py.
# N = D*W, longest chain D. recursion_limit scales with D only.
import sys


def emit(depth: int, width: int) -> str:
    out = [f'#![recursion_limit = "{max(256, depth * 4)}"]',
           "use lock_ordering::relation::{LockAfter, LockBefore};",
           "use lock_ordering::impl_transitive_lock_order;",
           ""]
    for c in range(width):
        out += [f"pub enum L{c}_{i} {{}}" for i in range(depth)]
    out.append("")
    for c in range(width):
        for i in range(1, depth):
            out.append(f"impl LockAfter<L{c}_{i-1}> for L{c}_{i} {{}}")
            out.append(f"impl_transitive_lock_order!(L{c}_{i-1} => L{c}_{i});")
    out += ["",
            "fn assert_before<A, B>() where A: LockBefore<B> {}",
            "// one root->deepest proof per chain forces each closure to resolve",
            "pub fn exercise() {"]
    for c in range(width):
        out.append(f"    assert_before::<L{c}_0, L{c}_{depth-1}>();")
    out.append("}")
    return "\n".join(out) + "\n"


if __name__ == "__main__":
    sys.stdout.write(emit(int(sys.argv[1]), int(sys.argv[2])))
