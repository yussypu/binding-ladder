#!/usr/bin/env python3
# probe crate: N lock levels in a total order on lock_ordering. recursion_limit is
# scaled to N (the chain hits the default 128 with E0275). one deep proof resolves
# the full closure; extra proof sites are free (solver caches within a crate).
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
