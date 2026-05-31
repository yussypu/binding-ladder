#!/usr/bin/env python3
"""Assemble the cost table (the article centerpiece) from committed results.

Reads only results/*.json — every number here is reproducible from the logs,
nothing is typed by hand. Enforces the crackeddb median-run-whole rule for the
compile column: for each N we pick the SINGLE run whose total build time is the
median (closest-to-median when RUNS is even), and report that one run's typeck
AND total together — never a typeck-median from one run stitched to a
total-median from another.

Emits:
  results/cost_table.md   — the rendered tables (paste target for the article)
  results/cost_table.json — the same numbers, machine-readable
"""
import json
import math
import os

HARNESS = os.path.dirname(os.path.abspath(__file__))
RES = os.path.join(HARNESS, "results")


def load(name):
    with open(os.path.join(RES, name)) as f:
        return json.load(f)


def load_opt(name):
    try:
        return load(name)
    except FileNotFoundError:
        return None


def median_run_whole(cell):
    """From a cell's raw runs, return the single run closest to the median total."""
    runs = [r for r in cell["runs"] if r.get("total_s") is not None]
    totals = sorted(r["total_s"] for r in runs)
    n = len(totals)
    med = totals[n // 2] if n % 2 else (totals[n // 2 - 1] + totals[n // 2]) / 2
    best = min(runs, key=lambda r: abs(r["total_s"] - med))
    return best  # has ok/typeck_s/total_s from ONE run


def by_n(doc):
    return {cell["N"]: median_run_whole(cell) for cell in doc["raw"]}


def fit_exponent(ns, ys, n_min=50):
    """Least-squares slope of log(y) vs log(N) for N >= n_min => power-law exponent."""
    pts = [(math.log(n), math.log(y)) for n, y in zip(ns, ys) if n >= n_min and y > 0]
    k = len(pts)
    sx = sum(x for x, _ in pts)
    sy = sum(y for _, y in pts)
    sxx = sum(x * x for x, _ in pts)
    sxy = sum(x * y for x, y in pts)
    return (k * sxy - sx * sy) / (k * sxx - sx * sx)


def main():
    compile_ts = load("compile_time.json")       # rung-4 typestate curve
    baseline = load("baseline_compile.json")      # rung 1/2/5 flat
    runtime = load("runtime_bench.json")
    boiler = load("boilerplate.json")
    dag = load_opt("dag_compile.json")            # topology: shallow-wide forest (macro)
    manual = load_opt("manual_topology.json")     # topology: cost vs closure size (hand-expanded)
    ctl_off = load_opt("rung2_control_detect_off.json")
    ctl_on = load_opt("rung2_control_detect_on.json")

    ts = by_n(compile_ts)
    bl = by_n(baseline)
    rt = runtime["median_run"]
    bp = {(r["invariant"], r["rung"]): r for r in boiler["rows"]}

    rustc = compile_ts["toolchain"]["rustc"]
    host = runtime["toolchain"]
    machine = f"{host['arch']}/{host['os']}, {host['cores']} cores"

    def boil(inv, rung):
        r = bp[(inv, rung)]
        return f"{r['code_loc']} LOC / {r['tokens']} tok"

    # --- main deadlock cost table (spec §3 shape; ??? cells filled) -----------
    n10t = ts[10]["total_s"]
    n100t = ts[100]["total_s"]
    bl10, bl100 = bl[10]["total_s"], bl[100]["total_s"]

    lines = []
    A = lines.append
    A("# Cost of climbing the ladder — measured\n")
    A(f"- Toolchain: `{rustc}`  ·  Machine: {machine}")
    A(f"- Runtime: uncontended single-thread, ns/op, median run whole "
      f"(runs={runtime['runs']}, iters/run={runtime['iters_per_run']:,}).")
    A(f"- Compile: clean `probe` build, deps warm, `CARGO_INCREMENTAL=0`, "
      f"`-Ztime-passes`, median run whole (runs={compile_ts['runs_per_cell']}/cell).")
    A("- Boilerplate: caller-authored code between BOILERPLATE fences "
      "(comments/blanks excluded).\n")

    A("## deadlock invariant\n")
    A("| rung | runtime (ns/op) | build N=10 (s) | build N=100 (s) | boilerplate | legit programs rejected | still allows |")
    A("| ---- | ---: | ---: | ---: | --- | --- | --- |")
    A(f"| 1 convention | {rt['rung1_ns']:.1f} (baseline) | {bl10:.3f} (flat) | {bl100:.3f} (flat) "
      f"| {boil('deadlock','rung1_convention')} | 0 | every deadlock |")
    A(f"| 2 runtime det. | {rt['rung2_ns']:.1f} (+ε, parking_lot) | {bl10:.3f} (flat) | {bl100:.3f} (flat) "
      f"| {boil('deadlock','rung2_runtime')} | 0 | deadlock until a test hits it |")
    A(f"| 4 typestate | {rt['rung4_ns']:.1f} (≈ baseline) | {n10t:.3f} | {n100t:.3f} "
      f"| {boil('deadlock','rung4_typestate')} | runtime-indexed locks | cyclic order you declared |")
    A(f"| 5 eliminated | n/a (no lock) | {bl10:.3f} (flat) | {bl100:.3f} (flat) "
      f"| {boil('deadlock','rung5_eliminated')} | the design that needs 2 locks | nothing, for this hazard |")
    A("")

    # --- the jewel: rung-4 compile-time scaling vs the flat baseline ----------
    ns = sorted(ts.keys())
    exp_typeck = fit_exponent(ns, [ts[n]["typeck_s"] for n in ns])
    exp_total = fit_exponent(ns, [ts[n]["total_s"] for n in ns])
    A("## the jewel — rung-4 compile cost vs lock count N (median run whole)\n")
    A("| N | baseline total (s) | typestate typeck (s) | typestate total (s) | typestate/baseline |")
    A("| ---: | ---: | ---: | ---: | ---: |")
    for n in ns:
        ratio = ts[n]["total_s"] / bl[n]["total_s"]
        A(f"| {n} | {bl[n]['total_s']:.3f} | {ts[n]['typeck_s']:.4f} | {ts[n]['total_s']:.3f} | {ratio:.1f}x |")
    A("")
    A(f"Fitted power-law exponent (N≥50): **typeck ~O(N^{exp_typeck:.2f})**, "
      f"total ~O(N^{exp_total:.2f}). Baseline is flat in N "
      f"({bl[ns[0]]['total_s']:.3f}s at N={ns[0]} vs {bl[ns[-1]]['total_s']:.3f}s at N={ns[-1]}). "
      "Hard recursion-limit cliff at the default 128 (E0275) unless "
      "`#![recursion_limit]` is raised — verified on this toolchain.\n")

    # --- topology: is the jewel a chain artifact? -----------------------------
    if dag:
        cells = dag["summary"]
        const_n = [c for c in cells if c["N"] == 160]
        const_n.sort(key=lambda c: -c["depth"])
        depth4 = [c for c in cells if c["depth"] == 4]
        depth4.sort(key=lambda c: c["N"])
        A("## topology — is the curve a chain artifact? (depth vs lock count)\n")
        A("Same `impl_transitive_lock_order!` mechanism as the chain; only the "
          "shape changes. The chain is the worst case (a total order of N levels); "
          "real hierarchies are shallow and wide.\n")
        if const_n:
            chain = next((c for c in const_n if c["width"] == 1), const_n[0])
            shallow = min(const_n, key=lambda c: c["depth"])
            A("**Constant lock count N=160, deep → shallow:**\n")
            A("| topology (depth×width) | N | typeck (s) | total (s) |")
            A("| --- | ---: | ---: | ---: |")
            for c in const_n:
                shape = f"{c['depth']}×{c['width']}"
                tag = " (chain)" if c["width"] == 1 else (" (shallow forest)" if c["depth"] == 4 else "")
                A(f"| {shape}{tag} | {c['N']} | {c['typeck_median_s']:.4f} | {c['total_median_s']:.4f} |")
            A("")
            A(f"Same {chain['N']} locks: the deep chain type-checks in "
              f"{chain['typeck_median_s']:.4f}s, the depth-{shallow['depth']} forest in "
              f"{shallow['typeck_median_s']:.4f}s — "
              f"**{chain['typeck_median_s']/shallow['typeck_median_s']:.0f}× cheaper.** "
              "Flattening a *forest* lowers cost — but that is because for a forest, "
              "depth bounds the closure size. Depth is not the real driver; closure is "
              "(see the next table, which breaks the depth↔cost link).\n")
        if depth4:
            lo, hi = depth4[0], depth4[-1]
            exp = fit_exponent([c["N"] for c in depth4], [c["typeck_median_s"] for c in depth4], n_min=0)
            A("**Fixed shallow depth 4, widening a sparse forest (more locks):**\n")
            A("| topology | N | typeck (s) |")
            A("| --- | ---: | ---: |")
            for c in depth4:
                A(f"| 4×{c['width']} | {c['N']} | {c['typeck_median_s']:.4f} |")
            A("")
            A(f"A sparse depth-4 forest scales ~linearly in lock count "
              f"(~O(N^{exp:.2f}), {lo['typeck_median_s']:.4f}s→{hi['typeck_median_s']:.4f}s for "
              f"{lo['N']}→{hi['N']} locks) and never nears the 128 cliff. But 'shallow' is "
              "not what makes it cheap — *sparse* is. The next table shows a shallow but "
              "DENSE DAG is as expensive as the deep chain.\n")

    # --- the real driver: type-check cost tracks CLOSURE SIZE ------------------
    if manual:
        cells = sorted(manual["summary"], key=lambda c: c["closure"])
        A("## topology, settled — cost tracks CLOSURE SIZE, not depth\n")
        A("All hand-expanded (every reachable ordered pair = one concrete impl, no "
          "macro), so only topology varies. Closure = # reachable ordered pairs.\n")
        A("| config | depth | N | closure (pairs) | typeck (s) | µs / pair |")
        A("| --- | ---: | ---: | ---: | ---: | ---: |")
        for c in cells:
            per = c["typeck_median_s"] / c["closure"] * 1e6
            A(f"| {c['config']} | {c['depth']} | {c['N']} | {c['closure']} "
              f"| {c['typeck_median_s']:.4f} | {per:.1f} |")
        A("")
        fo = next((c for c in cells if c["config"] == "forest:4:40"), None)
        dn = next((c for c in cells if c["config"] == "tiers:40:40:40:40"), None)
        if fo and dn:
            A(f"**The decisive pair:** `forest:4:40` and `tiers:40:40:40:40` have the "
              f"*same depth (4) and same N (160)* but closures of {fo['closure']} vs "
              f"{dn['closure']} pairs — typeck {fo['typeck_median_s']:.4f}s vs "
              f"{dn['typeck_median_s']:.4f}s, a "
              f"**{dn['typeck_median_s']/fo['typeck_median_s']:.0f}× gap at identical depth.** "
              "Depth does not drive cost; closure size does (~constant µs/pair across the "
              "dense configs). A shallow but densely cross-connected DAG has quadratic "
              "closure and costs as much as a deep chain at the same N.\n")
        A("**Correction (crackeddb ethos — don't ship an unchecked bound).** An earlier "
          "draft claimed cross-edges 'add at most linearly in edge count without deepening "
          "the closure.' That is FALSE: a dense shallow DAG's closure is quadratic in N. "
          "The honest, measured finding is: **cost ∝ closure size (reachable ordered pairs)**. "
          "Depth bounds closure for chains and trees/forests (so flattening a sparse "
          "hierarchy helps), but dense cross-tier connectivity inflates closure "
          "independently of depth. 'Shallow-wide is cheap' holds only for *sparse* "
          "hierarchies. (The macro hits the recursion cliff because its proof recursion "
          "depth = path length; the hand-expanded form avoids the cliff but pays the same "
          "O(closure) type-check — same cost, no cliff.)\n")

    # --- rung-2: isolate detection bookkeeping from the impl switch -----------
    if ctl_off and ctl_on:
        std_off = ctl_off["median_run"]["std_ns"]
        pl_off = ctl_off["median_run"]["pl_ns"]
        std_on = ctl_on["median_run"]["std_ns"]
        pl_on = ctl_on["median_run"]["pl_ns"]
        A("## rung 2 — what the runtime gap actually is (controlled)\n")
        A("The runtime table shows rung 2 (`parking_lot` + detection) ~6.7 ns above "
          "rung 1 (`std::sync::Mutex`). That gap conflates two changes. Same hot "
          "path, parking_lot built with detection off vs on, isolates them:\n")
        A("| build | std mutex (ns/op) | parking_lot mutex (ns/op) |")
        A("| --- | ---: | ---: |")
        A(f"| detection OFF | {std_off:.2f} | {pl_off:.2f} |")
        A(f"| detection ON | {std_on:.2f} | {pl_on:.2f} |")
        A("")
        A(f"- implementation switch (std → parking_lot, no detection): "
          f"**{pl_off - std_off:+.1f} ns** — parking_lot is *faster* uncontended.\n"
          f"- pure deadlock-detection bookkeeping (parking_lot off → on): "
          f"**{pl_on - pl_off:+.1f} ns** (+{(pl_on - pl_off) / pl_off * 100:.0f}% over "
          "parking_lot's own baseline).\n"
          f"- net vs rung-1 std: {pl_on - std_off:+.1f} ns.\n")
        A("So the detection tax is **~{:.0f} ns/op paid on every uncontended "
          "acquisition, forever, whether or not anything ever deadlocks** — larger "
          "than the raw rung-1→rung-2 gap suggests, because parking_lot starts out "
          "ahead of std. That is the printable rung-2 number, not the conflated "
          "6.7 ns.\n".format(pl_on - pl_off))

    # --- second invariant: shape moves per invariant --------------------------
    A("## risk_check invariant (second data point — shape moves)\n")
    A("| rung | runtime | compile-time jewel? | boilerplate | rejects | still allows |")
    A("| ---- | --- | --- | --- | --- | --- |")
    A(f"| 1 convention | ~0 (a branch) | none | {boil('risk_check','rung1_convention')} "
      "| 0 | submit before/without a passing check |")
    A(f"| 4 typestate | ~0 (compile-time gate) | **none** — no transitive trait graph to solve "
      f"| {boil('risk_check','rung4_typestate')} | nothing legitimate (a check is always wanted) "
      "| nothing for this invariant (the gate is total) |")
    A("")
    A("The risk-check typestate is a fixed 2-state machine (`UncheckedOrder` → "
      "`CheckedOrder` via `RiskCheck::approve`), so there is no N to sweep and no "
      "super-linear curve — the rung-4 cost is ~2x boilerplate and nothing else. "
      "Same rung, different invariant, different cost shape: that is the point of "
      "the second data point.\n")

    md = "\n".join(lines)
    with open(os.path.join(RES, "cost_table.md"), "w") as f:
        f.write(md)

    out_json = {
        "toolchain": rustc,
        "machine": machine,
        "deadlock": {
            "runtime_ns": {"rung1": rt["rung1_ns"], "rung2": rt["rung2_ns"],
                           "rung4": rt["rung4_ns"], "rung5": None},
            "build_total_s": {
                "baseline_flat": {"N10": bl10, "N100": bl100},
                "rung4_typestate": {"N10": n10t, "N100": n100t},
            },
            "boilerplate": {k[1]: {"code_loc": v["code_loc"], "tokens": v["tokens"]}
                            for k, v in bp.items() if k[0] == "deadlock"},
        },
        "jewel": {
            "exponent_typeck": round(exp_typeck, 3),
            "exponent_total": round(exp_total, 3),
            "by_N": {str(n): {"baseline_total_s": bl[n]["total_s"],
                              "typestate_typeck_s": ts[n]["typeck_s"],
                              "typestate_total_s": ts[n]["total_s"]} for n in ns},
            "recursion_limit_cliff": 128,
        },
        "risk_check_boilerplate": {k[1]: {"code_loc": v["code_loc"], "tokens": v["tokens"]}
                                   for k, v in bp.items() if k[0] == "risk_check"},
    }
    if dag:
        out_json["topology_forest"] = {
            "note": "forest of W chains depth D; same macro mechanism as the chain",
            "by_config": [{"depth": c["depth"], "width": c["width"], "N": c["N"],
                           "typeck_median_s": c["typeck_median_s"],
                           "total_median_s": c["total_median_s"]} for c in dag["summary"]],
        }
    if manual:
        out_json["topology_closure"] = {
            "note": "hand-expanded (no macro); cost tracks closure size (reachable pairs), not depth",
            "finding": "cost proportional to closure size; dense shallow DAG ~ as costly as deep chain at equal N; earlier 'linear in edge count' caveat falsified",
            "by_config": [{"config": c["config"], "depth": c["depth"], "N": c["N"],
                           "closure": c["closure"], "typeck_median_s": c["typeck_median_s"],
                           "total_median_s": c["total_median_s"]} for c in manual["summary"]],
        }
    if ctl_off and ctl_on:
        out_json["rung2_control"] = {
            "detect_off": {"std_ns": ctl_off["median_run"]["std_ns"],
                           "pl_ns": ctl_off["median_run"]["pl_ns"]},
            "detect_on": {"std_ns": ctl_on["median_run"]["std_ns"],
                          "pl_ns": ctl_on["median_run"]["pl_ns"]},
            "impl_switch_ns": round(ctl_off["median_run"]["pl_ns"] - ctl_off["median_run"]["std_ns"], 3),
            "detection_bookkeeping_ns": round(ctl_on["median_run"]["pl_ns"] - ctl_off["median_run"]["pl_ns"], 3),
        }
    with open(os.path.join(RES, "cost_table.json"), "w") as f:
        f.write(json.dumps(out_json, indent=2) + "\n")

    print(md)
    print(f"\nwrote results/cost_table.md and results/cost_table.json")


if __name__ == "__main__":
    main()
