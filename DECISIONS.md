# Decisions

Decision log for the measurement repo behind blog post 002. Every number traces
to a committed log, and every gap is recorded as a finding rather than left
silent. Newest decisions are at the bottom.

## ADR-001: Pin the toolchain and record that the exponent is toolchain bound

Pinned rustc 1.91.0 (f8297e351 2025-10-28) via rust-toolchain.toml. All columns
under results/ were produced on this toolchain and machine (aarch64 macOS, 8
cores). The validation spike ran on 1.75.0, the old trait solver, in a Linux
container; that data is preserved at harness/results/compile_time_spike_1.75.json
and not overwritten.

The compile time exponent depends on the trait solver, so it is toolchain bound;
the existence of the curve and the cliff is not. Rerunning on 1.91 confirms this:

| metric | 1.75 spike (container) | 1.91 this repo (aarch64) |
| --- | --- | --- |
| fitted typeck exponent (N at least 50) | about 2.1 | about 1.87 |
| typeck at N=256 | 0.857 s | 0.398 s |
| recursion cliff | default 128 (E0275) | default 128 (E0275) |

Faster absolute times from the newer solver and faster machine, a slightly lower
exponent, the same shape and the same cliff. State the toolchain next to any
number lifted into the article, and never mix toolchains within one results run.

## ADR-002: Repo layout and a standalone probe crate

The repo follows the spec layout (invariants/deadlock, invariants/risk_check,
harness). The spike's flat files were moved in, not rewritten: gen_levels.py and
compile_time_bench.py into harness, the verified rung 4 example into
invariants/deadlock/rung4_typestate/src/lib.rs. The probe crate is excluded from
the workspace and keeps its own Cargo.lock.

compile_time_bench.py regenerates probe/src/lib.rs per N and times cargo clean -p
probe plus a rebuild. Keeping probe out of the workspace isolates the timed build
from the rung crates and prevents workspace wide rebuilds from polluting the
measurement. Cargo.lock is committed everywhere, so deps are pinned and numbers
reproducible.

## ADR-003: Rung 3, a true CI gate, is intentionally absent for deadlock

We do not build a rung 3 for the deadlock invariant, and we document the gap.

A rung 3 control is a general mechanical gate where the build fails on violation,
distinct from rung 4 where the violation is unrepresentable in the type system.
For lock ordering there is no honest, invariant specific rung 3 between convention
and typestate.

A lint that flags locks acquired in the wrong order has to reason about which
lock is which and what order is intended across the whole call graph, which means
reconstructing the same analysis lock_ordering pushes into the type system. A
realistic lexical lint catches only the most obvious cases and is trivially
defeated by passing guards through a function. That is not a gate, it is a worse
rung 2. A dynamic CI gate that runs tests under the rung 2 detector is not a new
rung either, it is rung 2 wired to a pipeline, and it inherits rung 2's weakness:
it only fires if a test happens to drive threads into the cycle.

So a faithful rung 3 for this invariant would either collapse into rung 2 or
require the rung 4 machinery to be sound. Faking one, a toy lint that always
passes, would be a green cell that does not mean what the column header says. Not
every invariant has a clean rung at every level. risk_check has the opposite gap:
clean rung 1 and rung 4 but no natural rung 2 detector, because there is no
runtime symptom to detect. Gaps are invariant dependent, and that is content.

## ADR-004: Median run whole is enforced at table assembly time

compile_time_bench.py is reused as is. It prints per column medians, a typeck
median and a total median computed independently, which taken at face value is
the mistake the crackeddb audit caught. We do not consume those printed medians
for the cost table. Instead assemble_table.py reads the committed raw per run
records and, for each N, selects the single run whose total_s is the median
(closest to median when RUNS is even) and reports that run's typeck and total
together.

Every column from the same single run is the rule. The harness keeps per run
records precisely so this selection can happen at assembly. The runtime bench
does the same internally via median_run_index keyed on the rung 1 baseline.
Evidence the rule matters: in runtime_bench.json the noisy seventh run spikes
rung 1, rung 2, and rung 4 together, and per column medians would hide that
correlation while the whole run report preserves it.

## ADR-005: Boilerplate is fenced caller code, comments excluded

