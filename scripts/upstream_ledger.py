#!/usr/bin/env python3
"""Check the conversion ledger against the code (port plan §10, ADR 008).

    python3 -I -B scripts/upstream_ledger.py [--check]

The ledger is the table in docs/development/pramana-port-plan.md §10, after pravash's: one row per local stand-in for a
gap in ganita, hisab, bayan, hoosh or the Cyrius stdlib. The status column is `open`, `conditional` or `closed`, each
stand-in file is a backticked `src/…` path, and each request is a backticked `<repo>:<path>`. The block after the
table, "**Also filed at step 10**", lists requests with no stand-in (a coding rule covers each), in the same
`<repo>:<path>` form. This checks:

1. every `UPSTREAM(Ln)` note in `src/` names a row of the ledger that is not closed;
2. every open row whose stand-in file exists carries at least one `UPSTREAM(Ln)` note in that file, so temporary
   code is always marked;
3. every request an open row names exists, at `<parent>/<repo>/<path>`, and so does every request in the block after
   the table. The upstream repos are siblings of this one; where a sibling isn't checked out, as on CI, its requests
   are reported as unverified, not failed;
4. no row is listed twice, and a closed row has no notes left in `src/`.

It prints the ledger's state either way; with `--check` (the gate) it exits 1 on any failure.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REPOS = ROOT.parent
PLAN = ROOT / "docs" / "development" / "pramana-port-plan.md"
NOTE = re.compile(r"UPSTREAM\((L\d+)\)")
REQUEST = re.compile(r"^[a-z0-9_-]+:docs/.+\.md$")


def ledger():
    """({id: {status, standins, requests}}, [requests filed with no stand-in], [failures]) from §10."""
    rows = {}
    fails = []
    text = PLAN.read_text(encoding="utf-8")
    at = text.find("**The ledger.**")
    if at < 0:
        return rows, [], ["the plan has no '**The ledger.**' table"]
    lines = text[at:].splitlines()
    i = 0
    for i, line in enumerate(lines):
        if line.startswith("**Also filed"):
            break
        m = re.match(r"^\| (L\d+) \| (open|conditional|closed) \| (.*)$", line)
        if not m:
            continue
        rid, status, rest = m.groups()
        if rid in rows:
            fails.append(f"{rid} is listed twice")
        ticks = re.findall(r"`([^`]+)`", rest)
        standins = sorted({t for t in ticks if t.startswith("src/") and t.endswith(".cyr")})
        requests = sorted({t for t in ticks if REQUEST.match(t)})
        rows[rid] = {"status": status, "standins": standins, "requests": requests}
    others = []
    for line in lines[i:]:
        if line.startswith("---") or line.startswith("## "):
            break
        others.extend(t for t in re.findall(r"`([^`]+)`", line) if REQUEST.match(t))
    return rows, sorted(set(others)), fails


def notes_in_src():
    """{id: [file:line, ...]} for every UPSTREAM(Ln) note under src/."""
    found = {}
    for path in sorted((ROOT / "src").rglob("*.cyr")):
        for n, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
            for rid in NOTE.findall(line):
                found.setdefault(rid, []).append(f"{path.relative_to(ROOT)}:{n}")
    return found


def request_exists(req, unverifiable, fails, owner):
    repo, rel = req.split(":", 1)
    if not (REPOS / repo).is_dir():
        unverifiable.add(repo)  # CI has no sibling checkouts; checked locally
    elif not (REPOS / repo / rel).exists():
        fails.append(f"{owner}: the request {req} does not exist")


def main():
    check = "--check" in sys.argv[1:]
    rows, others, fails = ledger()
    notes = notes_in_src()
    unverifiable = set()
    if not rows:
        fails.append("no ledger rows found")

    for rid, where in sorted(notes.items()):
        if rid not in rows:
            fails.append(f"{where[0]}: UPSTREAM({rid}) names no ledger row")
        elif rows[rid]["status"] == "closed":
            fails.append(f"{where[0]}: UPSTREAM({rid}) is left over: the row is closed")

    for rid, row in sorted(rows.items(), key=lambda kv: int(kv[0][1:])):
        if row["status"] != "open":
            continue
        for f in row["standins"]:
            path = ROOT / f
            if path.exists() and f"UPSTREAM({rid})" not in path.read_text(encoding="utf-8"):
                fails.append(f"{f} exists but carries no UPSTREAM({rid}) note")
        if not row["requests"]:
            fails.append(f"{rid} is open but names no upstream request")
        for req in row["requests"]:
            request_exists(req, unverifiable, fails, rid)
    for req in others:
        request_exists(req, unverifiable, fails, "also filed")

    for rid, row in sorted(rows.items(), key=lambda kv: int(kv[0][1:])):
        present = [f for f in row["standins"] if (ROOT / f).exists()]
        marked = len(notes.get(rid, []))
        print(
            f"{rid:>4}  {row['status']:<11} stand-ins {len(row['standins'])} "
            f"(present {len(present)}), notes {marked}, requests {len(row['requests'])}"
        )
    print(f"also filed: {len(others)} requests with no stand-in")
    if unverifiable:
        print(f"note: no local checkout of {', '.join(sorted(unverifiable))}: those requests were not verified")
    for f in fails:
        print("FAIL", f)
    total_notes = sum(len(v) for v in notes.values())
    print(f"{len(rows)} ledger rows, {total_notes} UPSTREAM notes in src/, {len(fails)} failures")
    return 1 if (check and fails) else 0


if __name__ == "__main__":
    sys.exit(main())
