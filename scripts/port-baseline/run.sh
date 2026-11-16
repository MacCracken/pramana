#!/usr/bin/env bash
# Reproduces the Rust 1.3.0 accuracy baseline in docs/development/pramana-port-plan.md §3.2.
#
# Rust side: the frozen oracle's rust-old/src/math.rs (test-only erf enabled) and Rust std's f64 methods (glibc on
# this host) evaluated on seeded inputs. Reference side: MPFR at 1,024 bits (check.c), and exact
# rationals for the integer-shape incomplete beta (beta_check.py). Reports, per function, the max
# and mean error in ulps of the correctly rounded value and the share of results that are not
# correctly rounded.
#
# Then the port's side (M2 part c, M2's gate): scripts/port-baseline/port.cyr reads the same three outputs and writes
# them back with the port's results, so both are measured on identical rows. Both sides are also measured on all
# 20,000 incomplete gamma pairs by check.c's 320-bit `gref` (probe P9's re-measure), and lnΓ for x < 0 in ADR 009's
# mixed ulps of max(|lnΓ|, 1) as well as plain ones. Each side's rows are checked in ten interleaved chunks.
#
# Needs cargo, gcc, MPFR 4.2+ and GMP headers, python3, and cyrius for the port's side (skipped without it). About 4
# minutes on ten cores.
# Usage: scripts/port-baseline/run.sh [work-dir]
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
work=${1:-$(mktemp -d)}
mkdir -p "$work/src"
work=$(cd "$work" && pwd)
cp "$here/Cargo.toml" "$work/Cargo.toml"
cp "$here/main.rs" "$work/src/main.rs"
python3 -I - "$root/rust-old/src/math.rs" "$work/src/math.rs" <<'PY'
import re, sys
s = open(sys.argv[1]).read()
# Enable erf (compiled for tests and the oracle only); the tests module stays out.
s = re.sub(r'#\[cfg\((?:test|any\(test, feature = "oracle"\))\)\]\n(?!mod tests)', '', s)
open(sys.argv[2], 'w').write('#![allow(dead_code, clippy::all)]\n' + s)
PY
# Build with the oracle's recording toolchain (rust-old/rust-toolchain.toml), not whatever stable is.
toolchain=$(sed -n 's/^channel = "\(.*\)"/\1/p' "$root/rust-old/rust-toolchain.toml")
(cd "$work" && CARGO_TARGET_DIR="$work/target" cargo "+$toolchain" build --release -q)
bin="$work/target/release/port-baseline"
gcc -O2 -o "$work/check" "$here/check.c" -lmpfr -lgmp -lm
"$bin" > "$work/rust.txt"
"$bin" ncdf > "$work/ncdf.txt"
"$bin" beta > "$work/beta.txt"
# measure SIDE MAIN NCDF BETA: §3.2's sets (gamma on its sample of 4,000 P and 4,000 Q rows, by mpfr_gamma_inc), the
# gamma pairs again by gref, the mixed lnΓ rows, and the integer-shape beta rows by exact rationals.
measure() {
    local side=$1 main=$2 ncdf=$3 beta=$4 d=$work/m_$1
    mkdir -p "$d"
    {
        grep -v '^gamma' "$main"
        grep '^lgamma_neg ' "$main" | sed 's/^lgamma_neg /lgamma_neg_mixed /'
        cat "$ncdf"
        grep '^gamma' "$main" | sed 's|^gamma_p |gref_p/all |; s|^gamma_q |gref_q/all |'
        grep '^gamma' "$main" | awk 'NR%10<2'
    } > "$d/rows.txt"
    local pids=()
    for k in 0 1 2 3 4 5 6 7 8 9; do
        awk -v k="$k" 'int((NR - 1) / 2) % 10 == k' "$d/rows.txt" > "$d/rows$k.txt"
        "$work/check" < "$d/rows$k.txt" > "$d/report$k.txt" &
        pids+=($!)
    done
    for q in "${pids[@]}"; do wait "$q"; done
    echo "## $side"
    python3 -I -B "$here/merge.py" "$d"/report?.txt
    python3 -I "$here/beta_check.py" "$beta"
}
echo "## rustc $(rustc "+$toolchain" --version | cut -d' ' -f2), $(ldd --version | head -1 | sed 's/.* //') glibc, MPFR $(pkg-config --modversion mpfr 2>/dev/null || echo '?')"
measure "Rust 1.3.0" "$work/rust.txt" "$work/ncdf.txt" "$work/beta.txt"
if command -v cyrius > /dev/null; then
    (cd "$root" && cyrius build scripts/port-baseline/port.cyr "$work/port" 2>&1 | grep -v -e '^note' -e '^OK' -e '^compile' || true)
    "$work/port" < "$work/rust.txt" > "$work/port_main.txt"
    "$work/port" < "$work/ncdf.txt" > "$work/port_ncdf.txt"
    "$work/port" < "$work/beta.txt" > "$work/port_beta.txt"
    measure "the port (pramana 2.0, T1)" "$work/port_main.txt" "$work/port_ncdf.txt" "$work/port_beta.txt"
else
    echo "## the port: skipped (no cyrius)"
fi
echo "work dir: $work"
