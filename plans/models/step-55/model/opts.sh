#!/bin/bash
# the options and breaks, only lines not starting "lit "
cd "$(dirname "$0")" || exit 1
f() { V=1 bash some.sh "$@" | grep -E "^==|FAIL" | grep -vE " (lit|ez|mid) " | cut -c1-260; }
echo "### fault 32"; f straight 32 0 256 defaults "long holds" "held"
echo "### fault 33"; f straight 33 0 256 defaults; f pooled 33 0 256 defaults
echo "### EZ6 (101)"; f straight 101 0 256 defaults; f pooled 101 0 256 defaults
echo "### XX000 kept as it is outside (102)"; f straight 102 0 256 defaults
echo "### cast's 22P02 as it is (103)"; f straight 103 0 256 defaults
echo "### no evict in the app's transaction (104)"; f straight 104 0 256 defaults "held"
echo "### OPT real"; OPT=real f straight 0 0 256 defaults; OPT=real f pooled 0 0 256 defaults
echo "### OPT both"; OPT=both f straight 0 0 256 defaults quiet; OPT=both,a1s f straight 0 0 256 defaults quiet
echo "### hand relabels, row 2 first / row 7 first"; f straight 0 0 256 "hand relabels"; OPT=row7 f straight 0 0 256 "hand relabels"; f pooled 0 0 256 "hand relabels"; OPT=row7 f pooled 0 0 256 "hand relabels"
