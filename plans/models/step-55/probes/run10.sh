#!/bin/bash
# run10.sh <bindir> <scratch> <socketdir>: the twenty-fourth review's FB1 on a throwaway cluster. A statement under a
# per-transaction search_path over two tables of different shapes, with no pending entry to run it unnamed:
#  1. straight to Postgres with pending entries off: each failure evicts (DEALLOCATE) and the next transaction prepares
#     it again under its own path, as rows B1 and the cache do; ten transactions alternating b, a, b, ... from a copy
#     prepared under a;
#  2. behind PgBouncer with pending entries off: the server statement stays (no evict reaches it), the same ten;
#  3. .retry(1)'s next attempt after a failure: the same statement run unnamed under b.
B=$1; S=$2; K=$3
export TMPDIR=$S
D=$S/pg10-$($B/postgres --version | awk '{print $3}')
rm -rf "$D"; mkdir -p "$K"
$B/initdb -D "$D" -A trust -U postgres >/dev/null
$B/pg_ctl -D "$D" -o "-k $K -c listen_addresses=''" -l "$D/log" -w start >/dev/null
P="/usr/bin/psql -X -h $K -U postgres -d postgres -v ON_ERROR_STOP=0"
$B/postgres --version
$P -q <<'SQL'
\set VERBOSITY verbose
CREATE SCHEMA a; CREATE SCHEMA b;
CREATE TABLE a.items (id int, x int); INSERT INTO a.items VALUES (1, 10);
CREATE TABLE b.items (id int, x int, extra text); INSERT INTO b.items VALUES (1, 20, 'e');
\echo == 1. evict after each failure, prepare again on a miss (straight to Postgres, entries off)
SET search_path = a;
PREPARE q(int) AS SELECT * FROM items WHERE id = $1;
\echo -- t1 b
BEGIN; SET LOCAL search_path = b; EXECUTE q(1); ROLLBACK; DEALLOCATE q;
\echo -- t2 a
BEGIN; SET LOCAL search_path = a; PREPARE q(int) AS SELECT * FROM items WHERE id = $1; EXECUTE q(1); COMMIT;
\echo -- t3 b
BEGIN; SET LOCAL search_path = b; EXECUTE q(1); ROLLBACK; DEALLOCATE q;
\echo -- t4 a
BEGIN; SET LOCAL search_path = a; PREPARE q(int) AS SELECT * FROM items WHERE id = $1; EXECUTE q(1); COMMIT;
\echo -- t5 b
BEGIN; SET LOCAL search_path = b; EXECUTE q(1); ROLLBACK; DEALLOCATE q;
\echo -- t6 a
BEGIN; SET LOCAL search_path = a; PREPARE q(int) AS SELECT * FROM items WHERE id = $1; EXECUTE q(1); COMMIT;
\echo -- t7 b
BEGIN; SET LOCAL search_path = b; EXECUTE q(1); ROLLBACK; DEALLOCATE q;
\echo -- t8 a
BEGIN; SET LOCAL search_path = a; PREPARE q(int) AS SELECT * FROM items WHERE id = $1; EXECUTE q(1); COMMIT;
\echo -- t9 b
BEGIN; SET LOCAL search_path = b; EXECUTE q(1); ROLLBACK; DEALLOCATE q;
\echo -- t10 a
BEGIN; SET LOCAL search_path = a; PREPARE q(int) AS SELECT * FROM items WHERE id = $1; EXECUTE q(1); COMMIT;
DEALLOCATE q;
\echo == 2. the server statement stays (behind PgBouncer, entries off)
SET search_path = a;
PREPARE s(int) AS SELECT * FROM items WHERE id = $1;
\echo -- t1 b
BEGIN; SET LOCAL search_path = b; EXECUTE s(1); ROLLBACK;
\echo -- t2 a
BEGIN; SET LOCAL search_path = a; EXECUTE s(1); COMMIT;
\echo -- t3 b
BEGIN; SET LOCAL search_path = b; EXECUTE s(1); ROLLBACK;
\echo -- t4 a
BEGIN; SET LOCAL search_path = a; EXECUTE s(1); COMMIT;
\echo -- t5 b
BEGIN; SET LOCAL search_path = b; EXECUTE s(1); ROLLBACK;
\echo -- t6 a
BEGIN; SET LOCAL search_path = a; EXECUTE s(1); COMMIT;
\echo -- t7 b
BEGIN; SET LOCAL search_path = b; EXECUTE s(1); ROLLBACK;
\echo -- t8 a
BEGIN; SET LOCAL search_path = a; EXECUTE s(1); COMMIT;
\echo -- t9 b
BEGIN; SET LOCAL search_path = b; EXECUTE s(1); ROLLBACK;
\echo -- t10 a
BEGIN; SET LOCAL search_path = a; EXECUTE s(1); COMMIT;
\echo == 3. .retry(1): the failed attempt, then the next attempt run unnamed
\echo -- attempt 1
BEGIN; SET LOCAL search_path = b; EXECUTE s(1); ROLLBACK;
\echo -- attempt 2, unnamed
BEGIN; SET LOCAL search_path = b;
SELECT * FROM items WHERE id = $1 \bind 1 \g
COMMIT;
SQL
$B/pg_ctl -D "$D" -w stop >/dev/null
rm -rf "$D"
