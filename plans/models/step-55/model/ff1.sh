#!/bin/bash
# ff1.sh: the twenty-eighth review's FF1; usage: bash ff1.sh <model binary> [count]
# Straight to Postgres, stale-driven replacements in any span of stale_cooldown, an epoch's and a *replace* action's
# together: "I4 span replacements, FF1" fails more than 4 × max plus 2 × max per migration, R-1268's new "Shorter:"
# and R-1271's "Churn"; "lit FF1" is R-1268's older wording, 2 × max plus max per migration, an epoch's alone.
M=$1
n=${2:-256}
for s in "stored text only, no schema change" "record shapes only, no schema change" "defaults" "transactions without retry" "heavy" "max 1" "max 2, budget 2, heavy" "role limit" "refusals"; do
  V=1 "$M" "$s" 0 0 "$n" straight | grep -E "^==|FF1|R-0981|most stale-driven opens|ms simulated =" | grep -v " ez \| mid \|lit I[0-9]\|lit R-\|lit FD1"
done
