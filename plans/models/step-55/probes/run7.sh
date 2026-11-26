#!/bin/bash
# run7.sh <bindir> <name> <outdir>: a throwaway cluster, probe7.py, the cluster gone
S="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)/.claude/scratch"; mkdir -p "$S"   # the worktree's .claude/scratch
P=$(cd "$(dirname "$0")" && pwd)
B=$1; N=$2; OUT=$3
export TMPDIR=$S LC_ALL=C.UTF-8
D=$S/data7-$N; K=$HOME/.claude/scratch/rm7-$N
rm -rf "$D" "$K"; mkdir -p "$K"
"$B/initdb" -D "$D" -U pt -A trust --locale=C.UTF-8 -E UTF8 --no-sync > "$S/initdb7-$N.log" 2>&1 || { tail -3 "$S/initdb7-$N.log"; exit 1; }
"$B/pg_ctl" -D "$D" -w -l "$S/pg7-$N.log" -o "-c listen_addresses='' -c unix_socket_directories='$K' -c fsync=off -c shared_buffers=16MB" start > /dev/null
PY=$(python3 -c 'import sys; print(sys.executable)')
SP=$(python3 -c 'import asyncpg, os; print(os.path.dirname(os.path.dirname(asyncpg.__file__)))')
"$PY" -I -c "import sys; sys.path.append('$SP'); sys.argv = sys.argv[1:]; exec(compile(open(sys.argv[0]).read(), sys.argv[0], 'exec'))" "$P/probe7.py" "$D" "$K" > "$OUT/probe7-$N.txt" 2>&1
"$B/pg_ctl" -D "$D" -w -m immediate stop > /dev/null
rm -rf "$D" "$K"
