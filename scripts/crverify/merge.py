"""merge.py — merge check.c reports from interleaved chunks of one row file into one report.

    python3 -I -B scripts/crverify/merge.py report0.txt report1.txt … > report.txt

check.c checks one row at a time, so a row file split by `awk 'NR % k == j'` can be checked on k cores at once
(probe P9's erfc rows run about 500 a second at 1,024 bits). Per name: n and the skipped rows are summed, the max is
the largest chunk's (with its worst input), and the mean and the not-CR share are weighted by n. A chunk's not-CR
count is recovered from its four-decimal percentage, which is exact for chunks under 10⁶ rows.
"""

import re
import sys

ROW = re.compile(r"^(\S+)\s+(\d+)\s+(\S+)\s+(\S+)\s+(\S+)%\s+(x=.*?)(?:\s+\(skipped (\d+)\))?\s*$")


def main() -> int:
    acc = {}
    for path in sys.argv[1:]:
        for line in open(path, encoding="utf-8"):
            m = ROW.match(line.rstrip("\n"))
            if not m:
                continue
            name, n, mx, mean, ncr, worst, skipped = m.groups()
            n = int(n)
            a = acc.setdefault(name, {"n": 0, "max": -1.0, "worst": "", "sum": 0.0, "ncr": 0, "skipped": 0})
            a["n"] += n
            a["sum"] += float(mean) * n
            a["ncr"] += round(float(ncr) * n / 100.0)
            a["skipped"] += int(skipped or 0)
            if float(mx) > a["max"]:
                a["max"], a["worst"] = float(mx), worst
    print("%-14s %8s %12s %10s %9s  %s" % ("function", "n", "max ulp", "mean ulp", "not CR", "worst input"))
    for name in sorted(acc):
        a = acc[name]
        n = a["n"] or 1
        tail = "  (skipped %d)" % a["skipped"] if a["skipped"] else ""
        print("%-14s %8d %12.4g %10.3g %8.4f%%  %s%s"
              % (name, a["n"], a["max"], a["sum"] / n, 100.0 * a["ncr"] / n, a["worst"], tail))
    return 0


if __name__ == "__main__":
    sys.exit(main())
