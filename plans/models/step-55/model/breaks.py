"""Deliberate breaks of the clauses no switch breaks: python3 breaks.py <model binary>"""
import subprocess, sys

M = sys.argv[1]
RUNS = [("straight", x, "defaults") for x in (101, 102, 103, 107, 42, 45)]
RUNS += [("pooled", x, "defaults") for x in (101, 103, 105, 106, 107, 42, 44, 46)]
RUNS += [(m, 41, "old labels copied") for m in ("straight", "pooled")]
RUNS += [("straight", x, "hand relabels without the reset, casts") for x in (4, 5)]
RUNS += [("pooled", 105, "hand relabels without the reset"), ("pooled", 106, "hand relabels without the reset, casts")]
RUNS += [("pooled", 35, "max 1"), ("pooled", 35, "defaults")]
# the answer (2026-11-26): every cached statement's 22P02 from enum_in takes the second row. 120: only a statement
# that casts a parameter takes it; 123: the procedure before the answer, the name test reading a statement that casts
# one parameter, the conversion case among them (switch 36 reads every one)
RUNS += [(m, f, "set") for f in (120, 123) for m in ("straight", "pooled")]
RUNS += [(m, f, "defaults") for f in (120, 123) for m in ("straight", "pooled")]
RUNS += [(m, 123, "every call a cast beside a second type") for m in ("straight", "pooled")]
# I14, the second row's cost: a pending entry ignored, so a bad text's Error::SchemaChanged recurs while its entry lasts
RUNS += [(m, 107, "transactions without retry") for m in ("straight", "pooled")]
for mode, b, s in RUNS:
    out = subprocess.run([M, f"set{b}" if s == "set" else s, str(b), "0", "256", mode], capture_output=True, text=True).stdout
    print(f"-- break {b} [{mode}] {s}")
    for l in out.split("\n"):
        if (("FAIL" in l) if s != "set" else l) and not any(x in l for x in (" lit ", " ez ", " mid ", "ANY")):
            print("  ", l.strip())
