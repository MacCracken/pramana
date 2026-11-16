#!/usr/bin/env bash
# p9_lgamma.sh — probe P9, ln_gamma (port plan §5 step 9d): Rust 1.3.0, T0 and T1 against MPFR.
# 1. Builds the accuracy baseline (scripts/port-baseline/, on the oracle's toolchain) and keeps its lgamma rows.
# 2. Builds the MPFR checker and the Cyrius probe (kimiya's primitives by absolute path), and runs them.
# 3. Prints check.c's table: per set and variant, n, max and mean ulp, the share not correctly rounded, the worst x.
#   bash scripts/probes/p9_lgamma.sh
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
"$work/baseline/target/release/port-baseline" | grep '^lgamma' > "$work/rust.txt"
gcc -O2 -o "$work/check" "$root/scripts/port-baseline/check.c" -lmpfr -lgmp -lm
(cd "$root" && CYRIUS_ALLOW_ABSOLUTE_INCLUDES=1 cyrius build scripts/probes/p9_lgamma.cyr build/p9/p9_lgamma \
    | grep -v '^note' | grep -v '^OK' || true)
(cd "$root" && ./build/p9/p9_lgamma > "$work/lgamma_rows.txt")
mpfr=$(pkg-config --modversion mpfr 2>/dev/null || echo '?')
echo "## rustc $(rustc "+$toolchain" --version | cut -d' ' -f2), MPFR $mpfr"
"$work/check" < "$work/lgamma_rows.txt"
