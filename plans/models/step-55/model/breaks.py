"""Deliberate breaks of the clauses no switch breaks: python3 breaks.py <model binary>"""
import subprocess, sys

M = sys.argv[1]
RUNS = [("straight", x, "defaults") for x in (101, 102, 103, 107, 42, 43, 45)]
RUNS += [("pooled", x, "defaults") for x in (101, 103, 105, 106, 107, 42, 43, 44, 46)]
RUNS += [(m, 41, "old labels copied") for m in ("straight", "pooled")]
RUNS += [("straight", x, "hand relabels without the reset, casts") for x in (4, 5)]
RUNS += [("pooled", 105, "hand relabels without the reset"), ("pooled", 106, "hand relabels without the reset, casts")]
RUNS += [("pooled", 35, "max 1"), ("pooled", 35, "defaults")]
for mode, b, s in RUNS:
    out = subprocess.run([M, s, str(b), "0", "256", mode], capture_output=True, text=True).stdout
    print(f"-- break {b} [{mode}] {s}")
    for l in out.split("\n"):
        if "FAIL" in l and not any(x in l for x in (" lit ", " ez ", " mid ", "ANY")):
            print("  ", l.strip())
