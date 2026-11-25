"""Throwaway Postgres probe for the three answers' round: python3 -I probe5.py <bindir> <datadir> <sockdir> <messages.tsv>
A cluster with C.UTF-8, trust, unix sockets only; messages.tsv gets every message for the rule's own reader."""
import asyncio, os, subprocess, sys
import asyncpg

BIN, DATA, SOCK, TSV = sys.argv[1:5]
ENV = dict(os.environ, LC_ALL="C.UTF-8")
LOCS = "de_DE es_ES fr_FR id_ID it_IT ja_JP ka_GE ko_KR pl_PL pt_BR ru_RU sv_SE tr_TR uk_UA zh_CN zh_TW".split()
RESET = "CREATE OPERATOR public.### (LEFTARG = int4, RIGHTARG = int4, FUNCTION = int4eq); DROP OPERATOR public.### (int4, int4); "
tsv = open(TSV, "w")


async def con(db="postgres", **k):
    return await asyncpg.connect(host=SOCK, user="pt", database=db, **k)


async def msg(coro):
    try:
        return ("ok", await coro)
    except asyncpg.PostgresError as e:
        return (e.sqlstate, e.server_source_function, str(e))


async def go(label, coro):
    print("   " + label + ":", await msg(coro))


async def wipe(a):
    for s in ("old", "billing", "app", "My: \"S"):
        await a.execute(f'DROP SCHEMA IF EXISTS "{s.replace(chr(34), chr(34) * 2)}" CASCADE')
    await a.execute("DROP SCHEMA public CASCADE; CREATE SCHEMA public")


def q(x):
    return '"' + x.replace('"', '""') + '"'


