"""Row C2 through PgBouncer: python3 probe3.py <bindir> <datadir> <sockdir> <pgbouncer> <workdir>
One client connection kept, other clients busy: which server session does its next transaction reach?"""
import asyncio, os, subprocess, sys
import asyncpg

BIN, DATA, SOCK, PGB, WORK = sys.argv[1:6]
ENV = dict(os.environ, LC_ALL="C.UTF-8")
RND = "7c1e9a04b52fd386"
RESET = "CREATE OPERATOR public.### (LEFTARG = int4, RIGHTARG = int4, FUNCTION = int4eq); DROP OPERATOR public.### (int4, int4); "


def sh(*a):
    subprocess.run(a, check=True, env=ENV, stdout=subprocess.DEVNULL)


async def con(port):
    return await asyncpg.connect(host=SOCK, port=port, user="pt", database="postgres", statement_cache_size=0)


async def main():
    direct = await con(5432)
    print(await direct.fetchval("show server_version"))
    await direct.execute("DROP SCHEMA public CASCADE; CREATE SCHEMA public; CREATE TYPE st AS ENUM ('a','b'); CREATE FUNCTION f(t text) RETURNS text LANGUAGE plpgsql AS $$DECLARE v st; BEGIN v := t; RETURN v::text || ' ' || pg_backend_pid(); END$$")
    cs = [await con(6432) for _ in range(4)]
    # compile f on server session 1 only: alone, every transaction reaches the last released one
    for _ in range(5):
        r = await cs[0].fetchval("SELECT f('a')")
    pid = r.split()[1]
    print("   alone, the same client connection reaches server", len({(await cs[0].fetchval("SELECT pg_backend_pid()")) for _ in range(20)}), "session(s) in 20 transactions")
    await direct.execute(f"BEGIN; ALTER TYPE st RENAME TO _morm_retired_m2_st; ALTER TYPE _morm_retired_m2_st RENAME VALUE 'a' TO '_morm_retired_{RND}_a'; ALTER TYPE _morm_retired_m2_st RENAME VALUE 'b' TO '_morm_retired_{RND}_b'; CREATE TYPE st AS ENUM ('a','b','c'); {RESET} COMMIT")

    async def call(c):
        try:
            return "ok"if (await c.fetchval("SELECT f('a')")) else ""
        except asyncpg.PostgresError as e:
            return e.sqlstate
    print("   alone: first call", await call(cs[0]), "; run again on the same client connection", [await call(cs[0]) for _ in range(5)])

    async def busy(c, stop):
        while not stop.is_set():
            await c.fetchval("SELECT pg_sleep(0.003)")

    stop = asyncio.Event()
    ts = [asyncio.create_task(busy(c, stop)) for c in cs[1:]]
    first = again_ok = again_bad = 0
    other_ok = 0
    for i in range(400):
        if await call(cs[0]) == "22P02":
            first += 1
            if await call(cs[0]) == "ok":
                again_ok += 1
            else:
                again_bad += 1
        await asyncio.sleep(0.001)
    stop.set()
    await asyncio.gather(*ts)
    print(f"   with 3 other clients busy: {first} stale first runs of 400; the run again on the same client connection answered {again_ok} times, failed {again_bad}")
    for c in cs:
        await c.close()
    await direct.close()


os.makedirs(WORK, exist_ok=True)
open(WORK + "/u.txt", "w").write('"pt" ""\n')
open(WORK + "/b.ini", "w").write(f"""[databases]
postgres = host={SOCK} port=5432 dbname=postgres user=pt
[pgbouncer]
listen_port = 6432
listen_addr =
unix_socket_dir = {SOCK}
auth_type = trust
auth_file = {WORK}/u.txt
pool_mode = transaction
default_pool_size = 3
max_prepared_statements = 200
server_round_robin = {os.environ.get('RR', '0')}
logfile = {WORK}/log
pidfile = {WORK}/pid
""")
sh(f"{BIN}/pg_ctl", "-D", DATA, "-o", f"-c listen_addresses='' -c unix_socket_directories={SOCK} -c fsync=off", "-l", DATA + "/log", "-w", "start")
pb = subprocess.Popen([PGB, "-q", WORK + "/b.ini"], env=ENV)
try:
    import time
    time.sleep(1)
    asyncio.run(main())
finally:
    pb.terminate()
    pb.wait()
    sh(f"{BIN}/pg_ctl", "-D", DATA, "-m", "immediate", "stop")
