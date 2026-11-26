#!/bin/bash
# fa2breaks.sh <model binary>: break 124 and switch 28 against I14's entries-off clause, 256 sequences each
M=$1
for s in "entries off" "transactions without retry, entries off"; do
  "$M" "$s" 124 0 256 both | grep -E "^==|FAIL" | grep -v " lit \| ez \| mid "
done
"$M" "entries off" 28 0 256 both | grep -E "^==|FAIL" | grep -v " lit \| ez \| mid "
