#!/bin/bash
# builds the model to .claude/scratch/model in the worktree this folder is in
S="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)/.claude/scratch"; mkdir -p "$S"   # the worktree's .claude/scratch
cd "$(dirname "$0")" || exit 1
export TMPDIR="$S"
rustc -O --edition 2024 main.rs -o "$S/model" 2>&1 | grep -E "^error" -A8 | head -80
