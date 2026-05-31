#!/usr/bin/env python3
"""Hand expanded lock hierarchy: every reachable ordered pair gets a concrete
impl LockAfter, with no impl_transitive_lock_order macro.

lock_ordering's transitive macro is single parent (a node may have one transitive
predecessor, otherwise the blanket impls overlap with E0119). A multi parent DAG
can only be expressed by writing the transitive closure out by hand. To compare a
dense DAG against a chain or forest without also flipping the macro vs manual
axis, all three are emitted the same hand expanded way, so only topology varies.
The question is whether type check cost tracks closure size (reachable ordered
pairs) or depth (longest path).

A chain's closure is about N squared over 2; a forest's is the sum of small per
chain quadratics, linear in N; a dense tiered DAG's is quadratic at shallow depth.
If cost tracks closure, a dense shallow DAG is about as expensive as a deep chain
at the same N, which falsifies the idea that shallow and wide is always cheap.

Modes:
    chain N            total order, closure C(N,2)
    forest D W         W independent depth D chains, closure W*C(D,2)
    tiers s0 s1 ...    full cross tier order, closure sum over i<j of s_i*s_j
"""
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
           "// hand expanded transitive closure, no impl_transitive_lock_order macro:",
           "// one concrete LockAfter impl per reachable ordered pair, same style for",
           "// chain, forest, and tiers so the comparison isolates topology",
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