boilerplate.py counts caller code between BOILERPLATE-START and BOILERPLATE-END
fences in each rung's source, excluding comments and blank lines, and reports
both code_loc and a token count.

The cost a rung imposes is the code you write per use site, not the library size.
Excluding comments is deliberate: a rung 1 convention is itself a comment, so its
enforcement cost is near zero lines of code, which is the whole point of why it
decays. The result is deadlock rung 4 at 50 LOC and 341 tokens against rung 1 at
19 and 163, about 2.6 times the lines, and rung 2 about equal to rung 1 since the
detector is one time global setup, not per site. The token metric is reported
because it is robust to brace style and formatting differences.

The 50 LOC is not a fixed tax, it measures the 3 lock example. The rung 4
hierarchy declaration is O(N) in the number of locks, not constant: gen_levels.py
emits 3N minus 2 authored lines (N level types plus 2 times N minus 1 LockAfter
and impl_transitive_lock_order lines), verified at 7, 28, 73, 298 lines for N of
3, 10, 25, 100. This is the same declaration that feeds the compile time curve,
which is why both scale with lock count. The honest framing is ceremony that
scales with the number of locks you declare, not with how much you use them: you
pay per declared lock once, up front, and acquisition sites are nearly free
(ADR-006).

## ADR-006: Runtime bench is uncontended and single threaded, rung 5 is n/a

runtime_bench microbenchmarks the hot path uncontended and single threaded,
release only, with debug builds flagged invalid in the JSON. It benches rungs 1,
2, 4 and reports rung 5 as n/a, no lock.

The claim under test is that the type level safety is free at runtime, since
PhantomData is zero sized and erased by monomorphization. That is a property of
the mechanism, isolated by measuring uncontended acquisition rather than
contention. Median run whole, rung 4 is about equal to rung 1 (both floor at 29.4
ns/op; the 4.1 percent in the median run sits inside rung 4's own run to run
spread of 29.4 to 37.4). rung 2 with parking_lot is about 36 ns. Do not attribute
that whole gap to detection, since it mixes the std to parking_lot switch with
detection on and off; ADR-011 separates them and the pure detection tax is about
10 ns, larger than the raw gap. Rung 5 is excluded from the ns/op column on
purpose: its hot path is a cross thread channel round trip, a different cost
class, and forcing it into the same column would be a category error.

## ADR-007: A baseline generator isolates the type level cost

gen_baseline.py and baseline_compile.py use the same method as the reused compile
bench with a different generator: N lock types and no LockAfter graph, the rung 1,
2, 5 analog. They sweep the same N.

Build cost grows with lock count is only interesting against a control. Rungs 1,
2, 5 still have N locks, they just do not encode the order in types. The baseline
is flat (total 0.014 s at N=10, 0.016 s at N=256), so the gap between the two
curves is the type level ordering cost. Without it a reader cannot tell trait
solving cost from ordinary more code compiles slower.

## ADR-008: risk_check has no compile time blowup, and we do not fake an N

risk_check is implemented at rungs 1 and 4 only. We do not sweep a compile time
curve for it and state structurally that there is none.

The risk_check rung 4 is a fixed two state machine (UncheckedOrder to CheckedOrder
via RiskCheck::approve, enforced by a private constructor). It has no transitive
trait graph and no scale parameter N, so there is nothing to sweep and no super
linear cost to find. That absence is why it is the second data point: same rung,
unrepresentable, different invariant, different cost shape, about 2.2 times the
rung 1 boilerplate and essentially nothing else. Inventing an N to sweep would
manufacture a curve that is not there.

Claim discipline: two invariants is enough to show the cost shape is not
universal, that it moves, and that is the honest claim. It is not a predictive
taxonomy. Two contrasting points show variation exists, they do not let us
predict a third invariant's shape. The close should say the shape moves per
invariant, here are two that differ, and stop there.

## ADR-009: Rigidity holes are demonstrated with tests, not asserted

Each honesty ledger hole has a runnable demonstration in harness/src/rigidity/.

