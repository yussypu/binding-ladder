# Project Spec — "The Binding Ladder"

Working spec for blog post 002 and its companion artifact. Essay thesis, project
design, implementation plan, measurement methodology, and the honesty ledger of
what is verified vs. open.

Article working titles: *willpower doesn't scale* / *make it impossible, not improbable*.
Repo working name: `binding-ladder` (or `enforcement-cost`).

---

## 1. Thesis

A rule is only as strong as how hard it is to break, not how good the rule is.

Most of what we call "discipline" is a rule resting on the weakest enforcement
there is: a thing a tired person has to remember not to do. Remembering does not
scale. A rule that runs on willpower has scheduled its own violation. The
interesting question is never whether a rule is good — it is what happens when
someone tries to break it. And there is a ladder of answers.

### The ladder

From weakest to strongest enforcement:

1. **Convention** — a comment, a style-guide line, a meeting agreement. Pure
   willpower. Decays the instant someone is in a hurry.
2. **Review** — a human who is supposed to catch it. A tired reviewer says LGTM.
   Friction, not a wall.
3. **CI gate** — the build fails. Strong, but bypassable (`--no-verify`, or the
   same person edits the gate). This is the rung most teams stop on and mistake
   for the top.
4. **Unrepresentable** — the type system. The violation does not compile. The
   constraint moves from "remember not to" to "cannot be said."
5. **Nonexistent** — the dangerous capability is not reachable at all. The
   function is not in scope, the key is not on disk, the API cannot be named.

**Good engineering pushes invariants down this ladder** — from things you must
remember toward things that cannot happen. "Discipline" is the name for an
invariant still stuck on rung 1, and it is a tax paid on every future review,
forever.

### Pedigree (cite this — it is a strength, not a weakness)

This ladder is not invented. It is the **Hierarchy of Controls** from
occupational safety (NIOSH / OSHA), ~50 years old: elimination → substitution →
engineering controls → administrative controls → PPE, ordered most-effective
first. Rung 5 (nonexistent) is *elimination*; rung 1 (convention) is
*administrative controls*. Engineers keep half-rediscovering it. The essay states
it cleanly for code and — the part nobody has done — measures it.

### Two axes, not one

Strength alone is incomplete. Odysseus told the crew to ignore his pleas *until
safe* — the pact had an exit condition. The best constraints are **hard to break
by accident, easy to undo on purpose.** That is a 2×2:

- impulsive-break cost (want this high)
- deliberate-reversal cost (want this low)

Most CI gates fail this: equally annoying to bypass by accident or on purpose, so
people learn a reflexive `--no-verify`, which silently collapses rung 3 back to
rung 1. The one-way-door migration is the opposite failure: low impulsive cost,
infinite reversal cost.

### The regress: who binds the binder?

A CI gate is a file in the repo; whoever can edit the gate can delete it. Every
pact has a meta-question: what stops future-you from untying yourself? The
strongest pacts are **enforced by something you do not control** — branch
protection owned by someone else, or the laws of the type system, which you
genuinely cannot edit. The compiler does not get tired and does not take bribes.
(Same reason a constitution is harder to change than a statute.)

### The honest counter (the maturity beat — do not cut this)

Constraints are rigid, and rigidity is catastrophic when you bind the wrong
thing. A premature pact that bans the right answer is worse than no pact. The
skill is not "constrain everything" — it is knowing what is genuinely invariant
vs. what is still in flux, and only paying to bind the former. Premature
constraint is the twin of premature abstraction.

### The measured contribution (this is what makes it post 002, not a survey)

Every rung down is a claim that costs nothing. Check the bill. Take one invariant,
implement it at every rung, and measure the cost of climbing. **The cost curve
has a shape:** as you climb, runtime cost falls to zero (type-level guarantees
compile away) while compile-time cost and rigidity rise. There is a sweet-spot
rung, and it is *invariant-dependent*. That sentence is the whole post's thesis
backed by your own numbers.

---

## 2. The artifact

**One invariant, every rung, measured.** The deliverable is a repo plus a
crackeddb-style cost table — not a library to adopt, a measurement to trust.

### Primary invariant: deadlock (lock ordering)

Deadlock is the canonical rung-1 rule: "always acquire locks in the same global
order," written in a wiki, enforced by nobody, violated at 2am, nondeterministic
and miserable to debug. It is the most relatable concurrency bug there is, and it
dual-signals: gnarly-concurrency for systems/quant readers, types-as-proof for
the correctness-minded.

