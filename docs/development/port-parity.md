# pramana — port parity record

Measured parity baselines and probe results for the Rust → Cyrius port, one section per M0 step or milestone part
(port plan §5–§7). Each section gives what was run, the numbers, and what follows from them. Rows are never edited
after the fact; a re-run adds a dated note.

## 1. Toolchain and math check (M0 step 4, 2026-11-15)

**What ran.** `scripts/probes/p0_semantics.cyr` compares the bits of every f64 and u64 operation pramana 1.3.0 uses
(port plan §8.1) against its twin, `scripts/probes/p0_semantics.rs`.
- **The Rust twin** is built the way the oracle is: release mode (`rustc +1.100.0 -O`), with std calling glibc
  2.44, and every input through `black_box`.
- **The comparison:** `scripts/probes/p0_compare.py` joins the two outputs. Floats are compared by ulp distance, and
  two NaNs by sign and payload.
- **Target:** this run is x86_64 ELF only. The PE (wine) and aarch64 (qemu) runs come with the baselines (step 8).
- **Compile:** warning-free.

**Result.** 125 Rust cases, all covered, in 155 Cyrius rows (some cases probe two variants):
- 136 rows give the same bits as Rust;
- 4 are identical NaNs;
- 15 differ, and each is either a variant the port won't use or a transcendental that ledger L1 replaces.

