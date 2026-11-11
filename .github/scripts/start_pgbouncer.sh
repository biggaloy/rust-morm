#!/usr/bin/env bash
# Starts PgBouncer for the pooler job, on 127.0.0.1:6432, with
# .github/pgbouncer/pgbouncer.ini.
#
# It installs PgBouncer from the runner's apt sources, which reach Ubuntu's
# archive through azure.archive.ubuntu.com. On 2026-11-11 that name, and
# archive.ubuntu.com and security.ubuntu.com behind it in the runner's mirror
# list, failed to resolve for hours. So a failed install is tried once more,
# and then PgBouncer runs from CloudNativePG's image on ghcr.io instead, pinned
# by digest, with the same pgbouncer.ini. If that fails too, the step fails:
# the pooler tests never run without PgBouncer. Each apt command is cut off
# after 5 minutes, so a mirror that hangs counts as a failure.
#
# RETRY_DELAY exists for tests/tests/repo_policy.rs.
# covers: R-0792
set -euo pipefail

ini="$PWD/.github/pgbouncer/pgbouncer.ini"
image=ghcr.io/cloudnative-pg/pgbouncer:1.26.1-202610260932-trixie@sha256:7e21afe7cfd4f77fde5bcf80237b99538ac9c35550faae6b1ff8cf744188dd7c
retry_delay=${RETRY_DELAY:-10}

install() {
  sudo timeout 300 apt-get update && sudo timeout 300 apt-get install -y pgbouncer
}

if install; then
  installed=yes
elif sleep "$retry_delay" && install; then
  installed=yes
else
  installed=no
fi

if [ "$installed" = yes ]; then
  sudo systemctl stop pgbouncer
  pgbouncer -d "$ini"
  echo "Started PgBouncer from the runner's apt sources."
else
  echo "::warning::Installing PgBouncer from the runner's apt sources failed twice. Running it from $image instead."
  if docker run -d --name morm-pgbouncer --network host -v "$ini:/etc/pgbouncer/pgbouncer.ini:ro" "$image"; then
    echo "Started PgBouncer from $image."
  else
    echo "::error::PgBouncer could be installed neither from the runner's apt sources nor from $image."
    exit 1
  fi
fi
pg_isready -h 127.0.0.1 -p 6432 -t 30
