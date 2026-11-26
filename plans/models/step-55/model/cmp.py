"""cmp.py OLD NEW: compares two result files, setting aside timings, example seeds, FE1's, FF1's, FG1's and FG2's counters and checks
("lit FE1", "lit FF1", "I4 span replacements, FF1", "I3 span, an epoch's replacements, R-0981"), and the two settings
the twenty-eighth review's fix added."""
import re
import sys

NEW = ("stored text only, no schema change", "record shapes only, no schema change")


def norm(p):
    out, skip = [], False
    for l in open(p, encoding="utf-8"):
        if l.startswith("== "):
            skip = any(l.startswith("== " + n + " [") for n in NEW)
        if skip or "FE1" in l or "FF1" in l or "R-0981" in l or "FG1" in l or "FG2" in l:
            continue
        l = re.sub(r"\(\d+\.\d+s\)", "", l)
        l = re.sub(r"   e\.g\. .*$", "", l.rstrip("\n"))
        out.append(l)
    return out


a, b = norm(sys.argv[1]), norm(sys.argv[2])
if a == b:
    print("same", len(a), "lines")
else:
    import difflib
    for d in difflib.unified_diff(a, b, lineterm="", n=0):
        print(d)
