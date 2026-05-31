# DECISIONS — binding-ladder

ADR-style log for the measurement repo behind blog post 002. Same discipline as
crackeddb: every number traces to a committed log; every gap is a finding, not a
silence. Newest decisions append at the bottom.

---

## ADR-001 — Pin the toolchain; record that the exponent is toolchain-bound

**Decision.** Pin `rustc 1.91.0 (f8297e351 2025-10-28)` via `rust-toolchain.toml`.
All columns in `results/` were produced on this toolchain and machine
(aarch64 / macOS, 8 cores). The §5 validation spike ran on **1.75.0** (old trait
solver) in a Linux container; that data is preserved at
`harness/results/compile_time_spike_1.75.json`, not overwritten.

**Why.** The compile-time curve's *exponent* depends on the trait solver, so it
is toolchain-bound; its *existence* (super-linear, with a hard cliff) is not.
Re-running on 1.91 confirms this:

| metric | 1.75 spike (container) | 1.91 this repo (aarch64) |
| --- | --- | --- |
| fitted typeck exponent (N≥50) | ~2.1 | ~1.87 |
| typeck @ N=256 | 0.857 s | 0.398 s |
| recursion cliff | default 128 (E0275) | default 128 (E0275) |

Faster absolute times (newer solver + faster machine), slightly lower exponent,
**same shape and same cliff.** State the toolchain next to any number lifted
into the article. Never mix toolchains within a single results run.

---

## ADR-002 — Repo layout and a standalone `probe` crate

**Decision.** Lay the repo out per spec §3 (`invariants/{deadlock,risk_check}/`,
`harness/`). The spike's flat files were moved in, not rewritten:
`gen_levels.py` and `compile_time_bench.py` → `harness/`; the verified rung-4
example → `invariants/deadlock/rung4_typestate/src/lib.rs`. The `probe` crate is
**excluded** from the workspace (`exclude = ["probe"]`) and keeps its own
`Cargo.lock`.

**Why.** `compile_time_bench.py` regenerates `probe/src/lib.rs` per N and times
`cargo clean -p probe` + rebuild. Keeping probe out of the workspace isolates
the timed build from the rung crates and prevents workspace-wide rebuilds from
polluting the measurement. `Cargo.lock` is committed everywhere (deps pinned →
numbers reproducible).

---

## ADR-003 — Rung 3 (a true CI gate) is intentionally absent for deadlock (FINDING)

**Decision.** Do not build a rung-3 for the deadlock invariant. Document the gap.

**Why this is a finding, not a hole we failed to fill.** A rung-3 control is "the
build fails on violation" — a *general, mechanical* gate, distinct from rung 4
(the violation is unrepresentable in the type system). For lock ordering there
is no honest, invariant-specific rung-3 between convention and typestate:

- A lint that flags "locks acquired in the wrong order" must reason about which
  lock is which and what order is intended across the whole call graph — i.e. it
  must reconstruct exactly the alias/ordering analysis that `lock_ordering`
  pushes into the type system. A real one (e.g. a clippy-style lexical check)
  catches only the most syntactically obvious cases and is trivially defeated by
  passing guards through a function. That is not a gate; it is a worse rung 2.
- A dynamic CI gate (run tests under the rung-2 detector in CI) is **not a new
  rung** — it is rung 2 wired to a pipeline. It inherits rung 2's defining
  weakness: it only fires if a test happens to drive threads into the cycle.

So a faithful rung-3 for *this* invariant would either collapse into rung 2 or
require the rung-4 machinery to be sound. Faking one (a toy lint that "passes")
would be the crackeddb cardinal sin: a green cell that doesn't mean what the
column header says. **The thesis predicts exactly this** (§6): not every
invariant has a clean rung at every level. The risk_check invariant has the
opposite gap — it has clean rung-1 and rung-4 but no natural rung-2 detector,
because there is no runtime symptom to detect. Gaps are invariant-dependent;
that is content.

---

## ADR-004 — Median-run-whole is enforced at table-assembly time

**Decision.** Reuse `compile_time_bench.py` as-is (per the brief). It prints
per-column medians (a typeck median and a total median computed independently) —
which, taken at face value, is the exact mistake the crackeddb audit caught. We
do **not** consume those printed medians for the cost table. Instead
`assemble_table.py` reads the committed raw per-run records and, for each N,
selects the SINGLE run whose `total_s` is the median (closest-to-median when
RUNS is even) and reports that one run's typeck **and** total together.

