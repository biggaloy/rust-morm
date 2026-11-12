#!/usr/bin/env python3
"""Refuses a release unless the checked-out commit may be published.

  check_release_tag.py TAG

Run it from the repo root, with full git history. TAG must be `v` plus the
workspace version, and HEAD must be on `main`.
"""
import subprocess
import sys
import tomllib
from pathlib import Path

MAIN = "refs/remotes/origin/main"


def git(*args):
    return subprocess.run(["git", *args], capture_output=True, text=True)


tag = sys.argv[1]
manifest = tomllib.loads(Path("Cargo.toml").read_text(encoding="utf-8"))
version = manifest["workspace"]["package"]["version"]
if tag != f"v{version}":
    sys.exit(f"tag {tag} doesn't match the workspace version {version}")
if git("rev-parse", "--is-shallow-repository").stdout.strip() != "false":
    sys.exit("the checkout is shallow, so it can't show the commit is on main")
if git("rev-parse", "--verify", "--quiet", MAIN).returncode != 0:
    sys.exit(f"{MAIN} is missing from the checkout")
if git("merge-base", "--is-ancestor", "HEAD", MAIN).returncode != 0:
    sys.exit(f"the tagged commit isn't on main: tag {tag}")
print(f"{tag} matches the workspace version and is on main")
