#!/bin/bash
cd "$(dirname "$0")" || exit 1
f() { V=1 bash some.sh "$@" | grep -E "^==|FAIL" | grep -vE " (lit|ez|mid) |ANY" | cut -c1-110; }
for x in 101 102 103 104 107; do f straight $x 0 256 defaults; done
f straight 104 0 256 held
for x in 101 103 105 106 107; do f pooled $x 0 256 defaults; done
echo "## real"; OPT=real f straight 0 0 256 defaults; OPT=real f pooled 0 0 256 defaults
