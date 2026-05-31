#!/usr/bin/env python3
"""Compile time cost vs topology: shallow wide forests vs the deep chain.

Same method as compile_time_bench.py (clean -p probe, CARGO_INCREMENTAL=0,
type_check isolated via -Ztime-passes, RUNS per cell, raw per run records,
median run reported whole), only the generator differs (gen_dag.py). Configs
are DxW (depth by width) pairs, for example:

    python3 dag_compile.py 160x1,80x2,40x4,16x10,8x20,4x40   # constant N=160
    python3 dag_compile.py 4x10,4x25,4x40,4x64               # fixed depth 4

Result goes to results/dag_compile.json. Each cell records depth, width, N=D*W.
"""
import subprocess, statistics, re, os, json, platform, datetime, sys

PROBE = os.environ.get("PROBE_DIR", "/home/claude/probe")
HARNESS = os.path.dirname(os.path.abspath(__file__))
DEFAULT = "160x1,80x2,40x4,16x10,8x20,4x40,4x10,4x25,4x64"
CONFIGS = [c for c in (sys.argv[1] if len(sys.argv) > 1 else DEFAULT).split(",")]
RUNS = int(os.environ.get("RUNS", "4"))
ENV = {**os.environ, "CARGO_INCREMENTAL": "0", "RUSTC_BOOTSTRAP": "1"}


def gen(depth, width):
    src = subprocess.run(["python3", f"{HARNESS}/gen_dag.py", str(depth), str(width)],
                         capture_output=True, text=True).stdout
    open(f"{PROBE}/src/lib.rs", "w").write(src)


def build_once(depth, width):
    gen(depth, width)
    subprocess.run(["cargo", "clean", "-p", "probe"], cwd=PROBE, capture_output=True)
    o = subprocess.run(["cargo", "rustc", "-p", "probe", "--quiet", "--", "-Ztime-passes"],
                       cwd=PROBE, capture_output=True, text=True, env=ENV)
    txt = o.stdout + o.stderr
    ok = o.returncode == 0
    def grab(pat):
        m = re.search(rf"time:\s+([\d.]+);.*{pat}", txt)
        return float(m.group(1)) if m else None
    return {"ok": ok, "typeck_s": grab("type_check_crate"), "total_s": grab(r"\ttotal")}


def toolchain():
    rv = subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip()
    return {"rustc": rv, "host": platform.platform(), "cores": os.cpu_count(),
            "cargo_incremental": "0", "isolation": "trait-machinery only (no MutexLock plumbing)",
            "topology": "forest of W chains, depth D (shallow-wide)",
            "when": datetime.datetime.utcnow().isoformat() + "Z"}


def main():
    cells, raw = [], []
    print(f"{'D':>5} {'W':>5} {'N':>6} {'ok':>3} {'typeck_med(s)':>13} {'total_med(s)':>12}")
    for cfg in CONFIGS:
        depth, width = (int(x) for x in cfg.split("x"))
        n = depth * width
        runs = [build_once(depth, width) for _ in range(RUNS)]
        raw.append({"depth": depth, "width": width, "N": n, "runs": runs})
        ok = all(r["ok"] for r in runs)
        tc = statistics.median([r["typeck_s"] for r in runs if r["typeck_s"]])
        tot = statistics.median([r["total_s"] for r in runs if r["total_s"]])
        cells.append({"depth": depth, "width": width, "N": n, "ok": ok,
                      "typeck_median_s": round(tc, 4), "total_median_s": round(tot, 4)})
        print(f"{depth:>5} {width:>5} {n:>6} {str(ok):>3} {tc:>13.4f} {tot:>12.4f}")
    doc = {"benchmark": "dag_compile_time", "toolchain": toolchain(),
           "runs_per_cell": RUNS, "summary": cells, "raw": raw}
    open(f"{HARNESS}/results/dag_compile.json", "w").write(json.dumps(doc, indent=2))
    print(f"\nwrote results/dag_compile.json ({len(CONFIGS)} configs x {RUNS} runs)")


if __name__ == "__main__":
    main()
