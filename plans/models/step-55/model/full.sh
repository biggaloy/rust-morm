#!/bin/bash
# every setting and switch: full.sh, about 25 minutes on 11 threads
cd "$(dirname "$0")" || exit 1
M=../../../.claude/scratch/model
$M all 0 0 256 > ../all256.txt
$M faults 0 0 256 > ../faults256.txt
$M all 0 1000 2048 > ../all2048.txt
$M faults 0 1000 2048 > ../faults2048.txt
OPT=pooled2 $M all 0 0 2048 pooled > ../pooled2-2048.txt
$M all 0 10000 20000 > ../all20000.txt
echo done > ../full.done
