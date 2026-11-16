#!/usr/bin/env bash
# p9_ncdf.sh — probe P9, normal_cdf (port plan §5 step 9d-4): Rust 1.3.0, T0, the correctly rounded erfc alone, the
# plan's sketch and T1 against MPFR, on three targets, and their cost.
#
# Needs what p9_erf.sh needs (CORE-MATH's erf.c and erfc.c in build/p9/coremath/, for the tables), plus cargo.
# 1. The two halves of 1/√2 that T1 uses are RN(1/√2) and RN(1/√2 − RN(1/√2)) (Python's decimal, 80 digits).
# 2. Builds the accuracy baseline (scripts/port-baseline/, on the oracle's toolchain) and keeps its normal_cdf rows.
# 3. Builds the probe for x86_64, PE and aarch64, compares their rows, and checks every row against MPFR at 1,024
#    bits in ten interleaved chunks (check.c, merged by merge.py).
# 4. Times T0, the correctly rounded erfc alone and T1, pinned.
#   bash scripts/probes/p9_ncdf.sh
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
work=$root/build/p9
cd "$root"
say() { printf '\n=== %s\n' "$*"; }

say "1. the 1/√2 double-double"
python3 -I -B - scripts/probes/p9_ncdf_impl.cyr <<'PY'
import re, struct, sys
from decimal import Decimal, getcontext
getcontext().prec = 80
s = Decimal(1) / Decimal(2).sqrt()
h = float(s)
l = float(s - Decimal(h))
bits = lambda v: "%016X" % struct.unpack("<Q", struct.pack("<d", v))[0]
src = open(sys.argv[1], encoding="utf-8").read()
got = dict(re.findall(r"var (_P9N_RS2_[HL]): f64 = 0x([0-9A-F]{16});", src))
assert got == {"_P9N_RS2_H": bits(h), "_P9N_RS2_L": bits(l)}, got
print("RN(1/√2) =", h.hex(), " RN(1/√2 − RN(1/√2)) =", l.hex(), ": as in p9_ncdf_impl.cyr")
PY

say "2. the Rust baseline's normal_cdf rows"
mkdir -p "$work/baseline/src"
cp scripts/port-baseline/Cargo.toml "$work/baseline/Cargo.toml"
cp scripts/port-baseline/main.rs "$work/baseline/src/main.rs"
python3 -I - rust-old/src/math.rs "$work/baseline/src/math.rs" <<'PY'
import re, sys
s = open(sys.argv[1]).read()
s = re.sub(r'#\[cfg\((?:test|any\(test, feature = "oracle"\))\)\]\n(?!mod tests)', '', s)
open(sys.argv[2], 'w').write('#![allow(dead_code, clippy::all)]\n' + s)
PY
toolchain=$(sed -n 's/^channel = "\(.*\)"/\1/p' rust-old/rust-toolchain.toml)
(cd "$work/baseline" && CARGO_TARGET_DIR="$work/baseline/target" cargo "+$toolchain" build --release -q)
"$work/baseline/target/release/port-baseline" ncdf > "$work/ncdf_rust.txt"
wc -l < "$work/ncdf_rust.txt"

say "3. the rows, on three targets, against MPFR"
export CYRIUS_ALLOW_ABSOLUTE_INCLUDES=1
quiet() { grep -v -e '^note' -e '^OK' -e 'warning:lib/' -e 'unreachable fns' || true; }
cyrius build scripts/probes/p9_ncdf.cyr build/p9/p9_ncdf 2>&1 | quiet
cyrius build --win scripts/probes/p9_ncdf.cyr build/p9/p9_ncdf.exe 2>&1 | quiet
cyrius build --aarch64 scripts/probes/p9_ncdf.cyr build/p9/p9_ncdf_a64 2>&1 | quiet
./build/p9/p9_ncdf > build/p9/ncdf_rows.txt
printf 'x86_64  %s\n' "$(sha256sum < build/p9/ncdf_rows.txt | cut -c1-16)"
printf 'PE      %s\n' "$(wine ./build/p9/p9_ncdf.exe 2>/dev/null | tr -d '\r' | sha256sum | cut -c1-16)"
printf 'aarch64 %s\n' "$(qemu-aarch64 ./build/p9/p9_ncdf_a64 | sha256sum | cut -c1-16)"
gcc -O2 -o build/p9/checker scripts/port-baseline/check.c -lmpfr -lgmp -lm
mkdir -p build/p9/ncdf_chunks
pids=()
for k in 0 1 2 3 4 5 6 7 8 9; do
    awk -v k="$k" 'NR % 10 == k' build/p9/ncdf_rows.txt > "build/p9/ncdf_chunks/rows$k.txt"
    ./build/p9/checker < "build/p9/ncdf_chunks/rows$k.txt" > "build/p9/ncdf_chunks/report$k.txt" &
    pids+=($!)
done
for p in "${pids[@]}"; do wait "$p"; done
mpfr=$(pkg-config --modversion mpfr 2>/dev/null || echo '?')
echo "## rustc $(rustc "+$toolchain" --version | cut -d' ' -f2), MPFR $mpfr"
python3 -I -B scripts/port-baseline/merge.py build/p9/ncdf_chunks/report?.txt | tee build/p9_ncdf_report.txt

say "4. the cost (ns per call, pinned)"
taskset -c 5 cyrius bench scripts/probes/p9_ncdf.bcyr 2>&1 | grep -E '^\s+(T0|T1|CR)'
