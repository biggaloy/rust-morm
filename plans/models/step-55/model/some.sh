#!/bin/bash
# some.sh <mode> <fault> <seed0> <count> <setting>...
M="$(dirname "$0")/../../../.claude/scratch/model"
mode=$1; f=$2; s=$3; n=$4; shift 4
for x in "$@"; do timeout 250 "$M" "$x" "$f" "$s" "$n" "$mode"; done
