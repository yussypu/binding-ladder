#!/usr/bin/env python3
"""Compile-time cost vs CLOSURE SIZE, all hand-expanded (gen_manual.py).

Settles whether type-check cost tracks closure size (# reachable ordered pairs)
or depth. Same method as the other sweeps (clean -p probe, CARGO_INCREMENTAL=0,
-Ztime-passes, RUNS/cell, median run whole). Every config is hand-expanded, so
the macro-vs-manual axis is held fixed and only topology varies.

Configs (default, all N=160): chain:160, forest:4:40, tiers:80:80,
tiers:53:53:54, tiers:40:40:40:40. The decisive pair is forest:4:40 vs
tiers:40:40:40:40 — identical depth (4) and N (160), closures 240 vs 9600.

Records n_nodes (N), n_impls (= closure size), and depth per config.
Result -> results/manual_topology.json.
"""
import subprocess, statistics, re, os, json, platform, datetime, sys

PROBE = os.environ.get("PROBE_DIR", "/home/claude/probe")
HARNESS = os.path.dirname(os.path.abspath(__file__))
DEFAULT = "chain:160,forest:4:40,tiers:80:80,tiers:53:53:54,tiers:40:40:40:40"
CONFIGS = (sys.argv[1] if len(sys.argv) > 1 else DEFAULT).split(",")
RUNS = int(os.environ.get("RUNS", "4"))
ENV = {**os.environ, "CARGO_INCREMENTAL": "0", "RUSTC_BOOTSTRAP": "1"}


def gen(parts):
    src = subprocess.run(["python3", f"{HARNESS}/gen_manual.py", *parts],
                         capture_output=True, text=True).stdout
    open(f"{PROBE}/src/lib.rs", "w").write(src)
    n_nodes = src.count("pub enum ")
    n_impls = src.count("impl LockAfter<")
    return n_nodes, n_impls


def depth_of(parts):
    mode = parts[0]
    if mode == "chain":
        return int(parts[1])
    if mode == "forest":
        return int(parts[1])
    if mode == "tiers":
        return len(parts) - 1
    return None


def build_once(parts):
    n_nodes, n_impls = gen(parts)
    subprocess.run(["cargo", "clean", "-p", "probe"], cwd=PROBE, capture_output=True)
    o = subprocess.run(["cargo", "rustc", "-p", "probe", "--quiet", "--", "-Ztime-passes"],
                       cwd=PROBE, capture_output=True, text=True, env=ENV)
    txt = o.stdout + o.stderr
    ok = o.returncode == 0
    def grab(pat):
        m = re.search(rf"time:\s+([\d.]+);.*{pat}", txt)
        return float(m.group(1)) if m else None
    return {"ok": ok, "typeck_s": grab("type_check_crate"), "total_s": grab(r"\ttotal"),
            "n_nodes": n_nodes, "n_impls": n_impls}


def toolchain():
    rv = subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip()
    return {"rustc": rv, "host": platform.platform(), "cores": os.cpu_count(),
            "cargo_incremental": "0", "isolation": "hand-expanded closure (no macro)",
            "when": datetime.datetime.utcnow().isoformat() + "Z"}


def main():
    cells, raw = [], []
    print(f"{'config':>22} {'depth':>5} {'N':>5} {'closure':>8} {'ok':>3} {'typeck_med(s)':>13} {'total_med(s)':>12}")
    for cfg in CONFIGS:
        parts = cfg.split(":")
        runs = [build_once(parts) for _ in range(RUNS)]
        raw.append({"config": cfg, "runs": runs})
        ok = all(r["ok"] for r in runs)
        tc = statistics.median([r["typeck_s"] for r in runs if r["typeck_s"]])
        tot = statistics.median([r["total_s"] for r in runs if r["total_s"]])
        n_nodes, n_impls = runs[0]["n_nodes"], runs[0]["n_impls"]
        d = depth_of(parts)
        cells.append({"config": cfg, "depth": d, "N": n_nodes, "closure": n_impls,
                      "ok": ok, "typeck_median_s": round(tc, 4), "total_median_s": round(tot, 4)})
        print(f"{cfg:>22} {str(d):>5} {n_nodes:>5} {n_impls:>8} {str(ok):>3} {tc:>13.4f} {tot:>12.4f}")
    doc = {"benchmark": "manual_topology_closure", "toolchain": toolchain(),
           "runs_per_cell": RUNS, "summary": cells, "raw": raw}
    open(f"{HARNESS}/results/manual_topology.json", "w").write(json.dumps(doc, indent=2))
    print(f"\nwrote results/manual_topology.json ({len(CONFIGS)} configs x {RUNS} runs)")


if __name__ == "__main__":
    main()
