"""Throwaway Postgres probe for the owner's answer to the two answers' question (2026-11-26):
python3 -I probe7.py <datadir> <sockdir>

R-1245's second row as decided: a cached statement's 22P02 whose routine is enum_in, and whose message holds no
`_morm_retired_`, is a stale statement's first signal whatever the message says; no message text is read. This
probe runs that decision, as a few lines of Python beside asyncpg, on each case the answer names, in English,
Spanish and German (lc_messages), and prints what Postgres sent (SQLSTATE, routine, message) and what the call gets:
- outside a transaction: the cached run, and on the second row's signal one fresh run (a new prepare), returned;
- inside a transaction with no .retry(n): the cached run, and on that signal the rollback and Error::SchemaChanged,
  the statement entered as pending;
- inside .retry(1).transaction(..): the same first attempt, then the second attempt, whose run is unnamed (a new
  prepare), returned as it is.
The cases: the two conversion cases of the open question (a conversion of the statement's own SQL beside its
parameter's cast, after an exchange of names and after a composite was kept under another name), the language case (a
type named `entrada`, a word of the Spanish message, after a swap made by hand that keeps it), and a real bad label
with nothing changed."""
import asyncio, sys
import asyncpg

DATA, SOCK = sys.argv[1:3]
LANGS = ["C", "es_ES.utf8", "de_DE.utf8"]


async def con(lang):
    c = await asyncpg.connect(host=SOCK, user="pt", database="postgres")
    await c.execute(f"SET lc_messages = '{lang}'")
    return c


def second(e):
    """the decided second row: SQLSTATE and routine, and no `_morm_retired_` (the seventh row goes first)"""
    return e.sqlstate == "22P02" and e.server_source_function == "enum_in" and "_morm_retired_" not in str(e)


N = [0]


async def fresh(c, sql):
    """a statement prepared now: a text of its own, so asyncpg's statement cache can't hand back the cached one"""
    N[0] += 1
    return await c.prepare(f"{sql} -- fresh {N[0]}")


def show(r):
    if isinstance(r, asyncpg.PostgresError):
        return f"{r.sqlstate} {r.server_source_function}: {r}"
    return repr(r)


async def run(stmt, args):
    try:
        return await stmt.fetch(*args)
    except asyncpg.PostgresError as e:
        return e


async def call_out(c, cached, sql, args):
    r = await run(cached, args)
    first = show(r)
    if isinstance(r, asyncpg.PostgresError) and second(r):
        r = await run(await fresh(c, sql), args)  # one fresh run
        return f"cached {first} -> second row, fresh run -> {show(r)}"
    return f"cached {first} -> {show(r)}"


async def call_tx(c, cached, sql, args, retry):
    out = []
    pending = False
    for attempt in range(retry + 1):
        tr = c.transaction()
        await tr.start()
        stmt = (await fresh(c, sql)) if pending else cached
        r = await run(stmt, args)
        if isinstance(r, asyncpg.PostgresError):
            await tr.rollback()
            if not pending and second(r):
                pending = True  # rows B1 and D1: the pending entry; .retry(n)'s next attempt runs unnamed
                out.append(f"attempt {attempt + 1}: {show(r)} -> Error::SchemaChanged")
                continue
            out.append(f"attempt {attempt + 1}: {show(r)} -> as it is")
            break
        await tr.commit()
        out.append(f"attempt {attempt + 1}: {show(r)}")
        break
    return "; ".join(out)


async def case(title, setup, sql, before, change, after, lang):
    a0 = await con("C")
    for s in ("old", "app"):
        await a0.execute(f"DROP SCHEMA IF EXISTS {s} CASCADE")
    await a0.execute("DROP SCHEMA public CASCADE; CREATE SCHEMA public")
    await a0.execute(setup)
    # three connections, each with the statement cached before the change: outside, in a transaction, under retry
    cs = [await con(lang) for _ in range(3)]
    st = [await c.prepare(sql) for c in cs]
    for s in st:
        await s.fetch(*before)
    if change:
        await a0.execute(change)
    print(f"  {title} [lc_messages {lang}]")
    print("    outside a transaction:", await call_out(cs[0], st[0], sql, after))
    print("    transaction, no retry:", await call_tx(cs[1], st[1], sql, after, 0))
    print("    .retry(1).transaction:", await call_tx(cs[2], st[2], sql, after, 1))
    n = await con(lang)
    print("    a new session:", show(await run(await n.prepare(sql), after)))
    for c in cs + [n, a0]:
        await c.close()