**Prior art is an asset here, not a problem.** `lock_ordering` (crates.io, by
akonradi, Fuchsia-team lineage) is a serious, ready-made rung-4 implementation:
marker-type levels, a `LockBefore` relation, a transitive-order macro. You do not
rebuild it — you **measure** it. Cite it openly.

### The four rungs to implement for deadlock

| Rung | Mechanism | Sketch |
| ---- | --------- | ------ |
| 1 — convention | doc comment + a `// LOCK ORDER: A < B` note | nothing enforces it |
| 2 — review/lint | a custom lint or a runtime debug-mode deadlock detector (e.g. `parking_lot` deadlock detection) | catches at test time, probabilistically |
| 4 — unrepresentable | `lock_ordering` typestate: out-of-order acquisition fails to compile | the wrong program cannot be written |
| 5 — nonexistent | restructure so the second lock is unreachable (single owner, message passing, lock subsumed) | the hazard is eliminated, not guarded |

(Rung 3, a true CI gate specific to lock order, is hard to build honestly for this
invariant — note that explicitly. The gap itself is a finding: not every invariant
has a clean rung at every level. That is part of the thesis.)

### The jewel (the two-doctors-demo equivalent)

Type-level lock ordering is enforced through **trait-bound graphs**. As the lock
hierarchy grows, the trait solver does more work, and compile times may grow
super-linearly. **Nobody has measured what type-level deadlock-freedom costs in
build time as lock count scales.** The curve — "free at runtime, here is the bill
at the compiler, growing with lock count" — is the concrete, reproducible,
mildly-surprising result the post is built around.

> ⚠️ This curve is the one load-bearing claim that is NOT yet verified. See the
> validation gate in §5. Run the spike before writing the measured half.

### Second invariant (the generalization, kept brief)

To show the cost curve's *shape moves per invariant*, add one more data point —
measured briefly, not a full second worked example:

- **"an order cannot be submitted without a passing risk check"** — typestate
  where `UncheckedOrder` has no `submit`; only `RiskCheck::approve` yields a
  `CheckedOrder` that does. Pure quant signal, almost certainly no published
  measured version. Lacks the compile-time-blowup jewel, so it is a cleaner story
  with a less dramatic number — exactly why it works as the *second* point.

One invariant in depth, one in brief. Not two in depth.

---

## 3. Implementation plan

### Repo layout

```
binding-ladder/
  README.md
  DECISIONS.md                # adr-style, same discipline as crackeddb
  invariants/
    deadlock/
      rung1_convention/       # the unenforced version + a test that it CAN break
      rung2_runtime/          # detector-based, catches probabilistically
      rung4_typestate/        # built on lock_ordering
      rung5_eliminated/       # restructured so the hazard cannot form
    risk_check/
      rung1_convention/
      rung4_typestate/
  harness/
    src/
      compile_time.rs         # times cargo builds across lock-count N
      runtime_bench.rs        # proves rung-4 runtime cost ~ 0
      boilerplate.rs          # LOC / token count the caller pays per rung
      rigidity/               # legit programs each rung wrongly rejects
    results/
      *.json                  # raw, committed (crackeddb rule: numbers reproducible from logs)
  specs/                      # optional: a tiny model of the lock order if useful
```

### What the harness measures (per rung, per invariant)

1. **Runtime cost** — microbenchmark the hot path. Expect rung 4 ≈ rung 1
   (PhantomData is zero-sized, monomorphized away). The *expected null result* is
   the point: the safety is runtime-free.
2. **Compile-time cost** — wall-clock `cargo build` (clean + incremental), swept
   over lock count N = {2, 5, 10, 25, 50, 100}. This is where the deadlock jewel
   lives. Report clean and incremental separately.
3. **Boilerplate** — lines / tokens the *caller* must write to use each rung.
   Rung 4's `lock_ordering` setup (level types, `LockBefore` impls, threading
   `LockedAt`) is non-trivial; quantify it honestly.
4. **Rigidity** — a curated suite of *legitimate* programs that each rung rejects
   or cannot express, plus the failure mode each rung still *allows*. For rung 4:
   the runtime-indexed case (`account[i]` / `account[j]` with `i,j` dynamic) that
   static levels cannot express, and the declare-a-cyclic-order hole the type
   system will happily enforce. These are not footnotes — they are columns.

### Measurement methodology (crackeddb-grade)

- Pin the machine, the toolchain version, and the flags. State them.
- Clean vs. incremental builds reported separately; never mix.
- N runs per cell; report the **median run whole** (every column from the same
  run — the exact mistake the crackeddb audit caught).
- Commit raw `results/*.json`. Numbers must be reproducible from committed logs,
  not taken on trust.
- Each rung's "failure mode still allowed" gets a test that demonstrates it.

