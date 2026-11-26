#!/bin/bash
# fd1.sh: the twenty-sixth review's FD1 counts; usage: bash fd1.sh <model binary> [fault] [count]
# One statement under a per-transaction search_path with entries on and no .retry(n), E = W = 1 s: each
# SchemaChanged classed against the earliest earlier failure whose entry wasn't made when its run began, as in
# flight at that error, in a transaction started before it, or in one started in the gap; with none, the first.
# "lit FD1" is the README's older gap clause, which counted only transactions started in the gap; I8's check,
# "new I8 a failure begun while its entry exists", is the count R-1253 and R-1272 now state.
M=$1
f=${2:-0}
n=${3:-256}
for s in "search_path only, no retry" "search_path only, no retry, up to 100 ms to the run" "transactions without retry"; do
  V=1 "$M" "$s" "$f" 0 "$n" both | grep -E "^==|FAIL|^ +(search_path[^=]*|ms simulated) =" | grep -v " ez \| mid \|lit I[0-9]\|lit R-"
done
