#!/bin/bash
# cost.sh: what the answer costs at the load, 256 sequences each mode, beside the procedure before it (break 123)
# and beside a second row that took no statement casting no parameter (break 120)
S="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)/.claude/scratch"; mkdir -p "$S"   # the worktree's .claude/scratch
M=$S/model
for f in 0 123 120; do
  V=1 T=${T:-11} "$M" defaults $f 0 256 | grep -E "^==|^ +(runs|calls|SchemaChanged for bad text|SchemaChanged for a kept function's error|SchemaChanged for bad text or a kept function's error, no retry|a bad label[^=]*) =|FAIL" | grep -v " lit \| ez \| mid "
done
