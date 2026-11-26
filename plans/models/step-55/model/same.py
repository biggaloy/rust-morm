"""same.py A B: compares two model outputs line by line, setting aside timings and example seeds only"""
import re, sys


def norm(p):
    out = []
    for l in open(p, encoding='utf-8'):
        l = re.sub(r"\(\d+\.\d+s\)", "", l)
        l = re.sub(r"   e\.g\. .*$", "", l.rstrip("\n"))
        out.append(l)
    return out


a, b = norm(sys.argv[1]), norm(sys.argv[2])
if a == b:
    print("identical", len(a), "lines")
else:
    import difflib
    for d in list(difflib.unified_diff(a, b, lineterm="", n=0))[:60]:
        print(d)
