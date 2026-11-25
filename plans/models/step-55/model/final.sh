#!/bin/bash
cd "$(dirname "$0")" || exit 1
M=../../../.claude/scratch/model
for o in trig,row7,a1s trig trig,row7; do
  echo "## OPT=$o"
  OPT=$o timeout 60 $M defaults 0 0 256 | grep -vE " (lit|ez|mid) |ANY"
done
timeout 200 $M faults 0 0 256 > ../faults256.txt
timeout 900 $M faults 0 1000 2048 > ../faults2048.txt
timeout 300 $M all 0 0 256 > ../all256.txt
timeout 900 $M all 0 1000 2048 > ../all2048.txt
echo "no-fault settings failing a final check, 256 and 2048:"
grep FAIL ../all256.txt | grep -cvE " (lit|ez|mid) "
grep FAIL ../all2048.txt | grep -vE " (lit|ez|mid) "