EXCH = "ALTER TYPE st RENAME TO tmp; ALTER TYPE kind RENAME TO st; ALTER TYPE tmp RENAME TO kind; ALTER TYPE kind ADD VALUE 'k2'"


async def main():
    a = await con("C")
    print(await a.fetchval("show server_version"))
    await a.close()
    for lang in LANGS:
        print(f"1. the conversion cases [{lang}]")
        await case("SELECT ($1::st)::text, (c::kind)::text FROM s WHERE id = $2, after a hand change exchanged st and kind",
                   "CREATE TYPE st AS ENUM ('a', 'k'); CREATE TYPE kind AS ENUM ('k'); CREATE TABLE s (id int, c text); INSERT INTO s VALUES (1, 'k')",
                   "SELECT (($1::text)::st)::text, (c::kind)::text FROM s WHERE id = $2::int4", ("a", 1),
                   EXCH + "; UPDATE s SET c = 'k2' WHERE id = 1", ("k", 1), lang)
        await case("SELECT ($1::st)::text, (c::cmp)::text FROM s WHERE id = $2, cmp kept under another name and made again over st2",
                   "CREATE TYPE st AS ENUM ('a'); CREATE TYPE st2 AS ENUM ('a', 'b'); CREATE TYPE cmp AS (s st); CREATE TABLE s (id int, c text); INSERT INTO s VALUES (1, '(a)')",
                   "SELECT (($1::text)::st)::text, (c::cmp)::text FROM s WHERE id = $2::int4", ("a", 1),
                   "ALTER TYPE cmp RENAME TO cmp_old; CREATE TYPE cmp AS (s st2); UPDATE s SET c = '(b)' WHERE id = 1", ("a", 1), lang)
        print(f"2. the language case [{lang}]")
        await case("SELECT ($1::entrada)::text, entrada kept under another name by hand and made again with a new label",
                   "CREATE TYPE entrada AS ENUM ('a')",
                   "SELECT (($1::text)::entrada)::text", ("a",),
                   "ALTER TYPE entrada RENAME TO entrada_vieja; CREATE TYPE entrada AS ENUM ('a', 'c')", ("c",), lang)
        print(f"3. a real bad label, nothing changed [{lang}]")
        await case("SELECT ($1::st)::text given 'zz'",
                   "CREATE TYPE st AS ENUM ('a')",
                   "SELECT (($1::text)::st)::text", ("a",), None, ("zz",), lang)
        await case("SELECT ($1::entrada)::text given 'entrada'",
                   "CREATE TYPE entrada AS ENUM ('a')",
                   "SELECT (($1::text)::entrada)::text", ("a",), None, ("entrada",), lang)
        await case("SELECT ($1::st)::text, (c::kind)::text FROM s WHERE id = $2 given 'zz'",
                   "CREATE TYPE st AS ENUM ('a'); CREATE TYPE kind AS ENUM ('k'); CREATE TABLE s (id int, c text); INSERT INTO s VALUES (1, 'k')",
                   "SELECT (($1::text)::st)::text, (c::kind)::text FROM s WHERE id = $2::int4", ("a", 1), None, ("zz", 1), lang)
    # what else can carry SQLSTATE 22P02: none with the routine enum_in
    print("4. other 22P02s and other enum_in errors, nothing changed [C]")
    a = await con("C")
    await a.execute("DROP SCHEMA public CASCADE; CREATE SCHEMA public; CREATE TYPE st AS ENUM ('a')")
    for sql, args in [("SELECT $1::text::int", ("x",)), ("SELECT $1::text::uuid", ("x",)), ("SELECT $1::text::json", ("{",)),
                      ("SELECT $1::text::bool", ("x",)), ("SELECT $1::text::inet", ("x",)),
                      ("DO $$ BEGIN RAISE EXCEPTION USING ERRCODE = '22P02', MESSAGE = 'invalid input value for enum st: \"x\"'; END $$", ()),
                      ("DO $$ DECLARE v st; BEGIN v := 'x'; EXCEPTION WHEN others THEN RAISE; END $$", ())]:
        s = await a.prepare(sql)
        print("   ", sql, "->", show(await run(s, args)), "-> second row" if isinstance(await run(s, args), asyncpg.PostgresError) and second(await run(s, args)) else "")
    tr = a.transaction()
    await tr.start()
    await a.execute("ALTER TYPE st ADD VALUE 'b'")
    try:
        r = await run(await a.prepare("SELECT 'b'::st"), ())
    except asyncpg.PostgresError as e:
        r = e
    print("    a label added in the open transaction ->", show(r))
    await tr.rollback()
    await a.close()


asyncio.run(main())
