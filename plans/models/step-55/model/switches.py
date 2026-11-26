"""switches.py RESULTS_DIR: per switch and mode, the sequences caught (ANY) of 256 and of 2,048, and the lowest."""
import re
import sys

d = sys.argv[1]


def load(p):
    out = {}
    for l in open(p, encoding="utf-8"):
        m = re.match(r"switch (\d+) \[(\w+)\]: (.*?)   \| no fault: (.*)$", l.rstrip("\n"))
        if m:
            f = dict(x.rsplit("=", 1) for x in m[3].split("; ") if "=" in x)
            out[(int(m[1]), m[2])] = (int(f.get("ANY", 0)), f, m[4])
    return out


a, b = load(f"{d}/faults256.txt"), load(f"{d}/faults2048.txt")
low = []
for k in sorted(a):
    if k[0] == 40:
        continue
    x, y = a[k][0], b.get(k, (0,))[0]
    nf = (a[k][2] or "") + (b.get(k, (0, {}, ""))[2] or "")
    print(f"{k[0]:>3} {k[1]:8} {x:>4} {y:>5}" + (f"  NO-FAULT FAILS: {nf}" if nf.strip() else ""))
    low.append((x, y, k))
low.sort()
print("lowest by 256:", low[:4])
print("lowest by 2048:", sorted(low, key=lambda t: t[1])[:4])
