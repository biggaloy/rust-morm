#!/bin/bash
# fa2.sh: the twenty-third review's FA2 settings, 256 sequences each mode, with the cost counters
# usage: bash fa2.sh <model binary> [fault]
M=$1
f=${2:-0}
for s in "defaults" "entries off" "transactions without retry" "transactions without retry, E a fifth of W" "transactions without retry, entries off"; do
  V=1 "$M" "$s" "$f" 0 256 both | grep -E "^==|FAIL|^ +(calls|transactions|SchemaChanged for bad text[^=]*|entries off:[^=]*|a bad label[^=]*) =" | grep -v " lit \| ez \| mid "
done
