"""Throwaway Postgres probe for the four answers' round: python3 -I probe6.py <bindir> <datadir> <sockdir> <messages.tsv>
A cluster with C.UTF-8, trust, unix sockets only. Each shape: what a cached statement returns after a hand-made change
that keeps the old type, what a fresh statement on the same connection and a new session return, and what a real
bad label returns with nothing changed, cached and fresh. messages.tsv gets each single-cast message for the rule's
own reader (`ruletest <tsv>`): kind, the cast's schema, its name, the message."""
import asyncio, os, sys
import asyncpg

BIN, DATA, SOCK, TSV = sys.argv[1:5]
tsv = open(TSV, "w")


async def con(db="postgres"):
    return await asyncpg.connect(host=SOCK, user="pt", database=db)


async def msg(coro):
    try:
        return ("ok", await coro)
    except asyncpg.PostgresError as e:
        return (e.sqlstate, e.server_source_function, str(e))


async def go(label, coro, cast=None, kind=None):
    r = await msg(coro)
    print("   " + label + ":", r)
    if cast and r[0] == "22P02":
        sch, nm = cast
        # as decided: a cast written with its schema is named only by a name printed with it; and the two shapes
        # the decision leaves, whose stale message names the cast's own type, are read as a label's own
        if sch and f"{sch}." not in r[2]:
            kind = "second"
        elif kind == "own" and f"enum {nm}:" not in r[2] and not (sch and f"enum {sch}.{nm}:" in r[2]):
            kind = "second"  # a conversion of the statement's own SQL to another type, bad data in its column
        elif kind == "kept" and f"enum {nm}:" in r[2]:
            kind = "left"
        tsv.write("\t".join([kind, sch or "", nm, r[2].replace("\\", "\\\\").replace("\n", "\\n").replace("\t", "\\t")]) + "\n")
    return r


async def wipe(a):
    for s in ("old", "app"):
        await a.execute(f"DROP SCHEMA IF EXISTS {s} CASCADE")
    await a.execute("DROP SCHEMA public CASCADE; CREATE SCHEMA public")


async def shape(a0, title, setup, sql, before, change, after, bad, cast=None, path=None):
    """setup and change run on a connection of their own; sql is prepared before the change on a connection that
    stays; `before` and `after` are its arguments before and after; `bad` a real bad label's, with nothing changed"""
    print("  " + title)
    await wipe(a0)
    await a0.execute(setup)
    a = await con()
    if path:
        await a.execute(f"SET search_path = {path}")
    s = await a.prepare(sql)
    await s.fetch(*before)
    # a real bad label first, nothing changed: cached and fresh
    await go("nothing changed, a real bad label, cached", s.fetch(*bad), cast, "own")
    await go("nothing changed, a real bad label, fresh", a.fetch(sql + " -- fresh 0", *bad))
    await a0.execute(change)
    await go("after the change, cached", s.fetch(*after), cast, "kept")
    await go("after the change, cached again", s.fetch(*after))
    await go("after the change, fresh on the same connection", a.fetch(sql + " -- fresh 1", *after))
    b = await con()
    if path:
        await b.execute(f"SET search_path = {path}")
    await go("after the change, a new session", b.fetch(sql, *after))
    # a real bad label after the change, cached on the new session, and inside a transaction a fresh retry
    sb = await b.prepare(sql)
    await go("after the change, a real bad label, cached on the new session", sb.fetch(*bad), cast, "own")
    async with b.transaction():
        await go("after the change, a real bad label, fresh inside a transaction", b.fetch(sql + " -- fresh 2", *bad))
    await a.close()
    await b.close()


