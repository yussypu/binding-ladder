# binding-ladder

Companion measurement repo for blog post 002 — *willpower doesn't scale / make it
impossible, not improbable*. **One invariant, every rung of the enforcement
ladder, measured.** This is not a library to adopt; it is a measurement to trust:
every number in the cost table is reproducible from the committed `results/*.json`.

## The ladder

From weakest enforcement to strongest (this is the NIOSH/OSHA **Hierarchy of
Controls** — administrative → engineering → elimination — applied to code):

1. **convention** — a comment. Pure willpower.
2. **review / runtime detection** — caught probabilistically, after the fact.
3. **CI gate** — the build fails. *(Intentionally absent for deadlock — see below.)*
4. **unrepresentable** — the violation does not compile.
5. **nonexistent** — the dangerous capability is not reachable at all.

Good engineering pushes invariants *down* this ladder. Every step down is a claim
that costs nothing. This repo checks the bill.

## Headline result

The full rendered table is in [`harness/results/cost_table.md`](harness/results/cost_table.md);
it is generated from the raw logs by `assemble_table.py`. The shape of it:

**deadlock invariant** (`rustc 1.91.0`, aarch64/macOS):

| rung | runtime (ns/op) | build N=10 (s) | build N=100 (s) | boilerplate | rejects | still allows |
| ---- | ---: | ---: | ---: | --- | --- | --- |
| 1 convention | 29.5 (baseline) | 0.014 (flat) | 0.015 (flat) | 19 LOC | 0 | every deadlock |
| 2 runtime det. | 36.2 (≈+10 detection) | 0.014 (flat) | 0.015 (flat) | 19 LOC | 0 | deadlock until a test hits it |
| 4 typestate | 30.7 (≈ baseline) | 0.019 | 0.084 | 50 LOC | runtime-indexed locks | cyclic order you declared |
| 5 eliminated | n/a (no lock) | 0.014 (flat) | 0.015 (flat) | 33 LOC | the design that needs 2 locks | nothing, for this hazard |

The curve, plainly: **as you climb to rung 4, runtime cost stays at zero
(PhantomData is erased) while compile-time cost and rigidity rise.** That is the
post's thesis backed by these numbers.

The rung-2 ns/op deserves a controlled read (`rung2_control_*.json`; DECISIONS
ADR-011). The raw +6.7 ns over std rung 1 conflates two changes; isolating them,
`parking_lot` is actually ~3.6 ns *faster* than std uncontended, and turning on
deadlock detection costs **~+10 ns/op (+40%)** of pure bookkeeping — paid on every
acquisition, forever, whether or not anything deadlocks. That is the printable
rung-2 number.

### The jewel — type-level lock ordering compiles super-linearly

The cost of rung 4 is paid **once per crate, at the compiler, in declaring the
hierarchy** (not in using it). It grows ~quadratically in the lock count N, while
the same N locks *without* type-level ordering compile flat:

| N | baseline total (s) | typestate typeck (s) | typestate total (s) | ratio |
| ---: | ---: | ---: | ---: | ---: |
| 10 | 0.014 | 0.002 | 0.019 | 1.4x |
| 50 | 0.014 | 0.019 | 0.039 | 2.8x |
| 100 | 0.015 | 0.061 | 0.084 | 5.6x |
| 128 | 0.015 | 0.100 | 0.124 | 8.3x |
| 256 | 0.016 | 0.398 | 0.430 | 26.9x |

Fitted **typeck ~O(N^1.87)** for N≥50 (the trait solver is the super-linear part;
it dominates wall-clock total only at large N). There is a **hard recursion-limit
cliff at the default 128**: past ~128 chained levels it does not slow down, it
*fails to compile* (`E0275`) until you raise `#![recursion_limit]`. Both verified
on the pinned toolchain.

**Reality check (per the §6 honesty ledger):** even N=256 is <0.5 s of
type-checking. Real hierarchies have dozens of levels, not hundreds. The
contribution is "the curve is genuinely super-linear and there is a silent cliff
at 128," not "this will wreck your build."

**Topology — depth, not lock count, is the driver** (`dag_compile.json`; DECISIONS
ADR-010). Holding lock count constant at N=160 and flattening the hierarchy from a
160-deep chain to a depth-4 forest drops type-check from 0.157 s to 0.009 s —
**~17× cheaper at identical lock count**. At a realistic depth of 4, widening from
40 to 256 locks is ~linear (~O(N^0.82)) and never approaches the cliff. The
super-linearity and the wall are properties of chain *depth*; a shallow, wide
hierarchy stays cheap however many locks it holds.

