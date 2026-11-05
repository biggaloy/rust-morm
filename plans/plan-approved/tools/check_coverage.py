#!/usr/bin/env python3
"""Coverage checks for plan-approved.

  check_coverage.py                 plan level: every requirement and test
                                    obligation is in exactly one place
  check_coverage.py --code DIR...   also: every code requirement and test
                                    obligation of a finished step has a test
                                    tagged `covers: R-nnnn` under DIR
"""
import json
import re
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
man = json.loads((ROOT / "tools/manifest.json").read_text())
errs = []

ids = re.compile(r"\b[RT]-\d{4}\b")
seen = Counter()
for f in sorted((ROOT / "steps").glob("*.md")):
    seen.update((m, f.name) for m in set(ids.findall(f.read_text(encoding="utf-8"))))
counts = Counter(i for i, _ in seen.elements())
placed = {}
for (i, name) in seen:
    placed.setdefault(i, []).append(name)

for r in man["reqs"]:
    n = r["id"]
    if r["step"] in ("contracts", "index"):
        fn = "contracts.md" if r["step"] == "contracts" else "INDEX.md"
        if n not in (ROOT / fn).read_text(encoding="utf-8"):
            errs.append(f"{n}: missing from {fn}")
        continue
    if len(placed.get(n, [])) != 1:
        errs.append(f"{n}: expected in one step, found {placed.get(n, [])}")
for t in man["tests"]:
    if len(placed.get(t["id"], [])) != 1:
        errs.append(f"{t['id']}: expected in one step, found {placed.get(t['id'], [])}")

if "--code" in sys.argv:
    dirs = sys.argv[sys.argv.index("--code") + 1:]
    tags = set()
    tag = re.compile(r"covers:\s*((?:[RT]-\d{4}[,\s]*)+)")
    for d in dirs:
        for p in Path(d).rglob("*"):
            if p.is_file() and p.suffix in (".rs", ".stderr", ".py", ".sh", ".yml", ".md"):
                for m in tag.findall(p.read_text(encoding="utf-8", errors="ignore")):
                    tags.update(ids.findall(m))
    index = (ROOT / "INDEX.md").read_text(encoding="utf-8")
    done = set(re.findall(r"^\| (\d{2}) \|.*\| done \|$", index, re.M))
    names = {f.name.split("-", 1)[1][:-3]: f.name[:2] for f in (ROOT / "steps").glob("*.md")}
    for item in man["reqs"] + man["tests"]:
        kind = item.get("kind")
        if kind in ("info", "doc"):
            continue
        st = names.get(item["step"])
        if st in done and item["id"] not in tags:
            errs.append(f"{item['id']}: step {st} is done but no test is tagged with it")

for e in errs[:200]:
    print(e)
print(f"{len(man['reqs'])} requirements, {len(man['tests'])} test obligations, {len(errs)} problems")
sys.exit(1 if errs else 0)
