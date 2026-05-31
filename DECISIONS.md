# Decisions

Decision log for the measurement repo. Every number traces to a committed log.
Newest at the bottom.

## ADR-001: Pin the toolchain; the exponent is toolchain bound

Pinned rustc 1.91.0 (f8297e351 2025-10-28) via rust-toolchain.toml. All columns under
results/ were produced on this toolchain and machine (aarch64 macOS, 8 cores). The
validation spike ran on 1.75.0, the old solver, in a Linux container; that data is
preserved at harness/results/compile_time_spike_1.75.json.

The exponent depends on the trait solver, so it is toolchain bound; the curve and the
cliff are not. Rerunning on 1.91 confirms it:

| metric | 1.75 spike (container) | 1.91 this repo (aarch64) |
| --- | --- | --- |
| fitted typeck exponent (N at least 50) | about 2.1 | about 1.87 |
| typeck at N=256 | 0.857 s | 0.398 s |
| recursion cliff | default 128 (E0275) | default 128 (E0275) |

Faster absolute times, slightly lower exponent, same shape and cliff. State the
toolchain next to any number, and never mix toolchains within one results run.

## ADR-002: Repo layout and a standalone probe crate

Layout is invariants/deadlock, invariants/risk_check, harness. The spike's flat files
were moved in, not rewritten. The probe crate is excluded from the workspace and keeps
its own Cargo.lock.

compile_time_bench.py regenerates probe/src/lib.rs per N and times cargo clean -p
probe plus a rebuild. Keeping probe out of the workspace isolates the timed build and
prevents workspace wide rebuilds from polluting the measurement. Cargo.lock is
committed everywhere, so deps are pinned.

## ADR-003: Rung 3, a true CI gate, is intentionally absent for deadlock

A rung 3 is a general mechanical gate where the build fails on violation, distinct
from rung 4 where the violation is unrepresentable. For lock ordering there is no
honest, invariant specific rung 3 between convention and typestate.

A lint that flags out of order acquisition has to reconstruct the same whole call
graph analysis lock_ordering pushes into the type system; a lexical lint catches only
obvious cases and is defeated by passing guards through a function. That is a worse
rung 2. A CI gate running tests under the rung 2 detector is rung 2 wired to a
pipeline, and inherits its weakness: it only fires if a test drives threads into the
cycle.

So a faithful rung 3 here collapses into rung 2 or requires the rung 4 machinery.
Faking one would be a green cell that does not mean what the column says. risk_check
has the opposite gap: clean rungs 1 and 4 but no natural rung 2 detector, because
there is no runtime symptom. Gaps are invariant dependent.

## ADR-004: Median run whole, enforced at table assembly

compile_time_bench.py prints per column medians, a typeck median and a total median
computed independently. Taken at face value that is the mistake the audit caught, so
we do not consume the printed medians. assemble_table.py reads the committed raw per
run records and, for each N, selects the single run whose total_s is the median
(closest when RUNS is even) and reports that run's typeck and total together.

Every column from the same run is the rule; the harness keeps per run records so the
selection happens at assembly. The runtime bench does the same via median_run_index
keyed on the rung 1 baseline. In runtime_bench.json the noisy seventh run spikes all
three rungs together, and per column medians would hide that correlation while the
whole run report preserves it.

## ADR-005: Boilerplate is fenced caller code, comments excluded

boilerplate.py counts caller code between BOILERPLATE-START/END fences, excluding
comments and blank lines, and reports code_loc and a token count.

The cost a rung imposes is the code you write per use site, not the library size.
Excluding comments is deliberate: a rung 1 convention is itself a comment, so its
enforcement cost is near zero lines. Result: deadlock rung 4 at 50 LOC / 341 tokens
vs rung 1 at 19 / 163, about 2.6x the lines; rung 2 about equal to rung 1 since the
detector is one time global setup. Tokens are reported because they are robust to
brace style.

The 50 LOC measures the 3 lock example, not a fixed tax. The rung 4 hierarchy
declaration is O(N): gen_levels.py emits 3N-2 authored lines (N level types plus
2(N-1) LockAfter and impl_transitive_lock_order lines), verified at 7, 28, 73, 298
lines for N of 3, 10, 25, 100. Same declaration that feeds the compile time curve, so
both scale with lock count. The framing is ceremony that scales with declared locks,
not use: you pay per declared lock up front, and acquisition sites are nearly free
(ADR-006).

## ADR-006: Runtime bench is uncontended and single threaded, rung 5 is n/a

runtime_bench microbenchmarks the hot path uncontended and single threaded, release
only, debug builds flagged invalid. It benches rungs 1, 2, 4 and reports rung 5 as
n/a.

