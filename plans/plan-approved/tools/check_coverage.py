#!/usr/bin/env python3
"""Coverage checks for plan-approved.

  check_coverage.py                    plan level: every requirement and test
                                       obligation is in exactly one place, with
                                       its text intact
  check_coverage.py --code DIR...      also: each done step has every code
                                       requirement and obligation tagged
  check_coverage.py --step NN --code DIR...
                                       the same for one step, whatever its
                                       status, so a step can check itself

A tag is any line holding `covers:` followed by IDs, in any comment style.
"""
import json
import re
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
man = json.loads((ROOT / "tools/manifest.json").read_text(encoding="utf-8"))
errs = []
ids = re.compile(r"\b[RT]-\d{4}\b")
SUFFIXES = {".rs", ".stderr", ".py", ".sh", ".yml", ".yaml", ".toml", ".md", ".sql"}
STATUSES = {"todo", "in progress", "in review", "done"}


def norm(s):
    return re.sub(r"^(-|\d+\.)\s+", "", s).replace("|", "").replace("\\", "")


texts, placed = {}, defaultdict(list)
for f in sorted((ROOT / "steps").glob("*.md")):
    t = f.read_text(encoding="utf-8")
    texts[f.name] = t
    # IDs are read outside the context block, which quotes without IDs
    for i in set(ids.findall(t)):
        placed[i].append(f.name)
contracts = (ROOT / "contracts.md").read_text(encoding="utf-8")
index = (ROOT / "INDEX.md").read_text(encoding="utf-8")

for r in man["reqs"]:
    n = r["id"]
    if r["step"] in ("contracts", "index"):
        hay = contracts if r["step"] == "contracts" else index
        if n not in hay:
            errs.append(f"{n}: missing from {'contracts.md' if r['step'] == 'contracts' else 'INDEX.md'}")
        elif norm(r["text"]) not in hay.replace("|", "").replace("\\", ""):
            errs.append(f"{n}: text changed")
        continue
    if len(placed[n]) != 1:
        errs.append(f"{n}: expected in one step, found {placed[n]}")
    elif norm(r["text"]) not in texts[placed[n][0]].replace("|", "").replace("\\", ""):
        errs.append(f"{n}: text changed in {placed[n][0]}; an approved edit must update tools/manifest.json too")
for t in man["tests"]:
    if len(placed[t["id"]]) != 1:
        errs.append(f"{t['id']}: expected in one step, found {placed[t['id']]}")

rows = re.findall(r"^\| (\d{2}) \|.*\|\s*([^|]*?)\s*\|\s*$", index, re.M)
if len(rows) != len(texts):
    errs.append(f"INDEX.md lists {len(rows)} steps, steps/ has {len(texts)}")
for num, st in rows:
    if st not in STATUSES:
        errs.append(f"INDEX.md: step {num} has unknown status {st!r}")
done = {n for n, st in rows if st == "done"}

if "--code" in sys.argv:
    argv = sys.argv[1:]
    only = None
    if "--step" in argv:
        i = argv.index("--step")
        only = argv[i + 1]
        del argv[i:i + 2]
    dirs = argv[argv.index("--code") + 1:]
    tags = set()
    for d in dirs:
        if not Path(d).exists():
            continue
        for p in Path(d).rglob("*"):
            if p.is_file() and p.suffix in SUFFIXES:
                for line in p.read_text(encoding="utf-8", errors="ignore").splitlines():
                    if "covers:" in line:
                        tags.update(ids.findall(line.split("covers:", 1)[1]))
    names = {f[3:-3]: f[:2] for f in texts}
    wanted = {only} if only else done
    for item in man["reqs"] + man["tests"]:
        if item.get("kind") in ("info", "doc"):
            continue
        st = names.get(item["step"])
        if st in wanted and item["id"] not in tags:
            errs.append(f"{item['id']}: step {st} has no test tagged with it")

for e in errs[:300]:
    print(e)
print(f"{len(man['reqs'])} requirements, {len(man['tests'])} test obligations, {len(errs)} problems")
sys.exit(1 if errs else 0)
