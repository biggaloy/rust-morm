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
    print("a rename with no reset, and the reset run later, alone")
    await reset(a0)
    await a0.execute("CREATE TYPE st AS ENUM ('a','b'); CREATE FUNCTION k() RETURNS text LANGUAGE plpgsql AS $$DECLARE v st; BEGIN v := 'a'; RETURN v::text; END$$")
    a = await con()
    s = await a.prepare("SELECT (($1::text)::st)::text", name="p")
    c = await a.prepare("SELECT ('a'::st)::text", name="pc")
    for _ in range(7):
        await s.fetchval("a"); await a.fetchval("SELECT k()"); await c.fetchval()
    await a0.execute("ALTER TYPE st RENAME TO st_old; CREATE TYPE st AS ENUM ('b','a','c')")
    await a0.execute(f"BEGIN; ALTER TYPE st_old RENAME TO _morm_retired_byhand_st; ALTER TYPE _morm_retired_byhand_st RENAME VALUE 'a' TO '_morm_retired_{RND}_a'; ALTER TYPE _morm_retired_byhand_st RENAME VALUE 'b' TO '_morm_retired_{RND}_b'; COMMIT")
    for x in ("no reset", "after the reset alone"):
        print("  ", x)
        await go("cached cast, 'a'", s.fetchval("a"))
        await go("cached cast, 'c'", s.fetchval("c"))
        await go("cached constant", c.fetchval())
        await go("function with a constant", a.fetchval("SELECT k()"))
        await a0.execute(RESET)
    await a.close()
    await a0.close()


if not os.path.exists(DATA):
    sh(f"{BIN}/initdb", "-D", DATA, "-U", "pt", "-A", "trust", "-E", "UTF8", "--locale=C.UTF-8", "-N")
sh(f"{BIN}/pg_ctl", "-D", DATA, "-o", f"-c listen_addresses='' -c unix_socket_directories={SOCK} -c fsync=off", "-l", DATA + "/log", "-w", "start")
try:
    asyncio.run(main())
finally:
    sh(f"{BIN}/pg_ctl", "-D", DATA, "-m", "immediate", "stop")
