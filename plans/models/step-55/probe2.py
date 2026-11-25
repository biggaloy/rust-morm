"""Throwaway Postgres probe for the EZ-answers round: python3 probe2.py <bindir> <datadir> <sockdir>"""
import asyncio, hashlib, os, subprocess, sys
import asyncpg

BIN, DATA, SOCK = sys.argv[1:4]
ENV = dict(os.environ, LC_ALL="C.UTF-8")
RND = "7c1e9a04b52fd386"
RESET = "CREATE OPERATOR public.### (LEFTARG = int4, RIGHTARG = int4, FUNCTION = int4eq); DROP OPERATOR public.### (int4, int4); "


def sh(*a):
    subprocess.run(a, check=True, env=ENV, stdout=subprocess.DEVNULL)


async def con(**k):
    return await asyncpg.connect(host=SOCK, user="pt", database="postgres", **k)


async def go(label, coro):
    try:
        r = await coro
        print(f"   {label}: ok {r!r}")
        return r
    except asyncpg.PostgresError as e:
        print(f"   {label}: {e.sqlstate} {getattr(e, 'server_source_function', '')} {e}")


async def reset(a):
    await a.execute("DROP SCHEMA IF EXISTS old CASCADE; DROP SCHEMA public CASCADE; CREATE SCHEMA public")


