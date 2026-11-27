#!/bin/bash
# fh.sh: the thirtieth review's FH1 and FH2; usage: bash fh.sh <model binary> [count]
# FH1: a stored-text function's run on the new connection (row A2) or the next attempt's (row B3) that ends in another
# 22P02, another bad label's from enum_in or bad text for a number's from another routine, requests nothing; "FH1 a
# run ending in another 22P02 brought a replace" is the check (break 129), "lit FH1" R-1249's and R-1250's older
# wording. FH2: the span checks count the moves of the pool's last-read <n> from stale_cooldown before the span to its
# end; "lit FH2" counts the changes committed instead, as an older reading's check.
M=$1
n=${2:-256}
for s in "stored text only, no schema change" "defaults" "transactions without retry" "heavy" "max 1" "max 2, budget 2, heavy" "role limit" "refusals" "no client input"; do
  V=1 "$M" "$s" 0 0 "$n" straight | grep -E "^==|FH1|FH2|FG1 |FF1 most|I4 span|I3 span|most stale-driven opens|ms simulated =" | grep -v " ez \| mid \|lit I[0-9]\|lit R-\|lit FD1\|lit FF1\|lit FG"
done
