"""p7_compare.py — join probe P7's outputs into its tables (port plan §5 step 9c, D5).

    python3 -I -B scripts/probes/p7_compare.py build/p7_rust_corpus.tsv build/p7_cyrius.tsv \
        build/p7_rust_read_rt.tsv build/p7_rust_read_default.tsv

Verdicts map serde_json's to pramana's codes: `ok`; a validating constructor's refusal (`data:InvalidParameter`
and the like) to that variant's code; every other serde_json error to DESERIALIZE. Values compare as the twin's
tokens (every float by its bits). Exit status 0; the tables are the result.
"""

import sys
from collections import Counter, defaultdict

CODE = {
    "ok": "ok",
    "data:InvalidParameter": "INVALID_PARAMETER",
    "data:InvalidSample": "INVALID_SAMPLE",
    "data:DimensionMismatch": "DIMENSION_MISMATCH",
}


def expected(verdict: str) -> str:
    return CODE.get(verdict, "DESERIALIZE")


def rows(path: str, tag: str | None = None) -> list[list[str]]:
    out = []
    with open(path, encoding="utf-8") as fh:
        for line in fh:
            p = line.rstrip("\n").split("\t")
            if tag is None or p[0] == tag:
                out.append(p)
    return out


def main() -> int:
    corpus_path, cy_path, rt_path, df_path = sys.argv[1:5]
    rust = {p[0]: p for p in rows(corpus_path)}
    cy = {p[1]: p for p in rows(cy_path, "D")}
    read_rt = {(p[0], p[1]): p for p in rows(rt_path)}
    read_df = {(p[0], p[1]): p for p in rows(df_path)}

    tally = defaultdict(Counter)
    disagree = {"as parsed": [], "lifted": []}
    for rid, r in rust.items():
        c = cy[rid]
        want = expected(r[3])
        for label, col, tcol in (("as parsed", 3, 9), ("lifted", 4, 5)):
            got = c[col]
            if got != want:
                tally[label]["verdict differs"] += 1
                disagree[label].append((rid, r[3], got, r[6][:70]))
            elif want == "ok" and c[tcol] != r[4]:
                tally[label]["value differs"] += 1
                disagree[label].append((rid, "ok, value", "ok, value differs", ""))
            else:
                tally[label]["agrees"] += 1
        if want == "ok" and c[4] == "ok":
            tally["text"]["documents"] += 1
            canon = r[5]
            tally["text"]["fmtf = serde_json"] += c[6] == canon
            tally["text"]["grisu = serde_json"] += c[7] == canon
            tally["text"]["fmtf read back"] += c[8][0] == "1"
            tally["text"]["grisu read back"] += c[8][1] == "1"
            for which in ("fmtf", "grisu"):
                for name, table in (("float_roundtrip", read_rt), ("default", read_df)):
                    q = table[(rid, which)]
                    same = q[2] == "ok" and q[3] == r[4]
                    tally[f"serde_json ({name}) reads {which}"]["same"] += same
                    tally[f"serde_json ({name}) reads {which}"]["differs"] += not same

    print(f"documents: {len(rust)}; serde_json verdicts: {dict(Counter(expected(r[3]) for r in rust.values()))}\n")
    for label in ("as parsed", "lifted"):
        print(f"## {label}: {dict(tally[label])}\n")
        print("| document | serde_json | pramana | serde_json's message |")
        print("|---|---|---|---|")
        for rid, rv, got, msg in sorted(disagree[label]):
            print(f"| `{rid}` | {rv} | {got} | {msg} |")
        print()
    for key in sorted(tally):
        if key not in ("as parsed", "lifted"):
            print(f"{key}: {dict(tally[key])}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