**Why.** "Every column from the same single run" is the rule. The harness was
designed to keep per-run records precisely so the selection can happen at
assembly. The runtime bench does the same internally (`median_run_index`,
keyed on the rung-1 baseline). Evidence the rule matters: in
`runtime_bench.json` the noisy 7th run spikes rung1/rung2/rung4 *together* —
per-column medians would have hidden that correlation; the whole-run report
preserves it.

---

## ADR-005 — Boilerplate is fenced caller code, comments excluded

**Decision.** Count caller-authored code between explicit
`// BOILERPLATE-START/END` fences in each rung's source; exclude comments and
blank lines; report both `code_loc` and a token proxy (`boilerplate.py`).

**Why.** The cost a rung imposes is the code *you* write per use site, not the
library's size. Excluding comments is deliberate and load-bearing: a rung-1
convention literally *is* a comment (`// LOCK ORDER: A < B`), so its enforcement
cost is ~0 lines of code — which is the whole point of why it decays. Result:
deadlock rung 4 = 50 LOC / 341 tok vs rung 1 = 19 / 163 (~2.6× LOC), and rung 2
≈ rung 1 (the detector is one-time global setup, not per-site). The token metric
is reported because it is robust to brace-style/formatting differences between
the rung sources.

**Framing correction (the 50 LOC is not a fixed tax).** The 50 LOC measures the
3-lock example. The rung-4 hierarchy declaration is **O(N) in the number of
locks**, not constant: `gen_levels.py` emits exactly **3N−2** authored lines (N
level types + 2(N−1) `LockAfter`/`impl_transitive_lock_order!` lines) — verified
7 / 28 / 73 / 298 lines at N = 3 / 10 / 25 / 100. This is the *same* declaration
that feeds the compile-time curve, which is why both scale with lock count. The
honest framing is therefore **"ceremony that scales with the number of locks you
declare, not with how much you use them"** — you pay per-lock-declared, once, up
front; acquisition sites are nearly free (see ADR-006). That is sharper than
"fixed one-time tax," and it is what the numbers actually show.

---

## ADR-006 — Runtime bench: uncontended, single-thread; rung 5 is n/a

**Decision.** Microbenchmark the hot path uncontended and single-threaded
(`runtime_bench`, release only — debug builds are flagged invalid in the JSON).
Bench rungs 1, 2, 4. Report rung 5 as **n/a (no lock)**.

**Why.** The claim under test is "the type-level safety is runtime-free"
(PhantomData is zero-sized, erased by monomorphization). That is a property of
the *mechanism*, isolated by measuring uncontended acquisition, not lock
contention. Result (median run whole): rung 4 ≈ rung 1 (both floor at 29.4
ns/op; the +4.1% in the median run is inside rung 4's own run-to-run spread of
29.4–37.4). rung 2 (`parking_lot`) is ~36 ns. **Do not attribute that whole gap
to detection** — it conflates the std→parking_lot impl switch with detection
on/off; ADR-011 decomposes it (the pure detection tax is ~+10 ns, larger than the
raw gap). **Rung 5 is excluded from the ns/op column on purpose:** its hot path
is a cross-thread channel round-trip, a different cost *class*. Forcing it into
the same column would be a category error; "n/a" is the honest cell.

---

## ADR-007 — A baseline generator isolates the type-level cost

**Decision.** Add `gen_baseline.py` + `baseline_compile.py` (same method as the
reused compile bench, different generator): N lock *types*, NO `LockAfter` graph
— the rung-1/2/5 analog. Sweep the same N.

**Why.** "Build cost grows with lock count" is only interesting relative to a
control. Rungs 1/2/5 still have N locks; they just don't encode the order in
types. The baseline is flat (`total` 0.014 s at N=10, 0.016 s at N=256), so the
gap between the two curves is *exactly* the type-level ordering cost. Without it,
a reader can't tell trait-solving cost from ordinary "more code compiles slower."

---

## ADR-008 — risk_check has no compile-time jewel, and we don't fake an N

**Decision.** Implement risk_check at rungs 1 and 4 only (per brief). Do not
sweep a compile-time curve for it; state structurally that there is none.

**Why.** The risk-check rung-4 is a fixed two-state machine (`UncheckedOrder` →
`CheckedOrder` via `RiskCheck::approve`, enforced by a private constructor). It
has no transitive trait graph and no scale parameter N, so there is nothing to
sweep and no super-linear cost to find. That absence is the second data point's
reason for existing: same rung (unrepresentable), different invariant, different
cost shape — boilerplate ~2.2× rung 1 and essentially nothing else. Inventing an
N to sweep would manufacture a curve that isn't there.

