"""enum_in's message in every release of Postgres 15 to 18 and every language it ships: python3 -I po2.py <tags-file> <workdir>
Reads each tag's src/backend/po by blob, so a file unchanged between releases is fetched once; keeps only the message."""
import json, os, re, subprocess, sys, urllib.request
from concurrent.futures import ThreadPoolExecutor

tags = open(sys.argv[1]).read().split()
work = sys.argv[2]
H = {"User-Agent": "po-check"}


def api(path):
    return json.loads(subprocess.run(["gh", "api", path], check=True, capture_output=True, text=True).stdout)


def raw(tag, path):
    err = None
    for _ in range(4):
        try:
            return urllib.request.urlopen(urllib.request.Request(f"https://raw.githubusercontent.com/postgres/postgres/{tag}/{path}", headers=H), timeout=90).read()
        except Exception as e:
            err = e
    raise err


cache_f = os.path.join(work, "po2-cache.json")
cache = json.load(open(cache_f)) if os.path.exists(cache_f) else {}


def entry(text):
    """the msgstr of enum_in's msgid, whole, continuation lines joined; with its flags"""
    out = []
    for block in re.split(r"\n\s*\n", text):
        m = re.search(r'^msgid ((?:".*"\n?)+)msgstr ((?:".*"\n?)+)', block, re.M)
        if not m:
            continue
        j = lambda s: "".join(re.findall(r'"(.*)"', s))
        if j(m.group(1)) == r'invalid input value for enum %s: \"%s\"':
            refs = re.findall(r"^#: (.*)$", block, re.M)
            out.append([j(m.group(2)), "fuzzy" in "".join(re.findall(r"^#,.*$", block, re.M)), block.lstrip().startswith("#~"), refs])
    return out


res = {}
jobs = {}
for tag in tags:
    for f in api(f"repos/postgres/postgres/contents/src/backend/po?ref={tag}"):
        if f["name"].endswith(".po"):
            res.setdefault(tag, {})[f["name"][:-3]] = f["sha"]
            if f["sha"] not in cache:
                jobs[f["sha"]] = (tag, "src/backend/po/" + f["name"])
    print(tag, len(res[tag]), file=sys.stderr, flush=True)
print(len(jobs), "blobs", file=sys.stderr, flush=True)


def one(x):
    sha, (tag, path) = x
    return sha, entry(raw(tag, path).decode("utf8"))


with ThreadPoolExecutor(6) as ex:
    for k, (sha, e) in enumerate(ex.map(one, jobs.items())):
        cache[sha] = e
        if k % 40 == 0:
            json.dump(cache, open(cache_f, "w"))
            print(k, file=sys.stderr, flush=True)
json.dump(cache, open(cache_f, "w"))
json.dump(res, open(os.path.join(work, "po2-tags.json"), "w"))

# enum.c in each release: what raises the message, and under which routine
src = {}
for tag in tags:
    t = raw(tag, "src/backend/utils/adt/enum.c").decode("utf8")
    fn = None
    uses = []
    for line in t.split("\n"):
        m = re.match(r"^([a-z_]+)\(", line)
        if m:
            fn = m.group(1)
        if "invalid input value for enum" in line or "ERRCODE_" in line or re.search(r"\b(elog|errmsg_internal)\(", line):
            uses.append((fn, line.strip()))
    src.setdefault(json.dumps(uses), []).append(tag)
for u, ts in src.items():
    print(f"enum.c, {len(ts)} releases, {' '.join(ts)}:")
    for fn, l in json.loads(u):
        print("   ", fn, l)

by = {}
for tag, langs in res.items():
    for lang, sha in langs.items():
        es = cache[sha]
        assert len(es) == 1, (tag, lang, es)
        s, fuzzy, obs, refs = es[0]
        by.setdefault(lang, {}).setdefault((s, fuzzy, obs), []).append(tag)
for lang, d in sorted(by.items()):
    for (s, fuzzy, obs), ts in d.items():
        maj = {}
        for t in ts:
            maj.setdefault(t.split("_")[1], []).append(t)
        print(f"{lang}: {' '.join(f'{k}:{len(v)}' for k, v in maj.items())}{' FUZZY' if fuzzy else ''}{' OBSOLETE' if obs else ''}: {s}   {[hex(ord(c)) for c in s if ord(c) in (0xa0, 0x202f, 0xff1a) or c in '«»„“”「」']}")
print("releases:", len(res), "files:", sum(len(x) for x in res.values()), "distinct files:", len(set(s for x in res.values() for s in x.values())))
json.dump({lang: sorted({s for (s, _, _) in d}) for lang, d in by.items()}, open(os.path.join(work, "msgstr.json"), "w"), ensure_ascii=False, indent=1)