### Second data point — the shape moves per invariant

`risk_check` ("an order cannot be submitted without a passing risk check") at
rungs 1 and 4. Its rung-4 typestate is a fixed two-state machine, so it has the
same ~2× boilerplate cost but **no compile-time jewel** — no transitive trait
graph, nothing to sweep. Same rung, different invariant, different cost shape.

## Reproduce

```bash
# pinned automatically by rust-toolchain.toml (rustc 1.91.0)
export PROBE_DIR="$PWD/probe"

# everything compiles + every demonstration test and compile_fail doctest passes
cargo test --workspace

# runtime column (release is mandatory; debug builds are flagged invalid)
cargo run --release --bin runtime_bench

# boilerplate column
python3 harness/boilerplate.py

# compile-time column — rung-4 typestate curve (the jewel) and the flat baseline
python3 harness/gen_levels.py 10 > probe/src/lib.rs && (cd probe && cargo build -q)  # warm deps
RUNS=4 python3 harness/compile_time_bench.py     # -> results/compile_time.json
RUNS=4 python3 harness/baseline_compile.py       # -> results/baseline_compile.json

# topology audit — depth vs lock count (#1); shallow-wide forest vs deep chain
RUNS=4 python3 harness/dag_compile.py            # -> results/dag_compile.json

# rung-2 control (#2) — isolate detection bookkeeping from the impl switch
cargo run --release -q -p pl_control                  # detection OFF
cargo run --release -q -p pl_control --features detect # detection ON

# assemble the cost table from the committed logs (median run whole)
python3 harness/assemble_table.py                # -> results/cost_table.{md,json}
```

## Layout

```
invariants/
  deadlock/        rung1_convention  rung2_runtime  rung4_typestate  rung5_eliminated
  risk_check/      rung1_convention  rung4_typestate
harness/
  gen_levels.py            rung-4 hierarchy generator (N levels, total order)
  compile_time_bench.py    rung-4 compile-time sweep  (reused as-is)
  gen_baseline.py          N lock types, no trait graph (rung 1/2/5 control)
  baseline_compile.py      baseline compile-time sweep
  gen_dag.py               shallow-wide forest generator (depth D × width W)
  dag_compile.py           topology sweep: depth vs lock count (#1)
  boilerplate.py           caller-authored LOC/token count per rung
  src/runtime_bench.rs     ns/op hot path per rung
  src/rigidity/            legit-program-rejected · still-allows-cyclic · still-allows-drop
  assemble_table.py        builds results/cost_table.{md,json}
  results/*.json           raw, committed — numbers reproducible from logs
pl_control/                rung-2 detection control (#2): std vs parking_lot off/on
probe/                     regenerated per N by the compile harness (excluded from workspace)
```

## Methodology (crackeddb-grade)

- Machine, toolchain, and flags are pinned and stated in every results file.
- Clean vs. incremental builds are never mixed (`CARGO_INCREMENTAL=0`; the compile
  sweep `cargo clean -p probe` between timed builds, deps warm).
- N runs per cell; the **median run is reported whole** — every column from one
  run, selected from committed raw records, never per-column medians stitched
  together (the exact crackeddb-audit mistake; see `DECISIONS.md` ADR-004).
- Every "failure mode still allowed" is a runnable test, not a sentence.

See [`DECISIONS.md`](DECISIONS.md) for the ADR log, including why deadlock has no
honest rung 3 (ADR-003) and why the exponent is toolchain-bound (ADR-001).

## What rung 4 does NOT buy (rigidity — `harness/src/rigidity/`)

Type-level lock ordering is not "more correct for free." It enforces consistency
with the order *you declared*, and pays in expressiveness:

- **rejects legitimate programs** — runtime-indexed `account[i]`/`account[j]`
  transfers cannot be expressed; the safe order ("lower id first") is
  data-dependent, not type-dependent.
- **still allows a declared cycle** — write `A < B` and `B < A` and the type
  system enforces the unsound order without complaint.
- **still allows out-of-order release** — acquisition order is checked, guard
  `Drop` order is not.

## Credits & prior art

- **`lock_ordering`** (akonradi, Fuchsia-team lineage) — the rung-4 implementation
  measured here. We do not reimplement it; we measure it.
- **NIOSH/OSHA Hierarchy of Controls** — the ladder's ~50-year-old pedigree.
- The 2019 dev.to "Hierarchy of Controls for Software Engineering" post (prose
  mapping, no measurement) and the No Boilerplate "plain text" video (the seed).

The contribution is the *measured comparison* and the *cost-curve shape* — not an
invention of type-level deadlock freedom.
