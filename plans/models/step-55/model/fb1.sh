#!/bin/bash
# fb1.sh: the twenty-fourth review's FB1 settings, one statement under a per-transaction search_path with no
# .retry(n), entries on and off, with the counts; usage: bash fb1.sh <model binary> [fault] [count]
M=$1
f=${2:-0}
n=${3:-256}
for s in "search_path only, no retry" "search_path only, no retry, entries off" "search_path only, no retry, E off by 0s" "transactions without retry" "transactions without retry, entries off"; do
  V=1 "$M" "$s" "$f" 0 "$n" both | grep -E "^==|FAIL|^ +(search_path[^=]*|entries off: search_path[^=]*|ms simulated|a 22P02 the second row takes, as it is in a savepoint[^=]*) =" | grep -v " lit \| ez \| mid "
done
