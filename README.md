# binding-ladder

Companion measurement repo for blog post 002 (working titles: willpower doesn't
scale, make it impossible not improbable). One invariant, every rung of the
enforcement ladder, measured. This is not a library to adopt, it is a set of
measurements to trust. Every number in the cost table is reproducible from the
committed files under `results/`.

## The ladder

From weakest enforcement to strongest. This is the NIOSH and OSHA Hierarchy of
Controls (administrative controls, engineering controls, elimination) applied to
code.

1. convention: a comment. Pure willpower.
2. review or runtime detection: caught probabilistically, after the fact.
3. CI gate: the build fails. Intentionally absent for deadlock, see below.
4. unrepresentable: the violation does not compile.
5. nonexistent: the dangerous capability is not reachable at all.

Good engineering pushes invariants down this ladder. Every step down is a claim
that costs nothing. This repo checks the bill.

## Headline result

The full rendered table is in [`harness/results/cost_table.md`](harness/results/cost_table.md),
generated from the raw logs by `assemble_table.py`. The shape of it, for the
deadlock invariant on rustc 1.91.0, aarch64 macOS:

| rung | runtime (ns/op) | build N=10 (s) | build N=100 (s) | boilerplate | rejects | still allows |
| ---- | ---: | ---: | ---: | --- | --- | --- |
| 1 convention | 29.5 (baseline) | 0.014 (flat) | 0.015 (flat) | 19 LOC | 0 | every deadlock |
| 2 runtime det. | 36.2 (about +10 detection) | 0.014 (flat) | 0.015 (flat) | 19 LOC | 0 | deadlock until a test hits it |
| 4 typestate | 30.7 (about baseline) | 0.019 | 0.084 | 50 LOC | runtime indexed locks | cyclic order you declared |
| 5 eliminated | n/a (no lock) | 0.014 (flat) | 0.015 (flat) | 33 LOC | the design that needs 2 locks | nothing, for this hazard |

As you climb to rung 4, runtime cost stays at zero (PhantomData is erased) while
compile time cost and rigidity rise. That is the post's thesis backed by numbers.

The rung 2 ns/op deserves a controlled read (`rung2_control_*.json`, DECISIONS
ADR-011). The raw 6.7 ns over std rung 1 mixes two changes. Separating them,
parking_lot is about 3.6 ns faster than std uncontended, and turning on deadlock
detection costs about 10 ns/op (about 40 percent) of pure bookkeeping, paid on
every acquisition whether or not anything deadlocks. That is the rung 2 number to
print.

## Compile time cost grows with the declared hierarchy

The cost of rung 4 is paid once per crate, at the compiler, in declaring the
hierarchy and not in using it. It grows about quadratically in the lock count N,
while the same N locks without type level ordering compile flat:

| N | baseline total (s) | typestate typeck (s) | typestate total (s) | ratio |
| ---: | ---: | ---: | ---: | ---: |
| 10 | 0.014 | 0.002 | 0.019 | 1.4x |
| 50 | 0.014 | 0.019 | 0.039 | 2.8x |
| 100 | 0.015 | 0.061 | 0.084 | 5.6x |
| 128 | 0.015 | 0.100 | 0.124 | 8.3x |
| 256 | 0.016 | 0.398 | 0.430 | 26.9x |

Fitted typeck is about O(N^1.87) for N at least 50. The trait solver is the super
linear part and dominates wall clock total only at large N. There is a recursion
limit cliff at the default 128: past about 128 chained levels it does not slow
down, it fails to compile (E0275) until you raise `#![recursion_limit]`. Both
verified on the pinned toolchain.

Even N=256 is under 0.5 s of type checking. Real hierarchies have dozens of
levels, not hundreds. The contribution is that the curve is genuinely super
linear and there is a silent cliff at 128, not that this will wreck your build.

The cost tracks closure size (reachable ordered pairs), not depth
(`dag_compile.json`, `manual_topology.json`, DECISIONS ADR-010 and ADR-012).
Flattening a sparse forest helps (a 160 deep chain becomes a depth 4 forest at
N=160, 0.157 s to 0.009 s), but only because depth bounds closure for a forest.
Hand expanded at N=160, a forest of 240 reachable pairs type checks in 0.005 s
while a shallow but dense DAG of the same depth 4 and same N (40 by 40 by 40 by
40 tiers, 9600 pairs) takes 0.138 s, a 28x gap from connectivity alone, at about
constant cost per pair. So shallow and wide is cheap only for sparse hierarchies;
a densely cross connected lock graph pays the full quadratic even at depth 2 to 4.
An earlier draft asserted cross edges cost at most linearly in edge count, which
was unchecked and wrong, now measured and corrected. The recursion cliff is a
separate proof depth artifact: hand expansion avoids the cliff but pays the same
closure sized type check.