async def main():
    a0 = await con()
    print(await a0.fetchval("show server_version"))

    print("1. a statement that casts parameters to more than one non-built-in type")
    await shape(a0, "two enums whose names a hand change exchanged",
                "CREATE TYPE st AS ENUM ('a'); CREATE TYPE kind AS ENUM ('k')",
                "SELECT (($1::text)::st)::text, (($2::text)::kind)::text", ("a", "k"),
                "ALTER TYPE st RENAME TO tmp; ALTER TYPE kind RENAME TO st; ALTER TYPE tmp RENAME TO kind; ALTER TYPE st ADD VALUE 'a2'",
                ("a2", "a"), ("zz", "k"))
    await shape(a0, "a composite kept under another name beside its field's type",
                "CREATE TYPE st AS ENUM ('a'); CREATE TYPE st2 AS ENUM ('a', 'b'); CREATE TYPE cmp AS (s st)",
                "SELECT (($1::text)::cmp)::text, (($2::text)::st)::text", ("(a)", "a"),
                "ALTER TYPE cmp RENAME TO cmp_old; CREATE TYPE cmp AS (s st2)", ("(b)", "a"), ("(zz)", "a"))

    print("2. a cast to a domain or a composite alone")
    await shape(a0, "a domain kept under another name, made again over another enum",
                "CREATE TYPE st AS ENUM ('a'); CREATE TYPE st2 AS ENUM ('a', 'b'); CREATE DOMAIN d AS st",
                "SELECT (($1::text)::d)::text", ("a",),
                "ALTER DOMAIN d RENAME TO d_old; CREATE DOMAIN d AS st2", ("b",), ("zz",))
    await shape(a0, "a composite the same way",
                "CREATE TYPE st AS ENUM ('a'); CREATE TYPE st2 AS ENUM ('a', 'b'); CREATE TYPE cmp AS (s st)",
                "SELECT (($1::text)::cmp)::text", ("(a)",),
                "ALTER TYPE cmp RENAME TO cmp_old; CREATE TYPE cmp AS (s st2)", ("(b)",), ("(zz)",))

    print("3. a cast written with its schema, its kept type moved by SET SCHEMA, under each search_path")
    for path in ("app, old", "old, app", "old", "public", "app"):
        await shape(a0, f"search_path = {path}",
                    "CREATE SCHEMA old; CREATE SCHEMA app; CREATE TYPE app.st AS ENUM ('a')",
                    "SELECT (($1::text)::app.st)::text", ("a",),
                    "ALTER TYPE app.st SET SCHEMA old; CREATE TYPE app.st AS ENUM ('a', 'c')", ("c",), ("zz",),
                    cast=("app", "st"), path=path)

    print("4. a cast morm didn't write: no parameter cast to a non-built-in type")
    await shape(a0, "(c::st)::text over a text column",
                "CREATE TYPE st AS ENUM ('a'); CREATE TABLE s (id int, c text); INSERT INTO s VALUES (1, 'a'), (2, 'zz')",
                "SELECT (c::st)::text FROM s WHERE id = $1::int4", (1,),
                "ALTER TYPE st RENAME TO kept; CREATE TYPE st AS ENUM ('a', 'c'); UPDATE s SET c = 'c' WHERE id = 1", (1,), (2,))
    await shape(a0, "x::st over unnest($1::text[])",
                "CREATE TYPE st AS ENUM ('a')",
                "SELECT x::st::text FROM unnest($1::text[]) x", (["a"],),
                "ALTER TYPE st RENAME TO kept; CREATE TYPE st AS ENUM ('a', 'c')", (["c"],), (["zz"],))

    print("5. a kept type's stale function, reached by a cached statement that casts no parameter")
    await shape(a0, "SELECT f(c) FROM s WHERE id = $1, f declaring v st",
                "CREATE TYPE st AS ENUM ('a'); CREATE TABLE s (id int, c text); INSERT INTO s VALUES (1, 'a'), (2, 'zz');"
                "CREATE FUNCTION f(x text) RETURNS text LANGUAGE plpgsql AS $$ DECLARE v st; BEGIN v := x; RETURN v::text; END $$",
                "SELECT f(c) FROM s WHERE id = $1::int4", (1,),
                "ALTER TYPE st RENAME TO kept; CREATE TYPE st AS ENUM ('a', 'c'); UPDATE s SET c = 'c' WHERE id = 1", (1,), (2,))
    await shape(a0, "and one beside a parameter's cast, after an exchange that left the function's type under the cast's name",
                "CREATE TYPE st AS ENUM ('a', 'k'); CREATE TYPE kind AS ENUM ('k');"
                "CREATE FUNCTION f(x text) RETURNS text LANGUAGE plpgsql AS $$ DECLARE v kind; BEGIN v := x; RETURN v::text; END $$",
                "SELECT (($1::text)::st)::text, f($2)", ("a", "k"),
                "ALTER TYPE st RENAME TO tmp; ALTER TYPE kind RENAME TO st; ALTER TYPE tmp RENAME TO kind; ALTER TYPE kind ADD VALUE 'k2'",
                ("k", "k2"), ("zz", "k"), cast=(None, "st"))

    print("6. what the decision leaves: one parameter's cast beside a conversion of the user's own to another type, after an exchange")
    await shape(a0, "SELECT ($1::st)::text, (c::kind)::text FROM s WHERE id = $2",
                "CREATE TYPE st AS ENUM ('a', 'k'); CREATE TYPE kind AS ENUM ('k'); CREATE TABLE s (id int, c text); INSERT INTO s VALUES (1, 'k'), (2, 'zz')",
                "SELECT (($1::text)::st)::text, (c::kind)::text FROM s WHERE id = $2::int4", ("a", 1),
                "ALTER TYPE st RENAME TO tmp; ALTER TYPE kind RENAME TO st; ALTER TYPE tmp RENAME TO kind; ALTER TYPE kind ADD VALUE 'k2'; UPDATE s SET c = 'k2' WHERE id = 1",
                ("k", 1), ("a", 2), cast=(None, "st"))
    await shape(a0, "SELECT ($1::st)::text, (c::cmp)::text FROM s WHERE id = $2, cmp kept under another name and made again over st2",
                "CREATE TYPE st AS ENUM ('a'); CREATE TYPE st2 AS ENUM ('a', 'b'); CREATE TYPE cmp AS (s st); CREATE TABLE s (id int, c text); INSERT INTO s VALUES (1, '(a)'), (2, '(zz)')",
                "SELECT (($1::text)::st)::text, (c::cmp)::text FROM s WHERE id = $2::int4", ("a", 1),
                "ALTER TYPE cmp RENAME TO cmp_old; CREATE TYPE cmp AS (s st2); UPDATE s SET c = '(b)' WHERE id = 1",
                ("a", 1), ("a", 2), cast=(None, "st"))
    await a0.close()


asyncio.run(main())
