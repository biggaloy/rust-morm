#!/usr/bin/env bash
# Starts a hot standby for proof 17 on 127.0.0.1:5433, streaming from the
# job's Postgres service, from the same postgres:NN image. The service gets a
# replication line in pg_hba.conf, and the standby cancels a conflicting
# query after 5 s of replay delay instead of the default 30 s.
# PG_CONTAINER is the service's container and PG_MAJOR its version.
set -euo pipefail

docker exec "${PG_CONTAINER:?}" bash -c 'echo "host replication all all trust" >> "$PGDATA/pg_hba.conf"'
psql "host=127.0.0.1 port=5432 user=postgres dbname=postgres" -c "SELECT pg_reload_conf()"
docker run -d --name spike-standby --network host --user postgres --entrypoint bash "postgres:${PG_MAJOR:?}" -c \
  'pg_basebackup -h 127.0.0.1 -p 5432 -U postgres -D /var/lib/postgresql/standby -R -X stream && exec postgres -D /var/lib/postgresql/standby -p 5433 -c listen_addresses=127.0.0.1 -c hot_standby=on -c max_standby_streaming_delay=5s'

# pg_isready doesn't wait for a server that isn't listening yet, so poll.
for _ in $(seq 120); do
  if pg_isready -q -h 127.0.0.1 -p 5433; then break; fi
  sleep 1
done
if ! pg_isready -h 127.0.0.1 -p 5433; then
  echo "::error::The standby didn't answer on port 5433."
  docker logs spike-standby
  exit 1
fi
psql "host=127.0.0.1 port=5433 user=postgres dbname=postgres" -Atc "SELECT 'in recovery: ' || pg_is_in_recovery() || ', version ' || version()"