The claim under test: type level safety is free at runtime, since PhantomData is zero
sized and erased by monomorphization. Isolated by measuring uncontended acquisition.
Median run whole, rung 4 is about equal to rung 1 (both floor at 29.4 ns/op; the 4.1
percent in the median run sits inside rung 4's own 29.4 to 37.4 spread). rung 2 with
parking_lot is about 36 ns; do not attribute the whole gap to detection, since it
mixes the implementation switch (ADR-011 separates them, pure detection tax about
10 ns). Rung 5 is excluded from the ns/op column on purpose: its hot path is a cross
thread channel round trip, a different cost class.

## ADR-007: A baseline generator isolates the type level cost

gen_baseline.py and baseline_compile.py use the same method with a different
generator: N lock types, no LockAfter graph, the rung 1/2/5 analog, swept over the
same N.

Build cost grows with lock count is only interesting against a control. Rungs 1, 2, 5
still have N locks, they just do not encode the order in types. The baseline is flat
(total 0.014 s at N=10, 0.016 s at N=256), so the gap between the two curves is the
type level ordering cost.

## ADR-008: risk_check has no compile time blowup, and we do not fake an N

risk_check is implemented at rungs 1 and 4 only. We state structurally that there is
no compile time curve, rather than sweep one.

The rung 4 is a fixed two state machine (UncheckedOrder to CheckedOrder via
RiskCheck::approve, private constructor). No transitive trait graph, no scale
parameter N, so nothing to sweep and no super linear cost. That absence is why it is
the second data point: same rung, different invariant, different cost shape, about
2.2x the rung 1 boilerplate and nothing else. Inventing an N would manufacture a
curve that is not there.

Claim discipline: two invariants show the cost shape is not universal, that it moves.
It is not a predictive taxonomy. Two points show variation exists; they do not let us
predict a third.

## ADR-009: Rigidity holes are demonstrated with tests, not asserted

Each hole has a runnable demonstration in harness/src/rigidity/.

legit_program_rejected: a compile_fail doctest shows the runtime indexed two account
transfer cannot be expressed, since both share one AccountLevel and the second
acquisition asks for the unprovable AccountLevel: LockBefore<AccountLevel>.

still_allows_cyclic_order: a passing test declares Mempool before Connection and
Connection before Mempool, and both acquisition orders type check.

still_allows_drop_order: a passing test acquires A, B, C and releases A first.
Acquisition order is checked, release order is not.

Still-allowed claims that are not executed are just prose. Compiling and running them
is the difference between a measurement and an assertion.

Implementation note: to get a real acquisition path for the runtime bench,
rung4_typestate was extended with LockLevel and MutexLockLevel impls, an Unlocked root
edge, and a hot_path. The pre existing verified items are unchanged. The Unlocked root
is a single concrete impl LockAfter<Unlocked> for AccountsTable; a transitive macro on
Unlocked would collide under coherence (E0119) because Unlocked is upstream, and the
chain macros already carry the edge down.

## ADR-010: Topology, depth vs lock count

Every chain number is a property of a total order of N levels, but real hierarchies
are shallow and wide. gen_dag.py builds a forest of W independent chains of depth D
with the same impl_transitive_lock_order mechanism, and dag_compile.py sweeps it.
Results in results/dag_compile.json.

Constant lock count N=160, deep to shallow: chain (160 by 1) typeck 0.157 s, forest
(4 by 40) typeck 0.009 s, about 17x cheaper. Fixed shallow depth 4, widening from 40
to 256 locks: typeck 0.003 to 0.014 s, about linear (about O(N^0.82)), and the
recursion limit is never approached.

Among forests, flattening lowers cost and a sparse depth 4 forest scales about
linearly, cliff free. But the first cut of this decision drew the wrong general
conclusion: that depth drives cost and cross edges add at most linearly in edge count
without deepening the closure. That bound was asserted, not measured, and is false.
See ADR-012, which replaces it with the measured driver: cost tracks closure size
(reachable ordered pairs), which depth bounds for forests and chains but dense cross
connectivity inflates independently. The forest sweep is correct for the sparse
family, just not the whole envelope.

## ADR-011: Rung 2 runtime gap, separate detection from the implementation switch

The rung 1 to rung 2 gap, about 6.7 ns, mixed two variables: rung 1 is
std::sync::Mutex, rung 2 is parking_lot::Mutex with deadlock_detection. The control
crate pl_control is kept out of harness's dependency graph so the feature can be
toggled per build without unification forcing it on. It benches the same 3 lock hot
path on a std mutex and a parking_lot mutex, built twice. Results in
results/rung2_control_detect_off.json and results/rung2_control_detect_on.json.

Median run whole, spreads under 0.15 ns:

| | std mutex | parking_lot mutex |
| --- | ---: | ---: |
| detection off | 29.44 | 25.80 |
| detection on  | 29.41 | 36.19 |

The implementation switch (std to parking_lot, no detection) is 3.6 ns faster. The
detection bookkeeping (parking_lot off to on) is 10.4 ns, about 40 percent over
parking_lot's baseline. Net against std rung 1 is 6.8 ns, and it matches the
runtime_bench rung 2 number (36.19 vs 36.17), so the control validates against the
original.

The rung 2 number to print is about 10 ns/op of detection bookkeeping on every
uncontended acquisition, not the mixed 6.7 ns, and it is larger than that gap because
parking_lot starts ahead of std.

## ADR-012: Type check cost tracks closure size, not depth (corrects ADR-010)

A reviewer flagged ADR-010's caveat (cross edges add at most linearly without
deepening the closure) as an unchecked bound with a likely counterexample: a depth 2
dense bipartite DAG (every A before every B) is maximally shallow yet has (N/2)^2
reachable ordered pairs, quadratic closure at depth 2. If cost tracks closure size,
which fits the chain's roughly 1.87 exponent (a chain's closure is about N^2/2), a
dense shallow DAG is as costly as a deep chain. gen_manual.py hand expands the full
closure, one concrete impl LockAfter per reachable pair, the only way to express a
multi parent DAG in lock_ordering; manual_compile.py times it. To isolate topology
from the macro vs manual axis, chain, forest, and dense tiers are all hand expanded.
Results in results/manual_topology.json.

