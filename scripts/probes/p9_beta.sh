#!/usr/bin/env bash
# p9_beta.sh — probe P9, beta_inc (port plan §5 step 9d-3): Rust 1.3.0, T0 and T1 against a 320-bit MPFR reference.
# 1. Builds the accuracy baseline (scripts/port-baseline/, on the oracle's toolchain) and keeps its integer-shape rows.
# 2. Checks the reference against exact rationals on those rows (p9_beta_exact.py; `bref_vs_exact`).
# 3. Builds the checker and the Cyrius probe (kimiya's primitives by absolute path), and runs them.
# 4. Prints check.c's table: per function, set and variant, n, max and mean ulp, the share not correctly rounded and
#    the worst (a, b, x or −y).
#   bash scripts/probes/p9_beta.sh
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
work=$root/build/p9
mkdir -p "$work/baseline/src"
cp "$root/scripts/port-baseline/Cargo.toml" "$work/baseline/Cargo.toml"
cp "$root/scripts/port-baseline/main.rs" "$work/baseline/src/main.rs"
python3 -I - "$root/rust-old/src/math.rs" "$work/baseline/src/math.rs" <<'PY'
import re, sys
s = open(sys.argv[1]).read()
s = re.sub(r'#\[cfg\((?:test|any\(test, feature = "oracle"\))\)\]\n(?!mod tests)', '', s)
open(sys.argv[2], 'w').write('#![allow(dead_code, clippy::all)]\n' + s)
PY
toolchain=$(sed -n 's/^channel = "\(.*\)"/\1/p' "$root/rust-old/rust-toolchain.toml")
(cd "$work/baseline" && CARGO_TARGET_DIR="$work/baseline/target" cargo "+$toolchain" build --release -q)
"$work/baseline/target/release/port-baseline" beta > "$work/beta_rust.txt"
gcc -O2 -o "$work/check" "$root/scripts/port-baseline/check.c" -lmpfr -lgmp -lm
python3 -I -B "$root/scripts/probes/p9_beta_exact.py" "$work/beta_rust.txt" > "$work/beta_exact_rows.txt"
(cd "$root" && CYRIUS_ALLOW_ABSOLUTE_INCLUDES=1 cyrius build scripts/probes/p9_beta.cyr build/p9/p9_beta \
    | grep -v '^note' | grep -v '^OK' || true)
(cd "$root" && ./build/p9/p9_beta > "$work/beta_rows.txt")
mpfr=$(pkg-config --modversion mpfr 2>/dev/null || echo '?')
echo "## rustc $(rustc "+$toolchain" --version | cut -d' ' -f2), MPFR $mpfr"
"$work/check" < "$work/beta_exact_rows.txt"
"$work/check" < "$work/beta_rows.txt" | tail -n +2
