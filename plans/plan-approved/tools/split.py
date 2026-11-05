#!/usr/bin/env python3
"""One-time generator: splits ../../rust-morm-plan.md into step files.

Every block of the frozen plan becomes one requirement (R-nnnn), copied word
for word into exactly one place. Items in the long test lists of section 13
become test obligations (T-nnnn). Nothing is summarised, so nothing can be lost.

Run from this directory. After plan-approved/ goes live, edit the step files
directly: this script is kept as the record of how they were made.
"""
import hashlib
import json
import re
from collections import OrderedDict, defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
MASTER = ROOT.parent / "rust-morm-plan.md"

from steps_data import STEPS, beside, context_for, sub  # noqa: E402

# ---------------------------------------------------------------- parsing


def parse(text):
    """Yield dicts: kind (bullet|row|code|para), h2, h3, text, head, line."""
    lines = text.split("\n")
    h2 = h3 = ""
    h4 = ""
    units = []
    i = 0
    n = len(lines)
    para = []
    para_at = 0

    def flush():
        nonlocal para
        if para:
            units.append(dict(kind="para", h2=h2, h3=h3, h4=h4, text="\n".join(para),
                              head=None, line=para_at + 1))
            para = []

    while i < n:
        ln = lines[i]
        if ln.startswith("## "):
            flush()
            h2, h3, h4 = ln[3:].strip(), "", ""
            i += 1
            continue
        if ln.startswith("### "):
            flush()
            h3, h4 = ln[4:].strip(), ""
            i += 1
            continue
        if ln.startswith("# ") or ln.startswith("> ") or ln.strip() == "---":
            flush()
            i += 1
            continue
        if ln.startswith("```"):
            flush()
            start = i
            i += 1
            while not lines[i].startswith("```"):
                i += 1
            i += 1
            units.append(dict(kind="code", h2=h2, h3=h3, h4=h4,
                              text="\n".join(lines[start:i]), head=None, line=start + 1))
            continue
        if ln.startswith("|"):
            flush()
            head = lines[i] + "\n" + lines[i + 1]
            i += 2
            while i < n and lines[i].startswith("|"):
                units.append(dict(kind="row", h2=h2, h3=h3, h4=h4, text=lines[i],
                                  head=head, line=i + 1))
                i += 1
            continue
        m = re.match(r"^(\s*)(-|\d+\.)\s", ln)
        if m:
            flush()
            units.append(dict(kind="bullet", h2=h2, h3=h3, h4=h4, text=ln.strip(),
                              head=None, line=i + 1, nested=len(m.group(1)) > 0))
            i += 1
            continue
        if not ln.strip():
            flush()
            i += 1
            continue
        if not para:
            para_at = i
        para.append(ln)
        # a bold lead-in paragraph names the block below it
        i += 1
    flush()
    return units


def split_commas(s):
    """Split on commas outside backticks."""
    out, cur, tick = [], "", False
    for ch in s:
        if ch == "`":
            tick = not tick
        if ch == "," and not tick:
            out.append(cur.strip())
            cur = ""
        else:
            cur += ch
    if cur.strip():
        out.append(cur.strip())
    return out


# ---------------------------------------------------------------- routing

from routes import route, route_test  # noqa: E402
from overrides import apply_pieces, final_fix, refine, TEST_SPLIT, EXTRA_TESTS  # noqa: E402


def kind_of(u, step):
    t = u["text"]
    if step in ("contracts", "index") or u.get("forced_info"):
        return "info"
    if u["h2"].startswith(("1.", "2.", "15.", "17.")):
        return "info"
    if u["h2"].startswith("14."):
        return "doc"
    return "code"


def test_type(t):
    tl = t.lower()
    if "compile error" in tl or "doesn't compile" in tl or "won't compile" in tl:
        return "trybuild"
    if "lint" in tl or "`doctor`" in tl or "morm-admin" in tl:
        return "integration"
    return "integration"


