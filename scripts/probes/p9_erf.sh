#!/usr/bin/env bash
# p9_erf.sh — probe P9, the FMA-free CORE-MATH erf/erfc port (port plan §5 step 9e): every check, from the sources.
#
# Needs CORE-MATH's src/binary64/erf/erf.c, erf/erf.wc, erfc/erfc.c and erfc/erfc.wc in build/p9/coremath/ (MIT;
# fetched, never committed), gcc with MPFR, qemu-aarch64 and wine.
# 1. The committed tables (p9_erf_data.cyr) are what p9_coremath_tables.py makes from those sources.
# 2. Stage 1, in C: emu.h's exact fma against hardware fma on 10^8 triples; the FMA-free build of erf and erfc
#    against CORE-MATH's own, on both worst-case lists and 4·10^7 arguments.
# 3. Stage 2, the Cyrius port: its rows against CORE-MATH's own build bit for bit (cmp2), the same rows on x86_64,
#    PE and aarch64 (sha256), and every row against MPFR at 1,024 bits (check.c).
# 4. The cost: p9_erf.bcyr and the C builds on the same regions, pinned.
#   bash scripts/probes/p9_erf.sh
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
cm=$root/build/p9/coremath
s1=$root/build/p9/stage1
c=$root/scripts/probes/p9_erf_c
mkdir -p "$s1"
for f in erf.c erf.wc erfc.c erfc.wc; do
    [ -f "$cm/$f" ] || { echo "missing $cm/$f: fetch CORE-MATH's src/binary64/erf{,c}/ files first" >&2; exit 1; }
done
say() { printf '\n=== %s\n' "$*"; }

say "1. the tables match the sources"
python3 -I -B "$root/scripts/probes/p9_coremath_tables.py" "$cm/erf.c" "$cm/erfc.c" \
    | cmp - "$root/scripts/probes/p9_erf_data.cyr" && echo "p9_erf_data.cyr: current"

say "2. stage 1 (C)"
gcc -O2 -mfma -ffp-contract=off -I"$c" -o "$s1/fmatest" "$c/fmatest.c" -lm
"$s1/fmatest"
gcc -O2 -mfma -ffp-contract=off -c -Dcr_erf=orig_erf -o "$s1/orig_erf.o" "$cm/erf.c"
gcc -O2 -mfma -ffp-contract=off -c -Dcr_erfc=orig_erfc -o "$s1/orig_erfc.o" "$cm/erfc.c"
# erf's tiny path ends with an fma whose product can be subnormal: emu_fma_tiny rounds that sum exactly instead.
sed 's/double res = __builtin_fma (l, 0x1p-106, y);/double res = emu_fma_tiny (l, y);/' "$cm/erf.c" > "$s1/erf_s1.c"
grep -q emu_fma_tiny "$s1/erf_s1.c"
gcc -O2 -mno-fma -ffp-contract=off -I"$c" -include fmaoff.h -c -Dcr_erf=nf_erf -o "$s1/nf_erf.o" "$s1/erf_s1.c"
gcc -O2 -mno-fma -ffp-contract=off -I"$c" -include fmaoff.h -c -Dcr_erfc=nf_erfc -o "$s1/nf_erfc.o" "$cm/erfc.c"
if objdump -d "$s1/nf_erf.o" "$s1/nf_erfc.o" | grep -q -i 'vfmadd\|vfmsub\|vfnmadd'; then
    echo "the FMA-free build contains FMA instructions" >&2
    exit 1
fi
gcc -O2 -ffp-contract=off -o "$s1/cmp" "$c/cmp.c" "$s1"/orig_erf.o "$s1"/orig_erfc.o "$s1"/nf_erf.o "$s1"/nf_erfc.o -lm
"$s1/cmp" "$cm/erf.wc" "$cm/erfc.wc"

say "3. stage 2 (the Cyrius port)"
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
' > "$root/build/p9/${f}_wc.bits"
done
cd "$root"
export CYRIUS_ALLOW_ABSOLUTE_INCLUDES=1
quiet() { grep -v -e '^note' -e '^OK' -e 'warning:lib/' || true; }
cyrius build scripts/probes/p9_erf.cyr build/p9/p9_erf 2>&1 | quiet
cyrius build --win scripts/probes/p9_erf.cyr build/p9/p9_erf.exe 2>&1 | quiet
cyrius build --aarch64 scripts/probes/p9_erf.cyr build/p9/p9_erf_a64 2>&1 | quiet
./build/p9/p9_erf > build/p9/erf_rows.txt
gcc -O2 -o "$s1/cmp2" "$c/cmp2.c" "$s1"/orig_erf.o "$s1"/orig_erfc.o -lm
"$s1/cmp2" < build/p9/erf_rows.txt
printf 'x86_64  %s\n' "$(sha256sum < build/p9/erf_rows.txt | cut -c1-16)"
printf 'PE      %s\n' "$(wine ./build/p9/p9_erf.exe 2>/dev/null | tr -d '\r' | sha256sum | cut -c1-16)"
printf 'aarch64 %s\n' "$(qemu-aarch64 ./build/p9/p9_erf_a64 | sha256sum | cut -c1-16)"
gcc -O2 -o build/p9/checker scripts/crverify/check.c -lmpfr -lgmp -lm
# MPFR's erfc at 1,024 bits checks about 500 rows a second for large x: ten interleaved chunks at once, then merged.
mkdir -p build/p9/chunks
pids=()
for k in 0 1 2 3 4 5 6 7 8 9; do
    awk -v k="$k" 'NR % 10 == k' build/p9/erf_rows.txt > "build/p9/chunks/rows$k.txt"
    ./build/p9/checker < "build/p9/chunks/rows$k.txt" > "build/p9/chunks/report$k.txt" &
    pids+=($!)
done
for p in "${pids[@]}"; do wait "$p"; done
python3 -I -B scripts/crverify/merge.py build/p9/chunks/report?.txt | tee build/p9_erf_report.txt

say "4. the cost (ns per call, pinned)"
taskset -c 5 cyrius bench scripts/probes/p9_erf.bcyr 2>&1 | grep -E '^\s+erf'
gcc -O2 -ffp-contract=off -o "$s1/cbench" "$c/cbench.c" "$s1"/orig_erf.o "$s1"/orig_erfc.o "$s1"/nf_erf.o \
    "$s1"/nf_erfc.o -lm
taskset -c 5 "$s1/cbench"
