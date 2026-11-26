#!/bin/bash
# cost.sh: what the two answers cost at the load, 256 sequences each mode, beside each answer undone (breaks 120 to 122)
M="$(cd "$(dirname "$0")/../../../.." && pwd)/model"
for f in 0 120 121 122; do
  V=1 T=${T:-11} "$M" defaults $f 0 256 | grep -E "^==|^ +(runs|calls|SchemaChanged for bad text|SchemaChanged for a kept function's error|SchemaChanged for bad text or a kept function's error, no retry|a bad label the second row always takes: SchemaChanged|kept function's error[^=]*) =|FAIL" | grep -v " lit \| ez \| mid "
done