## Second data point: the shape moves per invariant

risk_check (an order cannot be submitted without a passing risk check) at rungs 1
and 4. Its rung 4 typestate is a fixed two state machine, so it has the same
roughly 2x boilerplate cost but no compile time blowup, since there is no
transitive trait graph to sweep. Same rung, different invariant, different cost
shape.

## Reproduce

```bash
# pinned automatically by rust-toolchain.toml (rustc 1.91.0)
export PROBE_DIR="$PWD/probe"

# everything compiles and every test and compile_fail doctest passes
cargo test --workspace

# runtime column (release is mandatory, debug builds are flagged invalid)
cargo run --release --bin runtime_bench

# boilerplate column
python3 harness/boilerplate.py

# compile time column: rung 4 typestate curve and the flat baseline
python3 harness/gen_levels.py 10 > probe/src/lib.rs && (cd probe && cargo build -q)  # warm deps
RUNS=4 python3 harness/compile_time_bench.py     # writes results/compile_time.json
RUNS=4 python3 harness/baseline_compile.py       # writes results/baseline_compile.json

# topology: depth vs lock count, shallow wide forest vs deep chain
RUNS=4 python3 harness/dag_compile.py            # writes results/dag_compile.json

# closure size: hand expanded chain, forest, and dense tiers at N=160
RUNS=4 python3 harness/manual_compile.py         # writes results/manual_topology.json

# rung 2 control: separate detection bookkeeping from the implementation switch
cargo run --release -q -p pl_control                   # detection off
cargo run --release -q -p pl_control --features detect # detection on

# assemble the cost table from the committed logs (median run whole)
python3 harness/assemble_table.py                # writes results/cost_table.md and .json
```

## Layout

```
invariants/
  deadlock/        rung1_convention  rung2_runtime  rung4_typestate  rung5_eliminated
  risk_check/      rung1_convention  rung4_typestate
harness/
  gen_levels.py            rung 4 hierarchy generator (N levels, total order)
  compile_time_bench.py    rung 4 compile time sweep
  gen_baseline.py          N lock types, no trait graph (rung 1, 2, 5 control)
  baseline_compile.py      baseline compile time sweep
  gen_dag.py               shallow wide forest generator (depth D, width W)
  dag_compile.py           topology sweep, depth vs lock count
  gen_manual.py            hand expanded closure generator (chain, forest, tiers)
  manual_compile.py        closure size sweep, cost vs reachable pairs
  boilerplate.py           caller LOC and token count per rung
  src/runtime_bench.rs     ns/op hot path per rung
  src/rigidity/            legit_program_rejected, still_allows_cyclic_order, still_allows_drop_order
  assemble_table.py        builds results/cost_table.md and .json
  results/*.json           raw, committed, numbers reproducible from logs
pl_control/                rung 2 detection control, std vs parking_lot off and on
probe/                     regenerated per N by the compile harness (excluded from workspace)
```

## Methodology

Machine, toolchain, and flags are pinned and stated in every results file. Clean
and incremental builds are never mixed: CARGO_INCREMENTAL=0, and the compile
sweep runs cargo clean -p probe between timed builds with deps warm. N runs per
cell, and the median run is reported whole, every column from one run, selected
from committed raw records, never per column medians stitched together (the
crackeddb audit mistake, see DECISIONS ADR-004). Every failure mode still allowed
is a runnable test, not a sentence.

See [`DECISIONS.md`](DECISIONS.md) for the decision log, including why deadlock
has no honest rung 3 (ADR-003) and why the exponent is toolchain bound (ADR-001).

## What rung 4 does not buy (rigidity)

Type level lock ordering is not more correct for free. It enforces consistency
with the order you declared, and pays in expressiveness. The suite under
`harness/src/rigidity/` demonstrates three holes:

- rejects legitimate programs: runtime indexed account[i] and account[j]
  transfers cannot be expressed, because the safe order (lower id first) is data
  dependent, not type dependent.
- still allows a declared cycle: write A before B and B before A and the type
  system enforces the unsound order without complaint.
- still allows out of order release: acquisition order is checked, guard Drop
  order is not.

## Credits and prior art

- lock_ordering (akonradi, Fuchsia team lineage), the rung 4 implementation
  measured here. We do not reimplement it, we measure it.
- NIOSH and OSHA Hierarchy of Controls, the ladder's roughly 50 year old pedigree.
- The 2019 dev.to post Hierarchy of Controls for Software Engineering (prose
  mapping, no measurement) and the No Boilerplate plain text video (the seed).

The contribution is the measured comparison and the cost curve shape, not an
invention of type level deadlock freedom.
