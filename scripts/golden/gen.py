"""Writes the golden corpus, tests/golden/<module>.tsv, from validate.py's models of pramana 1.3.0 (ADR 010):
Python on glibc for 1.3.0's own bits, and MPFR for everything the port must match more accurately than glibc.

    python3 -I -B scripts/golden/gen.py .                    # write tests/golden/ (then validate.py . --write-cr)
    python3 -I -B scripts/golden/gen.py . --only dist,json   # some modules
    python3 -I -B scripts/golden/gen.py . --compare DIR      # compare fn, class, inputs, outputs with DIR's files
    python3 -I -B scripts/golden/gen.py . --digests          # tests/golden/ against scripts/golden/digests.txt
    python3 -I -B scripts/golden/gen.py . --write-digests    # record them (after validate.py --write-cr)

The corpus is generated, never committed: tests/golden/*.tsv is in .gitignore."""
import importlib
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

MODULES = ["bridge", "combinatorics", "constants", "debug", "descriptive", "dist", "fmt", "hypothesis", "json", "linalg", "math", "regression", "rng", "special", "timeseries"]


def emit(name):
    return importlib.import_module(f"emit.m_{name}").emit()


def _nan_free(field):
    """A field's tokens with every NaN as `f:nan`: the harness matches any NaN with any NaN (class A)."""
    out = []
    for t in field.split(" "):
        if t.startswith("f:") and len(t) == 18:
            w = int(t[2:], 16)
            if (w >> 52) & 0x7FF == 0x7FF and w & ((1 << 52) - 1):
                t = "f:nan"
        out.append(t)
    return " ".join(out)


def rows_of(text):
    rows = []
    for line in text.split("\n"):
        if line and not line.startswith("#"):
            c = line.split("\t")[:4]
            rows.append([c[0], c[1], _nan_free(c[2]), _nan_free(c[3])])
    return rows


def digests(root):
    """The sha256 of every file in tests/golden/: the corpus, its overlays and modules.txt, sorted by name."""
    import hashlib
    out = root / "tests" / "golden"
    return [f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}\n"
            for p in sorted(out.iterdir(), key=lambda p: p.name) if p.is_file()]


DIGEST_HEAD = ("# sha256 of every file gen.py and validate.py --write-cr write to tests/golden/ (ADR 010). The corpus\n"
               "# is never committed; these digests are, so a machine whose libm gives other bits is caught.\n")


def check_digests(root, write):
    path = HERE / "digests.txt"
    got = digests(root)
    if write:
        path.write_text(DIGEST_HEAD + "".join(got), encoding="utf-8")
        print(f"wrote scripts/golden/digests.txt ({len(got)} files)")
        return 0
    want = [l for l in path.read_text(encoding="utf-8").splitlines(keepends=True) if not l.startswith("#")]
    if got == want:
        print(f"tests/golden/: {len(got)} files, every digest as recorded")
        return 0
    names = {l.split()[1]: l for l in want}
    for l in got:
        if names.pop(l.split()[1], None) != l:
            print("differs or unrecorded:", l.split()[1])
    for n in names:
        print("missing:", n)
    return 1


def main(argv):
    root = Path(argv[1] if len(argv) > 1 and not argv[1].startswith("--") else ".").resolve()
    if "--digests" in argv or "--write-digests" in argv:
        return check_digests(root, "--write-digests" in argv)
    only = MODULES
    if "--only" in argv:
        only = argv[argv.index("--only") + 1].split(",")
    compare = Path(argv[argv.index("--compare") + 1]) if "--compare" in argv else None
    from emit.tsv import require_platform
    require_platform()
    out = root / "tests" / "golden"
    out.mkdir(parents=True, exist_ok=True)
    bad = 0
    for name in only:
        t0 = time.time()
        m = emit(name)
        text = m.render()
        if compare is not None:
            want = rows_of((compare / f"{name}.tsv").read_text(encoding="utf-8"))
            got = rows_of(text)
            diff = next((k for k, (a, b) in enumerate(zip(got, want)) if a != b), None)
            if len(got) != len(want) or diff is not None:
                bad += 1
                k = diff if diff is not None else min(len(got), len(want))
                print(f"{name}: {len(got)} rows, {len(want)} recorded; first difference at row {k}")
                if k < len(got):
                    print("  generated:", "\t".join(got[k])[:300])
                if k < len(want):
                    print("  recorded: ", "\t".join(want[k])[:300])
            else:
                print(f"{name}: {len(got)} rows, identical in fn, class, inputs and outputs ({time.time() - t0:.0f} s)")
        else:
            (out / f"{name}.tsv").write_text(text, encoding="utf-8")
            print(f"wrote tests/golden/{name}.tsv ({len(m.rows)} rows, {time.time() - t0:.0f} s)")
    if compare is None and "constants" in only:
        from emit.m_constants import sources
        cyr, rows = sources()
        (root / "src" / "constants.cyr").write_text(cyr, encoding="utf-8")
        (root / "tests" / "constants_rows.cyr").write_text(rows, encoding="utf-8")
        print("wrote src/constants.cyr and tests/constants_rows.cyr")
    if compare is None and only == MODULES:
        (out / "modules.txt").write_text(
            "# pramana golden corpus: one module per line; tests/parity.tcyr replays each\n"
            + "".join(f"{n}\n" for n in MODULES), encoding="utf-8")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
