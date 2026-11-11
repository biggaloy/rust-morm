#!/usr/bin/env bash
# Installs PgBouncer for the pooler job, from the runner's apt sources.
#
# Those sources reach Ubuntu's archive through the Azure mirror,
# azure.archive.ubuntu.com, directly or through mirror+file:/etc/apt/apt-mirrors.txt,
# and that name has failed to resolve for hours at a time. So a failed install
# is tried once more, then every source that goes through the mirror is pointed
# at archive.ubuntu.com and the install runs from there. If that fails too, the
# step fails: the pooler tests never run without PgBouncer.
#
# APT_DIR and RETRY_DELAY exist for tests/tests/repo_policy.rs.
# covers: R-0792
set -euo pipefail

apt_dir=${APT_DIR:-/etc/apt}
retry_delay=${RETRY_DELAY:-10}
mirror_list=mirror+file:/etc/apt/apt-mirrors.txt
fallback=http://archive.ubuntu.com/ubuntu/

install() {
  apt-get update && apt-get install -y pgbouncer
}

if install; then
  echo "Installed PgBouncer from the runner's apt sources."
elif sleep "$retry_delay" && install; then
  echo "Installed PgBouncer from the runner's apt sources, on the second try."
else
  mapfile -t sources < <(grep -rlE "azure\.archive\.ubuntu\.com|mirror\+file:/etc/apt/apt-mirrors\.txt" "$apt_dir")
  if [ "${#sources[@]}" -eq 0 ]; then
    echo "::error::Installing PgBouncer failed twice, and no apt source under $apt_dir goes through the Azure mirror, so there is nothing to switch."
    exit 1
  fi
  echo "::warning::Installing PgBouncer failed twice. Pointing ${sources[*]} at $fallback and trying again."
  sed -i -e "s|$mirror_list|$fallback|g" -e "s|http://azure\.archive\.ubuntu\.com/ubuntu/\?|$fallback|g" "${sources[@]}"
  if install; then
    echo "Installed PgBouncer from $fallback."
  else
    echo "::error::Installing PgBouncer failed from the runner's apt sources twice and from $fallback."
    exit 1
  fi
fi
