#!/usr/bin/env python3
"""Boilerplate column: how much code the CALLER must write per rung.

The cost a rung imposes on every use site is not the library's line count — it
is the code *you* write to express the invariant. We mark that code in each
rung's source with explicit fences:

    // BOILERPLATE-START <tag>   ... caller-authored code ...   // BOILERPLATE-END <tag>

and count what is inside. Two metrics, comments and blanks excluded (a rung-1
convention lives in a *comment*, which is exactly why its enforcement cost is
~0 lines of code — that asymmetry is the finding):

  * code_loc : non-blank, non-comment source lines
  * tokens   : Rust token proxy (identifiers, numbers, and punctuation chars)

Raw per-rung records are written to results/boilerplate.json so the table is
reproducible from the committed source, not taken on trust.
"""
import json
import os
import re
import sys

HARNESS = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HARNESS)

# (invariant, rung, path-relative-to-root). Tags are discovered in-file.
TARGETS = [
    ("deadlock", "rung1_convention", "invariants/deadlock/rung1_convention/src/lib.rs"),
    ("deadlock", "rung2_runtime", "invariants/deadlock/rung2_runtime/src/lib.rs"),
    ("deadlock", "rung4_typestate", "invariants/deadlock/rung4_typestate/src/lib.rs"),
    ("deadlock", "rung5_eliminated", "invariants/deadlock/rung5_eliminated/src/lib.rs"),
    ("risk_check", "rung1_convention", "invariants/risk_check/rung1_convention/src/lib.rs"),
    ("risk_check", "rung4_typestate", "invariants/risk_check/rung4_typestate/src/lib.rs"),
]

START = re.compile(r"//\s*BOILERPLATE-START\s+(\S+)")
END = re.compile(r"//\s*BOILERPLATE-END\s+(\S+)")
TOKEN = re.compile(r"[A-Za-z_][A-Za-z0-9_]*|[0-9]+|[^\sA-Za-z0-9_]")


def strip_comment(line: str) -> str:
    # Good enough for our sources: no `//` appears inside string literals here.
    i = line.find("//")
    return line if i < 0 else line[:i]


def measure(path: str):
    code_loc, tokens, span, tags = 0, 0, 0, []
    in_region = False
    with open(path) as f:
        for raw in f:
            s = START.search(raw)
            e = END.search(raw)
            if s:
                in_region = True
                tags.append(s.group(1))
                continue  # the fence line itself is not counted
            if e:
                in_region = False
                continue
            if not in_region:
                continue
            span += 1
            code = strip_comment(raw).strip()
            if not code:
                continue
            code_loc += 1
            tokens += len(TOKEN.findall(code))
    return {"code_loc": code_loc, "tokens": tokens, "marked_span_lines": span, "regions": tags}


def main():
    rows = []
    print(f"{'invariant':>10} {'rung':>16} {'code_loc':>9} {'tokens':>7}")
    for invariant, rung, rel in TARGETS:
        m = measure(os.path.join(ROOT, rel))
        rows.append({"invariant": invariant, "rung": rung, "file": rel, **m})
        print(f"{invariant:>10} {rung:>16} {m['code_loc']:>9} {m['tokens']:>7}")
    doc = {
        "benchmark": "boilerplate",
        "metric": "caller-authored code between BOILERPLATE-START/END fences; comments+blanks excluded",
        "rows": rows,
    }
    out = os.path.join(HARNESS, "results", "boilerplate.json")
    with open(out, "w") as f:
        f.write(json.dumps(doc, indent=2) + "\n")
    print(f"\nwrote results/boilerplate.json ({len(rows)} rungs)")


if __name__ == "__main__":
    sys.exit(main())
