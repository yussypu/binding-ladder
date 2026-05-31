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
29.4–37.4). rung 2 (`parking_lot`) is ~36 ns — a real but small +ε from a
different lock primitive, not from the ordering check. **Rung 5 is excluded from
the ns/op column on purpose:** its hot path is a cross-thread channel round-trip,
a different cost *class*. Forcing it into the same column would be a category
error; "n/a" is the honest cell.

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
