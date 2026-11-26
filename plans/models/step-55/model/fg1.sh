#!/bin/bash
# fg1.sh: the twenty-ninth review's FG1 and FG2; usage: bash fg1.sh <model binary> [count]
# Straight to Postgres, each *replace* a stored-text function's confirming run requests, by what made it confirm (a
# good label, NULL, no row, or the function failing another way), in rows A2 and B3; "lit FG1" is R-0981's older
# wording, a good label only. FG2: the connection whose run requested the action, behind at once or once it's taken,
# replaced at its next checkout; "lit FG2" is R-1249's older "every other connection". With the FF1 span checks.
M=$1
n=${2:-256}
for s in "stored text only, no schema change" "defaults" "transactions without retry" "heavy" "max 1" "max 2, budget 2, heavy" "role limit" "refusals" "no client input"; do
  V=1 "$M" "$s" 0 0 "$n" straight | grep -E "^==|FG1|FG2|FF1|R-0981|most stale-driven opens|ms simulated =" | grep -v " ez \| mid \|lit I[0-9]\|lit R-\|lit FD1\|lit FF1"
done
