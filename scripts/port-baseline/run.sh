#!/usr/bin/env bash
# Reproduces the Rust 1.3.0 accuracy baseline in docs/development/pramana-port-plan.md §3.2.
#
# Rust side: the frozen oracle's rust-old/src/math.rs (test-only erf enabled) and Rust std's f64 methods (glibc on
# this host) evaluated on seeded inputs. Reference side: MPFR at 1,024 bits (check.c), and exact
# rationals for the integer-shape incomplete beta (beta_check.py). Reports, per function, the max
# and mean error in ulps of the correctly rounded value and the share of results that are not
# correctly rounded.
#
# Needs cargo, gcc, MPFR 4.2+ and GMP headers, python3. About 12 minutes on one core.
# Usage: scripts/port-baseline/run.sh [work-dir]
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
work=${1:-$(mktemp -d)}
mkdir -p "$work/src"
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
echo "## rustc $(rustc "+$toolchain" --version | cut -d' ' -f2), $(ldd --version | head -1 | sed 's/.* //') glibc, MPFR $(pkg-config --modversion mpfr 2>/dev/null || echo '?')"
grep -v '^gamma' "$work/rust.txt" | "$work/check"
"$work/check" < "$work/ncdf.txt" | tail -n +2
grep '^gamma' "$work/rust.txt" | awk 'NR%10<2' | "$work/check" | tail -n +2
python3 -I "$here/beta_check.py" "$work/beta.txt"
echo "work dir: $work"
