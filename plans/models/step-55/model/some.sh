#!/bin/bash
# some.sh <mode> <fault> <seed0> <count> <setting>...
S="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)/.claude/scratch"; mkdir -p "$S"   # the worktree's .claude/scratch
M=$S/model
mode=$1; f=$2; s=$3; n=$4; shift 4
for x in "$@"; do timeout 250 "$M" "$x" "$f" "$s" "$n" "$mode"; done
