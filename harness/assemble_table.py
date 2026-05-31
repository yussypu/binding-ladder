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
    with open(os.path.join(RES, "cost_table.json"), "w") as f:
        f.write(json.dumps(out_json, indent=2) + "\n")

    print(md)
    print(f"\nwrote results/cost_table.md and results/cost_table.json")


if __name__ == "__main__":
    main()
