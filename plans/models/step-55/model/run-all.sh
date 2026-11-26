#!/bin/bash
# run-all.sh <model binary> <results dir>: full.sh's commands, with the binary and output dir given
M=$1; R=$2; cd "$(dirname "$0")" || exit 1
$M all 0 0 256 > $R/all256.txt
$M faults 0 0 256 > $R/faults256.txt
python3 breaks.py $M > $R/breaks.txt
$M all 0 1000 2048 > $R/all2048.txt
$M faults 0 1000 2048 > $R/faults2048.txt
$M all 0 10000 20000 straight > $R/all20000-straight.txt
$M all 0 10000 20000 pooled > $R/all20000-pooled.txt
echo done > $R/full.done
