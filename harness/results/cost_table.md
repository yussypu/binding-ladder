# Cost of climbing the ladder — measured

- Toolchain: `rustc 1.91.0 (f8297e351 2025-10-28)`  ·  Machine: aarch64/macos, 8 cores
- Runtime: uncontended single-thread, ns/op, median run whole (runs=7, iters/run=5,000,000).
- Compile: clean `probe` build, deps warm, `CARGO_INCREMENTAL=0`, `-Ztime-passes`, median run whole (runs=4/cell).
- Boilerplate: caller-authored code between BOILERPLATE fences (comments/blanks excluded).

## deadlock invariant

| rung | runtime (ns/op) | build N=10 (s) | build N=100 (s) | boilerplate | legit programs rejected | still allows |
| ---- | ---: | ---: | ---: | --- | --- | --- |
| 1 convention | 29.5 (baseline) | 0.014 (flat) | 0.015 (flat) | 19 LOC / 163 tok | 0 | every deadlock |
| 2 runtime det. | 36.2 (+ε, parking_lot) | 0.014 (flat) | 0.015 (flat) | 19 LOC / 151 tok | 0 | deadlock until a test hits it |
| 4 typestate | 30.7 (≈ baseline) | 0.019 | 0.084 | 50 LOC / 341 tok | runtime-indexed locks | cyclic order you declared |
| 5 eliminated | n/a (no lock) | 0.014 (flat) | 0.015 (flat) | 33 LOC / 242 tok | the design that needs 2 locks | nothing, for this hazard |

## the jewel — rung-4 compile cost vs lock count N (median run whole)

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

Fitted power-law exponent (N≥50): **typeck ~O(N^1.87)**, total ~O(N^1.49). Baseline is flat in N (0.014s at N=10 vs 0.016s at N=256). Hard recursion-limit cliff at the default 128 (E0275) unless `#![recursion_limit]` is raised — verified on this toolchain.

## topology — is the curve a chain artifact? (depth vs lock count)

Same `impl_transitive_lock_order!` mechanism as the chain; only the shape changes. The chain is the worst case (a total order of N levels); real hierarchies are shallow and wide.

**Constant lock count N=160, deep → shallow:**

| topology (depth×width) | N | typeck (s) | total (s) |
| --- | ---: | ---: | ---: |
| 160×1 (chain) | 160 | 0.1570 | 0.1840 |
| 80×2 | 160 | 0.0830 | 0.1090 |
| 40×4 | 160 | 0.0495 | 0.0760 |
| 16×10 | 160 | 0.0255 | 0.0520 |
| 8×20 | 160 | 0.0130 | 0.0405 |
| 4×40 (shallow forest) | 160 | 0.0090 | 0.0365 |

Same 160 locks: the deep chain type-checks in 0.1570s, the depth-4 forest in 0.0090s — **17× cheaper.** Flattening a *forest* lowers cost — but that is because for a forest, depth bounds the closure size. Depth is not the real driver; closure is (see the next table, which breaks the depth↔cost link).

**Fixed shallow depth 4, widening a sparse forest (more locks):**

| topology | N | typeck (s) |
| --- | ---: | ---: |
| 4×10 | 40 | 0.0030 |
| 4×25 | 100 | 0.0060 |
| 4×40 | 160 | 0.0090 |
| 4×64 | 256 | 0.0140 |

A sparse depth-4 forest scales ~linearly in lock count (~O(N^0.82), 0.0030s→0.0140s for 40→256 locks) and never nears the 128 cliff. But 'shallow' is not what makes it cheap — *sparse* is. The next table shows a shallow but DENSE DAG is as expensive as the deep chain.

## topology, settled — cost tracks CLOSURE SIZE, not depth

All hand-expanded (every reachable ordered pair = one concrete impl, no macro), so only topology varies. Closure = # reachable ordered pairs.

| config | depth | N | closure (pairs) | typeck (s) | µs / pair |
| --- | ---: | ---: | ---: | ---: | ---: |
| forest:4:40 | 4 | 160 | 240 | 0.0050 | 20.8 |
| tiers:80:80 | 2 | 160 | 6400 | 0.0825 | 12.9 |
| tiers:53:53:54 | 3 | 160 | 8533 | 0.1340 | 15.7 |
| tiers:40:40:40:40 | 4 | 160 | 9600 | 0.1375 | 14.3 |
| chain:160 | 160 | 160 | 12720 | 0.1790 | 14.1 |

**The decisive pair:** `forest:4:40` and `tiers:40:40:40:40` have the *same depth (4) and same N (160)* but closures of 240 vs 9600 pairs — typeck 0.0050s vs 0.1375s, a **28× gap at identical depth.** Depth does not drive cost; closure size does (~constant µs/pair across the dense configs). A shallow but densely cross-connected DAG has quadratic closure and costs as much as a deep chain at the same N.

**Correction (crackeddb ethos — don't ship an unchecked bound).** An earlier draft claimed cross-edges 'add at most linearly in edge count without deepening the closure.' That is FALSE: a dense shallow DAG's closure is quadratic in N. The honest, measured finding is: **cost ∝ closure size (reachable ordered pairs)**. Depth bounds closure for chains and trees/forests (so flattening a sparse hierarchy helps), but dense cross-tier connectivity inflates closure independently of depth. 'Shallow-wide is cheap' holds only for *sparse* hierarchies. (The macro hits the recursion cliff because its proof recursion depth = path length; the hand-expanded form avoids the cliff but pays the same O(closure) type-check — same cost, no cliff.)

## rung 2 — what the runtime gap actually is (controlled)

The runtime table shows rung 2 (`parking_lot` + detection) ~6.7 ns above rung 1 (`std::sync::Mutex`). That gap conflates two changes. Same hot path, parking_lot built with detection off vs on, isolates them:

| build | std mutex (ns/op) | parking_lot mutex (ns/op) |
| --- | ---: | ---: |
| detection OFF | 29.44 | 25.80 |
| detection ON | 29.41 | 36.19 |

- implementation switch (std → parking_lot, no detection): **-3.6 ns** — parking_lot is *faster* uncontended.
- pure deadlock-detection bookkeeping (parking_lot off → on): **+10.4 ns** (+40% over parking_lot's own baseline).
- net vs rung-1 std: +6.7 ns.

So the detection tax is **~10 ns/op paid on every uncontended acquisition, forever, whether or not anything ever deadlocks** — larger than the raw rung-1→rung-2 gap suggests, because parking_lot starts out ahead of std. That is the printable rung-2 number, not the conflated 6.7 ns.

## risk_check invariant (second data point — shape moves)

| rung | runtime | compile-time jewel? | boilerplate | rejects | still allows |
| ---- | --- | --- | --- | --- | --- |
| 1 convention | ~0 (a branch) | none | 17 LOC / 114 tok | 0 | submit before/without a passing check |
| 4 typestate | ~0 (compile-time gate) | **none** — no transitive trait graph to solve | 37 LOC / 202 tok | nothing legitimate (a check is always wanted) | nothing for this invariant (the gate is total) |

The risk-check typestate is a fixed 2-state machine (`UncheckedOrder` → `CheckedOrder` via `RiskCheck::approve`), so there is no N to sweep and no super-linear curve — the rung-4 cost is ~2x boilerplate and nothing else. Same rung, different invariant, different cost shape: that is the point of the second data point.
