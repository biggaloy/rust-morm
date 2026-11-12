#!/usr/bin/env bash
# Proof 10: registration behind `cfg(morm_mgr)`.
#
# 1. `cargo build --workspace` with warnings denied: no model crate warns
#    (`unexpected_cfgs`), and neither the app nor `mgr` carries metadata.
# 2. `mgr` built with `--cfg morm_mgr`, in its own target folder, finds the
#    models of all three crates, including the one no code references.
# 3. Control: the app built with the cfg does carry metadata, so the search
#    in step 1 would have found it.
set -euo pipefail
cd "$(dirname "$0")"

fail() {
  echo "proof 10 failed: $*" >&2
  exit 1
}

RUSTFLAGS="-D warnings" cargo build --locked --workspace --target-dir target/p10-plain
for bin in app mgr; do
  if grep -qa "morm-meta:" "target/p10-plain/debug/$bin"; then
    fail "the plain build of $bin carries model metadata"
  fi
done
[ -z "$(target/p10-plain/debug/mgr)" ] || fail "mgr found models without the cfg"
echo "plain build: no metadata in app or mgr"

RUSTFLAGS="--cfg morm_mgr -D warnings" cargo build --locked -p mgr -p app --target-dir target/p10-mgr
found=$(target/p10-mgr/debug/mgr)
echo "mgr found:"
echo "$found"
expected="morm-meta:spike_customer:id i64,name String
morm-meta:spike_order:created_at i64,updated_at i64,created_by i64,external_id String,id i64,tenant i64,code String,email String,deleted bool,note Option<String>
morm-meta:spike_widget:id i64,label String"
[ "$found" = "$expected" ] || fail "mgr didn't find exactly the three crates' models"

grep -qa "morm-meta:spike_product" target/p10-mgr/debug/app || fail "the control build has no metadata: the search is broken"
echo "proof 10 passed"
