//! Runtime column: microbenchmark the hot path per rung.
//!
//! Expected null result: rung 4 is about equal to rung 1. The lock_ordering
//! levels are zero sized marker types and LockedAt is PhantomData, so the proof
//! apparatus is erased by monomorphization and the ordered acquisition compiles
//! to the same code as the plain Mutex version. That the safety is free at
//! runtime is the point, not a disappointing flat line.
//!
//! Method: uncontended and single threaded, so we measure the mechanism (acquire
//! three nested locks, touch a counter, release) and not contention. black_box
//! on input and output so nothing is optimized away. Each of RUNS runs times all
//! three rungs back to back so a run's columns are comparable, then we report the
//! median run whole, keyed on the rung 1 baseline, never per column medians. All
//! raw runs are committed. Build with release; a debug build is flagged invalid.

use std::hint::black_box;
use std::process::Command;
use std::time::Instant;

const ITERS: u64 = 5_000_000;
const RUNS: usize = 7;

fn bench<F: Fn() -> u64>(f: F) -> f64 {
    let start = Instant::now();
    let mut acc = 0u64;
    for _ in 0..ITERS {
        acc = acc.wrapping_add(black_box(f()));
    }
    let elapsed = start.elapsed();
    black_box(acc);
    elapsed.as_nanos() as f64 / ITERS as f64
}

struct Run {
    rung1_ns: f64,
    rung2_ns: f64,
    rung4_ns: f64,
}

fn rustc_version() -> String {
    Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn main() {
    let debug_build = cfg!(debug_assertions);

    let bank1 = rung1_convention::Bank::new();
    let bank2 = rung2_runtime::Bank::new();
    let bank4 = rung4_typestate::Bank::new();

    // warm up caches and branch predictor without recording
    for _ in 0..2 {
        black_box(bench(|| rung1_convention::hot_path(black_box(&bank1))));
        black_box(bench(|| rung2_runtime::hot_path(black_box(&bank2))));
        black_box(bench(|| rung4_typestate::hot_path(black_box(&bank4))));
    }

    let mut runs: Vec<Run> = Vec::with_capacity(RUNS);
    for _ in 0..RUNS {
        // all three in one run, so columns share the same conditions
        let rung1_ns = bench(|| rung1_convention::hot_path(black_box(&bank1)));
        let rung2_ns = bench(|| rung2_runtime::hot_path(black_box(&bank2)));
        let rung4_ns = bench(|| rung4_typestate::hot_path(black_box(&bank4)));
        runs.push(Run { rung1_ns, rung2_ns, rung4_ns });
    }

    // median run whole: order by the rung 1 baseline, take the middle run, report
    // every column from that one run
    let mut order: Vec<usize> = (0..runs.len()).collect();
    order.sort_by(|&a, &b| runs[a].rung1_ns.partial_cmp(&runs[b].rung1_ns).unwrap());
    let median_idx = order[order.len() / 2];
    let med = &runs[median_idx];

    let r = |x: f64| (x * 1000.0).round() / 1000.0;

    eprintln!("{:>14} {:>12} {:>12} {:>12}", "metric", "rung1", "rung2", "rung4");
    eprintln!(
        "{:>14} {:>12.3} {:>12.3} {:>12.3}  (median run, ns/op)",
        "hot_path", med.rung1_ns, med.rung2_ns, med.rung4_ns
    );
    let pct = (med.rung4_ns - med.rung1_ns) / med.rung1_ns * 100.0;
    eprintln!("rung4 vs rung1: {:+.1}% (expected near zero, PhantomData is erased)", pct);
    if debug_build {
        eprintln!("warning: debug build, numbers are not valid; rerun with --release");
    }

    // hand rolled JSON (no serde dep), all raw runs committed
    let raw_runs: Vec<String> = runs
        .iter()
        .map(|run| {
            format!(
                "    {{ \"rung1_ns\": {}, \"rung2_ns\": {}, \"rung4_ns\": {} }}",
                r(run.rung1_ns),
                r(run.rung2_ns),
                r(run.rung4_ns)
            )
        })
        .collect();

    let json = format!(
        "{{\n\
         \"benchmark\": \"runtime_hot_path\",\n\
         \"invariant\": \"deadlock\",\n\
         \"note\": \"uncontended single thread, ns/op to acquire 3 nested locks in order and bump counters\",\n\
         \"toolchain\": {{\n\
         \"rustc\": \"{}\",\n\
         \"arch\": \"{}\",\n\
         \"os\": \"{}\",\n\
         \"cores\": {},\n\
         \"profile\": \"{}\",\n\
         \"debug_assertions\": {}\n\
         }},\n\
         \"iters_per_run\": {},\n\
         \"runs\": {},\n\
         \"primary_metric\": \"rung1_ns\",\n\
         \"median_run_index\": {},\n\
         \"median_run\": {{ \"rung1_ns\": {}, \"rung2_ns\": {}, \"rung4_ns\": {}, \"rung5_ns\": null }},\n\
         \"rung5_note\": \"n/a, rung 5 has no lock; its hot path is a cross thread channel round trip, a different cost class (see invariants/deadlock/rung5_eliminated)\",\n\
         \"raw\": [\n{}\n  ]\n\
         }}\n",
        rustc_version(),
        std::env::consts::ARCH,
        std::env::consts::OS,
        std::thread::available_parallelism().map(|n| n.get()).unwrap_or(0),
        if debug_build { "debug" } else { "release" },
        debug_build,
        ITERS,
        RUNS,
        median_idx,
        r(med.rung1_ns),
        r(med.rung2_ns),
        r(med.rung4_ns),
        raw_runs.join(",\n"),
    );

    let out = concat!(env!("CARGO_MANIFEST_DIR"), "/results/runtime_bench.json");
    std::fs::write(out, json).expect("write runtime_bench.json");
    eprintln!("wrote {out}");
}
