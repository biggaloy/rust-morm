#!/bin/bash
cd "$(dirname "$0")"
M=../../../.claude/scratch/model
OPT=both timeout 60 $M slow 0 0 256 straight | grep -vE " (lit|ez|mid) |ANY"
OPT=both,a1s timeout 60 $M slow 0 0 256 straight | grep -vE " (lit|ez|mid) |ANY"
for o in trig trig,row7 trig,a1s trig,row7,a1s noreset noreset,row7 noreset,a1s noreset,row7,a1s; do echo "## $o"; for s in "hand relabels" defaults; do OPT=$o timeout 60 $M "$s" 0 0 256 | grep -vE " (lit|ez|mid) |ANY"; done; done
