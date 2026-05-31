#!/usr/bin/env python3
# baseline probe: N lock types, no ordering graph (rungs 1/2/5). gen_levels.py minus
# the LockAfter/impl_transitive_lock_order calls, so the diff is the trait graph cost.
import sys


def emit(n: int) -> str:
    out = ["// N lock types, order is convention not types (rungs 1, 2, 5)"]
    out += [f"pub enum L{i} {{}}" for i in range(n)]
    out += ["", "pub fn exercise() {}"]
    return "\n".join(out) + "\n"


if __name__ == "__main__":
    sys.stdout.write(emit(int(sys.argv[1])))
