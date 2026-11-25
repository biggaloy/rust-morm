"""Throwaway Postgres probe for the EZ round: python3 probe.py <bindir> <datadir> <sockdir>"""
import asyncio, os, subprocess, sys
import asyncpg

BIN, DATA, SOCK = sys.argv[1:4]
ENV = dict(os.environ, LC_ALL="C.UTF-8")


def sh(*a):
    subprocess.run(a, check=True, env=ENV, stdout=subprocess.DEVNULL)


async def con():
    return await asyncpg.connect(host=SOCK, user="pt", database="postgres")


async def go(label, coro):
    try:
        r = await coro
        print(f"   {label}: ok {r!r}")
        return r
    except asyncpg.PostgresError as e:
        print(f"   {label}: {e.sqlstate} {getattr(e, 'server_source_function', '')} {e}")


async def reset(a):
    await a.execute("DROP SCHEMA public CASCADE; CREATE SCHEMA public")


RESET = "CREATE OPERATOR public.### (LEFTARG = int4, RIGHTARG = int4, FUNCTION = int4eq); DROP OPERATOR public.### (int4, int4); "
SWAP = "BEGIN; ALTER TYPE st RENAME TO _morm_retired_m2_st; ALTER TYPE _morm_retired_m2_st RENAME VALUE 'a' TO '_morm_retired_a'; ALTER TYPE _morm_retired_m2_st RENAME VALUE 'b' TO '_morm_retired_b'; CREATE TYPE st AS ENUM ('a','b','c'); " + RESET + "COMMIT"
BYHAND = "BEGIN; ALTER TYPE st_old RENAME TO _morm_retired_byhand_st; ALTER TYPE _morm_retired_byhand_st RENAME VALUE 'a' TO '_morm_retired_a'; ALTER TYPE _morm_retired_byhand_st RENAME VALUE 'b' TO '_morm_retired_b'; "