async def main():
    a0 = await con()
    print(await a0.fetchval("show server_version"))

    print("1. the cutover's labels with a random part")
    await reset(a0)
    await a0.execute("CREATE TYPE st AS ENUM ('a','b'); CREATE TABLE t (id int, c st); INSERT INTO t VALUES (1,'a'),(2,'b'); CREATE FUNCTION f(t text) RETURNS text LANGUAGE plpgsql AS $$DECLARE v st; BEGIN v := t; RETURN v::text; END$$; CREATE FUNCTION k() RETURNS text LANGUAGE plpgsql AS $$DECLARE v st; BEGIN v := 'a'; RETURN v::text; END$$; CREATE FUNCTION less(x text, y text) RETURNS bool LANGUAGE plpgsql AS $$DECLARE v st; w st; BEGIN v := x; w := y; RETURN v < w; END$$")
    a = await con()
    await go("A, compiled before: f('a')", a.fetchval("SELECT f($1::text)", "a"))
    await go("A, compiled before: k()", a.fetchval("SELECT k()"))
    await go("A, compiled before: less", a.fetchval("SELECT less('a','b')"))
    await a0.execute(f"BEGIN; ALTER TABLE t RENAME c TO _morm_retired_m2_c; ALTER TYPE st RENAME TO _morm_retired_m2_st; ALTER TYPE _morm_retired_m2_st RENAME VALUE 'a' TO '_morm_retired_{RND}_a'; ALTER TYPE _morm_retired_m2_st RENAME VALUE 'b' TO '_morm_retired_{RND}_b'; CREATE TYPE st AS ENUM ('b','a','c'); {RESET} COMMIT")
    b = await con()
    for v in ("a", "c", "_morm_retired_a", f"_morm_retired_{RND}_a"):
        await go(f"A, stale, f({v!r})", a.fetchval("SELECT f($1::text)", v))
        await go(f"B, new session, f({v!r})", b.fetchval("SELECT f($1::text)", v))
    await go("A, stale, k(), a constant", a.fetchval("SELECT k()"))
    await go("A, stale, less('a','b')", a.fetchval("SELECT less('a','b')"))
    await go("the retired column", b.fetchval("SELECT string_agg(_morm_retired_m2_c::text, ',' ORDER BY id) FROM t"))
    await go("A, stale, f of the retired column's own text", a.fetchval("SELECT f(_morm_retired_m2_c::text) FROM t WHERE id = 1"))
    await b.execute("SET ROLE pg_monitor")
    await go("any role reads pg_enum", b.fetchval("SELECT string_agg(enumlabel, ',' ORDER BY enumsortorder) FROM pg_enum WHERE enumtypid = '_morm_retired_m2_st'::regtype"))
    await b.execute("RESET ROLE")
    await go("down maps each label by the recorded part", b.fetchval(f"SELECT string_agg(CASE _morm_retired_m2_c::text WHEN '_morm_retired_{RND}_a' THEN 'a' WHEN '_morm_retired_{RND}_b' THEN 'b' END, ',' ORDER BY id) FROM t"))
    await a.close()
    await b.close()

    print("2. 63 bytes")
    await reset(a0)
    pre = f"_morm_retired_{RND}_"
    print("   the prefix takes", len(pre), "bytes, leaving", 63 - len(pre))
    for n in (32, 33, 63):
        lab = "x" * n
        await a0.execute(f"DROP TYPE IF EXISTS q; CREATE TYPE q AS ENUM ('{lab}')")
        await go(f"a {n}-byte label renamed with the whole prefix, {len(pre) + n} bytes", a0.execute(f"ALTER TYPE q RENAME VALUE '{lab}' TO '{pre}{lab}'"))
    l1, l2 = "y" * 40 + "1" * 23, "y" * 40 + "2" * 23

    def short(x):
        if len(x.encode()) <= 63:
            return x
        h = hashlib.sha256(x.encode()).hexdigest()[:8]
        return x.encode()[:63 - 9].decode() + "_" + h

    await a0.execute(f"DROP TYPE IF EXISTS q; CREATE TYPE q AS ENUM ('{l1}','{l2}')")
    for l in (l1, l2):
        await go(f"a 63-byte label through the name function -> {short(pre + l)!r}", a0.execute(f"ALTER TYPE q RENAME VALUE '{l}' TO '{short(pre + l)}'"))
    await go("both", a0.fetchval("SELECT count(DISTINCT enumlabel) FROM pg_enum WHERE enumtypid = 'q'::regtype"))
    await go("a 64-byte label in CREATE TYPE", a0.execute(f"CREATE TYPE q2 AS ENUM ('{'z' * 64}')"))
    await go("two labels renamed to one", a0.execute(f"ALTER TYPE q RENAME VALUE '{short(pre + l1)}' TO '{short(pre + l2)}'"))

    print("3. how the message prints the type's name and the value")
    cases = [("st", "st_old", "c"), ("st", "st old", "c"), ("st", "my st old", "c"), ("st", "St", "c"), ("st", 'st"x', "c"), ("value", "value_old", "c"), ("enum", "enum_old", "c"), ("input", "input_old", "c"), ("for", "for_old", "c"), ("名", "名old", "c")]
    for name, kept, v in cases:
        await reset(a0)
        q = '"' + name.replace('"', '""') + '"'
        k = '"' + kept.replace('"', '""') + '"'
        await a0.execute(f"CREATE TYPE {q} AS ENUM ('a')")
        a = await con()
        s = await a.prepare(f"SELECT (($1::text)::{q})::text", name="p")
        await s.fetchval("a")
        await a0.execute(f"ALTER TYPE {q} RENAME TO {k}; CREATE TYPE {q} AS ENUM ('a','c')")
        await go(f"type {name!r}, kept as {kept!r}, cached cast of 'c'", s.fetchval("c"))
        await a.close()
    await reset(a0)
    await a0.execute("CREATE SCHEMA old; CREATE TYPE st AS ENUM ('a')")
    a = await con()
    s = await a.prepare("SELECT (($1::text)::st)::text", name="p")
    await s.fetchval("a")
    await a0.execute("ALTER TYPE st SET SCHEMA old; CREATE TYPE st AS ENUM ('a','c')")
    await go("kept by SET SCHEMA old, cached cast of 'c'", s.fetchval("c"))
    await a.close()
    await reset(a0)
    await a0.execute("CREATE SCHEMA old; CREATE TYPE old.st AS ENUM ('a'); CREATE TYPE st AS ENUM ('a')")
    await go("a bad label, the type in a schema off the search_path", a0.fetchval("SELECT ($1::text::old.st)::text", "zzz"))

    print("4. the value as the input function got it: scalar, list, composite, domain")
    await reset(a0)
    await a0.execute("CREATE TYPE st AS ENUM ('a'); CREATE TYPE cmp AS (n int, s st, l st[]); CREATE DOMAIN dst AS st")
    for v in ("in st", 'x "st"', "a\\b, (st)", "", 'st: "bad', "»st«", "é st"):
        await go(f"scalar {v!r}", a0.fetchval("SELECT (($1::text)::st)::text", v))
        await go(f"list ['a', {v!r}]", a0.fetchval("SELECT (($1::text[])::st[])::text", ["a", v]))
        esc = '"' + v.replace("\\", "\\\\").replace('"', '""') + '"'
        await go(f"composite field {v!r}, bound as (1,{esc},)", a0.fetchval("SELECT (($1::text)::cmp)::text", f"(1,{esc},)"))
        arr = "{" + '"' + v.replace("\\", "\\\\").replace('"', '\\"') + '"' + "}"
        esc2 = '"' + arr.replace("\\", "\\\\").replace('"', '""') + '"'
        await go(f"list in a composite {v!r}", a0.fetchval("SELECT (($1::text)::cmp)::text", f"(1,a,{esc2})"))
        await go(f"domain {v!r}", a0.fetchval("SELECT (($1::text)::dst)::text", v))

    print("5. a cached cast of stored text, the value not bound by the call")
    await reset(a0)
    await a0.execute("CREATE TYPE st AS ENUM ('a'); CREATE TABLE s (id int, c text); INSERT INTO s VALUES (1, 'a')")
    a = await con()
    s = await a.prepare("SELECT (c::st)::text, (($2::text)::st)::text FROM s WHERE id = $1::int4", name="p")
    await go("cached on st", s.fetchrow(1, "a"))
    await a0.execute("ALTER TYPE st RENAME TO kept; CREATE TYPE st AS ENUM ('a','in st'); UPDATE s SET c = 'in st'")
    await go("cached, kept type, stored 'in st'", s.fetchrow(1, "a"))
    s2 = await a.prepare("SELECT (c::st)::text, (($2::text)::st)::text FROM s WHERE id = $1::int4 -- fresh", name="p2")
    await go("fresh", s2.fetchrow(1, "a"))
    await a.close()

    print("6. real messages in other languages")
    await reset(a0)
    await a0.execute("CREATE TYPE st AS ENUM ('a')")
    for lc in ("C", "de_DE.utf8", "es_ES.utf8", "fr_FR.utf8", "it_IT.utf8", "ja_JP.utf8", "ka_GE.utf8", "ko_KR.utf8", "pl_PL.utf8", "ru_RU.utf8", "sv_SE.utf8", "tr_TR.utf8", "uk_UA.utf8"):
        try:
            await a0.execute(f"SET lc_messages = '{lc}'")
        except asyncpg.PostgresError as e:
            print("   ", lc, e)
            continue
        await go(lc, a0.fetchval("SELECT (($1::text)::st)::text", "in st"))
    await a0.execute("RESET lc_messages")

    print("7. a kept type renamed by hand with a random part, with the reset and without, a cast cached before")
    for rs in (RESET, ""):
        await reset(a0)
        await a0.execute("CREATE TYPE st AS ENUM ('a','b'); CREATE FUNCTION f(t text) RETURNS text LANGUAGE plpgsql AS $$DECLARE v st; BEGIN v := t; RETURN v::text; END$$")
        a = await con()
        s = await a.prepare("SELECT (($1::text)::st)::text", name="p")
        await s.fetchval("a")
        await a.fetchval("SELECT f('a')")
        await a0.execute("ALTER TYPE st RENAME TO st_old; CREATE TYPE st AS ENUM ('a','b','c')")
        await a0.execute(f"BEGIN; ALTER TYPE st_old RENAME TO _morm_retired_byhand_st; ALTER TYPE _morm_retired_byhand_st RENAME VALUE 'a' TO '_morm_retired_{RND}_a'; ALTER TYPE _morm_retired_byhand_st RENAME VALUE 'b' TO '_morm_retired_{RND}_b'; {rs} COMMIT")
        print("  ", "with the reset" if rs else "without the reset")
        await go("the cached cast, 'c'", s.fetchval("c"))
        await go("the cached cast, 'a'", s.fetchval("a"))
        await go("the function, 'a'", a.fetchval("SELECT f($1::text)", "a"))
        await go("the function, '_morm_retired_a'", a.fetchval("SELECT f($1::text)", "_morm_retired_a"))
        b = await con()
        await go("a new session, the cast, 'c'", b.fetchval("SELECT (($1::text)::st)::text", "c"))
        await a.close()
        await b.close()
    r = await a0.fetchval("SELECT substr(md5(gen_random_uuid()::text), 1, 16)")
    print("   16 hex digits from md5(gen_random_uuid()):", len(r))

    print("8. a fresh run that ends in a session-class error whatever the first was")
    await reset(a0)
    await a0.execute("CREATE TYPE st AS ENUM ('a'); CREATE TABLE t (id int, c int); INSERT INTO t VALUES (1, 1); CREATE FUNCTION g(i int) RETURNS text LANGUAGE plpgsql AS $$DECLARE r t%ROWTYPE; BEGIN SELECT * INTO r FROM t WHERE id = i; IF r.c > 0 THEN RETURN 'x'; END IF; RETURN 'y'; END$$")
    a = await con()
    s = await a.prepare("SELECT (($1::text)::st)::text, g(1)", name="p")
    await go("cached cast and function", s.fetchrow("a"))
    await a0.execute("ALTER TYPE st RENAME TO kept; CREATE TYPE st AS ENUM ('a','c'); ALTER TABLE t ALTER c TYPE bigint")
    await go("cached, 'c': the cast's", s.fetchrow("c"))
    s2 = await a.prepare("SELECT (($1::text)::st)::text, g(1) -- fresh", name="p2")
    await go("fresh, 'c': the function's", s2.fetchrow("c"))
    b = await con()
    await go("new session, 'c'", b.fetchrow("SELECT (($1::text)::st)::text, g(1)", "c"))
    await a.close()
    await b.close()
    await a0.close()


if not os.path.exists(DATA):
    sh(f"{BIN}/initdb", "-D", DATA, "-U", "pt", "-A", "trust", "-E", "UTF8", "--locale=C.UTF-8", "-N")
sh(f"{BIN}/pg_ctl", "-D", DATA, "-o", f"-c listen_addresses='' -c unix_socket_directories={SOCK} -c fsync=off", "-l", DATA + "/log", "-w", "start")
try:
    asyncio.run(main())
finally:
    sh(f"{BIN}/pg_ctl", "-D", DATA, "-m", "immediate", "stop")