**Claim discipline (do not overstate).** Two invariants is enough to show the
cost shape is **not universal** — that it *moves* — and that is the honest claim.
It is *not* a predictive taxonomy: two contrasting points demonstrate variation
exists, they do not let us predict a third invariant's shape. The close should
say "the shape moves per invariant, here are two that differ," and stop there —
not imply a law.

---

## ADR-009 — Rigidity holes are demonstrated with tests, not asserted

**Decision.** Each §6 honesty-ledger hole gets a runnable demonstration in
`harness/src/rigidity/`:

- **legit-program-rejected** — runtime-indexed `account[i]`/`account[j]`: a
  `compile_fail` doctest shows the two-account transfer cannot be expressed
  (both share one `AccountLevel`, so the second acquisition asks for the
  unprovable `AccountLevel: LockBefore<AccountLevel>`).
- **still-allows cyclic order** — a passing test declares `Mempool < Connection`
  AND `Connection < Mempool`; both acquisition orders type-check. Rung 4 enforces
  consistency with what you *declared*, not deadlock-freedom.
- **still-allows drop order** — a passing test acquires A→B→C and releases A
  first; acquisition order is checked, release order is not.

**Why.** "Failure mode still allowed" claims that aren't executed are just
prose. Compiling/running them is the difference between a measurement and an
assertion.

**Implementation note (additive change to the verified rung-4 lib).** To get a
real acquisition path for the runtime bench, `rung4_typestate` was extended with
`LockLevel`/`MutexLockLevel` impls, an `Unlocked` root edge, and a `hot_path`.
The pre-existing verified items (`may_acquire`, `legal`, the out-of-order
`compile_fail` doctest) are unchanged. The `Unlocked` root is a single concrete
`impl LockAfter<Unlocked> for AccountsTable`; adding the transitive macro on
`Unlocked` would collide under coherence (E0119) because `Unlocked` is an
upstream type — the existing chain macros already carry the edge down.

---

## ADR-010 — Topology: the curve is driven by chain DEPTH, not lock count (FINDING)

**Decision.** Before treating the compile-time jewel as load-bearing, audit the
obvious objection: every chain number is a property of a *total order* of N
levels, but real lock hierarchies are shallow, wide DAGs. Added `gen_dag.py`
(forest of W independent chains of depth D — same `impl_transitive_lock_order!`
mechanism, only the topology differs) and `dag_compile.py` (same method as the
chain sweep). Results → `results/dag_compile.json`.

**Why this had to be run, not hand-waved.** "The curve is real but only bites a
synthetic chain" is a top-comment objection a single sentence can't close. The
measurement closes it both ways:

- **Constant lock count N=160, deep→shallow:** chain (160×1) typeck 0.157 s →
  forest (4×40) typeck 0.009 s. Same 160 locks, **~17× cheaper from topology
  alone.**
- **Fixed shallow depth 4, widening 40→256 locks:** typeck 0.003 → 0.014 s,
  ~linear (~O(N^0.82)), and the recursion limit is never approached (depth 4 ≪
  128).

