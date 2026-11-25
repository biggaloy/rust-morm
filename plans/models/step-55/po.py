"""enum_in's message in every translation Postgres 15 to 18 ship, read from the branches' .po files"""
import re, urllib.error, urllib.request
from concurrent.futures import ThreadPoolExecutor

H = {"User-Agent": "po-check"}
GET = lambda u: urllib.request.urlopen(urllib.request.Request(u, headers=H)).read().decode("utf8")
out = {}
LANGS = "af ar az be bg bn bs ca cs da de el en_GB eo es et eu fa fi fr ga gl gu he hi hr hu hy id is it ja ka kk km kn ko ku lt lv mk ml mn ms my nb ne nl nn pa pl ps pt pt_BR ro ru sk sl sq sr sv sw ta te tg th tr uk ur uz vi zh_CN zh_HK zh_TW".split()
for br in ["REL_15_STABLE", "REL_16_STABLE", "REL_17_STABLE", "REL_18_STABLE"]:
    def one(x):
        try:
            return x, GET(f"https://raw.githubusercontent.com/postgres/postgres/{br}/src/backend/po/{x}.po")
        except urllib.error.HTTPError:
            return x, None
    with ThreadPoolExecutor(8) as ex:
        got = [(x, t) for x, t in ex.map(one, LANGS) if t]
    print(br, len(got), " ".join(x for x, _ in got))
    for x, t in got:
        m = re.search(r'(?m)^((?:#.*\n)*)msgid "invalid input value for enum %s: \\"%s\\""\nmsgstr "(.*)"$', t)
        out.setdefault(x, {})[br[4:6]] = (m.group(2) + (" FUZZY" if "fuzzy" in m.group(1) else "")) if m else None
FIT = re.compile(r'^([^:：]*%s[^:："»«“„]*[:：][   ]?(?:\\"|[»«“„「])[   ]?)%s[   ]?(?:\\"|[»«”“」])$')
for k, v in sorted(out.items()):
    s = set(v.values())
    print(k, {b: ("same" if len(s) == 1 else x) for b, x in v.items()} if len(s) > 1 else "15-18", s, "FITS" if all(x and FIT.match(x) for x in s if x) else "NO FIT")
