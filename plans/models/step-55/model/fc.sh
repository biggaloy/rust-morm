#!/bin/bash
# fc.sh: the twenty-fifth review's FC1 and FC2 counts; usage: bash fc.sh <model binary> [fault] [count]
# FC1: the search_path case with entries on, each SchemaChanged classed by when its call began, and the older
# wording's check ("lit I8 search_path once per E"), which excused only calls running at the first failure.
# FC2: with entries off behind PgBouncer, stale statements' SchemaChanged, and any begun after a pool-wide action
# taken since its change, which I8's clearing clause fails.
M=$1
f=${2:-0}
n=${3:-256}
for s in "search_path only, no retry" "transactions without retry" "entries off" "transactions without retry, entries off"; do
  V=1 "$M" "$s" "$f" 0 "$n" both | grep -E "^==|FAIL|^ +(search_path[^=]*|entries off[^=]*|ms simulated|SchemaChanged[^=]*) =" | grep -v " ez \| mid "
done
