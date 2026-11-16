#!/usr/bin/env bash
# p8_bundle.sh — M0 step 9, probe P8: the bundle (port plan §5 step 9c, D8, D9). Measured in scratch copies, so
# nothing tracked changes. Three layouts of the same modules (src/constants.cyr, p8_bundle/core.cyr, p8_bundle/ai.cyr)
# and the same tests (pramana's, plus p8_bundle/ai_test.tcyr, which includes lib/sandhi.cyr itself):
#   profile — sandhi and its companion leaves in [deps] stdlib (ushma's form), [lib] the core, [lib.ai] core + ai;
#   outside — [deps] stdlib without sandhi, [lib] the core, src/p8_ai.cyr in no bundle (a consumer that wants ai
#             names the file and sandhi itself);
#   hybrid  — [deps] stdlib without sandhi, [lib] the core and [lib.ai] core + ai.
# For each: cyrius deps, distlib --all, the bundles and their sidecars, distlib --check, and how long pramana's own
# build and test take (three interleaved runs, pinned); for the hybrid, deps --locked and audit too. Then consumers,
# built for x86_64 ELF, PE and aarch64 and run (wine, qemu): nothing called, one function, every core function, and
# the ai post (to a closed port) through the profile's bundle and from outside it.
#   OUT=build/p8 bash scripts/probes/p8_bundle.sh
set -u
here=$(cd "$(dirname "$0")/../.." && pwd)
OUT=${OUT:-$here/build/p8}
case "$OUT" in /*) ;; *) OUT=$here/$OUT ;; esac
rm -rf "$OUT"
mkdir -p "$OUT"
say() { printf '\n=== %s\n' "$*"; }
companions='"sandhi", "tls", "async", "random", "fdlopen", "dynlib"'

# A scratch copy of pramana: $1 the layout (profile | outside | hybrid).
scratch() {
    local s=$OUT/$1/pramana
    mkdir -p "$s"
    cp -r "$here/cyrius.cyml" "$here/cyrius.lock" "$here/VERSION" "$here/src" "$here/lib" "$here/tests" "$s/"
    cp "$here/scripts/probes/p8_bundle/core.cyr" "$s/src/p8_core.cyr"
    cp "$here/scripts/probes/p8_bundle/ai.cyr" "$s/src/p8_ai.cyr"
    cp "$here/scripts/probes/p8_bundle/ai_test.tcyr" "$s/tests/p8_ai.tcyr"
    if [ "$1" = profile ]; then
        python3 -I -B - "$s/cyrius.cyml" "$companions" <<'PY'
import sys
path, extra = sys.argv[1], sys.argv[2]
text = open(path, encoding="utf-8").read()
text = text.replace('"atomic", "bayan"]', '"atomic", "bayan",\n    ' + extra + ']', 1)
open(path, "w", encoding="utf-8").write(text)
PY
    fi
    cat >> "$s/cyrius.cyml" <<EOF

[lib]
modules = ["src/constants.cyr", "src/p8_core.cyr"]
EOF
    if [ "$1" != outside ]; then
        cat >> "$s/cyrius.cyml" <<EOF

[lib.ai]
modules = ["src/constants.cyr", "src/p8_core.cyr", "src/p8_ai.cyr"]
EOF
    fi
    say "$1: [deps] stdlib"
    sed -n '/^\[deps\]/,/^\[/p' "$s/cyrius.cyml" | grep -v '^\[' | tr -s ' \n' ' '
    echo
    say "$1: cyrius deps"
    (cd "$s" && cyrius deps 2>&1 | grep -i -E 'warn|error|refus' | sort -u)
    say "$1: cyrius distlib --all"
    (cd "$s" && cyrius distlib --all 2>&1 | grep -v '^\s*$' | sed "s|$s/||g")
    (cd "$s/dist" && wc -c ./* | sed 's|\./||')
    local d
    for d in "$s"/dist/*.deps; do
        printf '%s: ' "$(basename "$d")"
        grep -v '^#' "$d" | tr '\n' ' '
        echo
    done
    printf 'lines naming sandhi in dist/pramana.cyr: %s\n' "$(grep -c sandhi "$s/dist/pramana.cyr")"
    say "$1: cyrius distlib --check"
    (cd "$s" && cyrius distlib --check 2>&1 | grep -v '^\s*$' | sed "s|$s/||g")
}

# Seconds for one command in $1's scratch copy, pinned to one core.
timed() {
    local s=$OUT/$1/pramana
    shift
    local t0 t1
    t0=$(date +%s.%N)
    (cd "$s" && taskset -c 5 "$@" >/dev/null 2>&1)
    t1=$(date +%s.%N)
    python3 -c "print(f'{$t1 - $t0:.2f}')"
}

# A consumer: $1 name, $2 layout, $3 modules (TOML array body), $4 stdlib (TOML array body), $5 defines.
consumer() {
    local c=$OUT/$2/$1
    mkdir -p "$c/src"
    cp "$here/scripts/probes/p8_bundle/consumer.cyr" "$c/src/main.cyr"
    cat > "$c/cyrius.cyml" <<EOF
[package]
name = "$1"
version = "0.0.1"
language = "cyrius"
cyrius = "6.7.18"

[build]
entry = "src/main.cyr"
output = "build/$1"
dce = true

[deps]
stdlib = [$4]

[deps.pramana]
path = "../pramana"
modules = [$3]
EOF
    say "consumer $1 ($2; modules [$3]; $5)"
    (cd "$c" && cyrius deps 2>&1 | grep -i -E 'warn|error|refus' | sort -u)
    local t flag ext log rc run rrc t0 t1
    for t in elf win aarch64; do
        flag=""
        ext=""
        [ "$t" = win ] && flag=--win && ext=.exe
        [ "$t" = aarch64 ] && flag=--aarch64
        t0=$(date +%s.%N)
        # shellcheck disable=SC2086
        log=$(cd "$c" && cyrius build $flag $5 src/main.cyr "build/$1-$t$ext" 2>&1)
        rc=$?
        t1=$(date +%s.%N)
        printf '%-8s build rc %d, %s s: %s; stdlib warnings from: %s\n' "$t" "$rc" \
            "$(python3 -c "print(f'{$t1 - $t0:.2f}')")" \
            "$(printf '%s\n' "$log" | grep -o -E 'OK \([0-9]+ bytes\)|error[^;]*|undefined[^;]*' | tr '\n' ' ')" \
            "$(printf '%s\n' "$log" | grep -o -E 'warning:lib/[a-z_]+\.cyr' | sort -u | sed 's/warning://' \
                | tr '\n' ' ')"
        [ $rc -ne 0 ] && continue
        case $t in
            elf) run=$("$c/build/$1-$t" 2>&1) ;;
            win) run=$(wine "$c/build/$1-$t$ext" 2>/dev/null | tr -d '\r') ;;
            aarch64) run=$(qemu-aarch64 "$c/build/$1-$t" 2>&1) ;;
        esac
        rrc=$?
        printf '%-8s run rc %d: %s\n' "$t" $rrc "$(printf '%s' "$run" | tr '\n' ' ' | cut -c1-120)"
    done
}

scratch profile
scratch outside
scratch hybrid

say "hybrid: the ai bundle and sidecar against the profile layout's"
for f in pramana-ai.cyr pramana-ai.deps; do
    cmp -s "$OUT/hybrid/pramana/dist/$f" "$OUT/profile/pramana/dist/$f" && echo "$f: identical" || echo "$f: DIFFERS"
done
say "hybrid: cyrius deps --locked, cyrius audit"
(cd "$OUT/hybrid/pramana" && cyrius deps --locked > /dev/null 2>&1; echo "deps --locked exit $?")
(cd "$OUT/hybrid/pramana" && cyrius audit > "$OUT/hybrid/audit.txt" 2>&1; echo "audit exit $?")
grep -E '^── |^  ok:|undocumented|FAILED' "$OUT/hybrid/audit.txt" | tr '\n' ' '
echo

say "pramana's own compiles (seconds; three interleaved runs, pinned; the test suite includes the ai test)"
for i in 1 2 3; do
    for l in profile outside hybrid; do
        printf 'run %d %-8s build %s, test %s\n' "$i" "$l" "$(timed "$l" cyrius build src/main.cyr build/m)" \
            "$(timed "$l" cyrius test)"
    done
done
say "the ai test alone, and the suite without it (seconds, pinned)"
for l in profile hybrid; do
    printf '%-8s ai test %s' "$l" "$(timed "$l" cyrius test tests/p8_ai.tcyr)"
    mv "$OUT/$l/pramana/tests/p8_ai.tcyr" "$OUT/$l/p8_ai.tcyr.off"
    printf ', suite without it %s\n' "$(timed "$l" cyrius test)"
    mv "$OUT/$l/p8_ai.tcyr.off" "$OUT/$l/pramana/tests/p8_ai.tcyr"
done

core_deps=$(grep -v '^#' "$OUT/outside/pramana/dist/pramana.deps" | sed 's/.*/"&"/' | paste -sd, -)
ai_deps=$(grep -v '^#' "$OUT/profile/pramana/dist/pramana-ai.deps" | sed 's/.*/"&"/' | paste -sd, -)
consumer c_empty outside '"dist/pramana.cyr"' "$core_deps" ""
consumer c_one outside '"dist/pramana.cyr"' "$core_deps" "-D P8_ONE"
consumer c_all outside '"dist/pramana.cyr"' "$core_deps" "-D P8_ALL"
consumer c_one_profile profile '"dist/pramana.cyr"' "$(grep -v '^#' "$OUT/profile/pramana/dist/pramana.deps" \
    | sed 's/.*/"&"/' | paste -sd, -)" "-D P8_ONE"
consumer c_ai_profile profile '"dist/pramana-ai.cyr"' "$ai_deps" "-D P8_ALL -D P8_AI"
consumer c_ai_outside outside '"dist/pramana.cyr", "src/p8_ai.cyr"' "$core_deps, $companions" "-D P8_ALL -D P8_AI"

say "binary sizes (bytes)"
for f in "$OUT"/*/c_*/build/c_*; do
    printf '%-40s %s\n' "${f#"$OUT"/}" "$(wc -c < "$f")"
done