async def main():
    a0 = await con()
    print(await a0.fetchval("show server_version"))

    print("1. every language, name and value: the cast's own type's error, and a stale cast's on a kept type")
    names = [(None, "st"), (None, "St"), (None, "st old"), (None, 'a: "b'), (None, "»x: «y"), (None, "value"), (None, "values"), (None, "input"), (None, "入力"), (None, "a.b"), (None, "1x"), (None, ': "'), ("billing", "order_state"), ('My: "S', "st")]
    words = {"es_ES": "entrada", "it_IT": "l", "fr_FR": "valeur", "tr_TR": "tipi", "id_ID": "nilai", "pl_PL": "dla", "pt_BR": "valor", "sv_SE": "indata"}
    n = {"own": 0, "kept": 0, "word": 0}
    shown = set()
    for sch, nm in names:
        for how in ("rename", "quoted", "schema"):
            await wipe(a0)
            await a0.execute("CREATE SCHEMA old")
            if sch:
                await a0.execute(f"CREATE SCHEMA {q(sch)}")
            full = (q(sch) + "." if sch else "") + q(nm)
            await a0.execute(f"CREATE TYPE {full} AS ENUM ('a')")
            a = await con()
            p = await a.prepare(f"SELECT (($1::text)::{full})::text", name="p")
            await p.fetchval("a")
            for k in range(6):
                await p.fetchval("a")
            vals = ["c", "", f"in {nm}", f'x "{nm}"', f'{nm}: "bad', f"»{nm}«", f': "{nm}"', f": «{nm} ", f'invalid input value for enum {nm}: "z', nm]
            if how == "schema":
                await a0.execute(f"ALTER TYPE {full} SET SCHEMA old")
            else:
                await a0.execute(f"ALTER TYPE {full} RENAME TO {q(nm + ('_old' if how == 'rename' else ' old'))}")
            await a0.execute(f"CREATE TYPE {full} AS ENUM ('a', 'b')")
            for loc in ["C"] + [x + ".utf8" for x in LOCS]:
                for al in ("off", "on"):
                    await a.execute(f"SET lc_messages = '{loc}'; SET quote_all_identifiers = {al}")
                    for v in vals:
                        for kind, coro in (("kept", p.fetchval(v)), ("own", a.fetchval(f"SELECT (($1::text)::{full})::text /* {kind_id(loc, al)} */", v))):
                            r = await msg(coro)
                            assert r[0] == "22P02" and r[1] == "enum_in" and "\t" not in r[2] and "\n" not in r[2], r
                            k = kind
                            n[k] += 1
                            tsv.write(f"{k}\t{sch or ''}\t{nm}\t{r[2]}\n")
                            if v == "c" and how == "rename" and nm == "st" and al == "off" and kind == "kept":
                                print("   ", loc, r[2])
            await a.close()
    for loc, nm in words.items():
        await wipe(a0)
        await a0.execute(f"CREATE TYPE {q(nm)} AS ENUM ('a')")
        a = await con()
        p = await a.prepare(f"SELECT (($1::text)::{q(nm)})::text", name="p")
        await p.fetchval("a")
        await a0.execute(f"ALTER TYPE {q(nm)} RENAME TO {q(nm + '_old')}; CREATE TYPE {q(nm)} AS ENUM ('a', 'c')")
        await a.execute(f"SET lc_messages = '{loc}.utf8'")
        r = await msg(p.fetchval("c"))
        print("    a type named as a word:", loc, r)
        tsv.write(f"{'kept' if r[2].startswith('invalid input value') else 'word'}\t\t{nm}\t{r[2]}\n")
        n["word"] += 1
        await a.execute("SET lc_messages = 'C'")
        r = await msg(p.fetchval("c"))
        tsv.write(f"kept\t\t{nm}\t{r[2]}\n")
        await a.close()
    print("   messages:", n)

    print("2. a value the call didn't bind, and a type named as a word of the English message")
    await wipe(a0)
    await a0.execute("CREATE TYPE st AS ENUM ('a'); CREATE TABLE s (id int, c text); INSERT INTO s VALUES (1, 'a')")
    a = await con()
    s = await a.prepare("SELECT (c::st)::text, (($2::text)::st)::text FROM s WHERE id = $1::int4", name="p")
    await s.fetchrow(1, "a")
    await a0.execute("ALTER TYPE st RENAME TO kept; CREATE TYPE st AS ENUM ('a', 'in st'); UPDATE s SET c = 'in st'")
    await go("cached, kept type, stored 'in st'", s.fetchrow(1, "a"))
    await go("fresh", a.fetchrow("SELECT (c::st)::text, (($2::text)::st)::text FROM s WHERE id = $1::int4 -- fresh", 1, "a"))
    await a.close()

    print("3. no parameter cast at all: a cached statement that converts a column, or a parameter bound as text, by SQL of the user's own")
    await wipe(a0)
    await a0.execute("CREATE TYPE st AS ENUM ('a'); CREATE TABLE s (id int, c text); INSERT INTO s VALUES (1, 'a')")
    a = await con()
    s = await a.prepare("SELECT (c::st)::text FROM s WHERE id = $1::int4", name="p")
    s2 = await a.prepare("SELECT x::st::text FROM unnest($1::text[]) x", name="p2")
    await s.fetchrow(1)
    await s2.fetch(["a"])
    await a0.execute("ALTER TYPE st RENAME TO kept; CREATE TYPE st AS ENUM ('a', 'c'); UPDATE s SET c = 'c'")
    for k in range(3):
        await go(f"cached, run {k + 1}", s.fetchrow(1))
    await go("cached unnest, run 1", s2.fetch(["c"]))
    await go("fresh", a.fetchrow("SELECT (c::st)::text FROM s WHERE id = $1::int4 -- fresh", 1))
    await go("fresh unnest", a.fetch("SELECT x::st::text FROM unnest($1::text[]) x -- fresh", ["c"]))
    await a.close()

    print("4. a statement that casts to two types whose names a hand change exchanged")
    await wipe(a0)
    await a0.execute("CREATE TYPE st AS ENUM ('a'); CREATE TYPE kind AS ENUM ('k')")
    a = await con()
    s = await a.prepare("SELECT (($1::text)::st)::text, (($2::text)::kind)::text", name="p")
    await s.fetchrow("a", "k")
    await a0.execute("ALTER TYPE st RENAME TO tmp; ALTER TYPE kind RENAME TO st; ALTER TYPE tmp RENAME TO kind; ALTER TYPE st ADD VALUE 'a2'")
    await go("cached, after the exchange, a label of the new st", s.fetchrow("a2", "a"))
    await go("fresh", a.fetchrow("SELECT (($1::text)::st)::text, (($2::text)::kind)::text -- fresh", "a2", "a"))
    await a.close()
    print("   and a composite kept under another name, beside its field's own type")
    await wipe(a0)
    await a0.execute("CREATE TYPE st AS ENUM ('a'); CREATE TYPE st2 AS ENUM ('a', 'b'); CREATE TYPE cmp AS (s st)")
    a = await con()
    s = await a.prepare("SELECT (($1::text)::cmp)::text, (($2::text)::st)::text", name="p")
    await s.fetchrow("(a)", "a")
    await a0.execute("ALTER TYPE cmp RENAME TO cmp_old; CREATE TYPE cmp AS (s st2)")
    await go("cached", s.fetchrow("(b)", "a"))
    await go("fresh", a.fetchrow("SELECT (($1::text)::cmp)::text, (($2::text)::st)::text -- fresh", "(b)", "a"))
    await a.close()
    print("   and a domain the same way")
    await wipe(a0)
    await a0.execute("CREATE TYPE st AS ENUM ('a'); CREATE TYPE st2 AS ENUM ('a', 'b'); CREATE DOMAIN d AS st")
    a = await con()
    s = await a.prepare("SELECT (($1::text)::d)::text, (($2::text)::st)::text", name="p")
    await s.fetchrow("a", "a")
    await a0.execute("ALTER DOMAIN d RENAME TO d_old; CREATE DOMAIN d AS st2")
    await go("cached", s.fetchrow("b", "a"))
    await go("fresh", a.fetchrow("SELECT (($1::text)::d)::text, (($2::text)::st)::text -- fresh", "b", "a"))
    await a.close()

    print("5. a cast written with a schema, its kept type moved to another schema on the search_path")
    for path in ("app, old", "old, app", "public", "old"):
        await wipe(a0)
        await a0.execute("CREATE SCHEMA old; CREATE SCHEMA app; CREATE TYPE app.st AS ENUM ('a')")
        a = await con()
        await a.execute(f"SET search_path = {path}")
        s = await a.prepare("SELECT (($1::text)::app.st)::text", name="p")
        await s.fetchval("a")
        await a0.execute("ALTER TYPE app.st SET SCHEMA old; CREATE TYPE app.st AS ENUM ('a', 'c')")
        await go(f"search_path = {path}: cached, a new label", s.fetchval("c"))
        await go(f"search_path = {path}: fresh, a bad label", a.fetchval("SELECT (($1::text)::app.st)::text -- fresh", "zzz"))
        await a.close()
    print("   and one written without, shadowed by a type made in a schema earlier on the search_path")
    await wipe(a0)
    await a0.execute("CREATE SCHEMA old; CREATE SCHEMA app; CREATE TYPE old.st AS ENUM ('a')")
    a = await con()
    await a.execute("SET search_path = app, old")
    s = await a.prepare("SELECT (($1::text)::st)::text", name="p")
    await s.fetchval("a")
    await a0.execute("CREATE TYPE app.st AS ENUM ('a', 'c')")
    await go("cached, a new label", s.fetchval("c"))
    await go("fresh", a.fetchval("SELECT (($1::text)::st)::text -- fresh", "c"))
    await a.close()

    print("6. a database whose encoding can't write a language's quote marks")
    await wipe(a0)
    for enc in ("LATIN2", "WIN1251", "SQL_ASCII", "KOI8R", "EUC_KR"):
        db = "e_" + enc.lower()
        try:
            await a0.execute(f"CREATE DATABASE {db} TEMPLATE template0 ENCODING '{enc}' LC_COLLATE 'C' LC_CTYPE 'C'")
        except asyncpg.PostgresError as e:
            print("   ", enc, e)
            continue
        a = await con(db)
        await a.execute("CREATE TYPE st AS ENUM ('a')")
        for loc in ("C", "de_DE.utf8", "es_ES.utf8", "fr_FR.utf8", "ru_RU.utf8", "ja_JP.utf8"):
            await a.execute(f"SET lc_messages = '{loc}'")
            try:
                await go(f"{enc}, {loc}", a.fetchval("SELECT (($1::text)::st)::text", "zzz"))
            except UnicodeDecodeError as e:
                print(f"    {enc}, {loc}: not UTF-8", e)
                a = await con(db)
        await a.close()

    print("7. what else raises 22P02 for an enum: a function's own text, and the routine of a too-long label")
    await wipe(a0)
    await a0.execute("CREATE TYPE st AS ENUM ('a'); CREATE FUNCTION f() RETURNS text LANGUAGE plpgsql AS $$BEGIN RAISE EXCEPTION 'invalid input value for enum st: \"x\"' USING ERRCODE = '22P02'; END$$; CREATE FUNCTION g(t text) RETURNS text LANGUAGE plpgsql AS $$BEGIN RETURN t::st::text; EXCEPTION WHEN OTHERS THEN RAISE; END$$; CREATE FUNCTION h(t text) RETURNS text LANGUAGE sql SET search_path = '' AS $$SELECT t::public.st::text$$")
    await go("RAISE with the same text", a0.fetchval("SELECT f()"))
    await go("re-raised by a handler", a0.fetchval("SELECT g('x')"))
    await go("a 70-byte label", a0.fetchval("SELECT (($1::text)::st)::text", "y" * 70))
    await go("inside a function with a search_path of its own", a0.fetchval("SELECT h('x')"))
    await go("an integer", a0.fetchval("SELECT (($1::text)::int4)::text", "x"))
    await go("a new value not committed", unsafe(a0))

    print("8. the third silent case: text copied from pg_enum and from the retired column")
    await wipe(a0)
    await a0.execute("CREATE TYPE st AS ENUM ('a','b'); CREATE TABLE t (id int, c st); INSERT INTO t VALUES (1,'a'); CREATE FUNCTION f(t text) RETURNS text LANGUAGE plpgsql AS $$DECLARE v st; BEGIN v := t; RETURN v::text; END$$")
    a = await con()
    await a.fetchval("SELECT f('a')")
    r = "7c1e9a04b52fd386"
    await a0.execute(f"BEGIN; ALTER TABLE t RENAME c TO _morm_retired_m2_c; ALTER TYPE st RENAME TO _morm_retired_m2_st; ALTER TYPE _morm_retired_m2_st RENAME VALUE 'a' TO '_morm_retired_{r}_a'; ALTER TYPE _morm_retired_m2_st RENAME VALUE 'b' TO '_morm_retired_{r}_b'; CREATE TYPE st AS ENUM ('b','a','c'); {RESET} COMMIT")
    await go("stale, the retired column's text", a.fetchval("SELECT f(_morm_retired_m2_c::text) FROM t"))
    lab = await a0.fetchval("SELECT enumlabel::text FROM pg_enum WHERE enumlabel LIKE '\\_morm%b'")
    await go("stale, pg_enum's text, read by a client and bound", a.fetchval("SELECT f($1::text)", lab))
    await go("stale, a label of the old type", a.fetchval("SELECT f('a')"))
    b = await con()
    await go("a new session, the retired column's text", b.fetchval("SELECT f(_morm_retired_m2_c::text) FROM t"))
    await a.close()
    await b.close()


def kind_id(loc, al):
    return f"{loc} {al}"


async def unsafe(a):
    async with a.transaction():
        await a.execute("ALTER TYPE st ADD VALUE 'n'")
        return await a.fetchval("SELECT 'n'::text::st::text")


asyncio.run(main())