N=160, hand expanded, median run whole:

| config | depth | closure (pairs) | typeck (s) | us per pair |
| --- | ---: | ---: | ---: | ---: |
| forest 4 by 40 | 4 | 240 | 0.0050 | 20.8 |
| tiers 80 by 80 | 2 | 6400 | 0.0825 | 12.9 |
| tiers 53 by 53 by 54 | 3 | 8533 | 0.1340 | 15.7 |
| tiers 40 by 40 by 40 by 40 | 4 | 9600 | 0.1375 | 14.3 |
| chain | 160 | 12720 | 0.1790 | 14.1 |

The caveat was wrong. The decisive pair, forest 4 by 40 and dense tiers 40 by 40 by 40
by 40, have identical depth (4) and N (160) but typeck 0.005 s vs 0.138 s, a 28x gap
from connectivity alone. Cost is about constant per reachable pair (roughly 13 to 16
us across the dense and chain configs), so cost tracks closure size. Depth bounds
closure for chains and forests, so flattening a sparse hierarchy helps, but dense
cross tier connectivity inflates closure independently of depth.

Corrected claim: not that shallow and wide is always cheap (that holds only for sparse
hierarchies). Cost tracks the number of ordered lock pairs the type system must know
about; a realistic hierarchy is cheap because it is sparse, not merely shallow. A
shallow but densely connected graph pays the full quadratic.

The hand expanded chain at N=160 compiles fine, no E0275, because concrete impls need
no recursive resolution. Hand expansion trades the recursion cliff (a macro and proof
depth artifact) for an explicit closure sized impl set: same cost magnitude as the
macro chain (0.179 s vs 0.157 s), no cliff. The cliff is about proof recursion depth,
the cost is about closure size.

An asserted bound was checked, failed, and replaced with a measured one. The corrected
finding, that cost tracks closure size and depth is only a proxy through sparsity, is
the more interesting version.

## ADR-013: The typestate hot path compiles to the same machine code as the plain version

Runtime parity (ADR-006) showed rung 4 benchmarks the same as rung 1, but equal
timings are weaker than equal code. harness/asm_hotpath.py emits assembly for both
hot_path functions (release, codegen-units=1), normalizes per crate symbol hashes and
local labels, and compares.

On aarch64 with rustc 1.91, both functions are 183 instructions and the instruction
multiset is identical. The first 157 instructions, the entire acquire, increment, and
release path, match instruction for instruction. The two diverge only in the cold
panic unwind tail (the drop glue when a mutex is poisoned), where the same
instructions are laid out in a different order, plus per crate hashes on the unwrap
panic location constants.

So the proof apparatus is not made cheap, it is absent from the emitted hot path. The
result holds with codegen-units=1, which release sets; at the default multi unit
release the cold tail can shuffle further, but the hot path identity is robust. The
claim is identical machine code on the hot path, not a byte for byte identical binary.
Result in results/asm_hotpath.json.
