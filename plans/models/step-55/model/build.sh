#!/bin/bash
cd "$(dirname "$0")" || exit 1
export TMPDIR="$(cd ../../../.. && pwd)"   # .claude/scratch
rustc -O --edition 2024 main.rs -o "$TMPDIR/model" 2>&1 | grep -E "^error" -A8 | head -60
