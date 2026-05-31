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

## risk_check invariant (second data point — shape moves)

| rung | runtime | compile-time jewel? | boilerplate | rejects | still allows |
| ---- | --- | --- | --- | --- | --- |
| 1 convention | ~0 (a branch) | none | 17 LOC / 114 tok | 0 | submit before/without a passing check |
| 4 typestate | ~0 (compile-time gate) | **none** — no transitive trait graph to solve | 37 LOC / 202 tok | nothing legitimate (a check is always wanted) | nothing for this invariant (the gate is total) |

The risk-check typestate is a fixed 2-state machine (`UncheckedOrder` → `CheckedOrder` via `RiskCheck::approve`), so there is no N to sweep and no super-linear curve — the rung-4 cost is ~2x boilerplate and nothing else. Same rung, different invariant, different cost shape: that is the point of the second data point.
