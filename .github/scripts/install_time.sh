#!/usr/bin/env bash
# Installs GNU time at /usr/bin/time, for the bench job's RSS measurement.
#
# It installs the `time` package from the runner's apt sources, which reach
# Ubuntu's archive through azure.archive.ubuntu.com. On 2026-11-11 that name,
# and archive.ubuntu.com and security.ubuntu.com behind it in the runner's
# mirror list, failed to resolve for hours, as the pooler job saw. So a failed
# install is tried once more, and then GNU time comes from Debian's `time`
# 1.9-0.2 package, the same upstream release and Debian revision Ubuntu 24.04
# ships, downloaded from deb.debian.org or, failing that, snapshot.debian.org,
# and checked against its pinned SHA-256 from Debian's package index. If every
# source fails, the step fails: bench never runs without its RSS measurement.
# Each apt command is cut off after 5 minutes, and each download after 2, so a
# source that hangs counts as a failure.
#
# RETRY_DELAY exists for tests/tests/repo_policy.rs.
# covers: R-0794
set -euo pipefail

deb_sha256=10257c6fe888e6578550b1f20705d282dd630b76f8df0c006aec3031fefe5e23
urls=(
  https://deb.debian.org/debian/pool/main/t/time/time_1.9-0.2_amd64.deb
  https://snapshot.debian.org/file/4e02f73cb176bd142f766bb58c3286a5ad35cfc6
)
retry_delay=${RETRY_DELAY:-10}

from_apt() {
  sudo timeout 300 apt-get update && sudo timeout 300 apt-get install -y time
}

# Downloads the pinned package from $1, checks it and installs its binary.
from_debian() {
  local deb="$work/time.deb"
  rm -rf "$deb" "$work/root"
  curl -fsSL --max-time 120 -o "$deb" "$1" &&
    echo "$deb_sha256  $deb" | sha256sum -c - &&
    dpkg-deb -x "$deb" "$work/root" &&
    sudo install -m 0755 "$work/root/usr/bin/time" /usr/bin/time
}

if from_apt; then
  echo "Installed GNU time from the runner's apt sources."
elif sleep "$retry_delay" && from_apt; then
  echo "Installed GNU time from the runner's apt sources."
else
  echo "::warning::Installing GNU time from the runner's apt sources failed twice. Downloading Debian's package instead, pinned by SHA-256."
  work=$(mktemp -d)
  installed=no
  for url in "${urls[@]}"; do
    if from_debian "$url"; then
      installed=yes
      echo "Installed GNU time from $url."
      break
    fi
    echo "::warning::$url gave no package matching the pinned SHA-256."
  done
  if [ "$installed" = no ]; then
    echo "::error::GNU time could be installed neither from the runner's apt sources nor from ${urls[*]}."
    exit 1
  fi
fi
