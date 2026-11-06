#!/usr/bin/env python3
"""Writes this run's measurements as JSON, for bench_compare.py.

  bench_metrics.py BINARY RSS_FILE BENCH_OUTPUT METRICS.json

BINARY is morm-admin, RSS_FILE holds GNU time's %M figure, and BENCH_OUTPUT is
what `cargo bench` printed. It reads libtest and Criterion result lines, and
fails on a result line it can't read or on two results with the same name.
Criterion's percentage `time:` lines, under `change:`, compare with its own last
run, so they're skipped.
"""
import json
import re
import sys
from pathlib import Path

LIBTEST = re.compile(r"^test (\S+)\s+\.\.\. bench:\s+([\d,.]+) ns/iter")
CRITERION = re.compile(r"^\s*(.*?)\s*time:\s+\[[\d.]+ \S+ ([\d.]+) (\S+) [\d.]+ \S+\]")
CRITERION_CHANGE = re.compile(r"^\s*time:\s+\[\S+% ")
RESULT_LINE = re.compile(r"\.\.\. bench:|\btime:\s+\[")
NANOSECONDS = {"ps": 0.001, "ns": 1, "us": 1_000, "µs": 1_000, "ms": 1_000_000, "s": 1_000_000_000}

binary, rss_file, bench_output, metrics_file = sys.argv[1:]
metrics = {
    "morm-admin binary bytes": Path(binary).stat().st_size,
    "morm-admin peak RSS KiB": int(Path(rss_file).read_text()),
}
previous_line = ""
for line in Path(bench_output).read_text(encoding="utf-8").splitlines():
    if CRITERION_CHANGE.match(line):
        continue
    if libtest := LIBTEST.match(line):
        name, value = libtest[1], float(libtest[2].replace(",", ""))
    elif criterion := CRITERION.match(line):
        name = criterion[1] or previous_line.strip()
        value = float(criterion[2]) * NANOSECONDS[criterion[3]]
    elif RESULT_LINE.search(line):
        sys.exit(f"can't read this benchmark result: {line}")
    else:
        if line.strip():
            previous_line = line
        continue
    key = f"bench {name} ns"
    if key in metrics:
        sys.exit(f"two benchmark results share the name: {name}")
    metrics[key] = value
Path(metrics_file).write_text(json.dumps(metrics, indent=2) + "\n")
