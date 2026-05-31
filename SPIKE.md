# Validation spike: pass

Ran before writing any measurement prose. The compile time curve is real and super
linear, about O(N^2). Two findings changed the framing.

## Setup
- Crate under test: lock_ordering v0.2.0, the rung 4 implementation.
- Isolated the trait machinery only: marker levels, LockAfter/LockBefore, the
  impl_transitive_lock_order macro, one forced proof. No MutexLock runtime plumbing,
  which is constant boilerplate and would only add noise.
- Topology: total order chain L0 < L1 < ... < L(N-1).
- rustc 1.75.0 (82e1608df 2023-12-21), source tarball, single core, shared container.
  CARGO_INCREMENTAL=0, type_check via -Ztime-passes. Directional numbers, rerun on
  pinned hardware before publishing.

## Finding 1: super linear
Median type check time, deps warm, probe rebuilt clean per N:

| N   | typeck (s) | total (s) |
|-----|-----------:|----------:|
| 10  | 0.004 | 0.024 |
| 25  | 0.010 | 0.033 |
| 50  | 0.028 | 0.051 |
| 75  | 0.059 | 0.088 |
| 100 | 0.116 | 0.150 |
| 128 | 0.189 | 0.225 |
| 160 | 0.298 | 0.341 |
| 200 | 0.456 | 0.504 |
| 256 | 0.857 | 0.918 |

Fitted exponent for N at least 50 is about 2.1; the local exponent rises from 1.8 to
2.4 as N grows. Doubling the lock count roughly quadruples type check time. Raw data
in harness/results/compile_time.json.

## Finding 2: cost is in declaring the order, not using it
One deep proof vs N proofs gave identical times. The solver caches resolution within
a crate, so the curve is driven by the size of the declared hierarchy, not the number
of acquisition sites. Paid once per crate, up front.

## Finding 3: the recursion limit cliff
The transitive closure chain hits Rust's default trait recursion limit of 128. Past
about 128 chained levels it fails to compile:

    error[E0275]: overflow evaluating the requirement `L1: LockAfter<L128>`
    help: consider increasing the recursion limit by adding a
          `#![recursion_limit = "256"]` attribute

So the honest shape is a super linear climb with a hard wall at 128 that you cross
only by raising the limit, which lets the cost keep climbing. The all-N table above
uses a recursion_limit scaled to N. An earlier sweep that looked like it plateaued
past N=128 was the solver giving up at the limit, caught because the medians went
non monotonic.

## Magnitude
Even N=256 is under 1 s of type checking on a slow single core. Real hierarchies
(Fuchsia, Starnix) have dozens of levels, sub 100 ms territory. The finding is the
quadratic curve and the silent cliff at 128, not that this will wreck your build.

## Verified facts
- In order acquisition compiles, transitive edges proven for free. Verified.
- Out of order acquisition is a type error (E0277, bound unsatisfied). Verified.
- Cost about O(N^2.1) in lock count for the chain topology. Verified here.
- Hard recursion limit cliff at 128 without recursion_limit. Verified.
- Still-allowed holes (cyclic declared order, runtime indexed locks) were not yet
  demonstrated with tests at spike time; that is the rigidity column, since done.

## Toolchain caveat
Measured on rustc 1.75 (no rustup in this sandbox), the old trait solver. The curve's
existence is robust, the exact exponent is toolchain bound. Rerun on the pinned
toolchain and note the version.