Among *forests*, flattening lowers cost and a sparse depth-4 forest scales
~linearly in lock count, cliff-free. **But the first cut of this ADR drew the
wrong general conclusion from it** ("depth drives cost; cross-edges add at most
linearly in edge count without deepening the closure"). That bound was asserted,
not measured — and it is false. See ADR-012, which checks it and replaces it with
the measured driver: type-check cost tracks **closure size** (reachable ordered
pairs), which depth bounds for forests/chains but dense cross-connectivity
inflates independently. The forest sweep here is correct as far as it goes (it is
the *sparse* family); it just isn't the whole envelope.

---

## ADR-011 — Rung-2 runtime gap: isolate detection from the impl switch (FINDING)

**Decision.** The runtime table's rung-1→rung-2 gap (~+6.7 ns) conflated two
variables: rung 1 is `std::sync::Mutex`, rung 2 is `parking_lot::Mutex` *with*
`deadlock_detection`. Added an isolated control crate `pl_control` (kept OUT of
`harness`'s dependency graph so parking_lot's `deadlock_detection` feature can be
toggled per build without feature unification forcing it on) that benches the
same 3-lock hot path on both a std mutex and a parking_lot mutex, built twice
(feature off / on). Results → `results/rung2_control_detect_{off,on}.json`.

**Why.** Attributing the full 6.7 ns to "detection bookkeeping" (as the first
draft prose did) is the one runtime claim a parking_lot-literate reviewer would
catch. The decomposition (median run whole, spreads <0.15 ns, so robust):

| | std mutex | parking_lot mutex |
| --- | ---: | ---: |
| detection OFF | 29.44 | 25.80 |
| detection ON  | 29.41 | 36.19 |

- impl switch (std → parking_lot, no detection): **−3.6 ns** (parking_lot is
  *faster* uncontended).
- pure detection bookkeeping (parking_lot off → on): **+10.4 ns** (+40% over
  parking_lot's own baseline).
- net vs std rung 1: +6.8 ns (matches the runtime_bench rung-2 number, 36.19 ≈
  36.17 — the control validates against the original measurement).

**The printable rung-2 number is ~+10 ns/op of detection bookkeeping on every
uncontended acquisition, forever, whether or not anything ever deadlocks** — not
the conflated 6.7 ns, and *larger* than that gap because parking_lot starts ahead
of std. This is the un-expected half that pairs with the expected "free at
runtime" rung-4 result.

---

## ADR-012 — Type-check cost tracks CLOSURE SIZE, not depth (FINDING; corrects ADR-010)

**Decision.** A reviewer flagged ADR-010's caveat ("cross-edges add at most
linearly in edge count without deepening the closure") as an *unchecked bound*
with a likely counterexample: a depth-2 dense bipartite DAG (tier A × tier B,
every A before every B) is maximally shallow yet has (N/2)² reachable ordered
pairs — quadratic closure at depth 2. If type-check cost tracks closure size
(which fits the chain's ~N^1.87 better than anything depth-linear, since a chain's
closure is C(N,2)≈N²/2), then a dense shallow DAG is as costly as a deep chain.
We measured it rather than argue. Added `gen_manual.py` (hand-expanded full
closure — one concrete `impl LockAfter` per reachable pair, the only way to
express a multi-parent DAG in lock_ordering) and `manual_compile.py`. To isolate
topology from the macro-vs-manual axis, **chain, forest, and dense tiers are all
hand-expanded.** Result → `results/manual_topology.json`.

**Result (N=160, hand-expanded, median run whole).**

| config | depth | closure (pairs) | typeck (s) | µs/pair |
| --- | ---: | ---: | ---: | ---: |
| forest 4×40 | 4 | 240 | 0.0050 | 20.8 |
| tiers 80×80 | 2 | 6400 | 0.0825 | 12.9 |
| tiers 53×53×54 | 3 | 8533 | 0.1340 | 15.7 |
| tiers 40×40×40×40 | 4 | 9600 | 0.1375 | 14.3 |
| chain | 160 | 12720 | 0.1790 | 14.1 |

**The reviewer was right; the caveat was wrong.** The decisive pair: forest 4×40
and dense tiers 40×40×40×40 have *identical depth (4) and N (160)* but typeck
0.005 s vs 0.138 s — a **28× gap from connectivity alone**. Cost is ~constant per
reachable pair (~13–16 µs/pair across the dense/chain configs), i.e. **type-check
cost ∝ closure size (reachable ordered pairs)**. Depth bounds closure for chains
and forests/trees (so flattening a *sparse* hierarchy genuinely helps), but dense
cross-tier connectivity inflates closure independently of depth.

**Corrected claim for the article.** Not "shallow-wide is always cheap" — that
holds only for *sparse* hierarchies. The honest statement: the compile-time cost
tracks the number of ordered lock pairs the type system must know about; a
realistic hierarchy is cheap because it is *sparse* (few cross-tier edges), not
merely because it is shallow. A shallow but densely connected lock graph pays the
full quadratic.

**Bonus nuance.** The hand-expanded chain (N=160) compiles fine — no E0275 —
because concrete impls need no recursive resolution. So hand-expansion trades the
recursion *cliff* (a macro/proof-depth artifact) for an explicit O(closure) impl
set: same cost magnitude as the macro chain (0.179 s vs the macro's 0.157 s), no
cliff. The cliff is about proof recursion depth; the *cost* is about closure size.

**Meta.** This is the methodology working as intended: an asserted bound got
checked, failed, and was replaced with a measured one. The corrected finding
(cost ∝ closure, depth only a proxy via sparsity) is the more interesting,
non-obvious version — strictly better than the bound it replaces.
