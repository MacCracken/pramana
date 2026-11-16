#!/usr/bin/env bash
# run.sh — the correct-rounding check of pramana's primitives (M1 part b; port plan D2, ADR 009) and of its erf and
# erfc (M2 part b1; D3 = C).
#
# For each row set, builds its generator for x86_64, PE and aarch64, compares the three outputs (sha256), and checks
# every row against MPFR at 1,024 bits (scripts/port-baseline/check.c, ten interleaved chunks merged by merge.py). A
# function passes when its rows are all correctly rounded: max 0.5 ulp and 0% not CR.
# - `primitives`: scripts/crverify/rows.cyr, the ten correctly rounded primitives.
# - `erf`: scripts/crverify/rows_erf.cyr, erf and erfc (CORE-MATH's, src/crerf.cyr) with their accurate paths. These
#   rows are also compared bit for bit with CORE-MATH's own C build (hardware FMA), and include CORE-MATH's worst
#   cases. Needs CORE-MATH's erf.c, erf.wc, erfc.c and erfc.wc in build/coremath/ (scripts/vendor_coremath.py).
# - `ncdf`: scripts/crverify/rows_ncdf.cyr, Φ (M2 part b2), whose bound is ≤ 1 ulp (ADR 009), not correct rounding:
#   its report's max ulp is the figure to read.
# Needs gcc with MPFR (and FMA for CORE-MATH's build), wine and qemu-aarch64.
#   bash scripts/crverify/run.sh               # every set
#   bash scripts/crverify/run.sh erf           # one
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
work=$root/build/crverify
mkdir -p "$work"
cd "$root"
sets=("$@")
[ ${#sets[@]} -gt 0 ] || sets=(primitives erf ncdf)
quiet() { grep -v -e '^note' -e '^OK' -e 'warning:lib/' -e 'unreachable fns' || true; }
gcc -O2 -o "$work/check" scripts/port-baseline/check.c -lmpfr -lgmp -lm

# verify NAME SOURCE: the three targets' outputs, then MPFR.
verify() {
    local name=$1 src=$2 out=$work/$1
    mkdir -p "$out/chunks"
    cyrius build "$src" "$out/rows" 2>&1 | quiet
    cyrius build --win "$src" "$out/rows.exe" 2>&1 | quiet
    cyrius build --aarch64 "$src" "$out/rows_a64" 2>&1 | quiet
    "$out/rows" > "$out/rows.txt"
    echo "## $name: $(wc -l < "$out/rows.txt") rows"
    printf 'x86_64  %s\n' "$(sha256sum < "$out/rows.txt" | cut -c1-16)"
    printf 'PE      %s\n' "$(wine "$out/rows.exe" 2>/dev/null | tr -d '\r' | sha256sum | cut -c1-16)"
    printf 'aarch64 %s\n' "$(qemu-aarch64 "$out/rows_a64" | sha256sum | cut -c1-16)"
    if [ "$name" = erf ]; then
        "$work/cmp_coremath" < "$out/rows.txt"
    fi
    local pids=()
    for k in 0 1 2 3 4 5 6 7 8 9; do
        awk -v k="$k" 'NR % 10 == k' "$out/rows.txt" > "$out/chunks/rows$k.txt"
        "$work/check" < "$out/chunks/rows$k.txt" > "$out/chunks/report$k.txt" &
        pids+=($!)
    done
    for p in "${pids[@]}"; do wait "$p"; done
    echo "## MPFR $(pkg-config --modversion mpfr 2>/dev/null || echo '?')"
    python3 -I -B scripts/port-baseline/merge.py "$out"/chunks/report?.txt | tee "$out/report.txt"
}

for s in "${sets[@]}"; do
    case $s in
    primitives)
        python3 -I -B scripts/crverify/consts.py .
        verify primitives scripts/crverify/rows.cyr
        ;;
    erf)
        cm=$root/build/coremath
        for f in erf.c erf.wc erfc.c erfc.wc; do
            [ -f "$cm/$f" ] || { echo "missing $cm/$f: fetch CORE-MATH's sources (scripts/vendor_coremath.py)" >&2; exit 1; }
        done
        python3 -I -B scripts/vendor_coremath.py . --check
        # The worst-case lists as one bit pattern per line, for rows_erf.cyr.
        for f in erf erfc; do
            grep -v '^#' "$cm/$f.wc" | python3 -I -B -c '
import math, struct, sys
special = {"+inf": math.inf, "-inf": -math.inf, "+0": 0.0, "-0": -0.0}
nan = {"+nan": 0x7FF8000000000000, "-nan": 0xFFF8000000000000, "+snan": 0x7FF4000000000000, "-snan": 0xFFF4000000000000}
for line in sys.stdin:
    s = line.split("#")[0].strip()
    if not s:
        continue
    if s in nan:
        print("%016x" % nan[s])
        continue
    v = special[s] if s in special else float.fromhex(s)
    print("%016x" % struct.unpack("<Q", struct.pack("<d", v))[0])
' > "$work/${f}_wc.bits"
        done
        # CORE-MATH's own build, with hardware FMA, for the bit-for-bit comparison.
        gcc -O2 -mfma -ffp-contract=off -c -Dcr_erf=orig_erf -o "$work/orig_erf.o" "$cm/erf.c"
        gcc -O2 -mfma -ffp-contract=off -c -Dcr_erfc=orig_erfc -o "$work/orig_erfc.o" "$cm/erfc.c"
        gcc -O2 -ffp-contract=off -o "$work/cmp_coremath" scripts/crverify/cmp_coremath.c "$work/orig_erf.o" \
            "$work/orig_erfc.o" -lm
        verify erf scripts/crverify/rows_erf.cyr
        ;;
    ncdf)
        verify ncdf scripts/crverify/rows_ncdf.cyr
        ;;
    *)
        echo "unknown set $s (primitives, erf, ncdf)" >&2
        exit 1
        ;;
    esac
done