async def main():
    a0 = await con()
    print(await a0.fetchval("show server_version"))

    print("1. a relabelled type accepts its own relabelled label (third silent case)")
    await reset(a0)
    await a0.execute("CREATE TYPE st AS ENUM ('a','b'); CREATE FUNCTION f(t text) RETURNS text LANGUAGE plpgsql AS $$DECLARE v st; BEGIN v := t; RETURN v::text; END$$")
    a = await con()
    s = await a.prepare("SELECT f($1::text)", name="p1")
    await go("A, compiled before, 'a'", s.fetchval("a"))
    await a0.execute(SWAP)
    b = await con()
    for v in ("a", "c", "_morm_retired_x", "_morm_retired_a"):
        await go(f"A, stale, {v!r}", a.fetchval("SELECT f($1::text)", v))
        await go(f"B, new session, {v!r}", b.fetchval("SELECT f($1::text)", v))
    await a.close(); await b.close()

    print("2. one ALTER leaves a cached statement 0A000 and its fresh run 42804")
    await reset(a0)
    await a0.execute("CREATE TABLE t (id int, c int); INSERT INTO t VALUES (1, 1); CREATE FUNCTION g(i int) RETURNS text LANGUAGE plpgsql AS $$DECLARE r t%ROWTYPE; BEGIN SELECT * INTO r FROM t WHERE id = i; IF r.c > 0 THEN RETURN 'x'; END IF; RETURN 'y'; END$$")
    a = await con()
    s = await a.prepare("SELECT c, g(id) FROM t WHERE id = $1::int4", name="p2")
    await go("A, cached", s.fetchrow(1))
    await a0.execute("ALTER TABLE t ALTER c TYPE bigint")
    await go("A, cached copy again", s.fetchrow(1))
    s2 = await a.prepare("SELECT c, g(id) FROM t WHERE id = $1::int4 -- fresh", name="p3")
    await go("A, fresh run", s2.fetchrow(1))
    await go("A, fresh run again", a.fetchrow("SELECT c, g(id) FROM t WHERE id = $1::int4", 1))
    b = await con()
    await go("B, new session", b.fetchrow("SELECT c, g(id) FROM t WHERE id = $1::int4", 1))
    await a.close(); await b.close()

    print("3. a cached write that casts a parameter, with a trigger compiled before a cutover")
    await reset(a0)
    await a0.execute("CREATE TYPE st AS ENUM ('a','b'); CREATE TABLE t (n text, s text); CREATE FUNCTION tr() RETURNS trigger LANGUAGE plpgsql AS $$DECLARE v st; BEGIN v := NEW.n; RETURN NEW; END$$; CREATE TRIGGER x BEFORE INSERT ON t FOR EACH ROW EXECUTE FUNCTION tr()")
    a = await con()
    q = "INSERT INTO t(n, s) VALUES ($1::text, ((($2::text)::st)::text))"
    s = await a.prepare(q, name="q1")
    await go("A, cached", s.fetch("a", "a"))
    await a0.execute(SWAP)
    await go("A, cached copy again", s.fetch("a", "a"))
    await go("A, fresh run", a.fetch(q + " -- fresh", "a", "a"))
    b = await con()
    await go("B, new session", b.fetch(q, "a", "a"))
    await a.close(); await b.close()

    print("4. a relabel by hand after a kept swap, no reset: the cached cast")
    await reset(a0)
    await a0.execute("CREATE TYPE st AS ENUM ('a','b')")
    a = await con()
    q = "SELECT (($1::text)::st)::text"
    s = await a.prepare(q, name="p5")
    await go("A, cached", s.fetchval("a"))
    await a0.execute("ALTER TYPE st RENAME TO st_old; CREATE TYPE st AS ENUM ('a','b','c')")
    await go("A, cached, after the kept swap, 'c'", s.fetchval("c"))
    await go("A, cached, after the kept swap, 'a'", s.fetchval("a"))
    await a0.execute("ALTER TYPE st_old RENAME TO _morm_retired_byhand_st; ALTER TYPE _morm_retired_byhand_st RENAME VALUE 'a' TO '_morm_retired_a'; ALTER TYPE _morm_retired_byhand_st RENAME VALUE 'b' TO '_morm_retired_b'")
    await go("A, cached, after the relabel, 'c'", s.fetchval("c"))
    await go("A, fresh, 'c'", a.fetchval(q + " -- fresh", "c"))

    print("5. a new label that's the type's own name")
    await a0.execute("ALTER TYPE st RENAME TO st_old; CREATE TYPE st AS ENUM ('a','b','c','st')")
    s = None
    a2 = await con()
    s = await a2.prepare(q, name="p6")
    await go("A, cached on the new type, 'st'", s.fetchval("st"))
    await a0.execute("ALTER TYPE st RENAME TO st_old2; CREATE TYPE st AS ENUM ('a','b','c','st2','in st')")
    await go("A, cached, kept type, 'in st'", s.fetchval("in st"))
    await a.close(); await a2.close()

    print("6. the parameter test: a stale function, and a parameter holding _morm_retired_ that it doesn't convert")
    await reset(a0)
    await a0.execute("CREATE TYPE st AS ENUM ('a','b'); CREATE FUNCTION h(note text) RETURNS text LANGUAGE plpgsql AS $$DECLARE v st; BEGIN v := 'a'; RETURN note || v::text; END$$")
    a = await con()
    s = await a.prepare("SELECT h($1::text)", name="p7")
    await go("A, compiled before", s.fetchval("hello"))
    await a0.execute(SWAP)
    await go("A, stale, note '_morm_retired_x'", s.fetchval("_morm_retired_x"))
    b = await con()
    await go("B, new session, the same", b.fetchval("SELECT h($1::text)", "_morm_retired_x"))
    await a.close(); await b.close()

    print("7. a column still on the kept type, R-1294's condition")
    await reset(a0)
    await a0.execute("CREATE TYPE st AS ENUM ('a','b'); CREATE TABLE t (c st DEFAULT 'a' CHECK (c <> 'b')); INSERT INTO t VALUES ('a'); CREATE VIEW vw AS SELECT c FROM t WHERE c = 'a'; CREATE FUNCTION f(t text) RETURNS text LANGUAGE plpgsql AS $$DECLARE v st; BEGIN v := t; RETURN v::text; END$$")
    a = await con()
    await go("A, compiled before", a.fetchval("SELECT f('a')"))
    await a0.execute("ALTER TYPE st RENAME TO st_old; CREATE TYPE st AS ENUM ('a','b','c')")
    await go("A, stale function, 'c'", a.fetchval("SELECT f('c')"))
    await go("column before the rename", a0.fetchval("SELECT c::text FROM t"))
    await a0.execute("ALTER TYPE st_old RENAME TO _morm_retired_byhand_st; ALTER TYPE _morm_retired_byhand_st RENAME VALUE 'a' TO '_morm_retired_a'; ALTER TYPE _morm_retired_byhand_st RENAME VALUE 'b' TO '_morm_retired_b'")
    await go("A, stale function, 'c', after the rename", a.fetchval("SELECT f('c')"))
    b = await con()
    await go("column after the rename, new session", b.fetchval("SELECT c::text FROM t"))
    await go("WHERE c = 'a', new session", b.fetch("SELECT 1 FROM t WHERE c = 'a'"))
    await go("INSERT 'a', new session", b.execute("INSERT INTO t VALUES ('a')"))
    await go("the view, new session", b.fetchval("SELECT c::text FROM vw"))
    await go("the default, new session", b.fetchval("INSERT INTO t DEFAULT VALUES RETURNING c::text"))
    await go("SQL that names st_old", b.fetchval("SELECT 'a'::st_old"))
    await a.close(); await b.close()

    print("8. EZ2's held case, and the XX000 that repeats")
    await reset(a0)
    await a0.execute("CREATE TYPE st AS ENUM ('a','b'); CREATE FUNCTION f(t text) RETURNS text LANGUAGE plpgsql AS $$DECLARE v st; BEGIN v := t; RETURN v::text; END$$")
    a = await con()
    q = "SELECT (($1::text)::st)::text"
    s = await a.prepare(q, name="p8")
    await s.fetchval("a")
    await a.fetchval("SELECT f($1::text)", "a")
    await a0.execute("ALTER TYPE st RENAME TO st_old; CREATE TYPE st AS ENUM ('a','b','c')")
    await go("function, 'zzz', a label no type has", a.fetchval("SELECT f($1::text)", "zzz"))
    await a.execute("BEGIN")
    await go("cast cached before, 'c', in BEGIN", s.fetchval("c"))
    await go("a fresh prepare there", a.prepare(q + " -- fresh", name="p9"))
    await a.execute("ROLLBACK")
    await go("prepared again after ROLLBACK", a.fetchval(q + " -- again", "c"))
    await a0.execute("DROP TYPE st_old CASCADE")
    for i in range(3):
        await a.execute("BEGIN")
        await go(f"function in BEGIN, {i}", a.fetchval("SELECT f($1::text)", "a"))
        await a.execute("ROLLBACK")
    await go("function outside", a.fetchval("SELECT f($1::text)", "a"))
    await a.close()
    for label, tail in (("no reset", "COMMIT"), ("with the reset", RESET + "COMMIT")):
        print(f"9. R-1294's rename by hand, {label}: constants, and a label both types share")
        await reset(a0)
        await a0.execute("CREATE TYPE st AS ENUM ('a','b'); CREATE FUNCTION k() RETURNS text LANGUAGE plpgsql AS $$DECLARE v st; BEGIN v := 'a'; RETURN v::text; END$$; CREATE FUNCTION f(t text) RETURNS text LANGUAGE plpgsql AS $$DECLARE v st; BEGIN v := t; RETURN v::text; END$$")
        a = await con()
        qc = "SELECT ('a'::st < 'b'::st)::text || ('a'::st)::text"
        qp = "SELECT (($1::text)::st)::text"
        c = await a.prepare(qc, name="c1")
        s = await a.prepare(qp, name="c2")
        for _ in range(7):
            await c.fetchval(); await s.fetchval("a"); await a.fetchval("SELECT k()"); await a.fetchval("SELECT f($1::text)", "a")
        await a0.execute("ALTER TYPE st RENAME TO st_old; CREATE TYPE st AS ENUM ('b','a','c')")
        await go("after the kept swap: constant function", a.fetchval("SELECT k()"))
        await go("after the kept swap: cached constant statement", c.fetchval())
        await a0.execute(BYHAND + tail)
        await go("constant function", a.fetchval("SELECT k()"))
        await go("function, 'a'", a.fetchval("SELECT f($1::text)", "a"))
        await go("function, 'c'", a.fetchval("SELECT f($1::text)", "c"))
        await go("cached constant statement", c.fetchval())
        await go("cached cast, 'a'", s.fetchval("a"))
        b = await con()
        await go("new session: constant function", b.fetchval("SELECT k()"))
        await go("new session: constant statement", b.fetchval(qc))
        await a.close(); await b.close()
    await a0.close()


os.makedirs(SOCK, exist_ok=True)
sh(f"{BIN}/initdb", "-D", DATA, "-U", "pt", "-A", "trust", "--no-sync")
sh(f"{BIN}/pg_ctl", "-D", DATA, "-w", "-l", DATA + ".log", "-o", f"-c listen_addresses='' -c unix_socket_directories='{SOCK}' -c fsync=off", "start")
try:
    asyncio.run(main())
finally:
    sh(f"{BIN}/pg_ctl", "-D", DATA, "-m", "immediate", "stop")
