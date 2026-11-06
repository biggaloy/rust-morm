#!/usr/bin/env python3
"""Compares this run's measurements with the last run's.

  bench_compare.py CURRENT.json PREVIOUS.json

Both files map a measurement name to a number. PREVIOUS.json may be missing on
the first run. A name only the last run has is listed as removed. The table goes
to stdout, for the job summary.
"""
import json
import sys
from pathlib import Path

current = json.loads(Path(sys.argv[1]).read_text())
previous_file = Path(sys.argv[2])
previous = json.loads(previous_file.read_text()) if previous_file.exists() else {}

print("| Measurement | Last run | This run | Change |")
print("|---|---|---|---|")
for name in sorted(current.keys() | previous.keys()):
    value = current.get(name)
    last = previous.get(name)
    if value is None:
        print(f"| {name} | {last} | removed | n/a |")
        continue
    change = f"{(value - last) / last:+.1%}" if last else "n/a"
    print(f"| {name} | {'n/a' if last is None else last} | {value} | {change} |")