legit_program_rejected: runtime indexed account[i] and account[j]. A compile_fail
doctest shows the two account transfer cannot be expressed, since both share one
AccountLevel and the second acquisition asks for the unprovable
AccountLevel: LockBefore<AccountLevel>.

still_allows_cyclic_order: a passing test declares Mempool before Connection and
Connection before Mempool, and both acquisition orders type check. Rung 4
enforces consistency with what you declared, not deadlock freedom.

still_allows_drop_order: a passing test acquires A, B, C and releases A first.
Acquisition order is checked, release order is not.

Failure mode still allowed claims that are not executed are just prose. Compiling
and running them is the difference between a measurement and an assertion.

Implementation note: to get a real acquisition path for the runtime bench,
rung4_typestate was extended with LockLevel and MutexLockLevel impls, an Unlocked
root edge, and a hot_path. The pre existing verified items (may_acquire, legal,
the out of order compile_fail doctest) are unchanged. The Unlocked root is a
single concrete impl LockAfter<Unlocked> for AccountsTable; adding the transitive
macro on Unlocked would collide under coherence (E0119) because Unlocked is an
upstream type, and the existing chain macros already carry the edge down.

## ADR-010: Topology, depth vs lock count

Before treating the compile time curve as load bearing, audit the obvious
objection: every chain number is a property of a total order of N levels, but
real lock hierarchies are shallow and wide. gen_dag.py builds a forest of W
independent chains of depth D with the same impl_transitive_lock_order mechanism,
only the topology differs, and dag_compile.py sweeps it the same way as the chain.
Results in results/dag_compile.json.

The objection that the curve is real but only bites a synthetic chain cannot be
closed by a single sentence, so it was measured. Constant lock count N=160, deep
to shallow: chain (160 by 1) typeck 0.157 s, forest (4 by 40) typeck 0.009 s, the
same 160 locks about 17 times cheaper. Fixed shallow depth 4, widening from 40 to
256 locks: typeck 0.003 to 0.014 s, about linear (about O(N^0.82)), and the
recursion limit is never approached at depth 4.

Among forests, flattening lowers cost and a sparse depth 4 forest scales about
linearly in lock count, cliff free. But the first cut of this decision drew the
wrong general conclusion from it: depth drives cost, cross edges add at most
linearly in edge count without deepening the closure. That bound was asserted,
not measured, and it is false. See ADR-012, which checks it and replaces it with
the measured driver: type check cost tracks closure size (reachable ordered
pairs), which depth bounds for forests and chains but dense cross connectivity
inflates independently. The forest sweep here is correct for the sparse family,
it is just not the whole envelope.

## ADR-011: Rung 2 runtime gap, separate detection from the implementation switch

The runtime table's rung 1 to rung 2 gap, about 6.7 ns, mixed two variables: rung
1 is std::sync::Mutex, rung 2 is parking_lot::Mutex with deadlock_detection. The
control crate pl_control is kept out of harness's dependency graph so
parking_lot's deadlock_detection feature can be toggled per build without feature
unification forcing it on. It benches the same 3 lock hot path on a std mutex and
a parking_lot mutex, built twice with the feature off and on. Results in
results/rung2_control_detect_off.json and results/rung2_control_detect_on.json.

Attributing the full 6.7 ns to detection bookkeeping, as the first draft did, is
the one runtime claim a parking_lot literate reviewer would catch. The
decomposition, median run whole with spreads under 0.15 ns and therefore robust:

| | std mutex | parking_lot mutex |
| --- | ---: | ---: |
| detection off | 29.44 | 25.80 |
| detection on  | 29.41 | 36.19 |

The implementation switch, std to parking_lot with no detection, is 3.6 ns
faster, since parking_lot is faster uncontended. The pure detection bookkeeping,
parking_lot off to on, is 10.4 ns, about 40 percent over parking_lot's own
baseline. Net against std rung 1 is 6.8 ns, and it matches the runtime_bench rung
2 number (36.19 vs 36.17), so the control validates against the original.

The rung 2 number to print is about 10 ns/op of detection bookkeeping on every
uncontended acquisition, whether or not anything ever deadlocks, not the mixed
6.7 ns, and it is larger than that gap because parking_lot starts ahead of std.
This is the unexpected half that pairs with the expected free at runtime rung 4
result.