### Output: the cost table (the article's centerpiece)

```
deadlock invariant · cost of climbing the ladder

rung            runtime (ns/op)   build N=10 (s)   build N=100 (s)   boilerplate (LOC)   legit programs rejected   still allows
1 convention    <baseline>        <baseline>       <baseline>        0                   0                         every deadlock
2 runtime det.  ~baseline+ε       ~baseline        ~baseline         small               0                         deadlock until a test hits it
4 typestate     ~baseline (≈0)    ???              ???               high                runtime-indexed locks     cyclic order you declared
5 eliminated    n/a (no lock)     ~baseline        ~baseline         varies              the design that needs 2 locks   nothing, for this hazard
```

Fill `???` from the spike. The shape of those two cells is the post.

---

## 4. Article structure

1. **Open** (thesis flat, concrete anchor, no scene — drafted below).
2. **The ladder**, qualitative only. Build it from first principles, give it the
   Hierarchy-of-Controls pedigree, climb it. Resist pulling numbers in here.
3. **The turn**: "every rung up is a claim that costs nothing. let's check the bill."
4. **Deadlock, measured.** Walk it back down the ladder with numbers. The
   compile-time curve is the jewel. Keep it anchored to the ladder — not a
   benchmark dump.
5. **The two extra dimensions**: reversibility (the 2×2) and who-binds-the-binder.
6. **The honest counter**: rigidity / premature binding. This is where rung 4's
   rejected-legit-programs column does its work.
7. **The risk-check data point**: the cost curve's shape moves per invariant.
8. **Close**: the whole thing in one move — *measure what your discipline costs
   before you pay for it.* Mirror the crackeddb closer.
9. **Footer credits**: NIOSH Hierarchy of Controls; the No Boilerplate "plain
   text" video as the seed; the 2019 dev.to "Hierarchy of Controls for Software
   Engineering" post (shallow prior mapping — cite it before the commenters do);
   `lock_ordering` crate.

### Drafted opening (your voice, no story mode)

> a rule is only as strong as how hard it is to break. not how good the rule is.
> how hard it is to break.
>
> you have seen the proof of this. a comment that says do not call this directly.
> a line in a style guide everyone nodded at. a convention the whole team agreed
> to in a meeting and meant. and a week later someone calls it directly, at the
> end of a long day, because the thing was right there and nothing stopped them.
> the rule was not wrong. the rule was unenforced. those are different problems
> and we keep treating them as one.
>
> most of what we call discipline is a rule resting on the weakest enforcement
> there is: a thing you have to remember not to do. and remembering does not
> scale, because the person who has to remember is tired, or in a hurry, or new,
> or all three. a rule that runs on willpower has scheduled its own violation.
>
> so the question is never whether the rule is good. it is what happens when
> someone tries to break it. and there turns out to be a ladder.

---

## 5. Validation gate (do this FIRST)

The entire measured half rests on one unverified claim: that the `lock_ordering`
compile-time curve is real and large enough to be interesting. Spike it before
writing a word of the measurement sections.

- Build the typestate version at N = 10, 50, 100 locks.
- Time clean builds.
- If the curve climbs meaningfully → you have the jewel; structure stands.
- If it is flat → you found out in 30 minutes, not after three sections. Pivot
  the jewel to the boilerplate or rigidity axis instead (both are real either
  way), and lead with the runtime-is-free / flexibility-is-the-cost result.

This is the crackeddb audit instinct applied *before* the fact.

---

## 6. Honesty ledger

**Verified:**
- The ladder = NIOSH/OSHA Hierarchy of Controls. Cite it; it is pedigree.
- A 2019 dev.to post already maps the hierarchy to software (prose only, no code,
  no measurement). Cite it.
- Type-level lock ordering is real and sound for what it covers; `lock_ordering`
  is a published rung-4 implementation.
- No published work takes one invariant across every rung and measures the cost
  curve. That gap is the contribution.

**Open / to confirm:**
- The compile-time blowup curve (the jewel). See §5.

**Known holes to disclose, not paper over (these are content, not weaknesses):**
- Type-level lock ordering enforces consistency with the order *you declare*, not
  that the order is deadlock-free — you can declare a cyclic order and it compiles.
- It cannot express runtime-indexed lock sets (the `account[i]`/`account[j]`
  transfer deadlock).
- Guard `Drop` order can release locks out of acquisition order.

**Positioning rule:** the contribution is the *measured comparison* and the
*cost-curve shape*, never "I invented type-level deadlock freedom." Foreground the
synthesis and the numbers. Lead with the framing, cite the parts.
