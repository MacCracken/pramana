#!/usr/bin/env bash
# run.sh — the correct-rounding check of pramana's primitives (M1 part b; port plan D2, ADR 009).
#
# Builds scripts/crverify/rows.cyr for x86_64, PE and aarch64, compares the three outputs (sha256), and checks every
# row against MPFR at 1,024 bits (scripts/port-baseline/check.c, ten interleaved chunks merged by merge.py). A function
# passes when its rows are all correctly rounded: max 0.5 ulp and 0% not CR. Needs gcc with MPFR, wine and
# qemu-aarch64.
#   bash scripts/crverify/run.sh
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
work=$root/build/crverify
mkdir -p "$work/chunks"
cd "$root"
quiet() { grep -v -e '^note' -e '^OK' -e 'warning:lib/' -e 'unreachable fns' || true; }
cyrius build scripts/crverify/rows.cyr "$work/rows" 2>&1 | quiet
cyrius build --win scripts/crverify/rows.cyr "$work/rows.exe" 2>&1 | quiet
cyrius build --aarch64 scripts/crverify/rows.cyr "$work/rows_a64" 2>&1 | quiet
"$work/rows" > "$work/rows.txt"
printf 'x86_64  %s\n' "$(sha256sum < "$work/rows.txt" | cut -c1-16)"
printf 'PE      %s\n' "$(wine "$work/rows.exe" 2>/dev/null | tr -d '\r' | sha256sum | cut -c1-16)"
printf 'aarch64 %s\n' "$(qemu-aarch64 "$work/rows_a64" | sha256sum | cut -c1-16)"
gcc -O2 -o "$work/check" scripts/port-baseline/check.c -lmpfr -lgmp -lm
pids=()
for k in 0 1 2 3 4 5 6 7 8 9; do
    awk -v k="$k" 'NR % 10 == k' "$work/rows.txt" > "$work/chunks/rows$k.txt"
    "$work/check" < "$work/chunks/rows$k.txt" > "$work/chunks/report$k.txt" &
    pids+=($!)
done
for p in "${pids[@]}"; do wait "$p"; done
echo "## MPFR $(pkg-config --modversion mpfr 2>/dev/null || echo '?')"
python3 -I -B scripts/port-baseline/merge.py "$work"/chunks/report?.txt | tee "$work/report.txt"