## ADR-012: Type check cost tracks closure size, not depth (corrects ADR-010)

A reviewer flagged ADR-010's caveat, that cross edges add at most linearly in edge
count without deepening the closure, as an unchecked bound with a likely
counterexample: a depth 2 dense bipartite DAG (tier A and tier B, every A before
every B) is maximally shallow yet has (N/2) squared reachable ordered pairs,
quadratic closure at depth 2. If type check cost tracks closure size, which fits
the chain's roughly 1.87 exponent better than anything depth linear since a
chain's closure is about N squared over 2, then a dense shallow DAG is as costly
as a deep chain. We measured it rather than argue. gen_manual.py hand expands the
full closure, one concrete impl LockAfter per reachable pair, the only way to
express a multi parent DAG in lock_ordering, and manual_compile.py times it. To
isolate topology from the macro vs manual axis, chain, forest, and dense tiers are
all hand expanded. Results in results/manual_topology.json.

Result at N=160, hand expanded, median run whole:

| config | depth | closure (pairs) | typeck (s) | us per pair |
| --- | ---: | ---: | ---: | ---: |
| forest 4 by 40 | 4 | 240 | 0.0050 | 20.8 |
| tiers 80 by 80 | 2 | 6400 | 0.0825 | 12.9 |
| tiers 53 by 53 by 54 | 3 | 8533 | 0.1340 | 15.7 |
| tiers 40 by 40 by 40 by 40 | 4 | 9600 | 0.1375 | 14.3 |
| chain | 160 | 12720 | 0.1790 | 14.1 |

The reviewer was right and the caveat was wrong. The decisive pair, forest 4 by
40 and dense tiers 40 by 40 by 40 by 40, have identical depth (4) and N (160) but
typeck 0.005 s vs 0.138 s, a 28 times gap from connectivity alone. Cost is about
constant per reachable pair, roughly 13 to 16 us per pair across the dense and
chain configs, so type check cost tracks closure size, the reachable ordered
pairs. Depth bounds closure for chains and forests, so flattening a sparse
hierarchy genuinely helps, but dense cross tier connectivity inflates closure
independently of depth.

Corrected claim for the article: not that shallow and wide is always cheap, which
holds only for sparse hierarchies. The compile time cost tracks the number of
ordered lock pairs the type system must know about; a realistic hierarchy is
cheap because it is sparse, few cross tier edges, not merely because it is
shallow. A shallow but densely connected lock graph pays the full quadratic.

The hand expanded chain at N=160 compiles fine, no E0275, because concrete impls
need no recursive resolution. So hand expansion trades the recursion cliff, a
macro and proof depth artifact, for an explicit closure sized impl set: the same
cost magnitude as the macro chain (0.179 s against the macro's 0.157 s), no cliff.
The cliff is about proof recursion depth, the cost is about closure size.

This is the methodology working as intended: an asserted bound was checked,
failed, and was replaced with a measured one. The corrected finding, that cost
tracks closure size and depth is only a proxy through sparsity, is the more
interesting and non obvious version.

## ADR-013: The typestate hot path compiles to the same machine code as the plain version

Runtime parity (ADR-006) showed rung 4 benchmarks the same as rung 1, but equal
timings are weaker evidence than equal code. harness/asm_hotpath.py settles it: it
emits assembly for both hot_path functions (release, codegen-units=1), normalizes
per crate symbol hashes and local labels, and compares.

On aarch64 with rustc 1.91, both functions are 183 instructions and the
instruction multiset is identical. The first 157 instructions, the entire
acquire, increment, and release path, match instruction for instruction. The two
diverge only in the cold panic unwind tail, the drop glue that runs when a mutex
is poisoned, where the same instructions are laid out in a different order, plus
per crate hashes on the unwrap panic location constants.

So the type level proof apparatus is not made cheap, it is absent from the
emitted hot path. The result holds with codegen-units=1, which the release
profile sets; at the default multi unit release the cold tail layout can shuffle
further, but the hot path identity is the robust part. The claim is therefore
identical machine code on the hot path, not a byte for byte identical binary,
which the reordered cold tail rules out. Result in results/asm_hotpath.json.
