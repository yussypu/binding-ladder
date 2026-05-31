# Cost of climbing the ladder

Toolchain rustc 1.91.0 (f8297e351 2025-10-28). Machine aarch64/macos, 8 cores.
Runtime numbers are uncontended single thread ns/op, median run whole (runs=7, iters per run=5,000,000).
Compile numbers are clean probe builds, deps warm, CARGO_INCREMENTAL=0, -Ztime-passes, median run whole (runs=4 per cell).
Boilerplate is caller code between BOILERPLATE fences, comments and blanks excluded.

## deadlock invariant

| rung | runtime (ns/op) | build N=10 (s) | build N=100 (s) | boilerplate | legit programs rejected | still allows |
| ---- | ---: | ---: | ---: | --- | --- | --- |
| 1 convention | 29.5 (baseline) | 0.014 (flat) | 0.015 (flat) | 19 LOC / 163 tok | 0 | every deadlock |
| 2 runtime det. | 36.2 (about +10 detection) | 0.014 (flat) | 0.015 (flat) | 19 LOC / 151 tok | 0 | deadlock until a test hits it |
| 4 typestate | 30.7 (about baseline) | 0.019 | 0.084 | 50 LOC / 341 tok | runtime indexed locks | cyclic order you declared |
| 5 eliminated | n/a (no lock) | 0.014 (flat) | 0.015 (flat) | 33 LOC / 242 tok | the design that needs 2 locks | nothing, for this hazard |

## Compile cost vs lock count N for rung 4 (median run whole)

| N | baseline total (s) | typestate typeck (s) | typestate total (s) | typestate/baseline |
| ---: | ---: | ---: | ---: | ---: |
| 10 | 0.014 | 0.0020 | 0.019 | 1.4x |
| 25 | 0.014 | 0.0070 | 0.025 | 1.8x |
| 50 | 0.014 | 0.0190 | 0.039 | 2.8x |
| 75 | 0.015 | 0.0370 | 0.058 | 3.9x |
| 100 | 0.015 | 0.0610 | 0.084 | 5.6x |
| 128 | 0.015 | 0.1000 | 0.124 | 8.3x |
| 160 | 0.015 | 0.1530 | 0.179 | 11.9x |
| 200 | 0.016 | 0.2400 | 0.269 | 16.8x |
| 256 | 0.016 | 0.3980 | 0.430 | 26.9x |

Fitted power law exponent for N at least 50: typeck about O(N^1.87), total about O(N^1.49). The baseline is flat in N (0.014s at N=10 vs 0.016s at N=256). There is a recursion limit cliff at the default 128 (E0275) unless `#![recursion_limit]` is raised, verified on this toolchain.

## Topology: depth vs lock count

Same impl_transitive_lock_order mechanism as the chain, only the topology changes. The chain is the worst case, a total order of N levels.

Constant lock count N=160, deep to shallow:

| topology (depth x width) | N | typeck (s) | total (s) |
| --- | ---: | ---: | ---: |
| 160x1 (chain) | 160 | 0.1570 | 0.1840 |
| 80x2 | 160 | 0.0830 | 0.1090 |
| 40x4 | 160 | 0.0495 | 0.0760 |
| 16x10 | 160 | 0.0255 | 0.0520 |
| 8x20 | 160 | 0.0130 | 0.0405 |
| 4x40 (shallow forest) | 160 | 0.0090 | 0.0365 |

Same 160 locks: the deep chain type checks in 0.1570s, the depth 4 forest in 0.0090s, about 17x cheaper, because depth bounds closure size for a forest. The driver is closure, not depth (next table).

Fixed shallow depth 4, widening a sparse forest with more locks:

| topology | N | typeck (s) |
| --- | ---: | ---: |
| 4x10 | 40 | 0.0030 |
| 4x25 | 100 | 0.0060 |
| 4x40 | 160 | 0.0090 |
| 4x64 | 256 | 0.0140 |

A sparse depth 4 forest scales about linearly in lock count (about O(N^0.82), 0.0030s to 0.0140s for 40 to 256 locks) and never nears the 128 cliff. What makes it cheap is sparsity, not shallowness; the next table shows a shallow but dense DAG is as expensive as the deep chain.

## Topology: cost tracks closure size, not depth

All hand expanded (every reachable ordered pair is one concrete impl, no macro), so only topology varies. Closure is the number of reachable ordered pairs.

| config | depth | N | closure (pairs) | typeck (s) | us per pair |
| --- | ---: | ---: | ---: | ---: | ---: |
| forest:4:40 | 4 | 160 | 240 | 0.0050 | 20.8 |
| tiers:80:80 | 2 | 160 | 6400 | 0.0825 | 12.9 |
| tiers:53:53:54 | 3 | 160 | 8533 | 0.1340 | 15.7 |
| tiers:40:40:40:40 | 4 | 160 | 9600 | 0.1375 | 14.3 |
| chain:160 | 160 | 160 | 12720 | 0.1790 | 14.1 |

The decisive pair: forest:4:40 and tiers:40:40:40:40 have the same depth (4) and same N (160) but closures of 240 vs 9600 pairs, typeck 0.0050s vs 0.1375s, about 28x at identical depth. Depth does not drive cost, closure size does, at about constant us per pair across the dense configs. A shallow but densely cross connected DAG has quadratic closure and costs as much as a deep chain at the same N.

Cost tracks closure size (reachable ordered pairs). Depth bounds closure for chains and forests, so flattening a sparse hierarchy helps, but dense cross tier connectivity inflates closure independently of depth, so shallow and wide is cheap only when sparse. (An earlier draft claimed cross edges add at most linearly in edge count; a dense shallow DAG's closure is quadratic in N, so that was wrong.) The macro hits the recursion cliff because its proof depth equals the path length; the hand expanded form avoids the cliff but pays the same closure sized type check.

## Rung 2: what the runtime gap actually is

The runtime table shows rung 2 (parking_lot with detection) about 6.7 ns above rung 1 (std::sync::Mutex). That gap mixes two changes. The same hot path, parking_lot built with detection off and on, separates them:

| build | std mutex (ns/op) | parking_lot mutex (ns/op) |
| --- | ---: | ---: |
| detection off | 29.44 | 25.80 |
| detection on | 29.41 | 36.19 |

Implementation switch (std to parking_lot, no detection): -3.6 ns, parking_lot is faster uncontended. Detection bookkeeping (parking_lot off to on): +10.4 ns, about 40 percent over parking_lot's own baseline. Net vs rung 1 std: +6.7 ns.

So the detection tax is about 10 ns/op on every uncontended acquisition, deadlock or not, larger than the raw rung 1 to rung 2 gap because parking_lot starts ahead of std. That is the rung 2 number to print, not the mixed 6.7 ns.

## risk_check invariant (second data point)

| rung | runtime | compile time blowup? | boilerplate | rejects | still allows |
| ---- | --- | --- | --- | --- | --- |
| 1 convention | about 0 (a branch) | none | 17 LOC / 114 tok | 0 | submit before or without a passing check |
| 4 typestate | about 0 (compile time gate) | none, no transitive trait graph to solve | 37 LOC / 202 tok | nothing legitimate (a check is always wanted) | nothing for this invariant (the gate is total) |

The risk_check typestate is a fixed two state machine (UncheckedOrder to CheckedOrder via RiskCheck::approve), so there is no N to sweep and no super linear curve. The rung 4 cost is about 2x boilerplate and nothing else. Same rung, different invariant, different cost shape.
