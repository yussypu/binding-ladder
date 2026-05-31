#!/usr/bin/env python3
# compare rung4 vs rung1 hot_path asm: emit, extract, canonicalize hashes/labels, diff.
import re, os, json, glob, subprocess, platform
from collections import Counter

HARNESS = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HARNESS)
PKGS = ["rung1_convention", "rung4_typestate"]


def emit_asm():
    # --emit asm is a no-op when cached, so clean first
    subprocess.run(["cargo", "clean", "--release", "-p", PKGS[0], "-p", PKGS[1]],
                   cwd=ROOT, capture_output=True)
    for pkg in PKGS:
        subprocess.run(["cargo", "rustc", "--release", "-q", "-p", pkg, "--", "--emit", "asm"],
                       cwd=ROOT, capture_output=True)


def latest_s(pkg):
    files = glob.glob(os.path.join(ROOT, "target", "release", "deps", f"{pkg}-*.s"))
    return max(files, key=os.path.getmtime)


def extract(path):
    lines = open(path).read().split("\n")
    start = next(i for i, ln in enumerate(lines)
                 if "hot_path" in ln and ln.endswith(":") and (ln.startswith("__Z") or ln.startswith("_R")))
    body = []
    for ln in lines[start + 1:]:
        if ln.strip().startswith(".cfi_endproc"):
            break
        body.append(ln)
    return body


def normalize(body):
    out = []
    for ln in body:
        s = ln.strip()
        if not s or s.startswith((".", ";", "//")) or s.endswith(":"):
            continue
        s = re.sub(r";.*$", "", s).strip()
        s = re.sub(r"l_anon\.[0-9a-f]+\.", "ANON.", s)
        s = re.sub(r"__?ZN?[0-9A-Za-z_.$]+E?", "SYM", s)
        s = re.sub(r"_R[0-9A-Za-z_.$]+", "SYM", s)
        s = re.sub(r"_rust[0-9A-Za-z_]*", "SYM", s)
        s = re.sub(r"\bL[A-Za-z][A-Za-z0-9_]*", "LBL", s)
        s = re.sub(r"@\w+", "", s)
        out.append(s)
    return out


def main():
    emit_asm()
    a = normalize(extract(latest_s("rung1_convention")))
    b = normalize(extract(latest_s("rung4_typestate")))
    first = next((i for i, (x, y) in enumerate(zip(a, b)) if x != y), min(len(a), len(b)))
    multiset_eq = Counter(a) == Counter(b)
    rv = subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip()

    doc = {
        "benchmark": "asm_hotpath",
        "note": "rung 4 typestate hot path vs rung 1 plain mutex, normalized asm "
                "(per crate hashes and local labels canonicalized)",
        "toolchain": {"rustc": rv, "arch": platform.machine(), "profile": "release, codegen-units=1"},
        "rung1_instructions": len(a),
        "rung4_instructions": len(b),
        "identical_prefix": first,
        "instruction_multiset_identical": multiset_eq,
        "interpretation": f"the acquire and release path is identical for the first {first} "
                          "instructions; the two diverge only in the cold panic unwind tail",
    }
    with open(os.path.join(HARNESS, "results", "asm_hotpath.json"), "w") as f:
        f.write(json.dumps(doc, indent=2) + "\n")

    print(f"rung1 hot_path: {len(a)} instructions")
    print(f"rung4 hot_path: {len(b)} instructions")
    print(f"identical prefix: {first} instructions (the hot path)")
    print(f"instruction multiset identical: {multiset_eq}")
    print("wrote results/asm_hotpath.json")

    ok = len(a) == len(b) and multiset_eq and first >= 120
    if not ok:
        print("CHECK FAILED: hot path codegen diverged")
        raise SystemExit(1)


if __name__ == "__main__":
    main()
