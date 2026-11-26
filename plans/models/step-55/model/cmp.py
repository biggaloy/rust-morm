"""cmp.py OLD NEW: compares two result files, setting aside timings, example seeds, FE1's counters and "lit FE1" lines."""
import re
import sys


def norm(p):
    out = []
    for l in open(p, encoding="utf-8"):
        if "FE1" in l:
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