def main():
    text = MASTER.read_text(encoding="utf-8")
    units = parse(text)

    reqs, tests, plain = [], [], []
    for u in units:
        # section 13: the two long lists become single test obligations
        if u["h2"].startswith("13.") and u["kind"] == "bullet" and (
                u["text"].startswith("- **Compile-fail") or u["text"].startswith("- **Integration on real")):
            lead, rest = u["text"].split(":**", 1)
            tkind = "trybuild" if "Compile-fail" in lead else "integration"
            lead_rest = rest
            if tkind == "trybuild":
                lead_rest = rest.split("including", 1)[1]
            items = split_commas(lead_rest.rstrip(". "))
            # re-join items that only make sense together
            merged = []
            for it in items:
                it = it.strip()
                if it.startswith("and "):
                    it = it[4:]
                if merged and (it.startswith(("a deferrable one", "an ambiguous one", "with ", "update and delete form"))
                               or it == "hooks" or it.startswith("hooks and") or it == "read and JSON" or it in ("`numeric`", "`bytea` and timestamps", "1 and between")
                               or it.startswith("including the row deleted")
                               or re.match(r"^(null and value|`?value`? states|or )", it)
                               or it in ("`Target`)",)):
                    merged[-1] += ", " + it
                else:
                    merged.append(it)
            for it in merged:
                if it:
                    tests.append(dict(text=it, tkind=tkind, line=u["line"]))
            continue
        if u["h2"].startswith("13.") and u["kind"] == "bullet" and u["h3"] != "GitHub Actions (`.github/workflows/`)":
            tests.append(dict(text=re.sub(r"^-\s*", "", u["text"]), tkind="suite", line=u["line"]))
            continue
        plain.append(u)

    plain = final_fix(apply_pieces(plain, lambda u: refine(u, route(u))))
    for u in plain:
        u["rkind"] = kind_of(u, u["step"])
        reqs.append(u)

    for i, r in enumerate(reqs, 1):
        r["id"] = f"R-{i:04d}"
    final = []
    for t in tests:
        if t["text"] in TEST_SPLIT:
            for txt, step in TEST_SPLIT[t["text"]]:
                final.append(dict(t, text=txt, step=step))
        else:
            final.append(dict(t, step=route_test(t["text"], t["tkind"])))
    for txt, tk, step in EXTRA_TESTS:
        final.append(dict(text=txt, tkind=tk, line=0, step=step))
    tests = final
    for i, t in enumerate(tests, 1):
        t["id"] = f"T-{i:04d}"

    write_steps(reqs, tests)
    write_coverage(reqs, tests, text)
    write_manifest(reqs, tests, text)

    by = defaultdict(int)
    for r in reqs:
        by[r["step"]] += 1
    tby = defaultdict(int)
    for t in tests:
        tby[t["step"]] += 1
    for k in list(STEPS) + ["contracts", "index"]:
        print(f"{k:28} reqs {by.get(k,0):4}  tests {tby.get(k,0):4}")
    print("total", len(reqs), len(tests))


def render_requirements(rs):
    out = []
    groups = OrderedDict()
    for r in rs:
        key = (r["h2"], r["h3"])
        groups.setdefault(key, []).append(r)
    for (h2, h3), items in groups.items():
        title = h2.split(". ", 1)[-1] + (" / " + h3 if h3 else "")
        out.append(f"### {title}\n")
        i = 0
        while i < len(items):
            r = items[i]
            if r["kind"] == "row":
                head = r["head"]
                cols = head.split("\n")
                out.append("| ID " + cols[0])
                out.append("|---" + cols[1])
                while i < len(items) and items[i]["kind"] == "row" and items[i]["head"] == head:
                    out.append(f"| {items[i]['id']} " + items[i]["text"])
                    i += 1
                out.append("")
                continue
            if r["kind"] == "code":
                out.append(f"**{r['id']}**\n\n{r['text']}\n")
            elif r["kind"] == "para":
                out.append(f"**{r['id']}** {r['text']}\n")
            else:
                body = re.sub(r"^(-|\d+\.)\s+", "", r["text"])
                out.append(f"- **{r['id']}** {body}")
                nxt = items[i + 1] if i + 1 < len(items) else None
                if nxt is None or nxt["kind"] != "bullet":
                    out.append("")
            i += 1
    return "\n".join(out)


