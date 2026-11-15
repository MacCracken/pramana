"""p0_compare.py — join p0_semantics' Cyrius and Rust outputs into a Markdown table (pravash's, adapted).

    python3 -I -B scripts/probes/p0_compare.py build/p0_cyrius.tsv build/p0_rust.tsv

For float types the last column is the ulp distance from Rust's value (0 = same bits). Two NaNs are reported as
"both NaN", noting whether their sign or payload differ. For integer types it is "same" or "DIFFERS" at the type's
width. Exit status 0; the table is the result.
"""

import sys

WIDTH = {"f64": 64, "u32": 32, "u64": 64, "i64": 64}


def ordered(bits: int) -> int:
    """Map f64 bits to a monotone integer, so the ulp distance is a difference (+0 and -0 are 0 apart)."""
    sign = 1 << 63
    return -(bits & (sign - 1)) if bits & sign else bits


def is_nan(bits: int) -> bool:
    return (bits >> 52) & 0x7FF == 0x7FF and bits & ((1 << 52) - 1) != 0


def main() -> int:
    cy_path, rs_path = sys.argv[1], sys.argv[2]
    rust = {}
    with open(rs_path, encoding="utf-8") as fh:
        for line in fh:
            case, ty, bits = line.rstrip("\n").split("\t")
            rust[case] = (ty, int(bits, 16))
    rows, seen = [], set()
    with open(cy_path, encoding="utf-8") as fh:
        for line in fh:
            case, variant, ty, bits = line.rstrip("\n").split("\t")
            seen.add(case)
            mask = (1 << WIDTH[ty]) - 1
            cy = int(bits, 16) & mask
            rty, rbits = rust[case]
            rb = rbits & mask
            if ty == "f64":
                if is_nan(cy) and is_nan(rb):
                    verdict = "both NaN" + ("" if cy == rb else " (sign or payload differs)")
                else:
                    d = abs(ordered(cy) - ordered(rb))
                    verdict = f"**{d} ulp**" if d else "same bits"
                    if d == 0 and cy != rb:
                        verdict = "**sign of zero differs**"
            else:
                verdict = "same" if cy == rb else "**DIFFERS**"
            rows.append(f"| `{case}` | {variant} | `{rb:#x}` | `{cy:#x}` | {verdict} |")
    print("| Case | Cyrius | Rust bits | Cyrius bits | Result |")
    print("|---|---|---|---|---|")
    print("\n".join(rows))
    missing = sorted(set(rust) - seen)
    if missing:
        print(f"\nRust cases with no Cyrius probe: {', '.join(missing)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
