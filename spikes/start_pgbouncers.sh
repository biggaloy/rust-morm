#!/usr/bin/env bash
# Starts two PgBouncers for proof 16, in transaction mode in front of the
# job's Postgres on 127.0.0.1:5432: Ubuntu's apt package (1.22) on port 6432,
# and CloudNativePG's 1.26.1 image, pinned by digest as ci.yml's pooler job
# pins it, on port 6433. Each has one server connection, so every client
# reaches the same server session, and keeps prepared statements, as morm
# runs PgBouncer 1.22 and later. Any failure fails the step.
set -euo pipefail

image=ghcr.io/cloudnative-pg/pgbouncer:1.26.1-202610260932-trixie@sha256:7e21afe7cfd4f77fde5bcf80237b99538ac9c35550faae6b1ff8cf744188dd7c
dir="${RUNNER_TEMP:?}/spike-pgbouncer"
mkdir -p "$dir"

write_ini() {
  cat > "$1" <<EOF
[databases]
postgres = host=127.0.0.1 port=5432 user=postgres dbname=postgres

[pgbouncer]
listen_addr = 127.0.0.1
listen_port = $2
unix_socket_dir =
auth_type = any
admin_users = postgres
pool_mode = transaction
default_pool_size = 1
max_prepared_statements = 200
$3
EOF
}

write_ini "$dir/apt.ini" 6432 "logfile = $dir/apt.log
pidfile = $dir/apt.pid"
write_ini "$dir/image.ini" 6433 ""

sudo timeout 300 apt-get update
sudo timeout 300 apt-get install -y pgbouncer
sudo systemctl stop pgbouncer
pgbouncer -d "$dir/apt.ini"
docker run -d --name spike-pgbouncer --network host -v "$dir/image.ini:/etc/pgbouncer/pgbouncer.ini:ro" "$image"

for port in 6432 6433; do
  for _ in $(seq 30); do
    if pg_isready -q -h 127.0.0.1 -p "$port"; then break; fi
    sleep 1
  done
  if ! pg_isready -h 127.0.0.1 -p "$port"; then
    echo "::error::PgBouncer on port $port didn't answer."
    docker logs spike-pgbouncer
    exit 1
  fi
  psql "host=127.0.0.1 port=$port user=postgres dbname=pgbouncer" -c "SHOW VERSION"
done
