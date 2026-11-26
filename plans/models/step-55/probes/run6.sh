#!/bin/bash
# run6.sh <bindir> <name> <outdir>: a throwaway cluster, probe5.py and probe6.py, the cluster gone
S=$(cd "$(dirname "$0")/../../../../.." && pwd)   # .claude/scratch, the wip worktree being in it
P=$(cd "$(dirname "$0")" && pwd)
B=$1; N=$2; OUT=$3
export TMPDIR=$S LC_ALL=C.UTF-8
D=$S/data-$N; K=$HOME/.claude/scratch/rm6-$N
rm -rf "$D" "$K"; mkdir -p "$K"
"$B/initdb" -D "$D" -U pt -A trust --locale=C.UTF-8 -E UTF8 --no-sync > "$S/initdb-$N.log" 2>&1 || { tail -3 "$S/initdb-$N.log"; exit 1; }
"$B/pg_ctl" -D "$D" -w -l "$S/pg-$N.log" -o "-c listen_addresses='' -c unix_socket_directories='$K' -c fsync=off -c shared_buffers=16MB" start > /dev/null
PY=$(python3 -c 'import sys; print(sys.executable)')
SP=$(python3 -c 'import asyncpg, os; print(os.path.dirname(os.path.dirname(asyncpg.__file__)))')
run() { "$PY" -I -c "import sys; sys.path.append('$SP'); sys.argv = sys.argv[1:]; exec(compile(open(sys.argv[0]).read(), sys.argv[0], 'exec'))" "$@"; }
run "$P/probe6.py" "$B" "$D" "$K" "$S/msgs6-$N.tsv" > "$OUT/probe6-$N.txt" 2>&1
[ -n "$FIVE" ] && run "$P/probe5.py" "$B" "$D" "$K" "$S/msgs5-$N.tsv" > "$S/probe5-$N.txt" 2>&1
"$B/pg_ctl" -D "$D" -w -m immediate stop > /dev/null
rm -rf "$D" "$K"
