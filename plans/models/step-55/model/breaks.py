"""Deliberate breaks of the clauses no switch breaks: python3 breaks.py <model binary>"""
import subprocess, sys

M = sys.argv[1]
RUNS = [("straight", x, "defaults") for x in (101, 102, 103, 107, 42, 43, 45)]
RUNS += [("pooled", x, "defaults") for x in (101, 103, 105, 106, 107, 42, 43, 44, 46)]
RUNS += [(m, 41, "old labels copied") for m in ("straight", "pooled")]
RUNS += [("straight", x, "hand relabels without the reset, casts") for x in (4, 5)]
RUNS += [("pooled", 105, "hand relabels without the reset"), ("pooled", 106, "hand relabels without the reset, casts")]
RUNS += [(m, f, "set") for f in (110, 111, 112, 113, 114, 115, 116, 117, 118, 119) for m in ("straight", "pooled")]  # the name test: 110 the rule it replaces
RUNS += [("pooled", 35, "max 1"), ("pooled", 35, "defaults")]
# the four answers: 120 only a statement that casts a parameter takes the second row, 121 a name printed with no
# schema names a cast written with one, 122 a statement that casts to two types is read by the name test
RUNS += [(m, f, "set") for f in (120, 121, 122) for m in ("straight", "pooled")]
RUNS += [(m, f, "defaults") for f in (120, 121, 122) for m in ("straight", "pooled")]
RUNS += [(m, 121, "a cast written with its schema, its kept type moved where the search_path finds it") for m in ("straight", "pooled")]
# the cost check: a pending entry ignored, so a bad text's Error::SchemaChanged recurs while its entry lasts
RUNS += [(m, 107, "transactions without retry") for m in ("straight", "pooled")]
for mode, b, s in RUNS:
    out = subprocess.run([M, f"set{b}" if s == "set" else s, str(b), "0", "256", mode], capture_output=True, text=True).stdout
    print(f"-- break {b} [{mode}] {s}")
    for l in out.split("\n"):
        if (("FAIL" in l) if s != "set" else l) and not any(x in l for x in (" lit ", " ez ", " mid ", "ANY")):
            print("  ", l.strip())