**Exact as they are (no helper needed):**
- `floor`, `ceil`, `trunc`, `sqrt` and `abs`, including at −0 and −0.5;
- `sin` and `cos` (fdlibm on every target); `atan` and `log2` (builtin and polyfill); `ln` (builtin and polyfill);
  `cbrt` (ganita, like Rust's compiler_builtins); the `exp` builtin. All on the probed points;
- `powi` for all six exponents in `rust-old/src` (2, 3, 4, 9, 25, 49). The products follow LLVM's square-and-multiply
  order: the result accumulates the set bits from low to high, e.g. x²⁵ = (x · x⁸) · x¹⁶;
- `SimpleRng`: the xorshift64 words and draws for seeds 1, 42, 2⁶³ and u64::MAX, through 1,000 steps. In Cyrius `>>`
  is the logical shift, as on Rust's u64;
- the `f64::total_cmp` keys (`mode`'s sort), as `k ^ ((k >>> 63) >> 1)`;
- the fdlibm erf port's word operations: the high word `(bits >> 32) as u32`, and the low-word mask;
- **positional literals for every exponent constant**, among them 1e-300 (302 characters), 1e300, `MIN_POSITIVE`
  (326) and `MAX` (311). The lexer rounds each one correctly. The constant generator (step 6) can write all of
  them positionally;
- `F64_PI`, `F64_E` and `F64_TAU`, read into typed locals;
- IEEE comparisons on typed `f64` locals: NaN is unordered, and −0 == +0.

**Needs a helper** (ledger L2, `src/mathx.cyr`):

| Rust | Cyrius as it is | Helper (probed: same bits on every case) |
|---|---|---|
| `min` / `max` ignore a NaN on **either** side | `f64_min(1, NaN)` and `f64_max(1, NaN)` return NaN; a NaN first operand is ignored | `_pmn_min` / `_pmn_max` with Rust's rule (30 sites, and 10 `clamp`s) |
| `fract(x) = x − trunc(x)`: `fract(−2.75) = −0.75` | **`f64_fract` is floor-based: `f64_fract(−2.75) = 0.25`** | `x − f64_trunc(x)`, which also gives `fract(∞) = NaN`. Never call `f64_fract` |
| `n as f64` for a u64 rounds to nearest | `f64_from` reads the word as **signed**: 2⁶³ → −9.2e18, u64::MAX → −1 | `_pmn_u64_to_f64`: halve with a sticky bit, convert, double. Exact on 2⁶³, 2⁶³ + 2¹¹ + 1 and u64::MAX (Binomial's `n`) |
| `x as usize` saturates: NaN and negatives → 0, ≥ 2⁶⁴ → u64::MAX | `f64_to` truncates: −3 → `0xff…fc`, 1e29 → i64::MAX | `_pmn_sat_usize` (3 sites: `percentile`'s rank, the iteration budget, the histogram bin) |

**A rule, not a helper: every f64 sum starts at −0.0.** On rustc 1.100, `Sum for f64` starts from −0.0:
- `sum([]) = −0.0`;
- `sum([−0]) = −0.0`;
- `sum([+0]) = +0.0`.

A loop from −0.0 matches all three; a loop from 0.0 gets the sign of zero wrong on two. This reaches
`descriptive::sum` on empty data, and every `iter().sum()` whose terms are all −0.

**Transcendentals (ledger L1, replaced by the correctly rounded functions of ADR 009).** 5 of the 47 rows differ by
1 ulp. MPFR 4.2.2 shows which side misrounds:

| Case | glibc (Rust) | Cyrius counterpart | Which is correctly rounded |
|---|---|---|---|
| `exp(1)` | 0.326 ulp | `_f64_exp_polyfill` 0.674 ulp | glibc |
| `exp(239.01439753131297)` | 0.505 ulp | `_f64_exp_polyfill` 0.495 ulp | **Cyrius** (the x87 `f64_exp` copies glibc's misround) |
| `exp_m1(0.92043886641489558)` | 0.738 ulp | `_gn_expm1` 0.262 ulp | **Cyrius** |
| `pow(2, 0.5)` | 0.435 ulp | `ganita_f64_pow` 0.565 ulp | glibc |
| `tan(1.4938616554658477)` | 0.533 ulp | `ganita_f64_tan` 0.468 ulp | **Cyrius** |

Neither side is reliably correctly rounded: glibc misrounds three of the five, and the stdlib and ganita misround two.
Only the correctly rounded functions (D2) meet ADR 009, and the `*.cr.tsv` rows will record exactly these glibc
misrounds.

Also notable: `_gn_log1p` gives glibc's bits on all three probes, including glibc's 0.748-ulp misround at 0.2833.
Both are the fdlibm algorithm.

**For the constant generator (step 6):** `cyrius lint` flags any line over 120 characters, and `cyrius audit` lints
`src/`. So a constant whose positional literal is that long can't be written as a literal in `src/constants.cyr`:
1e-300 (`LENTZ_TINY`) is one, and so are 1e300, `MIN_POSITIVE` and `MAX`. It becomes a typed global holding its
exact bits (`var NAME: f64 = 0x…;`, pravash's form for ±∞, NaN and `f64::MAX`). Never a hex `const`, which would be
an integer. The probe file itself keeps four such lines (lint warnings only).

### 1.1 The probe table

| Case | Cyrius | Rust bits | Cyrius bits | Result |
|---|---|---|---|---|
| `min(nan,1)` | f64_min | `0x3ff0000000000000` | `0x3ff0000000000000` | same bits |
| `min(1,nan)` | f64_min | `0x3ff0000000000000` | `0x7ff8000000000000` | **4613937818241073152 ulp** |
| `max(nan,1)` | f64_max | `0x3ff0000000000000` | `0x3ff0000000000000` | same bits |
| `max(1,nan)` | f64_max | `0x3ff0000000000000` | `0x7ff8000000000000` | **4613937818241073152 ulp** |
| `min(+0,-0)` | f64_min | `0x8000000000000000` | `0x8000000000000000` | same bits |
| `max(+0,-0)` | f64_max | `0x8000000000000000` | `0x8000000000000000` | same bits |
| `clamp(nan,0,1)` | f64_clamp | `0x7ff8000000000000` | `0x7ff8000000000000` | both NaN |
| `clamp(2,0,1)` | f64_clamp | `0x3ff0000000000000` | `0x3ff0000000000000` | same bits |
| `clamp(-0,0,1)` | f64_clamp | `0x8000000000000000` | `0x8000000000000000` | same bits |
| `floor(-0.5)` | f64_floor | `0xbff0000000000000` | `0xbff0000000000000` | same bits |
| `floor(-0)` | f64_floor | `0x8000000000000000` | `0x8000000000000000` | same bits |
| `floor(2.5)` | f64_floor | `0x4000000000000000` | `0x4000000000000000` | same bits |
| `ceil(-0.5)` | f64_ceil | `0x8000000000000000` | `0x8000000000000000` | same bits |
| `trunc(-2.7)` | f64_trunc | `0xc000000000000000` | `0xc000000000000000` | same bits |
| `sqrt(2)` | f64_sqrt | `0x3ff6a09e667f3bcd` | `0x3ff6a09e667f3bcd` | same bits |
| `sqrt(-0)` | f64_sqrt | `0x8000000000000000` | `0x8000000000000000` | same bits |
| `abs(-0)` | f64_abs | `0x0` | `0x0` | same bits |
| `fract(2.75)` | f64_fract | `0x3fe8000000000000` | `0x3fe8000000000000` | same bits |
| `fract(2.75)` | x - f64_trunc(x) | `0x3fe8000000000000` | `0x3fe8000000000000` | same bits |
| `fract(-2.75)` | f64_fract | `0xbfe8000000000000` | `0x3fd0000000000000` | **9203105838531608576 ulp** |
| `fract(-2.75)` | x - f64_trunc(x) | `0xbfe8000000000000` | `0xbfe8000000000000` | same bits |
| `fract(-0)` | f64_fract | `0x0` | `0x0` | same bits |
| `fract(-0)` | x - f64_trunc(x) | `0x0` | `0x0` | same bits |
| `fract(3)` | f64_fract | `0x0` | `0x0` | same bits |
| `fract(3)` | x - f64_trunc(x) | `0x0` | `0x0` | same bits |
| `fract(1e300)` | f64_fract | `0x0` | `0x0` | same bits |
| `fract(1e300)` | x - f64_trunc(x) | `0x0` | `0x0` | same bits |
| `fract(inf)` | f64_fract | `0xfff8000000000000` | `0xfff8000000000000` | both NaN |
| `fract(inf)` | x - f64_trunc(x) | `0xfff8000000000000` | `0xfff8000000000000` | both NaN |
| `exp(-0.7)` | f64_exp | `0x3fdfc80db9dd5542` | `0x3fdfc80db9dd5542` | same bits |
| `exp(-0.7)` | _f64_exp_polyfill | `0x3fdfc80db9dd5542` | `0x3fdfc80db9dd5542` | same bits |
| `exp(1)` | f64_exp | `0x4005bf0a8b145769` | `0x4005bf0a8b145769` | same bits |
| `exp(1)` | _f64_exp_polyfill | `0x4005bf0a8b145769` | `0x4005bf0a8b14576a` | **1 ulp** |
| `exp(5.3)` | f64_exp | `0x40690ac725b6564c` | `0x40690ac725b6564c` | same bits |
| `exp(5.3)` | _f64_exp_polyfill | `0x40690ac725b6564c` | `0x40690ac725b6564c` | same bits |
| `exp(239.014)` | f64_exp | `0x557c57a25ee55a57` | `0x557c57a25ee55a57` | same bits |
| `exp(239.014)` | _f64_exp_polyfill | `0x557c57a25ee55a57` | `0x557c57a25ee55a56` | **1 ulp** |
| `ln(0.3)` | f64_ln | `0xbff34378fcbda721` | `0xbff34378fcbda721` | same bits |
| `ln(0.3)` | _f64_ln_polyfill | `0xbff34378fcbda721` | `0xbff34378fcbda721` | same bits |
| `ln(2)` | f64_ln | `0x3fe62e42fefa39ef` | `0x3fe62e42fefa39ef` | same bits |
| `ln(2)` | _f64_ln_polyfill | `0x3fe62e42fefa39ef` | `0x3fe62e42fefa39ef` | same bits |
| `ln(10)` | f64_ln | `0x40026bb1bbb55516` | `0x40026bb1bbb55516` | same bits |
| `ln(10)` | _f64_ln_polyfill | `0x40026bb1bbb55516` | `0x40026bb1bbb55516` | same bits |
| `ln_1p(1e-10)` | _gn_log1p (ganita, private) | `0x3ddb7cdfd9d1d693` | `0x3ddb7cdfd9d1d693` | same bits |
| `ln_1p(-0.5)` | _gn_log1p (ganita, private) | `0xbfe62e42fefa39ef` | `0xbfe62e42fefa39ef` | same bits |
| `ln_1p(0.2833)` | _gn_log1p (ganita, private) | `0x3fcfedcc1e64e650` | `0x3fcfedcc1e64e650` | same bits |
| `exp_m1(1e-10)` | _gn_expm1 (ganita, private) | `0x3ddb7cdfd9dda4e3` | `0x3ddb7cdfd9dda4e3` | same bits |
| `exp_m1(-3)` | _gn_expm1 (ganita, private) | `0xbfee6824f33314f5` | `0xbfee6824f33314f5` | same bits |
| `exp_m1(0.9204)` | _gn_expm1 (ganita, private) | `0x3ff82a90abad523e` | `0x3ff82a90abad523f` | **1 ulp** |
| `pow(2,0.5)` | ganita_f64_pow | `0x3ff6a09e667f3bcd` | `0x3ff6a09e667f3bcc` | **1 ulp** |
| `pow(1.5,2.3)` | ganita_f64_pow | `0x40045407da8d4014` | `0x40045407da8d4014` | same bits |
| `pow(200,-0.2)` | ganita_f64_pow | `0x3fd62e3e1817c1c4` | `0x3fd62e3e1817c1c4` | same bits |
| `sin(0.5)` | f64_sin | `0x3fdeaee8744b05f0` | `0x3fdeaee8744b05f0` | same bits |
| `cos(0.5)` | f64_cos | `0x3fec1528065b7d50` | `0x3fec1528065b7d50` | same bits |
| `sin(1)` | f64_sin | `0x3feaed548f090cee` | `0x3feaed548f090cee` | same bits |
| `cos(1)` | f64_cos | `0x3fe14a280fb5068c` | `0x3fe14a280fb5068c` | same bits |
| `sin(3)` | f64_sin | `0x3fc210386db6d55b` | `0x3fc210386db6d55b` | same bits |
| `cos(3)` | f64_cos | `0xbfefae04be85e5d2` | `0xbfefae04be85e5d2` | same bits |
| `tan(0.5)` | ganita_f64_tan | `0x3fe17b4f5bf3474a` | `0x3fe17b4f5bf3474a` | same bits |
| `tan(1.4939)` | ganita_f64_tan | `0x4029f1dc8e0c52c2` | `0x4029f1dc8e0c52c1` | **1 ulp** |
| `tan(-1)` | ganita_f64_tan | `0xbff8eb245cbee3a6` | `0xbff8eb245cbee3a6` | same bits |
| `atan(0.5)` | f64_atan | `0x3fddac670561bb4f` | `0x3fddac670561bb4f` | same bits |
| `atan(0.5)` | _f64_atan_polyfill | `0x3fddac670561bb4f` | `0x3fddac670561bb4f` | same bits |
| `atan(-3)` | f64_atan | `0xbff3fc176b7a8560` | `0xbff3fc176b7a8560` | same bits |
| `atan(-3)` | _f64_atan_polyfill | `0xbff3fc176b7a8560` | `0xbff3fc176b7a8560` | same bits |
| `atan(1e10)` | f64_atan | `0x3ff921fb543d4de0` | `0x3ff921fb543d4de0` | same bits |
| `atan(1e10)` | _f64_atan_polyfill | `0x3ff921fb543d4de0` | `0x3ff921fb543d4de0` | same bits |
| `log2(3)` | f64_log2 | `0x3ff95c01a39fbd68` | `0x3ff95c01a39fbd68` | same bits |
| `log2(3)` | _f64_log2_polyfill | `0x3ff95c01a39fbd68` | `0x3ff95c01a39fbd68` | same bits |
| `log2(8)` | f64_log2 | `0x4008000000000000` | `0x4008000000000000` | same bits |
| `log2(8)` | _f64_log2_polyfill | `0x4008000000000000` | `0x4008000000000000` | same bits |
| `log2(10)` | f64_log2 | `0x400a934f0979a371` | `0x400a934f0979a371` | same bits |
| `log2(10)` | _f64_log2_polyfill | `0x400a934f0979a371` | `0x400a934f0979a371` | same bits |
| `cbrt(2)` | ganita_f64_cbrt | `0x3ff428a2f98d728b` | `0x3ff428a2f98d728b` | same bits |
| `cbrt(0.001)` | ganita_f64_cbrt | `0x3fb999999999999a` | `0x3fb999999999999a` | same bits |
| `cbrt(1e-300)` | ganita_f64_cbrt | `0x2b2bff2ee48e0530` | `0x2b2bff2ee48e0530` | same bits |
| `powi(1.1,2)` | x*x | `0x3ff35c28f5c28f5d` | `0x3ff35c28f5c28f5d` | same bits |
| `powi(1.1,3)` | x * x^2 | `0x3ff54bc6a7ef9db4` | `0x3ff54bc6a7ef9db4` | same bits |
| `powi(1.1,4)` | (x^2)^2 | `0x3ff76cf41f212d79` | `0x3ff76cf41f212d79` | same bits |
| `powi(0.9,9)` | x * x^8 | `0x3fd8cb7f4e838a80` | `0x3fd8cb7f4e838a80` | same bits |
| `powi(0.99,25)` | (x * x^8) * x^16 | `0x3fe8e3e99e97fcb1` | `0x3fe8e3e99e97fcb1` | same bits |
| `powi(0.999,49)` | (x * x^16) * x^32 | `0x3fee78139aedb8db` | `0x3fee78139aedb8db` | same bits |
| `rng[1].state1` | xorshift64 (<< >> logical) | `0x40822041` | `0x40822041` | same |
| `rng[1].draw1` | (x >> 11) / 2^53 | `0x3dd0208800000000` | `0x3dd0208800000000` | same bits |
| `rng[1].state1000` | xorshift64 (<< >> logical) | `0x9e8b2325c8f3382d` | `0x9e8b2325c8f3382d` | same |
| `rng[1].draw1000` | (x >> 11) / 2^53 | `0x3fe3d16464b91e67` | `0x3fe3d16464b91e67` | same bits |
| `rng[42].state1` | xorshift64 (<< >> logical) | `0xa95514aaa` | `0xa95514aaa` | same |
| `rng[42].draw1` | (x >> 11) / 2^53 | `0x3e252aa290000000` | `0x3e252aa290000000` | same bits |
| `rng[42].state1000` | xorshift64 (<< >> logical) | `0x1f5fbcecfd77e4e9` | `0x1f5fbcecfd77e4e9` | same |
| `rng[42].draw1000` | (x >> 11) / 2^53 | `0x3fbf5fbcecfd77e0` | `0x3fbf5fbcecfd77e0` | same bits |
| `rng[2^63].state1` | xorshift64 (<< >> logical) | `0x8100000000000000` | `0x8100000000000000` | same |
| `rng[2^63].draw1` | (x >> 11) / 2^53 | `0x3fe0200000000000` | `0x3fe0200000000000` | same bits |
| `rng[2^63].state1000` | xorshift64 (<< >> logical) | `0xd881ec4ee7252b4b` | `0xd881ec4ee7252b4b` | same |
| `rng[2^63].draw1000` | (x >> 11) / 2^53 | `0x3feb103d89dce4a5` | `0x3feb103d89dce4a5` | same bits |
| `rng[u64max].state1` | xorshift64 (<< >> logical) | `0x3f801fc0` | `0x3f801fc0` | same |
| `rng[u64max].draw1` | (x >> 11) / 2^53 | `0x3dcfc00c00000000` | `0x3dcfc00c00000000` | same bits |
| `rng[u64max].state1000` | xorshift64 (<< >> logical) | `0xd9638309f63fbcb1` | `0xd9638309f63fbcb1` | same |
| `rng[u64max].draw1000` | (x >> 11) / 2^53 | `0x3feb2c70613ec7f7` | `0x3feb2c70613ec7f7` | same bits |
| `key(-nan)` | total_key | `0x8007ffffffffffff` | `0x8007ffffffffffff` | same |
| `key(-inf)` | total_key | `0x800fffffffffffff` | `0x800fffffffffffff` | same |
| `key(-1)` | total_key | `0xc00fffffffffffff` | `0xc00fffffffffffff` | same |
| `key(-0)` | total_key | `0xffffffffffffffff` | `0xffffffffffffffff` | same |
| `key(+0)` | total_key | `0x0` | `0x0` | same |
| `key(1)` | total_key | `0x3ff0000000000000` | `0x3ff0000000000000` | same |
| `key(inf)` | total_key | `0x7ff0000000000000` | `0x7ff0000000000000` | same |
| `key(nan)` | total_key | `0x7ff8000000000000` | `0x7ff8000000000000` | same |
| `f64(12345u64)` | u64_to_f64 (sticky halving) | `0x40c81c8000000000` | `0x40c81c8000000000` | same bits |
| `f64(12345u64)` | f64_from (signed) | `0x40c81c8000000000` | `0x40c81c8000000000` | same bits |
| `f64(2^53+1)` | u64_to_f64 (sticky halving) | `0x4340000000000000` | `0x4340000000000000` | same bits |
| `f64(2^53+1)` | f64_from (signed) | `0x4340000000000000` | `0x4340000000000000` | same bits |
| `f64(2^63)` | u64_to_f64 (sticky halving) | `0x43e0000000000000` | `0x43e0000000000000` | same bits |
| `f64(2^63)` | f64_from (signed) | `0x43e0000000000000` | `0xc3e0000000000000` | **9781818390648717312 ulp** |
| `f64(2^63+2^11+1)` | u64_to_f64 (sticky halving) | `0x43e0000000000001` | `0x43e0000000000001` | same bits |
| `f64(2^63+2^11+1)` | f64_from (signed) | `0x43e0000000000001` | `0xc3dffffffffffffe` | **9781818390648717311 ulp** |
| `f64(u64max)` | u64_to_f64 (sticky halving) | `0x43f0000000000000` | `0x43f0000000000000` | same bits |
| `f64(u64max)` | f64_from (signed) | `0x43f0000000000000` | `0xbff0000000000000` | **9502595213751746560 ulp** |
| `usize(floor(-3.7))` | sat_u64 | `0x0` | `0x0` | same |
| `usize(floor(-3.7))` | f64_to (no saturation) | `0x0` | `0xfffffffffffffffc` | **DIFFERS** |
| `usize(floor(7.9))` | sat_u64 | `0x7` | `0x7` | same |
| `usize(floor(7.9))` | f64_to (no saturation) | `0x7` | `0x7` | same |
| `usize(1e29)` | sat_u64 | `0xffffffffffffffff` | `0xffffffffffffffff` | same |
| `usize(1e29)` | f64_to (no saturation) | `0xffffffffffffffff` | `0x7fffffffffffffff` | **DIFFERS** |
| `usize(0)` | sat_u64 | `0x0` | `0x0` | same |
| `usize(0)` | f64_to (no saturation) | `0x0` | `0x0` | same |
| `usize(nan)` | sat_u64 | `0x0` | `0x0` | same |
| `usize(inf)` | sat_u64 | `0xffffffffffffffff` | `0xffffffffffffffff` | same |
| `hi(-2.5)` | bits >> 32 | `0xc0040000` | `0xc0040000` | same |
| `hi(1e-300)` | bits >> 32 | `0x1a56e1f` | `0x1a56e1f` | same |
| `hi(0.84375)` | bits >> 32 | `0x3feb0000` | `0x3feb0000` | same |
| `lowmask(1.2345678)` | bits & 0xFFFFFFFF00000000 | `0x3ff3c0ca00000000` | `0x3ff3c0ca00000000` | same bits |
| `EPSILON` | positional literal | `0x3cb0000000000000` | `0x3cb0000000000000` | same bits |
| `MIN_POSITIVE` | positional literal | `0x10000000000000` | `0x10000000000000` | same bits |
| `MAX` | positional literal | `0x7fefffffffffffff` | `0x7fefffffffffffff` | same bits |
| `INFINITY` | hex bits | `0x7ff0000000000000` | `0x7ff0000000000000` | same bits |
| `NAN` | hex bits | `0x7ff8000000000000` | `0x7ff8000000000000` | both NaN |
| `PI` | F64_PI | `0x400921fb54442d18` | `0x400921fb54442d18` | same bits |
| `E` | F64_E | `0x4005bf0a8b145769` | `0x4005bf0a8b145769` | same bits |
| `TAU` | F64_TAU | `0x401921fb54442d18` | `0x401921fb54442d18` | same bits |
| `lit(1e-15)` | positional literal | `0x3cd203af9ee75616` | `0x3cd203af9ee75616` | same bits |
| `lit(1e-300)` | positional literal | `0x1a56e1fc2f8f359` | `0x1a56e1fc2f8f359` | same bits |
| `lit(1e300)` | positional literal | `0x7e37e43c8800759c` | `0x7e37e43c8800759c` | same bits |
| `lit(1e9)` | positional literal | `0x41cdcd6500000000` | `0x41cdcd6500000000` | same bits |
| `lit(0.1)` | literal | `0x3fb999999999999a` | `0x3fb999999999999a` | same bits |
| `lit(LN_SQRT_2PI)` | literal | `0x3fed67f1c864beb5` | `0x3fed67f1c864beb5` | same bits |
| `sum([])` | loop from -0.0 | `0x8000000000000000` | `0x8000000000000000` | same bits |
| `sum([])` | loop from 0.0 | `0x8000000000000000` | `0x0` | **sign of zero differs** |
| `sum([+0])` | loop from -0.0 | `0x0` | `0x0` | same bits |
| `sum([-0])` | loop from -0.0 | `0x8000000000000000` | `0x8000000000000000` | same bits |
| `sum([-0])` | loop from 0.0 | `0x8000000000000000` | `0x0` | **sign of zero differs** |
| `lt(nan,1)` | < | `0x0` | `0x0` | same |
| `eq(nan,nan)` | == | `0x0` | `0x0` | same |
| `ne(nan,nan)` | != | `0x1` | `0x1` | same |
| `lt(-0,+0)` | < | `0x0` | `0x0` | same |
| `eq(-0,+0)` | == | `0x1` | `0x1` | same |
| `ge(nan,0)` | >= | `0x0` | `0x0` | same |

## 2. The golden-value generator (M0 step 5, 2026-11-15)

**What ran:**
- **The generator:** `scripts/golden/`, adapted from pravash's `scripts/oracle/`. It drives `rust-old/` with
  `oracle` and `soorat-compat`, on rustc 1.100.0 and glibc 2.44 (x86_64, FMA ifuncs).
- **The re-check:** `scripts/golden/validate.py`, which never runs the oracle.
- **Correct rounding:** MPFR 4.2.2, through kimiya's binding core (`scripts/golden/mpfr_ref.py`, 8 of 8 self-checks).

**Result.** `tests/golden/math.tsv` has 35,597 rows in 31 functions, and every one re-checks:

| Class | Functions (rows) | How `validate.py` recomputes them |
|---|---|---|
| A | sqrt (1,274), abs (374), floor (843), fract (871), max (760), min (760), clamp (520), is_finite (34), is_nan (34), total_cmp (169), as_usize (633), u64::as_f64 (536), high_word (680), low_masked (680) | Python, in Rust's operation order |
| B | powi 2, 3, 4 (510 each); powi 9, 25, 49 (410 each) | LLVM's square-and-multiply, low bits first |
| C, glibc | exp (3,274), ln (3,112), ln_1p (1,013), exp_m1 (1,010), powf (4,400), sin (2,510), cos (2,510), tan (2,010), atan (1,510), log2 (2,510) | the same glibc functions through ctypes |
| C, compiler_builtins | cbrt (810) | MPFR: **every row correctly rounded** |

**Where glibc misrounds.** MPFR's correctly rounded value differs on 76 of the 24,709 class-C rows, each by exactly
1 ulp. These 76 rows are `tests/golden/math.cr.tsv`, the 0-ulp expectations for M1's correctly rounded primitives
(ADR 009):

| Function | Not correctly rounded |
|---|---|
| `exp_m1` | **52 of 1,010 (5.1%)** |
| `tan` | 7 of 2,010 (0.35%) |
| `ln_1p` | 6 of 1,013 (0.59%) |
| `powf` | 6 of 4,400 (0.14%) |
| `sin` | 4 of 2,510 (0.16%) |
| `log2` | 1 of 2,510 (0.04%) |
| `exp`, `ln`, `cos`, `atan` | 0 on these grids |
| `cbrt` (compiler_builtins) | 0: correctly rounded |

`exp_m1` stands out, at 5.1%. kimiya measured 7% on its own corpus, and the plan's random baseline (§3.2) found
1.19%. The rate depends on where the inputs are:
- 39 of the 52 misrounds are at x ≥ 10, where expm1(x) ≈ exp(x). The 400-row sweep over [−40, 710] puts most of
  its rows there.
- 5 are at negative x, which is where pramana's Exponential and Weibull CDFs call it (−exp_m1(−x)).

Either way, `exp_m1` is the primitive glibc gets wrong most often.

**Determinism and tampering:**
- two runs give byte-identical files, and `--check` passes;
- a one-bit change to an `exp` row fails both `--check` (`DIFFERS`) and `validate.py`;
- an edited `.cr.tsv` row fails `validate.py`;
- a header naming glibc 2.45 is refused by `--check` with a `PROVENANCE` line naming both.

**Not in CI.** The rows are glibc 2.44's bits, so the generator check and `validate.py` are local gates, run at each
milestone, as pravash's were.

## 3. The constant generator (M0 step 6, 2026-11-15)

**What ran.** `scripts/golden/` (with `src/consts.rs`, `src/p_constants.rs`, `src/m_constants.rs` and
`src/t_constants.rs`) wrote `src/constants.cyr` and `tests/golden/constants.tsv`. It extracts **112 constants**:

| From | Constants |
|---|---|
| `math.rs` | `LN_SQRT_2PI`, `CONVERGENCE_EPS`, `LENTZ_TINY`, `MAX_ITERATIONS`, `GAMMA_WILSON_HILFERTY_SHAPE`; the 55 fdlibm coefficients; Lanczos's 9 and `G` |
| `hypothesis.rs` | Acklam's A[6], B[5], C[6], D[4], `P_LOW`, `P_HIGH` (= 1.0 − P_LOW, 0.97575); the KS series' `PI2_OVER_8`, `SQRT_2PI` |
| `distribution.rs` | `POISSON_PTRS_THRESHOLD` (30.0), `BINOMIAL_DIRECT_MAX_N` (64), `MVN_SYMMETRY_TOL` (1e-10) |
| `markov.rs` | `STEADY_STATE_STALL_LIMIT` (8), `MAX_PREALLOCATED_STEPS` (1 << 16 = 65,536) |
| `bayesian.rs` | `COHERENCE_TOL` (1e-12) |
| `ai.rs` (public) | `MAX_FORECAST_STEPS` (1,000), `MAX_AR_ORDER` (32) |
| Rust std | `f64::EPSILON`, `MIN_POSITIVE`, `INFINITY`, `NEG_INFINITY`, `NAN`, `consts::PI`, `E`, `TAU`, `SQRT_2` (`f64::MAX` is named only in tests) |

**The proofs** (every one passes; the generator writes nothing otherwise):

| Proof | Constants | Cases | Against |
|---|---|---|---|
| fdlibm erf table | 55 | 5,061 | `oracle::erf` |
| fdlibm erfc table | 55 | 5,061 | `oracle::erfc` |
| Lanczos table, `G`, `LN_SQRT_2PI` | 11 | 4,009 | `oracle::ln_gamma` |
| Acklam tables, `P_LOW`, `P_HIGH` | 23 | 3,514 | `oracle::z_quantile` |
| KS series constants | 2 | 3,007 | `oracle::kolmogorov_sf` |
| sampler thresholds | 2 | 4 | the draws of `Poisson` and `Binomial` either side of each threshold |

**Mutation (`GOLDEN_NUDGE`, +1 ulp):**
- **Caught (11):** `RB3`, `EFX8`, `PA0`, `COEFFICIENTS[4]`, `G`, `LN_SQRT_2PI`, `C[2]`, `B[4]`, the KS `SQRT_2PI` and
  `PI2_OVER_8`, `POISSON_PTRS_THRESHOLD`.
- **Not caught, because no output depends on that bit:**
  - `QQ5`'s term is about 10⁻²² of its polynomial;
  - `z_quantile`'s two branches agree at `P_LOW` after the Halley step, so moving the boundary by an ulp changes
    no result.
- **Not caught, because they are extraction-only by design:** `CONVERGENCE_EPS` and `MVN_SYMMETRY_TOL`. Their
  modules' rows test them at M2 and M3.

**Exactness.** Two independent extractors agree on all 112 values: the generator's (Rust's `str::parse`) and
`validate.py`'s (Python's `float`). The Cyrius compiler reads every generated literal and bit pattern back exactly
(`probe_tables.py`, warning-free build).

**Tampering.** A one-digit change to `src/constants.cyr` fails `probe_tables.py` and `--check`. An edited
`constants.tsv` value fails `validate.py`.

## 4. The parity harness (M0 step 7, 2026-11-15)

**What ran.** `cyrius test tests/parity.tcyr` (also part of plain `cyrius test` and `cyrius audit`). It uses
pravash's `tests/parity_core.cyr` and `tests/ulp.cyr`, pramana's hooks in `tests/parity.tcyr`, and the generated
`tests/constants_rows.cyr`.

**First run: 2 modules, 0 failing; 3,963 ok, 0 failed, 24,669 measured, 7,077 pending.** It is warning-free, and the
report is identical under `--bounds`.

| Rows | Functions |
|---|---|
| ok (exact) | all 112 constants; sqrt 1,274, abs 374, floor 843, high_word 680, low_masked 680 |
| measured (class C, no bound yet) | exp, ln, ln_1p, exp_m1, powf, sin, cos, tan, atan, log2, cbrt on their stdlib and ganita counterparts |
| pending (M1's helpers) | fract, max, min, clamp, is_finite, is_nan, total_cmp, as_usize, u64::as_f64, the six powi expansions |

**The measured rows are the step 8 baseline** (x86_64 ELF). Rows 1 ulp off Rust's (glibc's) value:

| Function | Counterpart | Rows 1 ulp off | Shadow (native builtin) |
|---|---|---|---|
| exp | `_f64_exp_polyfill` | 264 of 3,274 | `f64_exp`: 96 |
| ln | `_f64_ln_polyfill` | 64 of 3,112 | `f64_ln`: 14 |
| ln_1p | `_gn_log1p` | 6 of 1,013 | — |
| exp_m1 | `_gn_expm1` | 52 of 1,010 | — |
| powf | `ganita_f64_pow` | 348 of 4,400, plus 1 NaN/∞ | — |
| sin / cos | `f64_sin` / `f64_cos` | 89 / 78 of 2,510 | — |
| tan | `ganita_f64_tan` | 82 of 2,010 | — |
| atan | `_f64_atan_polyfill` | 22 of 1,510 | `f64_atan`: 0 |
| log2 | `_f64_log2_polyfill` | 1 of 2,510 | `f64_log2`: 0 |
| cbrt | `ganita_f64_cbrt` | 0 of 810 | — |

**Notes on the baseline:**
- **The NaN/∞ powf row is ganita's open `pow(−1, huge)` defect.** `powf(−1, f64::MAX)` gives +∞ where Rust gives 1.
- **The native builtins** agree with glibc more often on x86_64, since they are x87 there. They differ across targets
  (step 8), so the polyfills are dispatched.
- **The stdlib `f64_min` / `f64_max`** differ from Rust on all 69 rows with a NaN second operand, as the shadow lines
  show: ledger L2.

**Against MPFR:** the 76 correctly rounded rows (`math.cr.tsv`) load and each matches its golden row. Today's
counterparts agree with the corrected value on:
- powf: 4 of 6;
- tan: 3 of 7;
- log2: 1 of 1;
- exp_m1, ln_1p and sin: none. Their counterparts give glibc's misrounded value on all 52, 6 and 4.

Every corrected row is measured until M1 dispatches the correctly rounded functions at 0 ulp.

**Mutation checks**, each failing its module (exit 1):
- one bit of a golden `sqrt` row: 1 FAILED;
- `LN_SQRT_2PI` one ulp off in `src/constants.cyr`: 1 FAILED;
- `floor` dispatched to `f64_ceil`: 613 FAILED;
- a correction row that matches no golden row: the module fails with "a correction matches no golden row".

