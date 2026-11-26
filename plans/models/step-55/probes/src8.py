"""What can reach R-1245's second row as decided, a 22P02 whose routine is enum_in, in Postgres 15 to 18:
python3 -I src8.py <tags-file> <workdir> <source-tree>...

The routine field is the C function whose ereport, ereturn or errsave raised the error (__func__), or, for an error
plpgsql's bare RAISE throws again, the one that first raised it. So it reads:
1. in each source tree given (the latest release of each major, from ftp.postgresql.org, checked by SHA-256), every
   C, yacc and lex file of the whole tree, contrib and the procedural languages included: every function named
   enum_in, static or not, and every error site in each, with its SQLSTATE and message;
2. in every release tag listed, from GitHub, src/backend/utils/adt/enum.c: the same for enum_in."""
import os, re, sys, urllib.request

tags = open(sys.argv[1]).read().split()
work = sys.argv[2]
trees = sys.argv[3:]
H = {"User-Agent": "src-check"}
DEF = re.compile(r"^(?:static\s+)?(?:[A-Za-z_][\w\s\*]*\s+)?\**\s*enum_in\s*\(", re.M)


def sites(text):
    """the error sites inside each function named enum_in: (SQLSTATE macro or None, message, kind)"""
    out = []
    for m in re.finditer(r"^enum_in\s*\(", text, re.M):
        start = m.start()
        # the body: from the first '{' at column 0 after the name to the matching '}' at column 0
        b = text.index("\n{", start)
        e = text.index("\n}", b)
        body = text[b:e]
        for s in re.finditer(r"\b(ereport|ereturn|errsave|elog)\s*\(", body):
            chunk = body[s.start():s.start() + 400]
            code = re.search(r"errcode\((ERRCODE_\w+)\)", chunk)
            msg = re.search(r'errmsg(?:_internal)?\("((?:[^"\\]|\\.)*)"', chunk) or re.search(r'elog\(\w+,\s*"((?:[^"\\]|\\.)*)"', chunk)
            out.append((s.group(1), code.group(1) if code else None, msg.group(1) if msg else None))
    return out


for t in trees:
    defs, all_sites = [], []
    n = 0
    for root, _, files in os.walk(t):
        for f in files:
            if f.endswith((".c", ".y", ".l", ".h")):
                p = os.path.join(root, f)
                n += 1
                text = open(p, encoding="utf-8", errors="replace").read()
                if re.search(r"\benum_in\b", text):
                    for m in re.finditer(r"^enum_in\s*\(", text, re.M):
                        defs.append(os.path.relpath(p, t))
                    all_sites += [(os.path.relpath(p, t),) + s for s in sites(text)]
                    # a function whose own name ends in enum_in, or a literal routine name, would also be listed
                    for m in re.finditer(r'"enum_in"', text):
                        defs.append(os.path.relpath(p, t) + " (string literal)")
    print(f"{os.path.basename(t.rstrip('/'))}: {n} files read; functions named enum_in: {defs}")
    for s in all_sites:
        print("    ", s)


def raw(tag, path):
    err = None
    for _ in range(4):
        try:
            return urllib.request.urlopen(urllib.request.Request(f"https://raw.githubusercontent.com/postgres/postgres/{tag}/{path}", headers=H), timeout=90).read()
        except Exception as e:
            err = e
    raise err


by = {}
for tag in tags:
    c = os.path.join(work, f"enum-{tag}.c")
    if not os.path.exists(c):
        open(c, "wb").write(raw(tag, "src/backend/utils/adt/enum.c"))
    text = open(c, encoding="utf-8").read()
    by.setdefault(repr(sites(text)), []).append(tag)
for s, ts in by.items():
    print(f"enum.c's enum_in, {len(ts)} releases ({ts[0]} to {ts[-1]}): {s}")
print("releases:", len(tags))
