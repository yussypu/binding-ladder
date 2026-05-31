//! Rung 2 control: split parking_lot's deadlock detection bookkeeping from the
//! std-vs-parking_lot switch by benching the same hot path on both, built twice:
//!
//!   cargo run --release -p pl_control                    # detection off
//!   cargo run --release -p pl_control --features detect  # detection on
//!
//! std vs parking_lot (off) is the implementation switch; parking_lot off vs on
//! is the detection bookkeeping. Same method as runtime_bench, release only.

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

struct StdBank {
    a: std::sync::Mutex<u64>,
    b: std::sync::Mutex<u64>,
    c: std::sync::Mutex<u64>,
}

fn std_hot(bank: &StdBank) -> u64 {
    let mut ga = bank.a.lock().unwrap();
    *ga += 1;
    let mut gb = bank.b.lock().unwrap();
    *gb += 1;
    let mut gc = bank.c.lock().unwrap();
    *gc += 1;
    *ga + *gb + *gc
}

struct PlBank {
    a: parking_lot::Mutex<u64>,
    b: parking_lot::Mutex<u64>,
    c: parking_lot::Mutex<u64>,
}

fn pl_hot(bank: &PlBank) -> u64 {
    let mut ga = bank.a.lock();
    *ga += 1;
    let mut gb = bank.b.lock();
    *gb += 1;
    let mut gc = bank.c.lock();
    *gc += 1;
    *ga + *gb + *gc
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
    let detect = cfg!(feature = "detect");
    let std_bank = StdBank {
        a: std::sync::Mutex::new(0),
        b: std::sync::Mutex::new(0),
        c: std::sync::Mutex::new(0),
    };
    let pl_bank = PlBank {
        a: parking_lot::Mutex::new(0),
        b: parking_lot::Mutex::new(0),
        c: parking_lot::Mutex::new(0),
    };

    for _ in 0..2 {
        black_box(bench(|| std_hot(black_box(&std_bank))));
        black_box(bench(|| pl_hot(black_box(&pl_bank))));
    }

    let mut runs: Vec<(f64, f64)> = Vec::with_capacity(RUNS);
    for _ in 0..RUNS {
        let std_ns = bench(|| std_hot(black_box(&std_bank)));
        let pl_ns = bench(|| pl_hot(black_box(&pl_bank)));
        runs.push((std_ns, pl_ns));
    }

    // median run whole, keyed on the std baseline
    let mut order: Vec<usize> = (0..runs.len()).collect();
    order.sort_by(|&a, &b| runs[a].0.partial_cmp(&runs[b].0).unwrap());
    let median_idx = order[order.len() / 2];
    let (std_med, pl_med) = runs[median_idx];

    let r = |x: f64| (x * 1000.0).round() / 1000.0;

    eprintln!("detection: {}", if detect { "on" } else { "off" });
    eprintln!("std median:          {std_med:.3} ns/op");
    eprintln!("parking_lot median:  {pl_med:.3} ns/op");
    eprintln!("pl minus std (this build): {:+.3} ns/op", pl_med - std_med);

    let raw: Vec<String> = runs
        .iter()
        .map(|(s, p)| format!("    {{ \"std_ns\": {}, \"pl_ns\": {} }}", r(*s), r(*p)))
        .collect();

    let json = format!(
        "{{\n\
         \"benchmark\": \"rung2_detection_control\",\n\
         \"deadlock_detection\": {},\n\
         \"note\": \"same uncontended 3 mutex hot path on std vs parking_lot, built twice (feature detect off and on) to separate detection bookkeeping from the implementation switch\",\n\
         \"toolchain\": {{ \"rustc\": \"{}\", \"arch\": \"{}\", \"os\": \"{}\", \"profile\": \"release\" }},\n\
         \"iters_per_run\": {},\n\
         \"runs\": {},\n\
         \"primary_metric\": \"std_ns\",\n\
         \"median_run_index\": {},\n\
         \"median_run\": {{ \"std_ns\": {}, \"pl_ns\": {} }},\n\
         \"raw\": [\n{}\n  ]\n\
         }}\n",
        detect,
        rustc_version(),
        std::env::consts::ARCH,
        std::env::consts::OS,
        ITERS,
        RUNS,
        median_idx,
        r(std_med),
        r(pl_med),
        raw.join(",\n"),
    );

    let fname = if detect { "rung2_control_detect_on.json" } else { "rung2_control_detect_off.json" };
    let out = format!("{}/../harness/results/{}", env!("CARGO_MANIFEST_DIR"), fname);
    std::fs::write(&out, json).expect("write rung2 control json");
    eprintln!("wrote {out}");
}