def write_steps(reqs, tests):
    d = ROOT / "steps"
    d.mkdir(exist_ok=True)
    for old in d.glob("*.md"):
        old.unlink()
    for key, s in STEPS.items():
        rs = [r for r in reqs if r["step"] == key]
        ts = [t for t in tests if t["step"] == key]
        L = [f"# Step {s['n']:02d}: {s['title']}\n"]
        L.append(f"- **Phase:** {s['phase']}")
        L.append(f"- **Depends on:** {', '.join(f'{STEPS[x]['n']:02d}' for x in s['deps']) or 'nothing'}")
        L.append(f"- **Can run beside:** {', '.join(f'{STEPS[x]['n']:02d}' for x in beside(key)) or 'nothing'}")
        L.append(f"- **Crates:** {s['crates']}")
        L.append(f"- **PR approval:** {s.get('approve', 'CI and an independent review')}")
        L.append(f"\nRead `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.\n")
        L.append(f"## Goal\n\n{s['goal']}\n")
        L.append("## Deliverables\n")
        L.extend(f"- {x}" for x in s["deliver"])
        L.append("")
        L.append("## End-to-end check\n")
        L.append(s["e2e"] + "\n")
        if s.get("extra"):
            L.append(s["extra"].strip() + "\n")
        ctx = [u for u in context_for(key, reqs)]
        if ctx:
            L.append("## Context from other steps\n")
            L.append("Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.\n")
            L.extend(("> " + line) if line else ">" for u in ctx for line in (u["text"] + "\n").split("\n"))
            L.append("")
        L.append("## Out of scope\n")
        L.extend(f"- {x}" for x in s["out"])
        L.append("")
        L.append("## Requirements\n")
        L.append("Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as \"section 8\" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.\n")
        L.append(render_requirements(rs) if rs else "None beyond the deliverables above.\n")
        L.append("## Test obligations\n")
        if ts:
            L.append("| ID | Type | Obligation |\n|---|---|---|")
            for t in ts:
                L.append(f"| {t['id']} | {t['tkind']} | {t['text'].replace('|', chr(92) + '|')} |")
            L.append("")
        else:
            L.append("None beyond one tagged test per requirement.\n")
        (d / f"{s['n']:02d}-{key}.md").write_text(sub("\n".join(L)).rstrip() + "\n", encoding="utf-8")


def short(t, n=90):
    t = re.sub(r"\s+", " ", t).replace("|", "\\|")
    return t[:n] + ("…" if len(t) > n else "")


def write_coverage(reqs, tests, text):
    L = ["# Coverage\n",
         "Generated from the frozen plan. Each requirement and test obligation lives in exactly one step. "
         "`tools/check_coverage.py` fails if that stops being true, and with `--code` it fails if a `code` "
         "requirement of a finished step has no test tagged `covers: R-nnnn`.\n",
         "Kinds: `code` needs a tagged test, `doc` is checked in the README step, `info` is context with nothing to build.\n",
         "## Requirements\n", "| ID | Kind | Step | Plan line | Text |", "|---|---|---|---|---|"]
    for r in reqs:
        st = r["step"] if r["step"] in ("contracts", "index") else f"{STEPS[r['step']]['n']:02d}"
        L.append(f"| {r['id']} | {r['rkind']} | {st} | {r['line']} | {short(r['text'])} |")
    L += ["", "## Test obligations\n", "| ID | Type | Step | Text |", "|---|---|---|---|"]
    for t in tests:
        L.append(f"| {t['id']} | {t['tkind']} | {STEPS[t['step']]['n']:02d} | {short(t['text'])} |")
    (ROOT / "coverage.md").write_text("\n".join(L) + "\n", encoding="utf-8")


def block_of(r):
    """The exact text a requirement has in its file."""
    if r["kind"] == "row":
        return f"| {r['id']} " + r["text"]
    if r["kind"] == "code":
        return f"**{r['id']}**\n\n{r['text']}"
    if r["kind"] == "para":
        return f"**{r['id']}** {r['text']}"
    return f"- **{r['id']}** " + re.sub(r"^(-|\d+\.)\s+", "", r["text"])


def write_manifest(reqs, tests, text):
    m = dict(
        master_sha256=hashlib.sha256(text.encode()).hexdigest(),
        reqs=[dict(id=r["id"], kind=r["rkind"], step=r["step"], line=r["line"],
                   text=r["text"], block=block_of(r)) for r in reqs],
        tests=[dict(id=t["id"], step=t["step"], kind=t["tkind"], line=t["line"],
                    text=t["text"].replace("|", "\\|")) for t in tests],
    )
    (HERE / "manifest.json").write_text(json.dumps(m, indent=0) + "\n", encoding="utf-8")
    # info blocks that contracts.md and INDEX.md quote
    for key, fn in (("contracts", "_contracts_info.md"), ("index", "_index_info.md")):
        rs = [r for r in reqs if r["step"] == key]
        (HERE / fn).write_text(render_requirements(rs), encoding="utf-8")


if __name__ == "__main__":
    main()
