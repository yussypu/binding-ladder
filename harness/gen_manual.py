#!/usr/bin/env python3
# hand-expanded closures: one concrete LockAfter impl per reachable pair, no macro.
# modes: chain N | forest D W | tiers s0 s1 ...
import sys


def chain(n):
    nodes = [f"L{i}" for i in range(n)]
    pairs = [(nodes[i], nodes[j]) for i in range(n) for j in range(i + 1, n)]
    return nodes, pairs, [(nodes[0], nodes[-1])]


def forest(depth, width):
    nodes, pairs, proofs = [], [], []
    for c in range(width):
        cn = [f"L{c}_{i}" for i in range(depth)]
        nodes += cn
        for i in range(depth):
            for j in range(i + 1, depth):
                pairs.append((cn[i], cn[j]))
        proofs.append((cn[0], cn[-1]))
    return nodes, pairs, proofs


def tiers(sizes):
    tier_nodes, nodes = [], []
    for t, s in enumerate(sizes):
        tn = [f"T{t}_{k}" for k in range(s)]
        tier_nodes.append(tn)
        nodes += tn
    pairs = []
    for a in range(len(sizes)):
        for b in range(a + 1, len(sizes)):
            for x in tier_nodes[a]:
                for y in tier_nodes[b]:
                    pairs.append((x, y))
    return nodes, pairs, [(tier_nodes[0][0], tier_nodes[-1][0])]


def emit(nodes, pairs, proofs):
    out = ['#![recursion_limit = "1024"]',
           "use lock_ordering::relation::{LockAfter, LockBefore};",
           ""]
    out += [f"pub enum {n} {{}}" for n in nodes]
    out.append("")
    out += [f"impl LockAfter<{a}> for {b} {{}}" for a, b in pairs]
    out += ["", "fn assert_before<A, B>() where A: LockBefore<B> {}", "pub fn exercise() {"]
    out += [f"    assert_before::<{a}, {b}>();" for a, b in proofs]
    out.append("}")
    return "\n".join(out) + "\n"


if __name__ == "__main__":
    mode = sys.argv[1]
    if mode == "chain":
        spec = chain(int(sys.argv[2]))
    elif mode == "forest":
        spec = forest(int(sys.argv[2]), int(sys.argv[3]))
    elif mode == "tiers":
        spec = tiers([int(x) for x in sys.argv[2:]])
    else:
        raise SystemExit("mode: chain N | forest D W | tiers s0 s1 ...")
    sys.stdout.write(emit(*spec))
