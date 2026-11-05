#!/usr/bin/env python3
"""Checks every combination of each published crate's Cargo features."""
import itertools
import json
import subprocess
import sys

meta = json.loads(
    subprocess.check_output(["cargo", "metadata", "--format-version=1", "--no-deps", "--locked"])
)
for package in meta["packages"]:
    if package["publish"] == []:
        continue
    features = sorted(f for f in package["features"] if f != "default")
    for size in range(len(features) + 1):
        for combo in itertools.combinations(features, size):
            cmd = ["cargo", "check", "--locked", "-p", package["name"], "--all-targets"]
            cmd += ["--no-default-features", "--features", ",".join(combo)]
            print("+", " ".join(cmd), flush=True)
            if subprocess.call(cmd) != 0:
                sys.exit(1)
    cmd = ["cargo", "check", "--locked", "-p", package["name"], "--all-targets"]
    print("+", " ".join(cmd), flush=True)
    if subprocess.call(cmd) != 0:
        sys.exit(1)
