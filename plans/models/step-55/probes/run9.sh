#!/bin/bash
# run9.sh <bindir> <scratch> <socketdir>: FA3's savepoint case on a throwaway cluster. A savepoint that tries a bad
# label gets 22P02 from enum_in; rolled back to the savepoint, the transaction goes on and commits (what .retry(1)'s
# next attempt, run unnamed, meets); and a statement prepared before the bad label, run in the savepoint, gets the same
# 22P02, which is what morm's second row takes for a stale statement's and answers with Error::SchemaChanged.
B=$1; S=$2; K=$3
export TMPDIR=$S
D=$S/pg9-$($B/postgres --version | awk '{print $3}')
rm -rf "$D"; mkdir -p "$K"
$B/initdb -D "$D" -A trust -U postgres >/dev/null
$B/pg_ctl -D "$D" -o "-k $K -c listen_addresses=''" -l "$D/log" -w start >/dev/null
P="$B/psql -X -h $K -U postgres -d postgres -v ON_ERROR_STOP=0 -v VERBOSITY=verbose"
$P -q <<'SQL'
\set VERBOSITY verbose
CREATE TYPE st AS ENUM ('a', 'b');
CREATE TABLE t (id int, c st);
PREPARE q(text) AS SELECT (($1::text)::st)::text;
BEGIN;
INSERT INTO t VALUES (1, 'a');
SAVEPOINT s1;
EXECUTE q('zz');
ROLLBACK TO SAVEPOINT s1;
SAVEPOINT s2;
SELECT (('zz'::text)::st)::text;
ROLLBACK TO SAVEPOINT s2;
INSERT INTO t VALUES (2, 'b');
COMMIT;
SELECT id, c FROM t ORDER BY id;
SQL
$B/pg_ctl -D "$D" -w stop >/dev/null
rm -rf "$D"
