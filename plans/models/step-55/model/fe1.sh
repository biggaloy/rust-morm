#!/bin/bash
# fe1.sh: the twenty-seventh review's FE1; usage: bash fe1.sh <model binary> [count] [faults...]
# Behind PgBouncer, each row D2 Error::SchemaChanged classed against the statement's last one in that pool: the first,
# after a pool-wide action, after an adoption or a moved <n>, after a push-out by the cap, or after the expiry, each
# removal counted when it came before that entry's expiry. "new I8 FE1" fails two within stale_cooldown with no
# removal between; "lit FE1" is R-1249's and R-1258's older wording, "once" and "for stale_cooldown", which fails any
# two within stale_cooldown. Faults: 125, D2 never enters the cool-down inside a transaction; 126, it enters it a step
# after its failure; 127, a cool-down entry expiring half of stale_cooldown after it's made.
M=$1
n=${2:-256}
shift 2
faults=${*:-0}
for f in $faults; do
  for s in "defaults" "transactions without retry" "caps of 1" "heavy" "two processes" "three processes, heavy" "hand relabels without the reset"; do
    V=1 "$M" "$s" "$f" 0 "$n" pooled | grep -E "^==|FAIL|FE1|ms simulated =" | grep -v " ez \| mid \|lit I[0-9]\|lit R-\|lit FD1"
  done
done
