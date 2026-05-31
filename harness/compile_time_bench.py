#!/usr/bin/env python3
"""Compile time cost of the rung 4 lock ordering hierarchy vs lock count N.

Method: regenerate the probe crate's lib.rs per N via gen_levels.py, run cargo
clean -p probe before each timed build so deps stay warm and only the probe
recompiles, which isolates the trait solving cost. CARGO_INCREMENTAL=0 removes
incremental caching variance. The type_check pass is isolated via -Ztime-passes
(RUSTC_BOOTSTRAP=1 on stable). RUNS per N, and every per run record is kept so
the median run whole rule holds once more columns are added: pick the run whose
primary metric is the median and emit all its columns together. Raw records go to
results/compile_time.json so the numbers are reproducible from the committed log.

Toolchain and machine are recorded in the JSON. Rerun on pinned hardware before
publishing; container numbers are directional.
"""
import subprocess, statistics, re, os, json, platform, datetime, sys

PROBE = os.environ.get("PROBE_DIR", "/home/claude/probe")
HARNESS = os.path.dirname(os.path.abspath(__file__))
NS = [int(x) for x in (sys.argv[1].split(",") if len(sys.argv) > 1 else
      "10,25,50,75,100,128,160,200,256".split(","))]
RUNS = int(os.environ.get("RUNS", "4"))
ENV = {**os.environ, "CARGO_INCREMENTAL": "0", "RUSTC_BOOTSTRAP": "1"}

def gen(n):
    src = subprocess.run(["python3", f"{HARNESS}/gen_levels.py", str(n)],
                         capture_output=True, text=True).stdout
    open(f"{PROBE}/src/lib.rs", "w").write(src)

def build_once(n):
    gen(n)
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
            "topology": "total-order chain", "when": datetime.datetime.utcnow().isoformat() + "Z"}

def main():
    cells, raw = [], []
    print(f"{'N':>5} {'ok':>3} {'typeck_med(s)':>13} {'total_med(s)':>12}")
    for n in NS:
        runs = [build_once(n) for _ in range(RUNS)]
        raw.append({"N": n, "runs": runs})
        ok = all(r["ok"] for r in runs)
        tc = statistics.median([r["typeck_s"] for r in runs if r["typeck_s"]])
        tot = statistics.median([r["total_s"] for r in runs if r["total_s"]])
        cells.append({"N": n, "ok": ok, "typeck_median_s": round(tc, 4),
                      "total_median_s": round(tot, 4)})
        print(f"{n:>5} {str(ok):>3} {tc:>13.4f} {tot:>12.4f}")
    doc = {"benchmark": "rung4_compile_time", "toolchain": toolchain(),
           "runs_per_cell": RUNS, "summary": cells, "raw": raw}
    open(f"{HARNESS}/results/compile_time.json", "w").write(json.dumps(doc, indent=2))
    print(f"\nwrote results/compile_time.json ({len(NS)} cells x {RUNS} runs)")

if __name__ == "__main__":
    main()
