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

## 5. Cross-target baselines and the class-C bounds (M0 step 8, 2026-11-15)

**Method.** The same replay runs three ways:
- `cyrius test tests/parity.tcyr` (x86_64 Linux ELF);
- with `--win` (x86_64 PE, under wine);
- with `--aarch64` (aarch64 Linux, under qemu).

Each run is warning-free. Beside the histograms, the harness prints a **digest** per function (pravash's): a 64-bit
FNV-1a fold of every output word the port returns, in row order. Equal digests mean bit-identical results on every
row, which histograms alone can't show.

**Result: every dispatched function gives the same bits on all three targets.**

| Digest | x86_64 ELF | x86_64 PE | aarch64 |
|---|---|---|---|
| `const` | `0x3b0f41cb2166df8d` | same | same |
| `f64::exp (_f64_exp_polyfill)` | `0x32a64c8e05b9e1fc` | same | same |
| `f64::ln (_f64_ln_polyfill)` | `0x907874d5c725c32d` | same | same |
| `f64::ln_1p (_gn_log1p)` | `0xd4b8213088231be3` | same | same |
| `f64::exp_m1 (_gn_expm1)` | `0x4c3f65a2313500af` | same | same |
| `f64::powf (ganita_f64_pow)` | `0xec08705faae6b14e` | same | same |
| `f64::sin (f64_sin)` | `0xf6944bfca1fa873a` | same | same |
| `f64::cos (f64_cos)` | `0x4376d8853d781d23` | same | same |
| `f64::tan (ganita_f64_tan)` | `0x8283e403faee3d66` | same | same |
| `f64::atan (_f64_atan_polyfill)` | `0x1e6b9857f97bd6c0` | same | same |
| `f64::log2 (_f64_log2_polyfill)` | `0xd39f611333e1ba8f` | same | same |
| `f64::cbrt (ganita_f64_cbrt)` | `0x96d8cfa0746d7e84` | same | same |
| `class A/B math rows` | `0x3072bceffdb8a1fa` | same | same |
| `shadow: f64_exp (native)` | `0xe1c91bbeb7773532` | same | `0x32a64c8e05b9e1fc` |
| `shadow: f64_ln (native)` | `0x695843fe8604429` | same | `0x907874d5c725c32d` |
| `shadow: f64_atan (native)` | `0x3858beef12ee1f82` | same | `0x1e6b9857f97bd6c0` |
| `shadow: f64_log2 (native)` | `0x53e07328846fc92c` | same | `0xd39f611333e1ba8f` |

**The native builtins differ on aarch64, by design.** On x86_64, `f64_exp`, `f64_ln`, `f64_atan` and `f64_log2` are
x87 code. On aarch64 they are the polyfills, so there each shadow's digest equals the dispatched polyfill form's
x86_64 digest. That cross-match independently confirms the choice: the polyfills are the cross-target forms. PE
matches ELF in every line, shadows included, as wine runs the same x87 code. The per-function histograms are
identical on all three targets, except the four native shadows'.

**The class-C bounds** (`_par_c_bound` in `tests/parity.tcyr`), set before each function's first gated comparison:

| Function | K (ulp) | Why | Measured max (all 3 targets) |
|---|---|---|---|
| exp, ln, ln_1p, exp_m1, sin, cos, tan, atan, log2 | **2** | the siblings' rule for primitives: glibc and each Cyrius counterpart are within about 1 ulp of the correctly rounded value, so two implementations can differ by 2 | 1 |
| cbrt | **0** | Rust's (compiler_builtins) and ganita's are both correctly rounded | 0 |
| powf | — (measured until M1) | today's counterpart, ganita's, is within 1 ulp on 4,399 of 4,400 rows. The other row is ganita's open `pow(−1, \|y\| > 2^64)` defect: +∞ against Rust's 1. M1's correctly rounded pow replaces it, and no ADR accepts a counterpart that is being replaced | 1, plus that row |

These are the interim bounds. When M1 dispatches the correctly rounded functions, every bound becomes **0** (ADR 009).
Until then, a corrected row (`math.cr.tsv`) passes as "accepted (ADR 9)" when the counterpart is within 2 ulp of the
correctly rounded value. That is a weaker statement than M1's.

**Gated result, identical on all three targets:** 2 modules, 0 failing; 24,162 ok, 0 failed, 4,400 measured (powf),
7,077 pending, and 70 accepted (ADR 9). The other 6 corrected rows are powf's, measured.

**Mutation checks at the bounds' edges** (ELF; each fails, exit 1):
- exp's bound set from 2 to 0: its 264 one-ulp rows fail;
- exp moved 3 ulps under K = 2: 3,129 rows fail;
- exp moved 2 ulps under K = 2: only the 246 rows it pushes to 3 ulps or into NaN/∞ fail, so exactly 2 passes and
  3 fails;
- cbrt moved 1 ulp under K = 0: 809 of 810 rows fail.

The first attempt at these "ulp" mutations wrote `r + 3` on a typed `f64`, which is float arithmetic, not a bit step.
They were redone with an untyped word helper: the left-operand typing trap the plan warns about (§8.1).

## 6. Probes (M0 step 9)

The probes answer the plan's design questions with numbers before M1 writes code. The Cyrius probes are in
`scripts/probes/`. Their Rust twins are binaries of `scripts/probes/rust/`, a crate on the oracle by path, its lock
seeded from `scripts/golden/` (hisab 1.4.0). Run them from that directory, on the recording toolchain.

### 6.1 Part 9a: P1, P5, P6 (2026-11-15)

**P1, integers: everything identical to the oracle** (`p1_integers.cyr` against `p1_integers`, the oracle's own
functions). 3,736 cases, 3,736 the same:
- **`SimpleRng`:** 10⁶ draws from each of five seeds (1, 42, 2⁶³, u64::MAX, 0x9e3779b97f4a7c15), folded into an FNV-1a
  digest. `>>` is the logical shift, and the wrapping `<<` matches.
- **`PmnU128`, D10's prototype:** a two-word u128 with a 128 × 64-bit multiply over 32-bit limbs (no partial product
  overflows a word) and overflow detection, and a 128 / 64-bit restoring division.
  - It ran `factorial(0..=36)`, and `permutations` and `combinations` on 54 values of n (0–40, the u128 edge 130–133,
    2³², 2⁶³ and u64::MAX) × each n's r grid.
  - Outcomes and words are identical, errors included. `combinations(130, 65)` is an overflow error in Rust too: an
    intermediate product overflows although the result fits.
- **Mutation:** dropping `combinations`' `r = min(r, n − r)` changes 30 cases. Breaking the multiply's overflow check
  makes `permutations(2⁶³, …)` wrap instead of stopping, so the probe runs for about 2⁶³ steps.

**P5, the linear algebra: transcribe all four hisab 1.4.0 routines** (plan §8.2, ledger L4–L6). The Rust twin calls
1.4.0's routines. `p5_linalg.cyr` computes the same seeded inputs with ganita, and with hisab 3.5's
`eigen_symmetric`, whose 4.2 MB bundle is included by absolute path (`CYRIUS_ALLOW_ABSOLUTE_INCLUDES=1`). The inputs
are SPD matrices BᵀB + n·I for n = 2, 3, 4, 6, 8, 10 at 10 seeds, and a degree 1–4 fit to 40 points at 10 seeds.

| Routine (against hisab 1.4.0) | Elements | Identical | 1 ulp | 2 ulp | 3–16 ulp | More | Max |
|---|---|---|---|---|---|---|---|
| `ganita_mat_cholesky` | 1,310 | 1,046 | 204 | 41 | 15 | 4 | 40 ulp |
| `ganita_mat_cholesky_solve` (on ganita's L) | 330 | 231 | 46 | 29 | 23 | 1 | 18 ulp |
| `ganita_mat_eigen_sym` | 330 | 61 | 77 | 68 | 124 | 0 | 14 ulp |
| hisab 3.5 `eigen_symmetric` | 330 | the same as ganita's on every element | | | | | 14 ulp |
| `ganita_mat_least_squares` (Householder) | 140 | 9 | 4 | 8 | 71 | 48 | 13,619 ulp |

Why each differs:
- **Cholesky:** ganita accumulates Σ l·l and subtracts it once; 1.4.0 subtracts each term in turn.
- **The eigensolvers:** ganita's Jacobi picks the largest pivot and stops at an absolute 1e-12; 1.4.0's is cyclic
  with an absolute 1e-24 stop.
- **Least squares:** ganita is Householder QR; 1.4.0 is modified Gram–Schmidt on the raw Vandermonde matrix.

**What each call leaves on the bump heap:**

| Routine | Bytes left per call |
|---|---|
| `ganita_mat_cholesky_solve` | 8n |
| `ganita_mat_eigen_sym` | 64–896 (≈ 9n²) |
| hisab 3.5 `eigen_symmetric` | 80–1,680 (≈ 16n²) |
| `ganita_mat_least_squares` (40 × 2–5) | 1,296–2,256 |

**A caveat for M4.** 1.4.0's Jacobi takes its angle from glibc's `atan`, `cos` and `sin`. So a faithful
transcription on correctly rounded primitives will still differ wherever glibc misrounds. The PCA rows will need
`.cr` rows from a Python transcription, or class D.

**P6, sort and select: Rust's unstable selection leaks the sign of a zero.** `median` and `percentile` take the k-th
element from `select_nth_unstable_by`, and its successor as the first minimum of the rest, in whatever order the
algorithm left them. Equal non-zero floats have identical bits, so only ±0 can differ.

The Rust twin ran 20,000 zero-heavy inputs (elements from −0, +0, −1, 1, 2; lengths 1–24) through `median` and
8 percentiles. `p6_model.py` recomputes each with selection from a stable sort.

| Result | Differ from stable selection |
|---|---|
| `median` | 2,102 of 20,000 |
| `percentile(0)` | 0 |
| `percentile(10 / 25 / 33.3 / 50 / 75 / 90 / 100)` | 170 / 1,181 / 1,519 / 2,102 / 1,009 / 149 / 1,161 |
| all | **9,393 of 180,000**, every one only in the sign of a zero |

By length, the rate rises sharply from 17 elements, where Rust's selection leaves its small-slice path for
partitioning. It still occurs at every length from 2 (17–169 per length below 17; 677–1,348 from 17 to 24).

To reproduce it, the port would transcribe rustc 1.100's `select_nth_unstable_by` whole: its small-slice and min/max
paths, pivot choice, partitioning and fallback, about 300 lines of Rust's core library, pinned to that compiler.
Otherwise it accepts the divergence as ADR 007 row 14, with a documented stable rule.

**Decided (the maintainer, 2026-11-15): accept the divergence.**
- **The rule:** `median`, `percentile`, `quartiles` and `iqr` take the k-th element, and its successor, of a stable
  sort of the input. Ties, −0 and +0 among them, keep their input order. That is `p6_model.py`'s rule.
- **What the harness does:** the values always equal Rust's. Where Rust's sign of zero differs, `validate.py` maps
  the row to the stable rule's result (`descriptive.map.tsv`, at M4), and it passes as accepted (ADR 7).
- **The precedent:** pravash's sort ties (its ADR 0002 #3).

### 6.2 Part 9b: P2, P3, P4 (2026-11-15)

**How they were timed.** All three are `.bcyr` programs run by `cyrius bench`. Each was run on x86_64 ELF, pinned to one
core (`taskset -c 5`), three times, interleaved, at a load average of 5–7 on the shared box. The figures are ns per
call. The runs agree to within 1 ns, except one draw run that was 3–5 ns slower. Each row's loop tests a kind per
iteration, so compare the rows; don't read a row as a bare call.

**P2, dispatch (`p2_dispatch.bcyr`).** A distribution's pdf and an RNG's draw, 10⁶ calls per row:

| Call form | pdf (arithmetic only) | pdf with exp | draw |
|---|---|---|---|
| direct method call | 14 | 57 | 12 |
| bounded generic, `<T: PmnDistribution>` / `<R: PmnRng>` | 17 | | 14 |
| `dyn PmnDistribution` / `dyn PmnRng` | 18 | 64 | 15 |
| `callptr(&fn, ctx, …)` | 14 | 57 | 12 |

- **The wrapper:** the generic and dyn rows call through a one-line wrapper, so each makes one call more than the
  direct row. The generic's +2–3 ns is that call; its own call is static.
- **The dyn:** it costs 1 ns more than the generic through the same wrapper, and 3–7 ns more than a direct call.
  That is a quarter of a 12 ns draw, and the gamma, Poisson and beta samplers draw several times per sample.
- **`callptr`:** with the callee known as `&f`, it costs the same as a direct call.
- **What it means for D7:** generics give Rust's `&mut impl Rng` with static calls. A `dyn PmnRng` at every API
  boundary (D7 as first decided) costs about 3 ns per draw over that. So D7 was amended (2026-11-15): the RNG is
  generic over `R: PmnRng`, with `dyn` only for Gibbs' conditionals (ADR 007 row 6).

**P3, callbacks (`p3_callbacks.bcyr`).** A library-shaped fn receives the callback as a parameter and calls it 10⁶
times. Every form's sum equals the direct call's bits in all three runs.

| Callback | direct | `callptr(&fn, …)` | `callptr(&fn, ctx, …)` | closure, no capture | closure capturing its context |
|---|---|---|---|---|---|
| `f(x)` (Monte Carlo, the KS cdf) | 5 | 6 | 6 | 5 | 5–6 |
| `log_target(xs, 4)` (MH) | 15 | | 16 | | 18 |
| `cond(xs, 4, rng)` (Gibbs) | | | 21 | | 23 |

- **Heap:**
  - a capturing closure's environment is 16 bytes (its code pointer and one f64; 8 more per capture), made where the
    literal is evaluated;
  - calls allocate nothing, for either form;
  - 1,000 closures made in a loop leave 16,000 bytes, which the bump heap never frees.
- **One parameter takes every form.** `callptr` tells a closure from a function pointer at run time (the handle's top
  bit). So a library fn whose parameter is declared `f: *fn(f64): f64` takes `&fn`, a closure held in a variable, or
  an inline literal, all without a warning.
- **Windows:**
  - a capturing closure takes at most 3 parameters, and 4 is a compile error on `--win` ("capturing closure has too
    many parameters"), not a silent fault;
  - `callptr` itself has no limit: a 4-argument call through a function pointer runs under wine (exit 23, as
    computed).
- **A trap: a captured f64 is an untyped word in arithmetic.**
  - In `|x| k * x` with `k: f64` captured, the multiply is an integer one: 3.0 × 0.25 gives the bits 0x0, under the
    warning *integer arithmetic with an f64 right operand*.
  - `x * k` is right, but it warns *f64 arithmetic with a non-f64 right operand*.
  - The guide (6.7.16, its D3) makes a captured f64 bind `T = f64` as a generic argument, so arithmetic is the odd
    case out. Filed at step 10.
  - The rule: copy a capture into a typed local first (`var kf: f64 = k;`). `cyrius test` must be warning-free, so the
    warning catches a missed copy.
- **Gibbs' RNG.** A dyn can't be pointed to (`*dyn` is refused), and a generic fn can't hold one.
  - A sampler generic over `R: PmnRng` passes `rng` to a non-generic helper that takes `dyn PmnRng`, and the dyn is
    formed at that call.
  - The helper keeps a `struct PmnRngRef { o: dyn PmnRng; }` on its stack and hands `&rr` to each conditional.
  - A conditional reads it as `var rr: PmnRngRef = rp; rr.o.next_f64()`. Nothing allocates, and the probe checks
    that this path gives the conditional the same draws as a dyn formed directly.
  - Binding the untyped parameter to `*PmnRngRef` warns *assigning non-pointer to typed pointer*; the pointer-mode
    handle `var rr: PmnRngRef = rp;` does not.
- **`cyrius lint` and closures.** lint first compiles the file without the manifest's stdlib, so a capturing closure is
  then a hard error, *a capturing closure needs include "lib/alloc.cyr"*. Missing functions there are only warnings.
  A file that builds one includes `lib/alloc.cyr` itself; include-once makes that a no-op in the build. Filed at step
  10.
- **What it means for D7:**
  - The function pointer plus a context word stands: it costs what a closure costs, never allocates, and needs no
    capture rule.
  - One correction to D7's rejected column. A Gibbs conditional needs 4 parameters only with an explicit context. A
    closure that captures its context needs 3, which fits Windows.
  - Closures still work under D7. A non-capturing closure is a bare function pointer, of any arity.

**P4, the flat heap (`p4_memory.bcyr`).** It prototypes D11's conventions:
- **constructors** fill caller storage and return a value-form `Result`;
- **owned objects** (the MVN, the HMM, the MCMC result) keep their buffers in one freelist block, freed by `_drop`;
- **scratch** comes from the work-table pool (below);
- **a caller buffer** `(out, cap)` is checked for null, size and aliasing.

Each soak runs once to warm up, then compares four things after the loop: the bump heap's bytes, the freelist blocks
held, the process's mapped size (`/proc/self/statm`) and the pool slots claimed.

| Soak | Per iteration | Heap after |
|---|---|---|
| 10⁶ × `PmnNormal_new` into a stack struct, pdf, a refused constructor, and `PmnExponential`'s cdf | 172 ns | flat |
| 10⁵ × MVN ln pdf, d = 4 (the triangular solve in pool scratch) | 122 ns | flat |
| 10⁴ × MVN new + `_drop`, d = 4 (160 bytes: a freelist class) | 380 ns | flat |
| 10³ × MVN new + `_drop`, d = 32 (8,448 bytes: over 4 KB, so mmap + munmap) | 36 µs | flat |
| 100 MH runs of 10⁴ samples, d = 2, + `_drop` (10⁶ samples; a 160 KB result per run) | 333 ns a sample | flat |
| 10³ Baum–Welch iterations, 3 states, 4 symbols, T = 200 | 75 µs | flat |
| 10 medians of 10⁶ (a NaN-checked copy and a stable merge sort, in two 8 MB claims) | 222 ms | flat |
| 10⁶ × moving average into a caller buffer (16 points, w = 4) | 94 ns | flat |
| 10⁴ × nine nested claims, the ninth past the pool | 395 ns | flat |

Every check passes:
- Baum–Welch's log-likelihood never decreases over 10³ iterations;
- MH's acceptance rate and sample mean are in range;
- the median of a shuffled 0 … 10⁶ − 1 is 499,999.5;
- a NaN is refused, and both of its slots come back;
- the moving average refuses a null out, a short cap, an out that overlaps its input, and a window over n.

**The pool** is kimiya's ADR 0007 with one change. A slot grows on the freelist and frees the buffer it outgrew, and a
claim past the eighth slot is a freelist block, freed on release. kimiya takes both from the bump heap, which never
frees.
- **Measured:** growing one slot from 8,000,000 to 16,000,000 to 32,000,000 bytes mapped 23,436 KB more, against the
  pool's own growth of 23,437 KB. Bump growth would keep the outgrown 24,000,000 bytes besides.
- **The cost is kimiya's:** the pool holds its buffers for the process's lifetime, 39 MB after these soaks (the two
  median slots).

**Two findings for later milestones** (speed is measured, not gated):
- **Owned objects over 4 KB** pay the freelist's open large-block issue (cyrius, 2026-11-11): 36 µs per MVN new +
  drop at d = 32, and an mmap for each MH result. pramana adds these numbers to that issue at step 10.
- **The stable median costs a full sort:** 222 ms for 10⁶ elements by merge sort, where Rust selects in O(n). M4 can
  keep ADR 007 row 14's rule at O(n) expected with a stable three-way quickselect over pool scratch. Partitioning into
  <, = and > keeps input order within each part, so the k-th element is the stable sort's k-th. The parity rows decide.

### 6.3 Part 9c: P7, P8 (2026-11-15)

**P7, JSON (`p7_json.cyr`, `p7_compare.py`, and the twin `rust/src/bin/p7_json.rs`).** serde_json 1.0.154 wrote
1,495 documents for nine types. Between them they carry every shape pramana serializes:
- `Normal`: two validated f64s;
- `Binomial`: a u64 and an f64;
- `SimpleRng`: a u64, with zero mapped to 1;
- `KernelDensity`: a vector and a unit enum;
- `PramanaError`: a newtype-variant enum;
- `TestResult`: a string, a bool, and f64s that may be NaN;
- `MarkovChain`: a matrix and a usize;
- `MultivariateNormal`: validated, with a cached factor that decoding ignores;
- `HistogramData`: u64 vectors.

The documents are:
- 132 base values, each also in its pretty form;
- 1,231 variants of each type's first value:
  - whitespace and key order;
  - unknown, missing, duplicate and null fields;
  - 33 number forms and, in string fields, 15 string forms, in every field;
  - enum and sequence forms;
  - syntax faults;
  - the constructors' edge values.

Each row carries serde_json's verdict and the value it read, with `float_roundtrip` on (a correctly rounded parse). The
Cyrius side decodes each document as a hand codec would:
- bayan 1.11.1's value tree;
- serde's rules for the type: an unknown member is skipped, a known one twice refuses, a missing one refuses, and the
  sequence form is read;
- then the constructor's checks.

It then writes the value again with two printers, and serde_json reads both texts back. Run:
- `p7_json corpus > build/p7_rust_corpus.tsv`, then `p7_json floats > build/p7_rust_floats.tsv`;
- `./build/p7_json > build/p7_cyrius.tsv`;
- `p7_json read build/p7_cyrius.tsv`, built with and without `roundtrip`;
- `p7_compare.py`.

| Decoding | Agrees with serde_json | Verdict differs | Value differs |
|---|---|---|---|
| bayan's tree alone | 1,325 | 162 | 8 |
| through the stand-in pre-pass `p7_lift` | 1,460 | 35 | 0 |

| Cause | Differs, tree alone | Fixed by the pre-pass |
|---|---|---|
| an integer at or above 2^63, or below −2^63, in a u64 field, an f64 field or an unread member | 108 | 108 |
| a raw control byte in a string, which serde_json refuses anywhere | 14 | 14 |
| `-0`: an f64 read +0.0, and a u64 read 0 where serde_json refuses the float −0.0 | 13 | 13 |
| a duplicate key in a member the type doesn't read | 19 | 0 |
| nesting past 128 levels in an unread member | 8 | 0 |
| a lone surrogate in an unread member | 8 | 0 |

- **Saved RNG states.** 13 of the 35 `SimpleRng` base states are above 2^63. bayan alone refuses each of those
  documents, so a saved state could not be restored about a third of the time.
- **The pre-pass** rewrites the text before bayan parses it (86 lines):
  - a negative integer becomes a float (`-1.0`, `-0.0`), so an f64 field keeps −0.0 and a u64 field refuses, both as
    serde_json does;
  - an integer in 2^63 … 2^64 − 1 becomes the negative integer with the same bits, which nothing else now produces.
    A u64 reader takes the bits back, and an f64 reader rounds them as unsigned;
  - a larger integer becomes a float, which an f64 field reads correctly rounded and a u64 field refuses;
  - a raw control byte inside a string refuses the document.
  It covers three open bayan issues: integers above i64 (impetus), `-0` (pravash) and raw control bytes (ushma).
- **The 35 left** are documents serde_json accepts only because it skips a member without checking it, where bayan
  checks the whole document. serde_json never writes one for these types. bayan's open request for that opt-in is
  impetus's duplicate-key issue. The nesting and lone-surrogate cases are new to it, and pramana adds them at step 10.
- **Error codes.** Every agreeing refusal maps one to one:
  - a constructor's refusal gives its own code (`INVALID_PARAMETER` 39, `INVALID_SAMPLE` 2, `DIMENSION_MISMATCH` 7);
  - every other serde_json error gives `DESERIALIZE` (816).
- **Non-finite values.** serde_json writes NaN and ±∞ as `null`, and refuses `null` for an f64. So Rust can't read
  back its own document for a `TestResult` that holds one. pramana refuses it too, and its writer also writes `null`.
- **MVN.** Decoding ignores the cached `cholesky_l`, `log_det` and `dim`, as serde's repr does, even when they hold
  `"x"`, `null` or `1e400`. Re-encoding recomputes them with hisab 1.4.0's Cholesky, transcribed, and the stdlib ln.
  The text matches serde_json's on all 7 base matrices.

**The text** of the 596 documents both sides read:

| Printer | Text equals serde_json's | pramana reads it back | serde_json reads it, `float_roundtrip` | serde_json reads it, default |
|---|---|---|---|---|
| impetus's `fmtf.cyr`, serde_json form | 596 | 596 | 596 | 528 |
| bayan's Grisu2 | 566 | 596 | 596 | 528 |

serde_json's default parse is best effort. It misreads a float in 68 of the 596 documents, whichever printer wrote
them, and it does the same to documents Rust wrote (ushma measured 1–2 ulp). A Rust consumer that needs exact values
turns on `float_roundtrip`. pramana's golden JSON rows are generated with it on.

**The printers** on 280,562 doubles, with both signs: every f64 in the base values, every power of two, the powers of
ten and their neighbours, the integers 0 … 1,000, 100,000 random bit patterns and 50,000 short decimals.

| Comparison | Differs on |
|---|---|
| `fmtf.cyr`'s serde_json form against serde_json's text | 0 |
| `fmtf.cyr`'s Display form against Rust's `{}` | 0 |
| bayan's Grisu2 against serde_json's text | 9,714 (694 of them longer than the shortest) |
| `bayan_f64_parse` of serde_json's text against the bits written | 0 |

This agrees with ushma's probe 11, where Grisu2 differed on 6,276 of 160,378.

**What it means for D5:**
- Every document serde_json writes decodes to the same bits once the pre-pass is in, except the 35.
- Writing through `fmtf.cyr` instead of bayan's printer gives serde_json's text byte for byte, so text can compare
  as class A. pravash's `fmtf.cyr` is the same digit core with Rust's Debug form added, which D12 needs. One vendored
  file can carry both forms. **Decided 2026-11-15:** D5 is amended to write serde_json's text this way (ADR 007
  row 7).
- The 35 documents need an ADR 007 row until bayan's opt-in lands. **Decided 2026-11-15:** ADR 007 row 16 accepts
  them; their rows map to the refusal.

**P8, the bundle (`p8_bundle.sh`, `p8_bundle/`).** The probe works in scratch copies of pramana. Each holds
`src/constants.cyr` and two stand-ins:
- a core module with one public function per production stdlib leaf;
- an `ai` module that posts through sandhi to a closed port.

They are laid out three ways:
- **profile:** sandhi and its five companions (tls, async, random, fdlopen and dynlib) in `[deps] stdlib`, as ushma
  has it, plus `[lib]` and `[lib.ai]`;
- **outside:** no sandhi in `[deps]`, and `src/ai.cyr` in no bundle;
- **hybrid:** no sandhi in `[deps]`, plus `[lib]` and `[lib.ai]`.

Each copy's tests include an `ai` test that includes `lib/sandhi.cyr` itself.

**The bundles.** `dist/pramana.cyr` is 9,385 bytes, and its compile-verified sidecar lists 9 leaves (math, ganita,
bayan, atomic, freelist, overflow, result, vec, str). `dist/pramana-ai.cyr` is 9,905 bytes, and its sidecar adds
sandhi. The core bundle names sandhi nowhere. The hybrid's `ai` bundle and sidecar are byte-identical to the profile
layout's. In the hybrid, `distlib --check`, `deps --locked` and `audit` all pass.

**pramana's own compiles** (seconds, pinned, three interleaved runs):

| Layout | `cyrius build` | `cyrius test` with the ai test | without it | the ai test alone |
|---|---|---|---|---|
| profile | 1.29–1.30 | 3.43–3.50 | 2.40 | 1.22 |
| outside | 0.49–0.50 | 2.08–2.12 | | |
| hybrid | 0.49 | 2.06–2.08 | 0.99 | 1.19 |

`[deps] stdlib` is prepended to every compile. With sandhi listed there, every build and every test file pays about
0.8 s more. In the hybrid only the `ai` test pays. Its `include "lib/sandhi.cyr"` resolves from the pinned
toolchain's stdlib, so that copy is neither vendored nor locked.

**Consumers**, built with dead-code elimination, in bytes. Each one runs on all three targets with the same output.

| Consumer | x86_64 ELF | PE | aarch64 | Build |
|---|---|---|---|---|
| calls nothing | 411,928 | 1,338,880 | 1,357,992 | 0.51–0.57 s |
| calls one function | 411,928 | 1,338,880 | 1,357,992 | 0.51–0.58 s |
| calls every core function | 477,464 | 1,339,392 | 1,357,992 | 0.51–0.60 s |
| calls one function, through the profile layout's core bundle | 411,928 | 1,338,880 | 1,357,992 | 0.51–0.58 s |
| `ai`: the profile bundle, or the core bundle plus `src/ai.cyr` | 1,316,280 | 3,041,792 | 3,069,056 | 1.30–1.67 s |

- **A one-function consumer** is the size of an empty one. A core consumer never compiles sandhi, TLS or sigil in
  any layout.
- **`ai` adds 839 KB** on x86_64 and about 1 s of build. The profile bundle and the outside file give the same binary.
- **PE warnings.** A PE build of `ai` warns from `lib/dynlib.cyr` and `lib/fdlopen.cyr`: syscalls not routed on
  Windows. ushma met the same.
- **PE and aarch64 binaries keep their dead code.** On x86_64 ELF it is removed (805,391 bytes for the empty
  consumer). On PE and aarch64 it is NOPed but not compacted. The compiler says so, and cyrius's roadmap has it as
  "the DCE compaction arc".

**What it means for D8:** the `[lib.ai]` profile, with sandhi kept out of `[deps] stdlib` (the hybrid):
- consumers get both bundles and their sidecars, the same as ushma's form;
- pramana's own compiles stay at the core's cost;
- only the `ai` test carries sandhi.

### 6.4 Part 9d-1: P9, `ln_gamma` (2026-11-15)

**What it compares.** Three `ln_gamma`s, on the same inputs, against MPFR 4.2.2's lgamma at 1,024 bits
(`scripts/port-baseline/check.c`, errors in ulps of the correctly rounded value):
- **Rust 1.3.0's own results**, taken from the accuracy baseline's binary (§3.2's five sets);
- **T0** (`p9_lgamma_impl.cyr`): Rust's Lanczos form (g = 7, nine terms, reflection below ½) in Rust's operation
  order, on correctly rounded ln and sin. This is what M2 part a ports;
- **T1**, the prototype: fdlibm's `e_lgamma_r` layout, with polynomials centred on the roots at 1 and 2 and on the
  minimum at 1.4616, on correctly rounded ln. There are three changes from fdlibm where a rounding would cost an ulp:
  - every logarithm is kimiya's double-double ln core (`_km_ln_core`, relative error near 2⁻⁸⁰);
  - the product (y + 2)(y + 3)… on [3, 8), and (x − ½)(ln x − 1) for x ≥ 8 at every magnitude up to lnΓ's overflow,
    are double-doubles. fdlibm drops the −½ past 2⁵⁸;
  - for x < 0, sin(πx) is reduced exactly, as r = |x| − round(|x|), and the reflection subtracts in double-double.

  The result is rounded once. The coefficients come from `p9_fdlibm_consts.py`, which reads the libm crate's
  `lgamma_r.rs` and checks each decimal literal against the bits in its comment.

`p9_lgamma.cyr` adds six seeded sets weighted to the hard regions:
- 100,000 within 2⁻⁵² … ¼ of 1 or 2;
- 100,000 on [2, 8);
- 50,000 from 8 to 8·2¹⁰¹⁰;
- 50,000 from 2⁻¹⁰⁷⁰ to 2, subnormals included;
- 48,655 for x < 0, half of them within 2⁻⁵⁰ … 1/10 of a pole (1,345 more rounded onto a pole);
- 50,000 on (−8, −2), where lnΓ crosses zero.

For x < 0, the `mixed` rows measure ADR 009's bound: ulps of max(|lnΓ|, 1). `p9_lgamma.sh` runs it all, and the
1,675,000 rows take about 25 minutes to check.

| Set | n | Rust 1.3.0, max (mean) | T0, max | T1, max (mean) | T1 not CR |
|---|---|---|---|---|---|
| (0.5, 10) | 100,000 | 1.2×10⁶ (65.2) | 1.2×10⁶ | **1.25** (0.258) | 5.1% |
| \|x − 1\| or \|x − 2\| ∈ [1e-12, 0.1] | 25,000 | 4.3×10¹³ (3.8×10¹¹) | 4.3×10¹³ | **0.98** (0.270) | 11.3% |
| [10, 1e15] | 25,000 | 4.85 (0.578) | 4.85 | **0.54** (0.251) | 0.04% |
| (1e-300, 0.5) | 25,000 | 8.21 (0.028) | 8.21 | **0.50** (0.013) | 0.004% |
| P9: within 2⁻⁵² … ¼ of 1 or 2 | 100,000 | — | 8.0×10¹⁶ | **1.15** (0.274) | 11.4% |
| P9: [2, 8) | 100,000 | — | 9.7×10⁵ | **1.31** (0.258) | 4.6% |
| P9: 8 … 8·2¹⁰¹⁰ | 50,000 | — | 4.35 | **0.50** (0.249) | 0% |
| P9: 2⁻¹⁰⁷⁰ … 2 | 50,000 | — | +∞ on 8.4% | **0.78** (0.022) | 0.03% |
| (−20, 0), ulps of max(\|lnΓ\|, 1) | 25,000 | 1,583 (5.36) | 1,583 | **0.93** (0.250) | 9.0% |
| P9: x < 0 to −170, near poles | 48,655 | — | 1.3×10¹⁵ | **1.14** (0.250) | 1.2% |
| P9: (−8, −2), across the zeros | 50,000 | — | 4.1×10⁴ | **1.40** (0.247) | 20.8% |

- **T1 meets ADR 009's fixed bound everywhere:** at most 1.31 ulp for x > 0, against ≤ 2, and at most 1.40 ulp of
  max(|lnΓ|, 1) for x < 0, against ≤ 2. Beside the roots, where Rust is off by 4.3×10¹³ ulp, T1 is within 0.98.
- **The margin** is smallest on [2, 3), where lnΓ(2 + y) = ½y + P(y)/Q(y) is evaluated in plain double. M2 part b can
  carry that sum in double-double if the ≥ 1M verification finds less margin.
- **T0 is Rust.** On §3.2's sets T0 has Rust's maxima, and only one row's rounding differs (the negative set's
  not-CR share is 84.952% against 84.948%). The Lanczos form's error doesn't come from the primitives.
- **Rust 1.3.0 returns +∞ for 0 < x < 1e-300/π** (about 3.2e-301). It reflects below ½ and gives up when
  |sin(πx)| < 1e-300, where lnΓ(x) ≈ −ln x ≈ 690. T0 inherits it, on 8.4% of the 2⁻¹⁰⁷⁰ … 2 set. §3.2's small set
  starts at 1e-300, so it never met this. T1 returns −ln x there, correctly rounded.
- **One answer on every target:** the probe's 1,675,000 rows are byte-identical on x86_64 ELF, PE (wine) and
  aarch64 (qemu), sha256 `1526c4c1…`.

**Cost** (`p9_lgamma.bcyr`): ns per call, pinned, three runs, 10⁶ calls per row over 4,096 seeded inputs.

| Inputs | T0 | T1 |
|---|---|---|
| (0.9, 1.1) | 168 | 72–73 |
| (0.5, 10) | 167–168 | 169–170 |
| [2, 8) | 167–168 | 190–192 |
| [10, 1e6) | 167–168 | 152–154 |
| (−20, −0.5) | 397–400 | 568–573 |

T1 costs about what T0 does for x > 0. It is 2.3× faster beside 1, where no logarithm is taken, and 15% slower on
[2, 8). The reflection costs 1.4× T0's: three double-double logs, sin and cos.

**What it means for M2 part b:**
- T1's design stands: fdlibm's layout, double-double logarithms and products, and one rounding.
- `src/crmath.cyr` keeps the double-double ln core (kimiya's `_km_ln_core`) callable inside pramana, beside the
  correctly rounded ln. The prototype is its first consumer.

### 6.5 Part 9d-2: P9, `gamma_inc` (2026-11-15)

**The reference.** MPFR's own `gamma_inc` takes 0.05–0.8 s a call at 1,024 bits once a nears 1e5, too slow for
50,000 rows. So `check.c` gained `gref`: P and Q at 320 bits from x^a e^−x/Γ(a) and one of two expansions.
- **x ≤ a:** P from the positive series, stopped under 2⁻³³⁰. Q = 1 − P, which here is at least about 0.3, so it keeps
  about 318 bits.
- **x > a:** Q from Legendre's continued fraction by modified Lentz, stopped at 2⁻³¹⁰. P = 1 − Q.

Zero and subnormal results count in units of 2⁻¹⁰⁷⁴ (ADR 009). On 400 of the baseline's pairs, `gref` agrees with
`mpfr_gamma_inc` at 1,024 bits to 6×10⁻⁷⁵ ulp.

**What it compares** (`p9_gamma_impl.cyr`):
- **Rust 1.3.0's own results**, on §3.2's 20,000 pairs (a ∈ [0.01, 1000], x near a);
- **T0:** Rust's `gamma_inc` in its operation order, on correctly rounded primitives;
- **T1, the prototype,** with four changes:
  - **the prefactor's exponent a ln x − x − lnΓ(a) is a double-double.** Below a = 10, lnΓ(a) is exact to about 2⁻⁶⁰,
    from the ζ series of lnΓ(1 + f) and the recurrence. From a = 10, it uses Rust's Stirling kernel with
    u − ln(1 + u) as an exact series, and ln(1 + u) taken as ln x − ln a, so u's rounding never enters.
    kimiya's exp core takes the double-double;
  - **the series and Legendre's continued fraction run in double-double** and stop at 2⁻⁶²;
  - **the split moves to x = a,** so a complement is taken only where it can't cancel;
  - **for a < 1 and x < 1.1, Q is computed directly:** (Γ(1 + a) − x^a − a·x^a·Σ(−x)ⁿ/(n!(a + n)))/Γ(1 + a).
    Γ(1 + a) − 1 and x^a − 1 are expm1s of double-doubles.

  The ζ(k) table comes from `p9_zeta_table.c` (MPFR, checked to 2⁻¹⁰⁴).

**The sets:** five seeded sets over ADR 009's committed domain, a ∈ [10⁻³, 10⁵]:
- 20,000 with x = a + t√a, t ∈ [−5, 5];
- 10,000 in the lower tail, x/a down to 10⁻⁸;
- 10,000 in the upper tail, out to 50a and a + 40√a;
- 10,000 with small shapes, a ≤ 1 and x ∈ [10⁻⁸, 60];
- 10,000 at the switches, x ≈ 1.1 and x = a(1 ± δ).

That makes 360,400 rows, checked in about 25 minutes.

| Set | Function | Rust 1.3.0, max (mean) | T0, max | T1, max (mean) | T1 not CR |
|---|---|---|---|---|---|
| §3.2's 20,000 pairs | P | 97.0 (4.82) | 97.0 | **0.60** (0.249) | 0.09% |
| | Q | 485 (12.4) | 485 | **0.64** (0.249) | 0.11% |
| x near a, a ∈ [10⁻³, 10⁵] | P | — | 1.6×10⁴ | **0.59** (0.250) | 0.26% |
| | Q | — | 7,489 | **0.60** (0.246) | 0.10% |
| lower tail | P | — | 2.9×10⁹ | **1.00** (0.162) | 0.10% |
| | Q | — | 1,132 | **0.50** (0.120) | 0% |
| upper tail | P | — | 49.6 | **0.50** (0.127) | 0% |
| | Q | — | 2.8×10⁴ | **0.59** (0.209) | 0.14% |
| small shapes | P | — | 26.9 | **0.50** (0.244) | 0% |
| | Q | — | 2.6×10⁴ | **0.50** (0.248) | 0.01% |
| switches | P | — | 381 | **0.58** (0.249) | 0.26% |
| | Q | — | 405 | **0.55** (0.251) | 0.58% |

- **T1 meets ADR 009's fixed bound with room to spare.** Its worst row is 1.0 ulp, against ≤ 4. That row's P is about
  10⁻³⁰⁷, where the final scale by 2^E rounds a second time. Every other row is within 0.64, and the means are
  0.12–0.25.
- **Rust is worse than §3.2 recorded.** On its own 20,000 pairs, against this reference, it reaches 97 ulp on P and 485
  on Q. §3.2's 4,000-row subsample recorded 58 and 246.
- **Rust is far worse outside §3.2's sets.** Every T0 row that misses is Rust's algorithm (T0 = Rust on §3.2's pairs):
  - **in the deep lower tail, P is off by up to 2.9×10⁹ ulp** (a = 38, x = 4.6×10⁻⁷). From a = 10, Rust forms
    u = (x − a)/a ≈ −1, so u's rounding becomes a large relative error in 1 + u;
  - **for small a, Q is off by up to 2.8×10⁴ ulp,** from the 1 − P complement and the continued fraction's slow
    convergence in double.
- **One answer on every target:** the probe's 360,400 rows are byte-identical on x86_64 ELF, PE and aarch64, sha256
  `819b4f57…`.

**Cost** (`p9_gamma.bcyr`): ns per (P, Q) call, pinned, three runs.

| Inputs | T0 | T1 |
|---|---|---|
| a ∈ [0.5, 10], x = a ± 3√a | 542–544 | 9,546–9,771 |
| a ∈ [10, 10³], x = a ± 3√a | 843–847 | 11,734–12,010 |
| a ∈ [10³, 10⁵], x = a ± 3√a | 3,750–3,782 | 63,578–64,533 |
| a ∈ [10⁻³, 1], x ∈ [10⁻⁴, 1.1] | 615–620 | 8,080–8,086 |
| a ∈ [1, 10³], x ∈ [2×10³, 10⁵] | 234–235 | 2,467–2,502 |

T1 costs 10–17× T0, because every loop runs in double-double.

**Can the loops run in double?** T1b is T1 with Rust's double loops, a Neumaier-compensated sum and a 2⁻⁵⁶ stop,
keeping the double-double prefactor and the direct small-shape Q. It was checked on every fourth pair. It misses
the bound in two places:
- **x near a for large a:** up to 78.6 ulp on P (a = 65,354), where both expansions run for hundreds of steps;
- **the continued fraction's slow region just past x = 1.1 for small a:** up to 58 ulp on Q.

So the loops' extra precision is what meets the bound, and the cost has to be won by better algorithms in those two
regions, not by less precision.

**What it means for M2 part b:**
- T1's design stands.
- Speed is measured, not gated, so the 10–17× is a finding. The candidates to measure:
  - Temme's uniform asymptotic expansion near x = a for large a, which is O(1) and needs 9e's correctly rounded erfc;
  - the direct small-shape form carried further than x = 1.1, in place of the slow continued fraction.
- The double-double toolkit (add, multiply, divide, expm1 and ln of a double-double) becomes part of
  `src/crmath.cyr`, with the ζ(k) table generated beside it.

### 6.6 Part 9d-3: P9, `beta_inc` (2026-11-15)

**The reference.** MPFR has no incomplete beta, so `check.c` gained `bref`: x^a y^b/(a B(a, b)) times Lentz's
continued fraction (Rust's even and odd steps) at 320 bits.
- It works on the side x < (a + 1)/(a + b + 2) and takes the other value as 1 − it.
- It stops when a step changes the value by under 2⁻³¹⁰.
- The smaller of x and y is exact, and the other is 1 − it.

On the baseline's 4,000 integer-shape rows, `bref` agrees with the exact rationals (`p9_beta_exact.py`, 120 digits) to
4×10⁻⁷⁸ ulp. Rust's errors measured through it, 550 and 583 ulp, are §3.2's exactly.

**What it compares** (`p9_beta_impl.cyr`):
- **Rust 1.3.0's own results** on those 4,000 rows;
- **T0:** Rust's `beta_inc` (its continued fraction, `ln_beta`'s Stirling forms and TOMS 708's `bd0` prefactor) in its
  operation order, on correctly rounded primitives;
- **T1, the prototype,** with four changes:
  - **the smaller of x and y is taken as exact,** and the other recomputed as 1 − it in double-double. Rust's API passes
    both so that a caller who knows 1 − x can keep it;
  - **the prefactor's exponent a ln x + b ln y − ln B(a, b) − ln a is a double-double,** with lnΓ a double-double at
    every argument. Below 20 it comes from the ζ series and the recurrence; from 20 it uses Stirling, whose correction series
    is also a double-double, on the whole argument. So ln B's large cancellation costs nothing;
  - **the continued fraction runs in double-double** and stops at 2⁻⁶²;
  - **a complement below 2⁻⁶ is computed directly.** That happens only for a small first shape below its switch,
    where b·x ≲ 1. The direct form is TOMS 708's power series: 1 − I = (aB − x^a − a·x^a·Σ (1 − b)ⱼ/j!·xʲ/(a + j))/(aB),
    with aB − 1 and x^a − 1 as expm1s of double-doubles.

**The sets:** four seeded sets over a, b ∈ [10⁻³, 10⁵]:
- 12,757 with x = μ ± 5σ;
- 10,000 in each tail, down to 10⁻¹⁰ of the mean;
- 10,000 with one small shape;
- 4,929 at the switch, x = (a + 1)/(a + b + 2)·(1 ± δ) with δ down to 10⁻¹².

That makes 214,744 rows, checked in under 2 minutes.

| Set | Function | Rust 1.3.0, max (mean) | T0, max | T1, max (mean) | T1 not CR |
|---|---|---|---|---|---|
| §3.2's 4,000 integer-shape rows | I | 550 (14.1) | 550 | **0.50** (0.218) | 0% |
| | 1 − I | 583 (12.8) | 583 | **0.50** (0.221) | 0.03% |
| x = μ ± 5σ | I | — | 4.3×10⁴ | **0.50** (0.248) | 0.04% |
| | 1 − I | — | 1.7×10⁵ | **0.50** (0.250) | 0.02% |
| lower tail | I | — | 2,403 | **0.50** (0.150) | 0% |
| | 1 − I | — | 2,103 | **0.50** (0.112) | 0% |
| upper tail | I | — | 1,845 | **0.50** (0.113) | 0% |
| | 1 − I | — | 4,230 | **0.50** (0.156) | 0% |
| one small shape | I | — | 4.9×10⁴ | **0.50** (0.173) | 0% |
| | 1 − I | — | 1.3×10⁴ | **0.50** (0.177) | 0% |
| at the switch | I | — | 1.0×10⁵ | **0.51** (0.250) | 0.14% |
| | 1 − I | — | 1.2×10⁵ | **0.50** (0.246) | 0.04% |

- **T1 is within 0.51 ulp everywhere,** against ADR 009's fixed ≤ 8. It is correctly rounded on all but 0–0.14% of each
  set's rows.
- **Rust is worse than §3.2 recorded outside its integer shapes.** T0, which is Rust on §3.2's rows, reaches 1.7×10⁵
  ulp near the mean with one large shape, and 1.2×10⁵ at the switch.
- **Three defects found on the way.** Each was a T1 bug, not a bound to move, and each was reworked:
  - **a prefactor whose binary exponent fell between −1022 and −1000 was zeroed,** though the result is normal;
  - **the first direct complement used the other side's continued fraction,** which can't converge when x is tiny.
    It reached 3.6×10¹⁰ ulp at a = 0.001, b = 24,797, x = 8×10⁻¹² before the power series replaced it;
  - **Stirling's correction in plain double, on the high part of a + b only,** left lnΓ(a + b) at 2⁻⁶⁷ relative. Small
    shapes' 1 − I cancels about 14×, which made that 5.3 ulp. The double-double series on the whole argument brought it
    to 2⁻⁷⁵·⁶, and the row to 0.50.
- **One answer on every target:** the probe's 214,744 rows are byte-identical on x86_64 ELF, PE and aarch64, sha256
  `b2f05119…`.

**Cost** (`p9_beta.bcyr`): ns per (I, 1 − I) call, pinned, three runs.

| Inputs | T0 | T1 |
|---|---|---|
| a, b ∈ [1, 60], x uniform | 1,148–1,169 | 14,544–14,692 |
| a, b ∈ [10, 10³], x = μ ± 2σ | 1,614–1,636 | 21,229–21,452 |
| a, b ∈ [10³, 10⁵], x = μ ± 2σ | 4,160–4,220 | 70,536–71,319 |
| a ∈ [10⁻³, 1], b ∈ [1, 10³], x uniform | 1,342–1,359 | 13,343–13,484 |
| a, b ∈ [1, 10³], lower tail | 753–766 | 7,512–7,588 |

T1 costs 10–17× T0, as `gamma_inc` does, and for the same reason: the continued fraction runs in double-double.

**What it means for M2 part b:**
- T1's design stands, and its margin is large: 0.51 against 8.
- M2 part b can trade some of that margin for speed. TOMS 708's region-specific expansions (`bgrat` for one large shape,
  `basym` for two) are O(1) where the continued fraction takes O(√max(a, b)) steps.
- The ≥ 50k verification must keep the switch set and the small-shape set, where the defects above lived.

### 6.7 Part 9e: P9, erf and erfc, CORE-MATH without FMA (2026-11-15)

**The source.** CORE-MATH's binary64 `erf.c` (710 lines) and `erfc.c` (1,256 lines), with their worst-case lists
(65,957 and 118,638 inputs). They are MIT-licensed, fetched into `build/p9/coremath/` and never committed.
- `p9_coremath_tables.py` writes every table and hex constant as bit patterns into `p9_erf_data.cyr`: 17 tables of
  3,929 words, and 10 constants. Each literal is read by `float.fromhex`, so no digit is typed by hand. The tables
  `erfc.c` repeats from `erf.c` are checked identical and written once. The header carries the sources' sha256 and
  their MIT notice.
- `p9_erf_impl.cyr` transcribes both files function for function, on kimiya's two-sum, Dekker product and exact
  three-term sum. fenv flags and errno are not ported: Cyrius has neither, and Rust's erf sets neither.

**The fma sites.** There are 59 distinct ones: 28 in `erf.c`, and 31 more in `erfc.c`, which repeats `erf.c`'s fast
and accurate paths. §3.4 asked for each to be classified:
- **a Dekker product** (2 sites): `a_mul`'s lo = fma(a, b, −a·b) is the exact error of a product. Dekker's product with
  Veltkamp's split gives the same word;
- **an exact fma** (the other 57): Horner steps, `s_mul`, `d_mul`, the exp reductions. CORE-MATH's bits depend on
  their single rounding, so each one is `p9e_fma`. It computes a·b = p + e exactly by Dekker's product, then
  RN(p + e + c) exactly by Boldo and Melquiond's round-to-odd sum (kimiya's `_km_rn3`), in place of the plan's
  integer fma (musl's). It is exact whenever the product's error is representable, which holds at every site but one.
- **That site's product can fall below the subnormal grid,** where Dekker's product is not exact: erf's tiny path
  (|x| < 2⁻⁶¹) ends with fma(l, 2⁻¹⁰⁶, y). `p9e_fma_tiny` rounds y + l·2⁻¹⁰⁶ once instead, on a 2¹⁰⁶-scaled TwoSum
  and, for a subnormal result, an exact rn3 against 2⁵² at the scaled subnormal grid. The tiny path's other products
  stay exact: x is a multiple of 2⁻¹⁰⁷⁴, and the constants' short significands put them on grids of 2⁻¹⁰²⁰ and
  2⁻¹⁰⁶⁹.

**Stage 1, in C** (`p9_erf_c/`):
- the emulation against hardware fma on 10⁸ seeded triples, weighted to cancellation and to far addends: 0 differ;
- CORE-MATH built FMA-free (every `__builtin_fma` forced to the emulation, `-mno-fma`, and `objdump` showing no FMA
  instruction) against its own build with hardware FMA, bit for bit: 0 differ, on all 369,190 worst cases (each input
  and its negation) and 40 million seeded arguments.
- The first tiny-path emulation added 1.5·2⁵², which rounded 480 results just above 2⁻¹⁰²³ on the wrong grid. 2⁵²
  keeps every subnormal result's scaled sum in [2⁵², 2⁵³), and the diffs went to 0.

**Stage 2, the Cyrius port** (`p9_erf.cyr`): 2,768,534 rows.
- **The worst cases,** each input and its negation: 131,914 for erf and 237,276 for erfc.
- **Seeded arguments:** 1,049,344 for erf and 1,150,000 for erfc, weighted to every interval edge (i/16 and i/8, the
  clamps, 2⁻⁶¹), the subnormal range, erfc's asymptotic range up to its underflow at 27.2, and random bit patterns.
- **The accurate paths alone,** 100,000 each, inside the domains their callers give them. Beyond those domains the
  `C2` index runs past its end, as it would in C. An early sampler sent arguments there, and the only cross-target
  difference P9 found (180 rows on aarch64) was that out-of-domain read.

**Against MPFR** (`check.c` at 1,024 bits, run as ten interleaved chunks and merged by `scripts/port-baseline/merge.py`;
zero and subnormal results count, in units of 2⁻¹⁰⁷⁴):

| Set | n | Max ulp | Mean ulp | Not CR |
|---|---|---|---|---|
| erf, the worst cases, both signs | 131,906 | 0.5 | 0.246 | 0% |
| erf, (−6, 6) | 400,000 | 0.5 | 0.249 | 0% |
| erf, \|x\| from 2⁻¹⁰⁷⁴ to 16 | 300,000 | 0.5 | 0.238 | 0% |
| erf, \|x\| in [2⁻⁶¹, 2⁻⁴) | 100,000 | 0.5 | 0.250 | 0% |
| erf, any finite bits | 100,000 | 0.5 | 0.124 | 0% |
| erf, the interval edges | 149,344 | 0.5 | 0.244 | 0% |
| erf, the accurate path alone | 100,000 | 0.5 | 0.250 | 0% |
| erfc, the worst cases, both signs | 237,268 | 0.5 | 0.188 | 0% |
| erfc, (−6, 28) | 400,000 | 0.5 | 0.238 | 0% |
| erfc, \|x\| from 2⁻¹⁰⁷⁴ to 32 | 200,000 | 0.4999 | 0.0137 | 0% |
| erfc, [25, 27.3), the tail to underflow | 150,000 | 0.5 | 0.167 | 0% |
| erfc, (−5.9, 0) | 100,000 | 0.5 | 0.250 | 0% |
| erfc, any finite bits | 100,000 | 0.4999 | 0.0071 | 0% |
| erfc, the branch edges | 200,000 | 0.5 | 0.250 | 0% |
| erfc, the accurate path alone | 100,000 | 0.5 | 0.247 | 0% |

- **Every row is correctly rounded:** all 2,768,518 (the 16 NaN inputs are not compared), the accurate paths' 200,000
  included. Rust 1.3.0 is at 0.82 ulp on erf and 2.49 on erfc, misrounding 3.1% and 30.6% of §3.2's rows.
- **One answer on every target:** the 2,768,534 rows are byte-identical on x86_64 ELF, PE and aarch64, sha256
  `c4a04ad9…`.
- **A sampler defect, found by the check:** the first run's `p9_scale` shifted with `>>>`, which sign-extends in
  Cyrius, so 300,310 of its draws came out as NaN inputs. MPFR skips NaN rows, which left erf at about 850,000
  seeded arguments, short of §3.4's 10⁶. Every shift is now `>>`, and the tables above are the corrected run.
  `p9_erf.sh` reproduced them from the sources.

**Cost** (`p9_erf.bcyr`, `p9_erf_c/cbench.c`): ns per call, pinned, four runs, 10⁶ calls per row over 4,096 seeded
inputs.

| Inputs | Cyrius port | CORE-MATH, hardware FMA | CORE-MATH, FMA emulated (C) | glibc |
|---|---|---|---|---|
| erf, [10⁻⁴, 1/16) | 555–560 | 5.7–6.1 | 114.6–116.0 | 21.6–22.6 |
| erf, [1/16, 5.9) | 604–610 | 10.5–10.9 | 91.9–94.3 | 29.1–30.2 |
| erf, the accurate path alone | 1,263–1,268 | — | — | — |
| erfc, (−5.8, 0) | 508–511 | 12.4–12.6 | 82.0–84.2 | 27.3–28.0 |
| erfc, [0, 2.88] | 667–675 | 15.8–16.3 | 108.0–110.1 | 34.4–37.4 |
| erfc, [2.9, 25.8] (asymptotic) | 2,259–2,279 | 37.3–37.6 | 413.9–425.8 | 72.8–75.6 |
| erfc, the accurate path alone | 4,746–4,798 | — | — | — |

- **The emulated fma is most of the cost.** In C it takes CORE-MATH from 6–38 ns to 82–426 ns, 6.5–20×. The Cyrius port
  then costs 4.8–6.6× the emulated C.
- **Against glibc,** the port costs 18–31×, inside the sibling ratios for the correctly rounded primitives (the plan's
  §3.7: 12–62×).
- **A scalar fma would remove the emulation.** Hardware fma (x86_64 FMA3, aarch64 `fmadd`) rounds once, so it gives
  the emulation's bits, and an exact software fallback keeps every target's bits the same. It is requested at step 10
  (ledger L12).

**What it means for M2 part b:**
- **erf and erfc are correctly rounded in Cyrius,** inside 9e's slot. D3 = C holds without the ≤ 1 ulp fallback.
- The port moves into `src/special.cyr` as it is. `p9_coremath_tables.py` moves to `scripts/golden/`, and its tables
  are regenerated, never edited. The exact fma and the tiny-path rounding join `src/crmath.cyr`.
- 9d-4's `normal_cdf` can now be prototyped on the correctly rounded erfc, and 9d-2's Temme option can use it.

### 6.8 Part 9d-4: P9, `normal_cdf` (2026-11-15)

**What it compares.** Five forms of Φ(z) = ½·erfc(−z/√2) on the same inputs, against MPFR's at 1,024 bits
(`check.c`; zero and subnormal results count, in units of 2⁻¹⁰⁷⁴). `normal_sf(z)` is `normal_cdf(−z)` bit for bit,
in Rust (−(−z)/√2 is z/√2) and in T1, so one function covers both.
- **Rust 1.3.0's own results** on §3.2's two sets, from the baseline's binary;
- **T0** (`p9_ncdf_impl.cyr`): Rust's 0.5·erfc(−z/SQRT_2), its fdlibm erfc in Rust's operation order, on correctly
  rounded exp. It differs from Rust on 69 of §3.2's 100,000 rows, where glibc's exp misrounds;
- **the correctly rounded erfc alone:** Rust's form with 9e's erfc, which shows what the rounded argument costs;
- **the plan's sketch,** §3.4 as written: h + l = −z/√2 as a double-double, then the correctly rounded erfc(h) plus
  erfc(h)·ρ, rounded, then halved. Here ρ = −l·(2/√π)·e^(−h²)/erfc(h) is the first-order correction;
- **T1, the prototype:** the same ρ, applied to CORE-MATH's double-double erfc(h) before it is rounded, with the ½
  folded into one rounding. That rounding is exact on the subnormal grid too, as 9e's tiny path is. The
  double-double comes from CORE-MATH's fast path where its proven bound is under 2⁻⁶⁰ relative, and from its accurate
  paths otherwise. The thresholds for Φ = 0, ½ and 1 are `cr_erfc`'s, applied to h.

**T1's bound.** The double-double erfc(h) is within 2⁻⁶⁰ relative (2⁻¹⁰⁰ on the accurate paths). Everything T1 adds
stays under about 2⁻⁸⁵ relative:
- the error in ρ (|ρ| ≤ 2⁻⁴²·⁴, by the Mills-ratio bound on e^(−h²)/erfc(h));
- the second-order term;
- the argument's double-double (about 2⁻¹⁰⁴);
- the two roundings in rl + rh·ρ.

So T1 is within 0.5 + 2⁻⁷ ≈ 0.508 ulp, against ADR 009's ≤ 1. The sketch rounds twice, and halving a subnormal
rounds a third time, so it can reach 1.5 ulp where the correction crosses a binade.

**The sets:** §3.2's two, 50,000 each, then nine seeded sets with T1 on every row and the other forms on every fourth:
- 250,000 in the core, (−5, 5), and 250,000 in the lower tail, (−38.5, −5);
- 100,000 in (−4.1, −2.5), around (−4.07, −2.96), where CORE-MATH's fast bound passes 2⁻⁶⁰ and T1 takes the
  accurate path;
- 100,000 in the deep tail, (−38.5, −36.5), with the subnormal results, and 100,000 in (5, 8.3), where Φ → 1;
- 150,000 with |z| from 2⁻¹⁰⁷⁴ to 32, and 50,000 of any finite bits;
- 46,801 on the decimal grid z = k/1000 from −38.5 to 8.3;
- 100,008 at the eight branch points of `cr_erfc` in h, at ±1,000 ulps and within 2⁻²⁰ relative.

That makes 2,506,918 rows. T1 has 1,246,809 of them, 1,146,809 seeded.

| Set | n (T1) | Rust, max (mean) | T0, max (mean) | CR erfc alone, max | Sketch, max (not CR) | T1, max (mean) | T1 not CR |
|---|---|---|---|---|---|---|---|
| §3.2's (−38, 8) | 50,000 | 1,590 (174) | 1,590 (174) | 1,590 | 1.00 (20.7%) | **0.50** (0.246) | 0 |
| §3.2's (−5, 5) | 50,000 | 29.1 (2.19) | 29.1 (2.19) | 28.7 | 0.99 (13.0%) | **0.50** (0.250) | 0 |
| (−5, 5) | 250,000 | — | 29.1 (2.19) | 29.5 | 1.00 (13.1%) | **0.50** (0.250) | 1 row |
| (−38.5, −5) | 250,000 | — | 1,674 (238) | 1,674 | 1.00 (25.2%) | **0.50** (0.242) | 0 |
| (−4.1, −2.5) | 100,000 | — | 23.4 (5.56) | 22.4 | 0.99 (25.2%) | **0.50** (0.250) | 1 row |
| (−38.5, −36.5) | 100,000 | — | 1,688 (313) | 1,688 | 1.00 (25.4%) | **0.50** (0.125) | 0 |
| (5, 8.3) | 100,000 | — | 0.50 (0.251) | 0.50 | 0.50 (0%) | **0.50** (0.250) | 0 |
| \|z\| from 2⁻¹⁰⁷⁴ to 32 | 150,000 | — | 936 (0.164) | 936 | 0.97 (0.10%) | **0.50** (0.014) | 0 |
| any finite bits | 50,000 | — | 1,026 (0.191) | 1,026 | 0.84 (0.07%) | **0.50** (0.008) | 0 |
| z = k/1000 | 46,801 | — | 1,624 (172) | 1,624 | 1.00 (20.9%) | **0.50** (0.245) | 0 |
| `cr_erfc`'s branch points | 100,008 | — | 1,650 (78.1) | 1,651 | 0.99 (11.7%) | **0.50** (0.281) | 2 rows |

- **T1 is within 0.5 ulp on every row,** against the fixed ≤ 1, and correctly rounded on all but 4 of 1,246,809. Rust
  reaches 1,590 ulp on §3.2's rows and 1,688 in the deep tail.
- **The argument's rounding is all of Rust's tail error.** The correctly rounded erfc alone, on Rust's rounded
  −z/√2, reaches 1,688 ulp, as T0 does. A rounding error of 2⁻⁵³ in h becomes about 2h² times that in erfc(h).
- **The sketch is at the bound, not under it.** It reaches exactly 1.0 unit on subnormal results (z < −37.5),
  where its halving rounds again, and 0.996 ulp above them. It misrounds up to 25% of a set. One rounding is what
  gives T1 its margin.
- **One answer on every target:** the 2,506,918 rows are byte-identical on x86_64 ELF, PE and aarch64, sha256
  `67b52ed6…`.

**Cost** (`p9_ncdf.bcyr`): ns per call, pinned, three runs, 10⁶ calls per row over 4,096 seeded inputs.

| Inputs | T0 | CR erfc alone | T1 |
|---|---|---|---|
| z ∈ (−5, 5) | 147–154 | 705–738 | 1,283–1,329 |
| z ∈ (−4.1, −2.5) | 193–198 | 926–952 | 3,769–3,887 |
| z ∈ (−36, −5) | 179–184 | 2,172–2,253 | 2,285–2,363 |
| z ∈ (−38.4, −36.6) | 229–239 | 4,473–4,661 | 4,436–4,610 |

- T1 costs 8–20× T0. Against the correctly rounded erfc alone, it adds about 0.6 µs in the core, much of it the
  accurate path below, and 0.1 µs in the tail (an exp and the argument's double-double).
- **The 2⁻⁶⁰ bound costs most in (−4.1, −2.5).** There, 1 − erf(h) cancels, CORE-MATH's fast bound passes 2⁻⁶⁰, and
  T1 takes the accurate path. With a scratch copy at 2⁻⁵⁸ (bound 0.5 + 2⁻⁵ ulp), T1 takes 1,134 ns in the core and
  2,593 ns there. At 2⁻⁵⁶ (0.5 + 2⁻³ ulp), it takes 927 and 1,475 ns (one pinned run each, timing only).

**What it means for M2 part b:**
- T1's design stands: CORE-MATH's erfc before it rounds, the first-order correction for the argument's low part, and
  one rounding with the ½ folded in. `src/special.cyr` therefore exposes erfc's double-double core, not only
  `cr_erfc`.
- §3.4's sketch is replaced by T1's form. Its two roundings sit at the bound.
- The fast bound is a speed knob within the bound. 2⁻⁵⁶ still leaves 0.375 ulp of margin and takes 30% off the core.
  M2 part b chooses, and its ≥ 1M verification keeps the (−4.1, −2.5) set.

## 7. M1: the foundation

M1 lands in parts, in the manifest's module order (error → constants → crmath_data → crmath → mathx → json → rng →
combinatorics → bridge):
- **a:** `src/error.cyr`;
- **b1–b4:** `src/crmath.cyr`, kimiya's correctly rounded primitives first, then the new log1p, tan and log2, one part
  each;
- **c:** `src/mathx.cyr`;
- **d1–d4:** serde_json's f64 text, the work-table pool, the JSON read core, then the write core and the error codec;
- **e and f:** the RNG, combinatorics with `PmnU128` and the bridges, then M1's gate.

### 7.1 Part a: the error codes (2026-11-15)

`src/error.cyr` holds `PMN_ERR_*`: the five Rust variants in their order (1–5), then the Cyrius-only `OUT_OF_MEMORY`
(6) and `DESERIALIZE` (7) (D6). `pmn_err_name` gives the variant's name, and `pmn_err_message` thiserror's message up
to its field. Rust 1.3.0 has no tests of `error.rs`, so `tests/error.tcyr` pins the codes, the names, the messages and
a Result that carries one (30 checks). The parity harness now names a golden `E:` token through `pmn_err_name`. `[lib]`
starts here, so `dist/pramana.cyr` is written and checked from now on.

### 7.2 Part b1: kimiya's correctly rounded primitives (2026-11-15)

**What it vendors.** `scripts/vendor_crmath.py`, after soorat's, copies from kimiya's git tag 2.0.1 (`9cb3936`), not
its working tree. It takes the dependency closure of exp, expm1, ln, pow, sin, cos and atan2, with the double-double ln
and exp cores and the exact two-sum, Dekker product and round-to-odd sum that M2's special functions build on (probe
P9; ledger L12): 58 functions in `src/crmath.cyr`, a `const`, and 59 tables in `src/crmath_data.cyr`. Every name is
renamed `_km_` → `_pmn_km_`, and a comment line the longer prefix pushes past 120 columns is wrapped. `--check`
regenerates and compares, and runs in CI. Two things a closure over `fn` and `var` items alone would miss: kimiya's
`_KM_INF` is a `const` in `mathx.cyr`, not a table; and the headers must not be renamed.

**The golden rows.** The harness dispatches exp, ln, exp_m1, powf, sin, cos and atan (as atan2(x, 1), the same value)
to them, with their class-C bounds at **0 ulp** (ADR 009). ln_1p, tan and log2 keep their interim counterparts and
bound 2 until parts b2–b4.

| Function | Rows | 0 ulp | Of the MPFR-corrected rows (`math.cr.tsv`) |
|---|---|---|---|
| exp | 3,274 | 3,274 | — |
| ln | 3,112 | 3,112 | — |
| exp_m1 | 1,010 | 1,010 | 52, all now the correctly rounded value |
| powf | 4,400 | 4,400 | 6, all; and `pow(−1, DBL_MAX)` is 1, where ganita's gave +∞ |
| sin / cos | 2,510 each | all | 4 (sin), all |
| atan | 1,510 | 1,510 | — |

All 76 corrected rows are accepted, and nothing is left measured: 28,556 rows ok, 0 failed, identically on x86_64
ELF, PE and aarch64 (the seven digests match).

**Against MPFR** (`scripts/crverify/`): 7,700,000 seeded rows, about 1.1M per function, weighted to each one's hard
regions, checked by `check.c` at 1,024 bits in ten chunks. The `cr:` rows count zero and subnormal results in units of
2⁻¹⁰⁷⁴, and compare ±∞ and NaN results too (right only when MPFR's is the same), so overflow and invalid cases are
checked, not skipped.

| Function | Rows | Sets | Max ulp | Not CR |
|---|---|---|---|---|
| exp | 1,100,000 | [−745.2, 709.8], \|x\| from 2⁻¹⁰⁷⁴ to ½, ±10⁻⁶ of the overflow and underflow ends, ±1 of the subnormal edge, (−20, 20), any bits | 0.5 | 0 |
| ln | 1,100,000 | every binade from 2⁻¹⁰⁷⁴, 1 ± 2⁻⁶⁰ … 2⁻⁶, (½, 2), integers to 10⁶, any bits | 0.5 | 0 |
| expm1 | 1,100,000 | [−40, 709.7], \|x\| from 2⁻¹⁰⁷⁴ to 8, (−1, 1), the overflow end, any bits | 0.5 | 0 |
| pow | 1,200,000 | x in every binade of 2^±1000 with y in (−1, 1); x in (½, 2) with y to ±1000; integer and half-integer y; x = 1 + 2⁻⁵³ … 2⁻²⁰ with y to 2⁶⁰; within 10⁻⁶ of the overflow edge; any bits | 0.5 | 0 |
| sin, cos | 1,100,000 each | (−10, 10), (−10⁶, 10⁶), 2²⁰ … 2¹⁰²³, \|x\| from 2⁻¹⁰⁷⁴, ±4 ulps of k·π/2 for \|k\| ≤ 10⁵, any bits | 0.5 | 0 |
| atan | 1,000,000 | (−10, 10), every binade, near ±1, any bits | 0.5 | 0 |

The rows are byte-identical on x86_64 ELF, PE and aarch64, sha256 `6e4295ca…`.

**Cost** (`tests/crmath.bcyr` against its Rust twin `scripts/probes/rust/src/bin/m1_primitives.rs`, the same inputs):
ns per call, pinned, three interleaved rounds. The record is `docs/benchmarks-rust-v-cyrius.md`.

| Function | Cyrius, correctly rounded | Rust (glibc 2.44) | Ratio |
|---|---|---|---|
| exp | 46 | 5.7–5.8 | 8.0× |
| ln | 54–55 | 5.3–5.4 | 10.2× |
| exp_m1 | 50–51 | 5.7 | 8.9× |
| powf | 134–136 | 12.1–12.2 | 11.1× |
| sin | 91 | 9.2–9.3 | 9.8× |
| cos | 87–89 | 9.4 | 9.4× |
| atan | 119–120 | 8.4–8.5 | 14.1× |

These are inside the siblings' 12–62× against glibc (port plan §3.7). The price buys correct rounding and the same bits
on every target, which glibc's results don't have.

### 7.3 Part b2: a correctly rounded log1p (2026-11-15)

**What it is.** `_pmn_cr_log1p` in `src/crmath_ext.cyr`, pramana's own: no sibling has a public log1p. It is kimiya's
method on kimiya's ln, with no rounding of 1 + x:
- **|x| < 2⁻⁶⁰:** x itself. log1p(x) − x is below half an ulp of x on either side, and ±0 and the subnormals stay
  exact.
- **|x| < 2⁻¹⁰:** x is ln's reduced argument itself. kimiya's ln core on its interval 256 (c = 1, L = 0) evaluates
  log1p(r) for an exact |r| < 2⁻¹⁰, so `_pmn_log1p_small_core` is that core term for term with r = x, and its proven
  bound (2⁻⁴⁹·|p| + 2⁻⁸³·|hi|, relative to r) holds as it is.
- **Otherwise:** 1 + x = s + t exactly (TwoSum), and τ = t/s has |τ| ≤ 2⁻⁵³. Then log1p(x) = ln s + τ − τ²/2 + …. The fast
  path adds RN(τ) to ln's fast core at s, and widens the bound by |τ|·2⁻⁵⁰ + 2⁻¹⁰⁵, below 2⁻⁹⁴ of a result that is
  at least 2⁻¹⁰·⁰¹ here.
- **The accurate path:** ln s by ln's triple-double path (below 2⁻¹³⁵ by its operation count, measured 2⁻¹⁵⁰), plus
  τ as a double-double minus τ²/2. On |x| < 2⁻¹⁰ it is ln's triple-double series on the exact x. It is rounded once.

The entry points are kimiya's three: the whole function, `_fast` (returns whether its rounding test passed) and `_acc`
(the accurate path alone).

**The golden rows.** `ln_1p` is dispatched to it with its bound at 0: all 1,013 rows at 0 ulp, the 6 where glibc
misrounds included. The digest is the same on x86_64 ELF, PE and aarch64.

**Against MPFR** (`scripts/crverify/`, now 9,000,000 rows with the part b1 functions):

| Set | Rows | Max ulp | Not CR |
|---|---|---|---|
| \|x\| from 2⁻⁷⁰ to 2⁻¹⁰, both signs | 300,000 | 0.5 | 0 |
| (−0.999, 1) | 200,000 | 0.5 | 0 |
| −1 + 2⁻⁵³ … −½ | 150,000 | 0.5 | 0 |
| every binade from 2⁻¹⁰⁷⁴ to 2¹⁰²³ | 200,000 | 0.5 | 0 |
| within 2⁻²⁰ of ±2⁻¹⁰ and ±2⁻⁶⁰, the two switches | 100,000 | 0.5 | 0 |
| any bits (x < −1 gives NaN, checked) | 150,000 | 0.5 | 0 |
| the accurate path alone, 2⁻⁶⁰ … 2⁻¹¹ | 100,000 | 0.5 | 0 |
| the accurate path alone, (−0.999, 4) and 2⁻¹⁰ … 2¹⁰²³ | 100,000 | 0.5 | 0 |

The rows are byte-identical on the three targets, sha256 `bf4bdcf0…`.

**A defect the accurate-path set found.** The first run had 1,328 accurate-path rows wrong (NaN) at x above 2⁹⁹⁶,
and those were also the only rows that differed on aarch64. Dekker's product, which forms τ's low part, splits s with
Veltkamp's constant and overflows past 2⁹⁹⁶. The NaN that follows is −NaN on x86 and +NaN on ARM. In use the fast
path never declined there, which is why every whole-function set passed. The accurate path now scales s and t by
2⁻⁵¹² first when s > 2⁵¹², which leaves τ as it is. This is the trap probe P9 noted for lnΓ.

**The fast path** declines on 7 of 2,000,000 arguments in (−2⁻¹⁰, 2⁻¹⁰), and on none of 4,000,000 in (−0.999, 1)
and (1, 1000).

**Cost** (`tests/crmath.bcyr` and its Rust twin, pinned, three interleaved rounds):

| Inputs | Cyrius | Rust (glibc 2.44) | Ratio |
|---|---|---|---|
| x ∈ [−0.5, 1) | 95–96 ns | 6.8–6.9 ns | 13.9× |
| x ∈ [−1/1024, 1/1024) | 51–53 ns | 5.5–5.7 ns | 9.4× |

The general path costs about 40 ns more than ln (54–55 ns). It makes a separate fast-path call with a two-value
return, and does a TwoSum and a division; kimiya inlines ln's core for the same reason. That is a finding for later,
not a change now.

### 7.4 Part b3: a correctly rounded tan (2026-11-15)

**What it is.** `_pmn_cr_tan` in `src/crmath_ext.cyr`, on kimiya's sin and cos. Reducing |x| = k·π/2 + r makes
sin|x| and cos|x| its ±sin r and ±cos r for quadrants k and k + 1, and tan x = sgn(x)·sin|x|/cos|x|.
- **|x| < 2⁻²⁷:** x itself (tan x − x < |x|³/3, below half an ulp).
- **The fast path:** one reduction (Cody–Waite below 2²⁰, Payne–Hanek above, as kimiya's sin). kimiya's fast
  evaluation then runs twice, giving S and C with their absolute bounds, and each is renormalised exactly. S/C is
  a double-double quotient, with the bound (e_S + |q|·e_C)/(|c_h| − |c_l| − e_C), widened by 2⁻⁴⁰, plus 2⁻⁹⁸·|q|
  for the division. Next to a multiple of π/2, the reduction's 2⁻¹¹⁹·|k| is large against the small one of sin r
  and cos r, so the test fails and the accurate path takes over.
- **The accurate path:** kimiya's accurate reduction and evaluation give sin|x| and cos|x| together in
  triple-double (below 2⁻¹³⁸ relative at worst), then `_pmn_km_td_div` (below 2⁻¹⁵¹), rounded once. No double is
  within 2⁻⁶¹ of a multiple of π/2, so |tan x| < 2⁶³.

**A defect the golden rows found first.** kimiya's fast evaluation returns hi + lo with hi only a partial sum, and
lo up to 2⁻²⁷ of the value. The first double-double division assumed a normalised pair, so its dropped term was ulps,
not 2⁻¹⁰⁶. The fast path passed its test with wrong values on 1,803 of 2,010 rows: tan(1) was 7 ulp off. An exact
Fast2Sum on each pair before dividing fixed it. That bound must hold for the result to be correctly rounded, and the
MPFR rows below check it.

**The golden rows.** `tan` is dispatched to it with its bound at 0: all 2,010 rows at 0 ulp, the 7 where glibc
misrounds included. The digest is the same on x86_64 ELF, PE and aarch64.

**Against MPFR** (`scripts/crverify/`, now 10,300,000 rows):

| Set | Rows | Max ulp | Not CR |
|---|---|---|---|
| (−10, 10) | 300,000 | 0.5 | 0 |
| (−10⁶, 10⁶) | 100,000 | 0.5 | 0 |
| 2²⁰ … 2¹⁰²³, both signs (Payne–Hanek) | 200,000 | 0.5 | 0 |
| \|x\| from 2⁻¹⁰⁷⁴ to ½ | 150,000 | 0.5 | 0 |
| ±4 ulps of k·π/2 for 0 < \|k\| ≤ 10⁵ (tan huge for odd k, near 0 for even) | 200,000 | 0.5 | 0 |
| within 2⁻²⁰ of ±2⁻²⁷, the switch | 50,000 | 0.17 | 0 |
| any bits (±∞ gives NaN, checked) | 100,000 | 0.5 | 0 |
| the accurate path alone: (−10, 10), ±4 ulps of k·π/2, 2²⁰ … 2¹⁰²³, any bits | 200,000 | 0.5 | 0 |

The rows are byte-identical on the three targets, sha256 `99ae1269…`. The fast path declines on 278, 261 and 292 of
2,000,000 arguments in (−10, 10), (1.5, 1.64) (next to π/2) and (10⁶, 10⁷): about 1.4·10⁻⁴, near kimiya's sin and cos
(0.7–0.9·10⁻⁴).

**Cost** (`tests/crmath.bcyr` and its Rust twin, pinned, three interleaved rounds): `tan`, x ∈ [−10, 10), takes
188–190 ns against Rust's 10.8–10.9 ns, 17.4×. That is about one sin plus one cos (95–96 and 88 ns in the same
rounds): two evaluations of the shared reduction, then the division. kimiya's joint sin-and-cos evaluation, which
shares the table lookups, is the candidate to measure if `tan` ever matters; Rust 1.3.0 calls it once, in the Cauchy
sampler.

### 7.5 Part b4: a correctly rounded log2, and part b's result (2026-11-15)

**What it is.** `_pmn_cr_log2` in `src/crmath_ext.cyr`, on kimiya's ln with the exponent kept exact:
x = 2^E·m with m in [0.7495, 1.499), as ln reduces it, so log2 x = E + ln(m)/ln 2, and |log2 m| < 0.585.
- **x = 2^E:** E, exactly. log2 x is rational only at the powers of two, so these are the only exact cases.
- **The fast path:** ln's fast core at m (E = 0 there), times 1/ln 2 by Dekker's product, then E added by
  Fast2Sum. The bound is 1.5·eln + 2⁻¹⁰⁰·|ph| + 2⁻¹⁰³·|hi|.
- **The accurate path:** ln's triple-double path at m, times 1/ln 2 in triple-double, plus E, rounded once.
- **1/ln 2** is three bit patterns that `scripts/crverify/consts.py` checks against 90-digit decimal arithmetic, in CI
  and in `run.sh`.

**The golden rows.** `log2` is dispatched to it with its bound at 0: all 2,510 rows at 0 ulp, the one where glibc
misrounds included. The digest is the same on x86_64 ELF, PE and aarch64.

**Against MPFR** (`scripts/crverify/`, now 11,618,874 rows):

| Set | Rows | Max ulp | Not CR |
|---|---|---|---|
| every binade from 2⁻¹⁰⁷⁴ to 2¹⁰²³ | 400,000 | 0.5 | 0 |
| 1 ± 2⁻⁶⁰ … 2⁻⁶ | 300,000 | 0.5 | 0 |
| (½, 2) | 150,000 | 0.5 | 0 |
| integers to 10⁶ | 100,000 | 0.5 | 0 |
| every power of two, 2⁻¹⁰⁷⁴ … 2¹⁰²³, and ±1 … 4 ulps of each | 18,874 | 0.4987 (the powers exact) | 0 |
| any bits (x < 0 gives NaN, checked) | 150,000 | 0.5 | 0 |
| the accurate path alone: 1 ± 2⁻⁶⁰ … 2⁻⁶, every binade | 200,000 | 0.5 | 0 |

The rows are byte-identical on the three targets, sha256 `6026da23…`. The fast path declines on 11 of 2,000,000
arguments in (0.999, 1.001), and on none of 4,000,000 in (½, 2) and (1, 10³⁰).

**Cost:** `log2`, x ∈ [0.001, 1000), takes 104–109 ns against Rust's 5.1–5.3 ns, 20.6×. That is about 50 ns over ln,
from the separate split and fast-path calls (with their multi-value returns), the core's call, and the product.
Inlining, as kimiya inlines ln's core, is the candidate to measure. Rust 1.3.0 calls it once, in the Hick's-law fit.

**Part b's result.** Every transcendental pramana 1.3.0 calls is correctly rounded in the port, and every class-C
math row of the golden corpus is gated at 0 ulp:

| Function | Golden rows at 0 ulp | MPFR rows | Source | Cost (Rust, ns) |
|---|---|---|---|---|
| exp | 3,274 | 1,100,000 | kimiya, vendored | 46 (5.7) |
| ln | 3,112 | 1,100,000 | kimiya | 54–55 (5.3) |
| exp_m1 | 1,010 | 1,100,000 | kimiya | 50–51 (5.7) |
| powf | 4,400 | 1,200,000 | kimiya | 134–136 (12.1) |
| sin, cos | 2,510 each | 1,100,000 each | kimiya | 87–96 (9.0–9.4) |
| atan | 1,510 | 1,000,000 | kimiya (atan2(x, 1)) | 119–120 (8.4) |
| ln_1p | 1,013 | 1,300,000 | new (b2) | 51–96 (5.5–6.9) |
| tan | 2,010 | 1,300,000 | new (b3) | 188–190 (10.8) |
| log2 | 2,510 | 1,318,874 | new (b4) | 104–109 (5.1–5.3) |
| cbrt | 810 | — | ganita (exact, as Rust's) | — |

All 76 MPFR-corrected rows are accepted, nothing is measured, and the replay is identical on three targets: 28,556
rows ok, 0 failed. The cost is 8–21× glibc. That is the price of correct rounding and the same bits everywhere; speed
is measured, not gated.

### 7.6 Part c: the Rust f64 surface (2026-11-15)

`src/mathx.cyr` holds every std f64 operation pramana 1.3.0 calls, with Rust's semantics, as the names the rest of the
port calls (ledger L2; port plan §8.1):
- **min, max, clamp** ignore a NaN on either side. Equal operands, ±0 included, give the second operand, as rustc's
  maxsd and minsd do;
- **fract** is x − trunc(x), never the floor-based `f64_fract`;
- **`u64 as f64`** rounds the unsigned value: above 2⁶³, the word halved with a sticky bit, converted, then doubled;
- **`as usize`** saturates: NaN and negatives give 0, 2⁶⁴ and above give u64::MAX, and 2⁶³ … 2⁶⁴ go through an exact
  subtraction;
- **is_nan, is_finite and total_cmp** (the sign-folded bits);
- **powi** is compiler-rt's and LLVM's square-and-multiply, the set bits multiplied in low to high from an exact 1;
- **the transcendentals** are thin wrappers over the correctly rounded functions of parts b1–b4, so a conversion to
  ganita's tier (ledger L1) changes only this file.

**The golden rows.** The harness dispatches every `f64::*` and `u64::*` function through these names. The 7,077 rows
that waited on them all pass:

| Function | Rows | Class | Result |
|---|---|---|---|
| fract | 871 | A | all identical |
| max, min | 760 each | A | all identical (the stdlib's differ on the 69 with a NaN second operand) |
| clamp | 520 | A | all identical |
| as_usize | 633 | A | all identical |
| u64::as_f64 | 536 | A | all identical |
| total_cmp | 169 | A | all identical |
| is_nan, is_finite | 34 each | A | all identical |
| powi 2, 3, 4 / 9, 25, 49 | 510 each / 410 each | B | all at 0 ulp |

**The whole corpus is now gated:** 35,633 rows ok and 76 accepted (ADR 9), with 0 failed, 0 measured and 0 pending.
The digests are identical on x86_64 ELF, PE and aarch64. `tests/mathx.tcyr` names the semantics each helper exists
for, and the cases the corpus doesn't reach, in 30 checks: a negative powi, the ties of the u64 conversion above 2⁶³,
and NaN's place in the total order.

**Cost** (`tests/mathx.bcyr` against the Rust twin's `mathx` rows, pinned, three interleaved rounds): 8–24 ns a call
in Cyrius against Rust's 0.8–3.4 ns. Rust inlines each into a few instructions. The Cyrius rows also pay a call and the
bench loop's per-iteration dispatch, as probe P2 noted, so compare the rows with each other, not as bare costs.

### 7.7 Part d1: serde_json's f64 text (2026-11-15)

Part d is split in four, the JSON core being large: **d1** the float printer, **d2** the work-table pool (moved
forward from part f, §7.8), **d3** the read core (bayan's tree, probe P7's pre-pass, serde's struct rules), **d4** the
write core and the error codec. The `SimpleRng` codec moves to part e, with the RNG.

**The printer.** `src/fmtf.cyr` is impetus 2.0.0's serde_json form, vendored by `scripts/vendor_fmtf.py` from its
git tag (`4b72df7`). It takes the dependency closure of `im_fmt_f64_json`, 15 items, renamed `_pmn_im_`, with
`_pmn_im_bits` defined in the header, and its `--check` runs in CI. The digits are Burger & Dybvig's shortest
round-trip digits on a small bignum, ties to even as serde_json's zmij. Rust's Display form isn't vendored, because
1.3.0 uses `{}` on an f64 only inside error messages, which D6 drops. Probe P7 had checked this printer once, on
280,562 doubles.

**The golden rows.** The generator gained a `fmt` module (`scripts/golden/src/m_fmt.rs`, with serde_json pinned at
1.0.154): serde_json's own text for 25,200 doubles:
- the specials;
- every power of two and of ten, with its neighbours;
- the integers to 1,000;
- 5,000 short decimals;
- a log-uniform sweep across the form switches (10⁻⁸ … 10¹⁸);
- 5,000 seeded bit patterns, 1,000 of them also negated.

`validate.py` rebuilds every row independently, from Python's `repr` digits put into serde_json's form, and all
25,200 agree. 62 of the inputs are exact ties, where serde_json's even digit and Rust's round-up differ.

**The result.** All 25,200 rows match byte for byte on x86_64 ELF, PE and aarch64; the replay is now three modules,
60,833 rows ok. One mutation check, the printer switched to Rust's round-up tie rule, fails the 37 ties whose even
candidate is the lower one. The other 25 are ties where both rules agree.

**Cost** (`tests/fmtf.bcyr` against the Rust twin's `fmt` rows, serde_json writing into a reused buffer; pinned, three
interleaved rounds):

| Inputs | Cyrius | Rust | Ratio |
|---|---|---|---|
| x ∈ [0, 10⁶) | 2.82–2.85 µs | 19.2–22.4 ns | about 135× |
| any finite bits | 13.7–13.9 µs | 20.2–23.2 ns | about 650× |

The exact bignum search costs more as the decimal exponent grows; zmij and Ryu use tables. JSON writing is on no hot
path, so this is recorded as a finding: the stdlib or bayan printer that ledger L10 asks for would replace it.

### 7.8 Part d2: the work-table pool (2026-11-15)

**Why now.** The pool was planned for part f. The JSON read core (d3) needs scratch that leaves the heap as it found
it: bayan's `_a` parse builds a document's value tree with an Allocator it is given, and the tree is dead once the
document is decoded. So the pool comes first.

**The design** (`src/pool.cyr`) is kimiya ADR 0007 as probe P4 prototyped it (§6.2):
- **Eight slots,** each claimed by compare-and-swap. A free slot already big enough is taken first.
- **Growth.** Otherwise a free slot grows on the freelist, to twice its size or to the claim if that is larger, and
  frees the buffer it outgrew. A slot grows only while it is too small (the race below). When twice the slot can't be
  had, the claim alone is tried, as kimiya does.
- **Overflow.** Past the eighth concurrent claim, a claim is a freelist block of its own (slot −2), freed on release.
- **The release** is a compare-and-swap 1 → 0. The stdlib's CAS orders as acquire-release on both arches (x86
  `lock cmpxchg`, aarch64 `ldaxr`/`stlxr`), so the buffer's last writes are visible before the slot reads free.
  kimiya releases with a full fence and a plain store. The fence alone costs 18 ns, and the CAS halves a claim's cost
  (36 → 18 ns).
- **A bump allocator** over a claim (`_pmn_bump_allocator`) is a stdlib Allocator for bayan's `_a` calls. Its vtable
  (40 bytes) and state (24) are in the caller's storage, because `allocator_new` would take 40 bytes of the bump heap
  for every document. An allocation that doesn't fit returns 0 and leaves the claim as it was.

**Tests.**
- **`tests/pool.tcyr`**, 21 checks on one thread:
  - a claim and its release, and slot reuse;
  - growth to 1 MiB;
  - nine nested claims, the ninth an overflow block;
  - a refused claim of 0 bytes;
  - the allocator's alignment, its realloc copy, a refused allocation and its reset;
  - a soak of 10⁴ claims that leaves `alloc_used` unchanged.
- **`tests/pool_threads.tcyr`**, under contention. Four threads each hold three claims of 64 … 2,047 bytes at a time,
  for 10⁴ rounds, so up to twelve claims race for eight slots. Each claim is filled with a tag unique to its thread,
  round and depth, held through a spin, and read back before its release. Every run checks 15,772,362 words, sees all
  eight slots claimed at once, and takes 23,000–25,500 overflow blocks. `thread` is a test-only stdlib leaf
  (`cyrius.cyml` `[test]`); the library takes none.

**The race the threaded test found.** Its first run failed 1,323 claims and left slots of up to 64 GB:
- A slot that was busy during a claim's first scan can be released before the second scan, already big enough.
- The second scan grew whatever slot it won, so each such race doubled a slot, without bound, until `fl_alloc` refused.
- On one thread nothing changes between the scans, so `tests/pool.tcyr` could not see it.
- kimiya's `_km_scratch_grow` has the check (a slot that fits is returned as it is). P4's prototype, and so this module,
  had lost it. With the check, no slot exceeds twice the largest claim.

**Mutation checks.**
- **A claim by load-then-store** instead of CAS: 4,821,752 words changed under another thread.
- **The growth check removed:** 1,356 failed claims, and the slot bound broken.
- **A claim left unreleased** in the parity harness's `fmt` dispatcher: the harness fails that module and the one after
  it, which sees the same slot ("1 work-table pool slots still claimed after the module"; 3 modules, 2 failing).

**On three targets.** Both test files and the replay pass, warning-free, on x86_64 ELF, PE (wine) and aarch64 (qemu),
with real threads on all three. Every dispatched function's digest is unchanged.

**Untested:** the retry at the claim's size. It needs an allocation that fails at twice a slot's size and succeeds at
the claim's.

**Cost** (`tests/pool.bcyr` against the Rust twin's `pool` row; pinned, three interleaved rounds). Rust has no pool:
where pramana claims scratch, a Rust function allocates a `Vec` and drops it.

| Row | Cyrius | Rust |
|---|---|---|
| a 256-byte claim and its release (a slot hit) | 18–19 ns | 8.5–8.7 ns (a 256-byte `Vec`) |
| a claim, a bump allocator, 4 allocations, the release | 66 ns | — |
| nine nested 256-byte claims, the ninth an overflow block | 262–267 ns | — |

### 7.9 Part d3a: the JSON read core, values (2026-11-15)

Part d3 is split in two, both verified against serde_json:
- **d3a:** the read core for values;
- **d3b:** serde's struct rules, the two enum forms, and ADR 007 row 16's mapped rows (they arise only with struct
  members).

**The read core** (`src/json.cyr`) reads a document in three steps:
- **The pre-pass** (`_pmn_json_lift`, ledger L11) is probe P7's, with the same rules:
  - a negative integer becomes a float;
  - an integer in 2^63 … 2^64 − 1 becomes the negative integer with its bits;
  - a larger integer becomes a float;
  - a raw control byte in a string refuses the document.

  UTF-8 is deliberately not checked there. serde_json validates a string it reads (`parse_str`), and skips one it
  ignores without checking (`ignore_str`, which still refuses control bytes). So the readers check their own strings.
- **The tree.** bayan parses the rewritten text with `bayan_json_v_parse_ctx_a` into a bump allocator over a pool
  claim. The claim starts at 24 bytes per byte of text plus 4 KiB, and doubles while the allocator refuses. Part d3a
  added a refusal count to `_pmn_bump_allocator` for this. One of bayan's out-of-memory paths (`str_new_a`) sets no
  error message, so a message can't tell an exhausted arena from a parse error, and the count can.
- **The readers:** f64, u64 (and usize), bool, String, an array's length and elements, Vec<f64>, Vec<u64>, and a
  matrix's shape (rows, the first row's length, a ragged flag) and its row-major fill. Any refusal is
  `DESERIALIZE`; a caller's null or short buffer is `INVALID_PARAMETER`.

`_pmn_json_decode(text, n, decoder, ctx)` opens the document, calls the type's decoder on the root, and closes it on
every path.

**The arena's size.** Measured on bayan's trees:

| Document | Tree bytes per text byte |
|---|---|
| `[]`, `[[]]`, and 127 levels of nesting | 88 |
| `[[],[],…]` and `[{},{},…]`, 10⁵ elements | 64 |
| `[[0],[0],…]` | 54–55 |
| `[{"a":0},…]` | 32 |
| an object of 10⁵ distinct short keys | 24–25 |
| `[0,0,…]` and `["",…]` | 20–23 |
| `[1.5,…]`, `[true,…]`, `[null,…]` | 8–11 |

Every array or object costs 176 bytes even when empty (its cell, a vec header and 16 slots), so 88 is the bound. 24
covers the numeric documents pramana's types are made of in one claim. The rest double once or twice.

**The pool, refined.** A JSON arena is up to 88 times its text, and a pool slot keeps its buffer for the process's
lifetime. So a claim over 1 MiB now takes no slot: it is a freelist block of its own, freed on release. No slot holds
more than 2 MiB (`tests/pool.tcyr`: 26 checks).

**A guard for a stdlib abort (ledger L13).** `vec_push_a` ends the process ("vec: capacity overflow") one element
past `VEC_CAP_MAX` (2^28), although it returns −1 when the allocator refuses. bayan pushes every array element through
it, so an array of 2^28 + 1 elements, 512 MiB of text, would end the consumer's process. Lowered to 64, the cap shows
it: bayan exits on a 131-byte document of 65 elements. `_pmn_json_open` refuses, unread, any document longer than
2·`VEC_CAP_MAX` − 1 bytes, which can't hold such an array, as `OUT_OF_MEMORY`. pramana's numbers were added to the
open cyrius vec issue.

**The golden rows.** The new `json` module (`scripts/golden/src/m_json.rs`) records `serde_json::from_slice::<T>`
with `float_roundtrip`, the value as tokens or `E:Deserialize`, on 3,886 documents:

| Reader | Rows | Refused |
|---|---|---|
| `f64` | 1,827 | 65 |
| `u64` | 319 | 106 |
| `usize` | 119 | 106 |
| `bool` | 53 | 48 |
| `String` | 584 | 67 |
| `Vec<f64>` | 360 | 103 |
| `Vec<u64>` | 228 | 115 |
| `Vec<String>` | 231 | 97 |
| `Vec<Vec<f64>>` | 165 | 55 |

The documents are:
- the other JSON types and the syntax faults, for every reader;
- every number form, well-formed and not, including integers at 2^53, i64 and u64, `-0`, the f64 range edges and
  subnormals, and exponents past 2^64;
- 600 doubles in serde_json's text, 400 in Rust's `{:e}` and `{:.25e}`, and 400 random decimals;
- 120 exact midpoints between adjacent doubles, each also just below and just above. Their decimals run to 774
  significant digits, and the exact ones are ties, to even;
- every string escape, surrogate pairs and lone surrogates, raw control bytes, invalid UTF-8 (overlong, encoded
  surrogates, past U+10FFFF, truncated), and 500 seeded strings, raw and escaped;
- each form as a vector element, seeded vectors, and matrices, ragged ones included, nested 3 to 200 deep.

**The independent check.** `validate.py` models serde_json's reading from its source, without the oracle: the strict
grammar, the integer and float split (`-0` is the float −0.0), the conversions as Rust's `as f64`, surrogate pairing,
UTF-8 by Python's strict decoder, and every float by Python's correctly rounded `float`. All 3,886 rows agree.
serde_json with `float_roundtrip` is correctly rounded on every midpoint.

**The result.** All 3,886 rows match on x86_64 ELF, PE and aarch64, warning-free. The replay is now four modules,
64,719 rows ok. bayan's float parse is correctly rounded on every midpoint and every long digit string.

**Mutation checks:**
- **no `-0` rewrite:** 48 rows fail: 8 each of `f64`, `u64` and `usize`, 14 `Vec<f64>`, 9 `Vec<u64>` and 1
  `Vec<Vec<f64>>`;
- **no UTF-8 check on a read string:** 9 `String` rows and 18 `Vec<String>` rows fail;
- **a signed integer-to-f64 conversion:** 3 `f64`, 6 `Vec<f64>` and 1 `Vec<Vec<f64>>` rows fail;
- **the arena starting at 1 byte per byte, plus 8:** every row still passes, so the doubling alone carries every
  document;
- **that, with the doubling removed:** 141 `f64` rows, 158 `u64` rows and more fail;
- **2^64 + 1 in the pre-pass's decimal complement:** 61 rows fail, 49 of them `Vec<u64>`;
- **one bit of a golden f64, and one byte of a golden string:** `validate.py` and `--check` each catch both.

**Tests.** `tests/json.tcyr`, 77 checks:
- each pre-pass rule;
- a document's claims on every path (open, close twice, each refusal);
- the size guard at a lowered cap (65 elements refused, 63 read);
- the arena past its first claim (20,000 empty arrays, 64 times the text);
- each reader's edges and caller errors;
- the decode wrapper's error paths;
- a soak of 10⁴ decodes that leaves the bump heap unchanged and nothing claimed.

**Cost** (`tests/json.bcyr` against the Rust twin's `json` rows, byte-identical documents, whose lengths and FNV-1a
hashes both sides print; pinned, three interleaved rounds):

| Document | Cyrius | serde_json | Ratio |
|---|---|---|---|
| Vec<f64>, 100 values in [−10⁶, 10⁶), 1,880 bytes | 66.4–67.1 µs | 3.91–3.98 µs | 17× |
| Vec<Vec<f64>>, 10 × 10 in [0, 1), 1,954 bytes | 74.4–74.6 µs | 5.01 µs | 15× |
| Vec<u64>, 100 words, 2,047 bytes | 69.5–71.8 µs | 3.21–3.25 µs | 22× |
| String, 130 bytes with escapes | 2.83–2.85 µs | 391–398 ns | 7.2× |
| f64, `1.5` | 390–392 ns | 21.5–23.2 ns | 18× |

On the 100-word document, the pre-pass takes about a third and bayan's parse most of the rest. The readers take
2.2 µs and a claim 20 ns. Rewriting the pre-pass took it from 27 to 23 µs, about 10% of a decode:
- one bound check instead of one a byte;
- inline digit tests;
- a decimal subtraction in place of a division loop for the integers at or above 2⁶³.

Most of what remains is Cyrius's code generation, which keeps every local in memory, at about 11 ns a byte. No hot
path decodes JSON, and the pre-pass goes when bayan's L11 fixes land. This is recorded as a finding.

### 7.10 Part d3b: serde's struct rules and the two enum forms (2026-11-15)

**The readers** (`src/json.cyr`):
- **`_pmn_json_struct(v, names, nf, slots)`** is serde derive's rules:
  - the map form: each member named by a field fills its slot, and an unknown member is skipped unread;
  - a field named twice refuses, and so does one never named;
  - every key must be UTF-8, because serde_json parses a struct's keys as strings, the unknown ones too;
  - the sequence form takes exactly nf values, in field order.
- **`_pmn_json_unit_variant`:** an externally tagged unit variant, as `"Name"` or `{"Name": null}` with exactly one
  member.
- **`_pmn_json_newtype_variant`:** `{"Name": value}` with exactly one member. The bare string refuses, because serde
  reads it as a unit variant.

These are serde_json 1.0.154's own rules, read from its source: `deserialize_struct`, `MapAccess`, `SeqAccess` and
`end_seq`, `deserialize_enum` and `VariantAccess`. A type's decoder reads every field before it judges any, because
serde reports a malformed field before `try_from` sees a value.

**Unreachable today, kept:** a field named twice. bayan refuses a duplicate key in the whole document before the
struct reader sees it, and the verdict is the same either way. The check is there for when bayan's opt-in for unread
members lands.

**The golden rows.** The `json` module gains 839 rows, for 4,725 in all:

| Function | Rows | Refused | Mapped (ADR 7) |
|---|---|---|---|
| `json::Pair` (`Normal`'s fields: `mean`, `std_dev`) | 353 | 78 | 10 |
| `json::Shapes` (one field of every kind 1.3.0's types hold) | 310 | 154 | 10 |
| `json::Kernel` (1.3.0's unit-variant enum) | 74 | 61 | 0 |
| `json::PramanaError` (1.3.0's newtype-variant enum) | 102 | 80 | 0 |

The two structs are test types in the generator (serde derive, `serde` =1.0.229 as `rust-old/Cargo.lock` has it).
serde's struct rules are derive's for every struct, so the codecs that M3 onward port meet the same rules. Their
documents are P7's variants of a base value:
- each field missing, named twice, `null`, and set to every form of its kind;
- unknown members of every kind: nested, holding a duplicate key, nested to 100 and to 128–130 levels, holding a lone
  surrogate or a control byte or invalid UTF-8 or a bad escape, and numbers out of range;
- a duplicate unknown key, plain and escaped;
- a key that is not UTF-8, an escaped known key, another case, a trailing space;
- the sequence form exact, short, long, empty and with a trailing comma;
- the syntax faults, and seeded values, plain and pretty.

The enums get every variant in both forms, the wrong form, other cases, extra members, malformed objects, and every
string form as `PramanaError`'s payload.

**The mapped rows** (`tests/golden/json.map.tsv`, ADR 007 row 16). `validate.py` models bayan's three whole-document
refusals, taken from its source:
- a duplicate key in any object, compared decoded;
- nesting past 128 levels, counting the top-level container;
- a lone surrogate escape, high or low.

A row maps to `E:Deserialize` when serde_json reads it and that model refuses it. There are 20, ten per struct, all
in unknown members:
- a duplicate key nested inside, plain and escaped;
- nesting to 128, 129 and 130 levels inside the struct's own object;
- a lone high surrogate, a lone low one, and a lone surrogate in a nested key;
- the struct's own unknown key twice, plain and escaped.

These are the same three causes P7 found in 35 of its 1,495 documents. `--write-cr` writes the file beside the
`.cr.tsv` files, and a run without it checks it.

**The independent check.** `validate.py`'s serde model now also has serde's ignore path (`ignore_value` and
`ignore_str`: syntax only, no UTF-8 or surrogate checks, no depth limit), derive's struct rules and the two enum
forms. All 4,725 rows agree, and the mapped file is exactly what the bayan model gives.

**The result.** All 4,725 rows pass on x86_64 ELF, PE and aarch64, warning-free: 4,705 match, and the 20 mapped rows
are accepted (ADR 7). The replay is four modules, 65,538 rows ok and 96 accepted (76 by ADR 9, 20 by ADR 7).

**Mutation checks:**
- **no UTF-8 check on keys:** 1 `Pair` row and 1 `Shapes` row fail (the non-UTF-8 unknown key);
- **an unknown member refused, not skipped:** 7 `Pair` rows and 7 `Shapes` rows fail;
- **a long sequence read:** 1 `Pair` row and 1 `Shapes` row fail;
- **a unit variant's map form holding anything:** 3 `Kernel` rows and 1 `Shapes` row fail;
- **a newtype variant with a second member read:** 1 `PramanaError` row fails;
- **`json.map.tsv` removed:** its 20 rows fail;
- **one mapped row dropped:** `validate.py` reports the file differs from the model, and the replay fails that row.

**Tests.** `tests/json.tcyr` gains 27 checks, for 104:
- the map form in any order with an unknown member skipped, and the sequence form;
- short and long sequences, a missing field, an escaped key;
- a field named twice (bayan refuses it first) and a key that is not UTF-8;
- the caller's errors;
- both variant forms, exact names, a unit's `null`, one member only, no index form;
- the newtype's payload node, and its bare name refused.

**Cost** (`tests/json.bcyr` against the twin, byte-identical documents; pinned, three interleaved rounds):

| Document | Cyrius | serde_json | Ratio |
|---|---|---|---|
| a `Normal`: the struct rules, two f64s, `Normal::new`'s checks (27 bytes) | 1.65–1.68 µs | 79.1–80.9 ns | 21× |
| a `Kernel`: a unit variant (14 bytes) | 518–523 ns | 34.5–35.4 ns | 15× |

A small document pays mostly fixed costs: the pre-pass's two claims, an arena of 24 bytes per byte of text plus
4 KiB, bayan's 176 bytes per container, and the call chain. This is recorded with part d3a's finding.

### 7.11 Part d4: the write core and the error codec (2026-11-15)

**The writers** (`src/json.cyr`) append serde_json's compact text to the caller's str_builder:
- f64s through `fmtf.cyr`'s serde_json form (`null` for NaN and ±∞);
- u64s and usizes in unsigned decimal, and bools;
- strings with serde_json's escapes (`format_escaped_str`'s table: `\"` and `\\`, `\b \t \n \f \r` named, the other
  bytes below 0x20 as `\u00xx` in lowercase hex, DEL and every other byte as it is), which the stdlib's
  `str_builder_add_json_str` writes exactly;
- vectors, row-major matrices (`[]` for no rows, `[[]]` for one empty row);
- struct members in field order (`_pmn_json_w_key`), unit variants and newtype variants.

Each writer returns `PMN_ERR_NONE`, or `OUT_OF_MEMORY` when the builder can't grow.

**A guard for the stdlib's aborting writers (ledger L14).** `str_builder_putc`, `_add`, `_add_cstr`, `_add_int` and
`_add_json_str` end the process when a grow fails. Three of them have `_a` forms that return −1, but `putc` and
`add_json_str` have none, and `fmtf.cyr`'s printer writes through `putc`. So `_pmn_json_w_reserve` makes room first
with `_sb_grow_a`, which returns −1:
- 32 bytes before each f64 (serde_json's longest f64 text in `fmt.tsv`'s 25,200 rows is 24);
- the exact escaped length before each string.

The aborting writers then never grow the builder. A reservation of 2^62 bytes comes back as `OUT_OF_MEMORY`.
pramana's numbers were added to the open cyrius str_builder issue.

**The error codec** (ADR 007 row 8, amended 2026-11-15 by the maintainer's choice):
- **`pmn_err_to_json(code, sb)`** writes a code as its 1.3.0 variant with an empty message, `{"InvalidParameter":""}`,
  which 1.x reads as `InvalidParameter("")`. It refuses `PMN_ERR_NONE` and the Cyrius-only `OUT_OF_MEMORY` and
  `DESERIALIZE` with `INVALID_PARAMETER`, writing nothing, because no 1.3.0 reader takes them.
- **`pmn_err_from_json(buf, len)`** returns `Ok(code)` for a 1.3.0 error document. The message is read as serde reads
  a String, so one that is not UTF-8 still refuses, and then dropped.

**The golden rows.** The `json` module gains 1,484 rows, for 6,209 in all:
- **`json::write::<T>`**, 1,382 rows: `serde_json::to_vec` of seeded values for every shape.
  - f64: 208, the specials among them;
  - u64: 210, the boundaries among them;
  - bool: 2;
  - String: 343, every byte below 0x20 among them;
  - Vec<f64>, Vec<u64>, Vec<String>: 150, 100 and 100, with NaN and ∞ elements;
  - Vec<Vec<f64>>: 100 rectangular matrices, empty ones included;
  - the two test structs: 100 and 60;
  - `Kernel`: its 4 variants;
  - `PramanaError`: its 5 variants, each with an empty message.
- **`json::PramanaError::code`**, 102 rows: the codec's reading of every error document, the variant alone.

`validate.py` models serde_json's writing independently: the escape table, `json_f64_text` (already checked on
`fmt.tsv`), and the struct and enum forms. All 1,484 agree, and the 4,725 earlier rows are unchanged.

**The result.** All 6,209 rows pass on x86_64 ELF, PE and aarch64, warning-free. The replay is four modules, 67,022
rows ok and 96 accepted.

**Mutation checks:**
- **u64 written as a signed integer:** 103 rows fail;
- **strings written without escapes:** 394 rows fail;
- **a space after a vector's commas:** 224 rows fail;
- **the error codec writing the next variant:** its 5 rows fail;
- **the matrix stride taken from the row count:** 59 rows fail.

**Tests.** `tests/json.tcyr` gains 33 checks, for 137:
- each writer's text, its caller errors, and the reservation's `OUT_OF_MEMORY`;
- the error codec: the empty message, all five codes round-tripping, the four refusals writing nothing, 1.3.0's
  document reading to its code, the bare name and a non-UTF-8 message refused;
- round trips of 2,000 seeded finite f64s, 2,000 u64s, 2,000 ASCII strings (every control byte among them) and 300
  matrices, each read back to the same bits;
- a soak of 10⁴ writes into one cleared builder that leaves the bump heap unchanged.

**Cost** (`tests/json.bcyr`'s write rows against the twin's, serde_json `to_writer` into one cleared `Vec`; the texts
are byte-identical, as both sides print each one's length and FNV-1a hash; pinned, three interleaved rounds, run once
the shared box's load had fallen below 2.5):

| Write | Cyrius | serde_json | Ratio |
|---|---|---|---|
| Vec<f64>, 100 values in [−10⁶, 10⁶), 1,880 bytes | 290.7–292.0 µs | 1.90 µs | 153× |
| a `Normal`, 27 bytes | 1.21–1.23 µs | 63.7–65.3 ns | 19× |
| a String, 5 escapes in 24 bytes | 688–711 ns | 30.9–33.0 ns | 22× |
| an error code, 24 bytes | 215–218 ns | 16.7–17.4 ns | 13× |

The vector's cost is `fmtf.cyr`'s exact digit search, about 2.9 µs a value (part d1's finding, ledger L10). The rest
is the call chain: a reservation, then the stdlib writer byte by byte. No hot path writes JSON.

### 7.12 Part e1: the RNG and its codec (2026-11-15)

Part e is split in three, its modules being independent:
- **e1:** the RNG;
- **e2:** combinatorics with `PmnU128`;
- **e3:** the bridges.

Part f is M1's gate.

**The module** (`src/rng.cyr`) follows D7 as amended after probe P2:
- **`trait PmnRng { fn next_f64(self): f64; }`** is Rust's `Rng`. A consumer brings its own generator by implementing
  it, and a pramana API takes `rng: *R` with `<R: PmnRng>`, so a draw is a static call.
- **`PmnSimpleRng { state }`** is 1.3.0's xorshift64 (13, 7, 17), mapped to [0, 1) by `(x >> 11) / 2^53`, which is
  exact. `PmnSimpleRng_new(out, seed)` fills caller storage (D11) and maps a seed of 0 to 1.
- **Its JSON codec:** `g.to_json(sb)` writes `{"state":N}`. `PmnSimpleRng_from_json(out, buf, len)` reads through
  derive's struct rules on the repr's one u64 field, then `new`, so a saved state of 0 reads as 1, as 1.3.0's
  `#[serde(from)]` does.

**The golden rows:**
- **A new `rng` module**, 46 rows:
  - `SimpleRng::next_f64`: the first 256 draws from 40 seeds: 0, 1, 2, 42, 12345, 2⁶³, u64::MAX, two well-known
    constants, and 31 seeded words;
  - `SimpleRng::digest`: 10⁶ draws from six seeds, folded into an FNV-1a digest of their bits (the plan's "10⁶ RNG
    words").

  `validate.py` re-draws every stream with its own xorshift in Python.
- **`json::SimpleRng`**, 126 rows: derive's variants of a saved state, every integer form in the field (`0`, `-0`,
  2⁶³, u64::MAX, 2⁶⁴, floats, strings), and 64 saved states. The serde model reads the repr, then maps 0 to 1. Its
  unknown members add ten ADR 007 row 16 rows, the same four causes as the test structs' (`json.map.tsv`: 30 rows).
- **`json::write::SimpleRng`**, 64 rows: serde_json's text for `SimpleRng::new(seed)`.

**The result.** All pass on x86_64 ELF, PE and aarch64, warning-free. The replay draws through a bounded generic
`<R: PmnRng>`, the form pramana's APIs use. It is now five modules, 67,248 rows ok and 106 accepted (76 by ADR 9, 30
by ADR 7).

**Tests.** `tests/rng.tcyr`, 26 checks:
- the zero seed, determinism over 10⁴ draws, and the range [0, 1);
- a consumer's own generator (`Eighths`, implementing `PmnRng`) drawn through a generic API;
- the generic giving the same draws as direct calls;
- the codec: `new(0)` writing `{"state":1}`, u64::MAX in full, a saved 0 reading as 1, the sequence form, and the
  refusals (a negative or float state, a missing field, a null out);
- a stream saved after 1,000 draws and restored, whose next 1,000 draws match bit for bit;
- a soak of 10⁴ saves and restores that leaves the bump heap unchanged.

**Cost** (`tests/rng.bcyr` against the twin's `rng` rows, the same stream, whose 10⁶th draw both sides print:
`0x3fe327c85d8ff932`; pinned, three interleaved rounds):

| Draw | Cyrius | Rust | Ratio |
|---|---|---|---|
| direct | 7 ns | 1.71–1.84 ns | 4× |
| through a bounded generic | 10 ns | 1.70–1.72 ns | 6× |

Rust inlines the draw either way; Cyrius makes a call, and the generic a second one (P2 measured the same 2–3 ns).

### 7.13 Part e2: combinatorics with `PmnU128` (2026-11-15)

**The module** (`src/combinatorics.cyr`) is 1.3.0's four functions:
- **`pmn_factorial(n, out)`, `pmn_permutations(n, r, out)` and `pmn_combinations(n, r, out)`** return Rust's
  `Result<u128>` as a code, with the value in a caller's `PmnU128 { lo, hi }`, written only on success (D10). Their
  arithmetic is 1.3.0's step for step:
  - every u64 is compared as unsigned;
  - each product is checked, so an overflow stops a loop at once, whatever r is;
  - `combinations` takes r = min(r, n − r), and checks each product before it divides. So C(130, 65) refuses although
    the result fits, as in Rust.
- **`pmn_stirling_approx(n)`** is √((2π)·n) · (n / e)^n in Rust's order, with n converted unsigned and powf
  correctly rounded.

**The u128 arithmetic** is probe P1's:
- a 128 × 64-bit multiply over 32-bit limbs, with overflow detection;
- a 128 / 64-bit restoring division.

The division was the one cost cliff. It runs 128 steps whatever the divisor, and `combinations` divides at every
step. So part e2 gives it a fast path for d < 2³¹, which covers every divisor `combinations` makes in practice:
long division by 32-bit limbs, where each partial numerator stays below 2⁶³ and the native division is exact.
C(60, 30) fell from 28.5 µs to 1.14 µs, measured in the same pinned run. Larger divisors keep the restoring
division.

**The golden rows.** A new `combinatorics` module, 2,405 rows:
- **`factorial`**, 48 rows: n from 0 to 40, 2³², 2⁶³ and u64::MAX; 13 overflow.
- **`permutations` and `combinations`**, 1,021 rows each, on P1's grid:
  - n from 0 to 40, the u128 edge 130–133, 2³², 2⁶³ and u64::MAX;
  - each n against r in {0, 1, 2, 3, 5, 10, 20, 33–35, 64–67, n − 2, n − 1, n, n + 1};
  - 200 seeded pairs.

  Each has 442 InvalidParameter rows; there are 183 overflows in permutations and 54 in combinations.
- **`stirling_approx`**, 315 rows, class C: n from 0 to 200 (it is finite to 170, then ∞), every power of two,
  u64::MAX and seeded words.

A u128 is two `i:` tokens, its low word then its high word. `validate.py` recomputes the counts with Python's exact
integers in 1.3.0's checked loops, and Stirling's rows through glibc's `pow` by ctypes. It writes
`combinatorics.cr.tsv` from MPFR's correctly rounded pow, and that file has 0 rows: glibc's pow is correctly rounded
on all 315 inputs, so those rows compare directly at 0 ulp.

**The result.** All 2,405 rows pass on x86_64 ELF, PE and aarch64, warning-free. The replay is six modules, 69,653
rows ok and 106 accepted.

**Mutation checks:**
- **no r = min(r, n − r):** 30 `combinations` rows fail, the 30 cases P1 found;
- **signed comparisons:** 66 rows fail across the three counting functions;
- **a signed `n as f64` in Stirling:** 2 rows fail (2⁶³ and u64::MAX);
- **the division taking the low word only:** 124 `combinations` rows fail.

A broken overflow check is not run as a mutation: P1 found it makes `permutations(2⁶³, …)` wrap and loop for about
2⁶³ steps.

**Tests.** `tests/combinatorics.tcyr`, 40 checks:
- 1.3.0's unit tests carried over, with 34! in full;
- the u64::MAX and r > n edges, compared unsigned;
- the out-pointer written only on success, and null outs;
- the multiply at the 128-bit edge;
- the division on both paths, and 20,000 seeded fast-path divisions checked by multiplying back;
- Stirling at 0, 10, its overflow, and u64::MAX.

**Cost** (`tests/combinatorics.bcyr` against the twin's `comb` rows; both print every result's words, and they agree;
pinned, three interleaved rounds, run once the 1-minute load had fallen to 2.0):

| Call | Cyrius | Rust | Ratio |
|---|---|---|---|
| `factorial(34)` | 930–935 ns | 65.3–74.2 ns | 14× |
| `permutations(30, 15)` | 372–376 ns | 15.7–15.8 ns | 24× |
| `combinations(60, 30)` | 1.10 µs | 131.2–131.3 ns | 8.4× |
| `combinations(130, 65)`, an overflow | 1.87–1.89 µs | 474.9–481.5 ns | 3.9× |
| `stirling_approx(100)` | 145–147 ns | 13.9–14.2 ns | 10× |

Rust's u128 multiply is two machine multiplies; pramana's is eight 32-bit partial products and their carries (P1's
form), about 28 ns each. Stirling's cost is the correctly rounded powf. None of these sits on a hot path in 1.3.0,
whose only other caller is a test.

### 7.14 Part e3: the bridges (2026-11-15)

**The module** (`src/bridge.cyr`) is 1.3.0's eleven conversions, each in Rust's operation order:
- **badal:** `ensemble_spread_to_confidence` and `observation_error_to_likelihood`;
- **kimiya:** `yields_to_mean_variance` and `replicates_to_t_statistic`;
- **ushma:** `uncertainty_to_gaussian` and `sensor_noise_to_std_dev`;
- **bodh:** `item_variances_to_alpha`, `z_rates_to_d_prime`, `trait_scores_to_correlation`,
  `learning_trials_to_power_law` and `response_times_to_hick_params`, the last two on 1.3.0's private line fit.

Two §8.3 conventions are set here:
- **A returned tuple** fills the caller's `out` array in order, and the function returns a code, nonzero only for the
  caller's own errors (1.3.0's cannot fail). `out` is written last, so it may alias an input.
- **A slice of pairs** (`&[(u32, f64)]`, `&[(usize, f64)]`) is n records of 16 bytes: the integer word, then the
  f64.

The details that decide bits:
- **Sums:** f64 sums start at −0.0, as Rust's `Iterator::sum` does; the accumulators 1.3.0 wrote as `0.0` start at
  +0.0.
- **Conversions:** counts convert as Rust's `as f64` does, unsigned.
- **Literals:** 1e-15 is written positionally, as Cyrius has no exponent form, and the golden rows pin it at and
  beside the threshold.
- **The fits** keep their points in a pool claim, given back on every path.

**The golden rows.** A new `bridge` module, 5,553 rows:
- **The five two-argument functions** get 661 rows each: a 19 × 19 grid of special values (±0, ±1, the f64
  extremes, NaN, ±∞, and 1e-15 with its neighbours) and 300 seeded pairs.
- **`item_variances_to_alpha`**, 854 rows: counts from 0 to u64::MAX, the floor at and beside 1e-15, and seeds.
- **`replicates_to_t_statistic`**, 552 rows: the same counts, 60 standard errors straddling 1e-15, and seeds.
- **The slice functions:** empty, single, constant, mismatched and NaN-bearing slices, slices near 10⁸ (where the
  variance cancels), and seeds.
- **The fits**, 206 rows each: records with zero, negative and non-finite entries, an exact power law (T = 60 / N),
  an exact Hick line (n = 2^k), equal abscissae (no line), the u32 and u64 extremes, and seeds.

The inputs use no transcendental, as the golden README requires. Rows are class B, except the likelihood (exp) and
the two fits (ln, exp, log2), which are class C. `validate.py` models all eleven in Rust's order, the recorded rows
through glibc by ctypes and the correctly rounded ones through MPFR. All 5,553 agree. `bridge.cr.tsv` has 0 rows:
glibc's exp, ln and log2 are correctly rounded on every input here.

**The result.** Every output of all 5,553 rows matches at 0 ulp, on x86_64 ELF, PE and aarch64, warning-free. The
replay is seven modules, 75,206 rows ok and 106 accepted.

**Mutation checks:**
- **the 1e-15 floor one ulp higher:** 54 rows fail (50 in `item_variances_to_alpha`, 4 in
  `replicates_to_t_statistic`);
- **a signed count conversion:** 12 `replicates_to_t_statistic` rows fail;
- **the power-law fit keeping non-positive times:** 178 of its 206 rows fail;
- **Rust's sums started at +0.0:** the replay passes. The harness counts ±0 as 0 ulp apart, and class B tolerates
  it. `tests/bridge.tcyr` pins the rule bit for bit instead, and fails 2 checks under the same mutation.

**Tests.** `tests/bridge.tcyr`, 44 checks:
- all 19 of 1.3.0's unit tests carried over, the fits' skipped-points cases among them;
- the −0.0 means;
- the caller's errors (null and negative inputs), and `out` aliasing an input;
- a soak of 10⁴ fits that leaves the bump heap unchanged and nothing claimed.

**Cost** (`tests/bridge.bcyr` against the twin's `bridge` rows; both print their outputs' bits, which agree;
pinned, three interleaved rounds, at a load average of 1.3):

| Call | Cyrius | Rust | Ratio |
|---|---|---|---|
| `observation_error_to_likelihood` | 60 ns | 7.0–7.1 ns | 8.5× |
| `yields_to_mean_variance`, 100 yields | 575–578 ns | 134.4–135.4 ns | 4.3× |
| `trait_scores_to_correlation`, 100 pairs | 1.23 µs | 190.9–192.2 ns | 6.4× |
| `learning_trials_to_power_law`, 20 trials | 2.76–2.82 µs | 362.7–365.2 ns | 7.6× |
| `response_times_to_hick_params`, 20 records | 838–844 ns | 209.8–218.3 ns | 4.0× |

The likelihood's cost is the correctly rounded exp, and the power law's its 40 correctly rounded logarithms. The
rest is the per-element call and memory traffic of Cyrius's code generation.

### 7.15 Part f: M1's gate (2026-11-15)

Every part ran the per-part gates: `build`; `test` and `test --bounds` warning-free; `bench`, recorded against Rust;
`vet`, `lint` and `fmt --check`; `distlib`; `deps --locked`; `audit`; the ledger; the three targets. The plan's §7
list adds what a milestone's gate covers across its parts.

**Debug (D12).** 1.3.0 derives `Debug` on M1's two types, and no part had ported it.
- **`src/debug.cyr`** holds the derive's two shapes as builders over the JSON writers' non-aborting appends: a struct,
  `Name { a: 1 }` or `Name {\n    a: 1,\n}`, and a tuple, `Name(x)` or `Name(\n    x,\n)`.
- **`g.debug(sb, alternate)`** writes `SimpleRng { state: N }`.
- **`pmn_err_debug(code, sb, alternate)`** writes the variant with an empty message, `InvalidParameter("")`, as
  `pmn_err_to_json` does. It refuses the codes 1.3.0 has no variant for, as the JSON writer does (ADR 007 row 8).
- **A golden `debug` module**, 60 rows: Rust's own `{:?}` and `{:#?}` of SimpleRng from 25 seeds and of each error
  variant. `validate.py` models the derive's text, and all 60 match on three targets.

M3 adds nested values' indentation and the f64 Debug form with its first type that has either.

**Surface parity.** `scripts/surface_parity.py --check`, now in CI:
- **What it lists:** every public item of 1.3.0's ported modules, from `rust-old/src/`: functions, types, traits,
  their methods, and their derives.
- **What it checks:** each item maps, in its table, to the Cyrius symbols `src/` defines, or to a note for a form with
  no symbol (a plain value's Clone).
- **M1's result:** 30 items in error, rng, combinatorics and bridge, all ported.
- **Mutation checks:** a renamed function and an unmapped derive each fail it.

Each later milestone adds its modules to the table. M8's full diff adds lib.rs, whose `pub use hisab` plan §8.2
drops.

**The soak** (`tests/soak.tcyr`, probe P4's measures). After a warm-up, 10⁴ rounds of every M1 path that allocates:
- JSON decodes, small and large; the large document is 367,879 bytes (20,000 values), so its arena is a block that is
  mapped and unmapped each time;
- a refused decode, JSON writes, the error codec both ways, Debug;
- SimpleRng's codec both ways;
- both bridge fits;
- nine nested pool claims.

The bump heap, the freelist's arena cursor and the mapped size (`/proc/self/statm`) were unchanged, and nothing was
left claimed. All eight checks ran and passed on x86_64 ELF, PE and aarch64; wine and qemu both expose
`/proc/self/statm`, so the mapped size was measured on each. One mutation, `_pmn_json_decode` skipping its close, makes the mapped size grow by 2.4 GB and leaves 7
slots claimed.

**The audit of `src/`** against CLAUDE.md's library rules:
- **No aborts.** Nothing aborts outside `fmtf.cyr`, whose `str_builder` writers can't grow the builder after ledger
  L14's reservation.
- **No heap resets or hashmap iteration:** no `alloc_reset`, no hashmap iteration, no `vec_get`.
- **Names:** every top-level name in pramana's own modules carries a pmn prefix.
- **`#must_use`** now marks the four functions whose result a caller must read (`_pmn_json_open`,
  `_pmn_json_decode`, `_pmn_json_lift`, `_pmn_json_neg_complement`). Three ignored results in `tests/json.tcyr` are now
  asserted. The functions still without it return a constant 0 or only fill storage.
- **Null and aliasing:** every public buffer parameter is checked for null, and the bridges' `out`, written last,
  may alias an input.

**The plan's minimum grid for M1** (§7):
- every primitive's correctly rounded rows at 0 ulp: 76 corrections, and every class-C row at bound 0;
- 10⁶ RNG words: six 10⁶-draw digests, besides the 40 streams;
- factorial 0–35 and the overflow edges: 0–40, 2³², 2⁶³ and u64::MAX;
- the bridges' inputs over their ranges, NaN, ±∞ and empty slices among them.

**M1's result.** Eight golden modules: 75,266 rows ok and 106 accepted (76 under ADR 9, 30 under ADR 7). They pass
on x86_64 ELF, PE (wine) and aarch64 (qemu), warning-free, with identical digests. There are eleven test files and
nine bench files with their Rust twins. The ledger has 14 rows, all open, with 11 notes in `src/`.

## 8. M2: the special functions

M2 ports 1.3.0's `src/math.rs` into `src/special.cyr` (D3 = C). Part a is T0: Rust's algorithms in Rust's operation
order on the correctly rounded primitives, at 0 ulp against the `special.cr.tsv` model. Part b is T1: the five
replacements of plan §3.4, gated against `special.ref.tsv` at ADR 009's bounds. The parts are listed in plan §7.

The golden module is `special`. Its rows keep Rust's names, `math::<fn>`, because `math.tsv` already holds the std
f64 operations. The functions are crate-private in 1.3.0, recorded through the `oracle` hooks, and private here
(`_pmn_`), so the surface check doesn't list them.

### 8.1 Part a1: erf, erfc, Φ and 1 − Φ, T0 (2026-11-15)

**The module.** fdlibm's erf and erfc (FreeBSD's `s_erf.c`, as 1.3.0 carries it through the Rust libm crate), and
`normal_cdf` = ½·erfc(−z/√2) and `normal_sf` = ½·erfc(z/√2):
- **The branches** read the high word as Rust's `high_word` does: `load64(&x) >> 32`, a logical shift.
- **The tail** builds z by clearing x's low 32 bits through `store64`, so −z·z is exact. It calls the correctly
  rounded exp twice, in Rust's order: exp(−z·z − 0.5625) · exp((z − x)(z + x) + R/S) / x.
- **The coefficients** are `src/constants.cyr`'s generated `_PMN_MATH_FDLIBM_*`, proven against the oracle in M0
  step 6.
- **Every polynomial** is Rust's nested Horner form, written out step by step (plan §8.3).

The file carries ledger row L8's note, the scope request that would hand these functions to ganita.

**The golden rows.** A new `special` module, 28,290 rows, all class C (each function calls exp):
- **`erf` and `erfc`,** 6,246 rows each:
  - every high word fdlibm branches on (2^-56, 2^-28, 1/4, 0.84375, 1.25, 1/0.35, 6 and 28), with the five doubles
    on either side and 24 seeded low words in the high word at and below it, both signs;
  - 200 seeded arguments in each branch's interval, both signs;
  - tiny (to 10^-320) and wide (to 10^300) log-uniform sweeps;
  - k/32 on [−30, 30];
  - the specials.
- **`normal_cdf` and `normal_sf`,** 7,899 rows each: the erf grid times √2, so that z/√2 lands on or beside each
  branch point; k/16 on [−40, 40], where Φ underflows below −38.4; and 400 seeded arguments.

The inputs use no transcendental (the golden README's rule). The first grid, with only the five doubles at each
branch point, missed one mutant: the RA/RB boundary moved by a single high word. The seeded low words are the fix.

**The model.** `validate.py` transcribes the four functions in Python, reads the fdlibm coefficients with its own
extractor from `rust-old/src`, and re-derives every row through glibc's exp (ctypes). All 28,290 agree. Then it
re-derives each row with MPFR's exp: 10 rows move, each by 1 ulp, and go into `special.cr.tsv`. They are erfc near
6.1·10⁻⁶⁴ and 1.4·10⁻¹³⁸, and Φ and 1 − Φ at the same points.

**The result.** Every row matches at 0 ulp: 28,280 against 1.3.0's own bits and the 10 against `special.cr.tsv`.
This holds on x86_64 ELF, PE (wine) and aarch64 (qemu), warning-free. The replay is nine modules: 103,546 rows ok,
and 116 accepted (86 under ADR 9, 30 under ADR 7). The digests are identical on all three targets, except the
native-builtin shadows on aarch64, as by design (§5).

**Mutation checks** (the whole corpus replayed with each change; the unmutated run passes):

| Mutant | Rows failed |
|---|---|
| erfc_mid's `(1 − erx) − P/Q` as `1 − (erx + P/Q)` | 838 |
| z not masked to its high word | 3,664 |
| the tail's `(e1 · e2) / x` as `e1 · (e2 / x)` | 2,180 |
| QQ1 in QQ2's place | 4,176 |
| 1 − Φ by multiplying with 1/√2 rather than dividing by √2 | 2,710 |
| the RA/RB boundary one high word up | 7 |
| erfc_mid's end one high word up | 46 |
| erf's saturation at 0x40170000 | 48 |
| erfc's saturation at 0x403B0000 | 44 |
| erfc's 1/4 split at 0x3FD10000 | 6 |
| erf's tiny branch at 0x3E200000 | 2 |

Three mutants were equivalent, and no replay could catch them:
- **erfc's 2^-56 branch at 2^-57:** below 2^-54, 1 − x and the small rational both round to 1.
- **½·e as e·½:** multiplication commutes.
- **−(z/√2) for (−z)/√2:** negation is exact.

**Tests.** `tests/special.tcyr` has 25 checks:
- all six of 1.3.0's erf and Φ tests (known values, erf + erfc = 1, glibc's reference values, the specials, the
  tiny path, Φ's tails);
- what a class-C replay doesn't pin bit for bit: erf(−0) = −0, a NaN returned with its payload, erf(−x) = −erf(x)
  over 400 arguments from 10⁻¹⁹ to 170, erfc(−40) = 2 − MIN_POSITIVE, and Φ(−40) = +0.

**Cost** (`tests/special.bcyr` against `m2_special.rs`; the record is `docs/benchmarks-rust-v-cyrius.md`):
- **the tails:** 7–7.5× Rust, mostly the two correctly rounded exps;
- **the small rationals:** 10.6–13× Rust, the compiler's f64 code.

Writing the Horner steps out, rather than calling a three-argument helper that `#inline` declines, took 18–38 ns
off every row.

### 8.2 Part a2: lnΓ and its relatives, T0 (2026-11-15)

**The module** adds five functions to `src/special.cyr`, each in Rust's operation order:
- **`ln_gamma`:** Lanczos with g = 7 and nine coefficients. Below ½ it uses the reflection ln π − ln|sin πx| −
  lnΓ(1 − x).
- **`stirling_error`:** the asymptotic series in 1/a² from a = 10, and the difference lnΓ(a) − [(a − ½) ln a − a +
  ln √(2π)] below.
- **`bd0`:** Loader's deviance term, m·((1 + u) ln(1 + u) − u).
- **`ln_gamma_kernel`:** a ln x − x − lnΓ(a), in its Stirling form from a = 10.
- **`ln_beta`:** in R's lbeta scheme, on p = min(a, b) and q = max(a, b).

The primitives are the correctly rounded ln, ln_1p and sin.

Three of 1.3.0's behaviours are kept as they are, for T1 to change:
- **The cutoff:** `ln_gamma` returns +∞ when |sin πx| < 1e-300. That holds at ±0, and for 0 < |x| ≲ 3.2·10⁻³⁰¹,
  where lnΓ is about 691. The literal is 1e-300, and the port reads it as the generated `LENTZ_TINY`, which is that
  literal's double.
- **Negative integers:** they are not poles. sin(π·n) rounds to about n·1.2·10⁻¹⁶, so lnΓ(−1) is 37.8.
- **lnΓ(+∞) is NaN,** from ∞ − ∞ in the last line.

`ln_beta` uses Rust's `min` and `max`, which ignore a NaN, so ln B(NaN, 15) is ln B(15, 15). Probe P9's T0
transcription swapped the shapes with one comparison, which disagrees with 1.3.0 on exactly those rows.

**The golden rows.** The `special` module grows to 45,070 rows, all class C:
- **`ln_gamma`,** 3,377 rows:
  - the switch at ½;
  - the cutoff, x = 1e-300/π with 40 doubles on either side, both signs;
  - the negative integers and half-integers to −30 with their neighbours;
  - the roots at 1 and 2;
  - k/16 on [−20, 30], and the integers to 200;
  - seeded sweeps of [−10, ½), [½, 10) and [10, 200);
  - log-uniform sweeps to 10³⁰⁶ and to −10¹⁵.
- **`stirling_error`,** 1,225 rows: the switch at 10 with its neighbours, the integers to 100, seeded sweeps on either
  side, and log-uniform sweeps from 10⁻³¹⁰ to 10³⁰⁰.
- **`bd0`,** 5,050 rows:
  - x = 0;
  - x = m·r for twelve ratios from 0 to 1000, on 300 seeded m in [10⁻³, 10¹²];
  - x within 4 doubles of m;
  - integer counts against seeded means;
  - seeded pairs.
- **`ln_gamma_kernel`,** 4,782 rows: the switch at a = 10 with its neighbours; x = a·r for nine ratios, on 300 seeded
  a; x = a(1 + k/64) near the mode; and seeded pairs.
- **`ln_beta`,** 2,346 rows:
  - 19 shapes from 10⁻³ to 10¹² taken pairwise, 10 and its neighbours among them;
  - seeded pairs, 1,000 of them with both shapes in [10, 60);
  - NaN, ±0 and negative shapes.

The 1,000 pairs were added after the first grid let one mutant through: the both-large branch's `(t1 + ln √2π) +
corr` reordered. corr is about 0.008 against results near 14, so a 1-ulp change in the partial sum rarely reaches
the output.

**The model.** `validate.py` transcribes the five functions as well. The coefficients come from its own reading of
`rust-old/src`, and a zero divisor gives ±∞ or NaN as IEEE does, not Python's exception. Every row is re-derived
through glibc's exp, ln, log1p and sin, and all 45,070 agree. With each primitive correctly rounded (MPFR), 92 more
rows move, and `special.cr.tsv` has 102 in all:

| Function | Rows corrected | Max | The glibc primitive that moves them |
|---|---|---|---|
| `bd0` | 15 of 5,050 | 13 ulp | log1p |
| `ln_gamma_kernel` | 21 of 4,782 | 101 ulp | log1p |
| `ln_beta` | 56 of 2,346 | 2 ulp | log1p (55), ln (1) |
| `ln_gamma`, `stirling_error` | 0 | — | — |

The large distances are cancellation. In bd0, (1 + u) ln(1 + u) − u is small next to its terms when x is near m. In
the kernel, so is u − ln(1 + u). A 1-ulp difference in log1p then becomes 13 or 101 ulp of the result. These rows
are still Rust's operations on correctly rounded primitives, so ADR 009 accepts them. T1's bounds apply to
`ln_gamma` and the incomplete functions, not to these intermediate terms.

**The result.** Every row matches at 0 ulp: 44,968 against 1.3.0's own bits and 102 against `special.cr.tsv`. The
replay is nine modules: 120,234 rows ok and 208 accepted (178 under ADR 9, 30 under ADR 7), warning-free, on x86_64
ELF, PE (wine) and aarch64 (qemu). Every `math::` result line is identical on the three.

**Mutation checks** (the whole corpus replayed with each change; the unmutated run passes):

| Mutant | Rows failed |
|---|---|
| the reflection at x ≤ ½ | the replay crashes: lnΓ(½) recurses into itself |
| the cutoff compared with ≤ | 2 (two rows land on 1e-300 exactly) |
| the cutoff doubled | 83 |
| Lanczos's c0 dropped | 5,853 |
| Lanczos's terms summed from c8 down | 1,425 |
| lnΓ's last line as `(a2 + ln sum) − t` | 1,848 |
| the Stirling switch at a > 10 | 38 |
| the series' 1/156 as 1/157 | 11 |
| bd0 as the naive x ln(x/m) + m − x | 3,554 |
| the kernel's switch at 9 | 35 |
| the kernel's `(m + h) − ln √2π` as `m + (h − ln √2π)` | 863 |
| ln B's min and max as one comparison (P9's form) | 2 (the NaN rows) |
| ln B's both-large branch at p > 10 | 9 |
| ln B's mixed branch at q > 10 | 22 |
| ln B's `(t1 + ln √2π) + corr` as `(t1 + corr) + ln √2π` | 8 |

**Tests.** `tests/special.tcyr` grows to 41 checks:
- 1.3.0's four tests for these functions: lnΓ's known values, the Stirling series continuous at 10, ln B against
  lnΓ on five pairs and ln B(10⁷, 1) = −ln 10⁷, and the kernel against its direct form;
- the poles and cutoff pinned: lnΓ(±0) = +∞, lnΓ(3·10⁻³⁰¹) = +∞ and lnΓ(4·10⁻³⁰¹) finite, lnΓ(−1) finite, and NaN
  for ±∞ and NaN;
- bd0's identities, which 1.3.0 never tested: bd0(0, m) = m, bd0(m, m) = +0, and bd0 ≥ 0 and equal to the naive
  form on 60 arguments;
- ln B(NaN, 15) = ln B(15, NaN) = ln B(15, 15) bit for bit, and ln B symmetric bit for bit on 40 pairs across all
  three branches.

**Cost** (`docs/benchmarks-rust-v-cyrius.md`): 5–8× Rust where ln and sin lead, and 11–12× where the correctly
rounded log1p does (`bd0`, the large-shape kernel, the both-large ln B). All 21 rows' sums agree with the twin.

### 8.3 Part a3: the regularized incomplete gamma, T0 (2026-11-15)

**The module** adds `gamma_inc(a, x)`, which returns (P, Q). Each value is computed without cancellation:
- **the series** for P below x = a + 1, with Q = 1 − P;
- **Lentz's continued fraction** for Q from x = a + 1, with P = 1 − Q;
- **past a = 10⁹, Wilson–Hilferty:** (x/a)^⅓ is close to N(1 − 1/(9a), 1/(9a)), through part a1's Φ and 1 − Φ.

Both loops start from part a2's kernel through exp, and they stop at a relative 1e-15. Each runs for at most
`iteration_budget(a)` = min(200 + 20√a, 10⁶) iterations, truncated, and then clamps its result to [0, 1].
- **Rust's semantics** come through mathx: the budget's `min` and saturating `as usize`, and `clamp`, which passes a
  NaN through.
- **The two wrappers,** `regularized_lower_incomplete_gamma` and `regularized_upper_incomplete_gamma`, are
  gamma_inc's two halves.
- **The primitives** are exp, ln and ln_1p (correctly rounded), sqrt, and ganita's correctly rounded cbrt, which is
  what Rust's compiler_builtins cbrt gives.

Probe P9's T0 transcription had no Wilson–Hilferty branch, so the port starts from Rust's source rather than from P9.

**The golden rows.** The `special` module grows to 51,841 rows. `gamma_inc` has 5,971, each with two outputs:
- the specials in either argument;
- the switch x = a + 1 on 20 shapes from 10⁻³ to 10⁶, with three doubles on either side;
- the Wilson–Hilferty switch: a = 10⁹ with its neighbours and a(1 ± 10⁻¹²), against x = a + j√a;
- 50 seeded shapes past 10⁹ (to 10¹²);
- x = a·r for eleven ratios on 300 seeded shapes in [10⁻³, 10⁹];
- x = a + j√a near the mode on 60 seeded shapes in [1, 10⁹], where the loops run longest;
- the χ² shapes k/2;
- the prefactor's underflow at both ends;
- 70 rows with a from 10⁻³⁰⁰ to 10⁻⁸ against x up to 3, added for the clamp (below);
- seeded pairs.

The two wrappers get 400 seeded rows each. The grid uses √a, which IEEE 754 rounds correctly; the golden README's rule
now names it.

**The model.** `validate.py` transcribes the loops, the budget and Rust's clamp. Every row is re-derived through
glibc, and all 6,771 agree. With correctly rounded primitives, 36 rows move, so `special.cr.tsv` has 138:

| Function | Rows corrected | Max | The glibc primitive that moves them |
|---|---|---|---|
| `gamma_inc` | 22 of 5,971 | 237 ulp | log1p (13), exp (9) |
| `regularized_lower_incomplete_gamma` | 3 of 400 | 1,417 ulp | log1p |
| `regularized_upper_incomplete_gamma` | 11 of 400 | 1,101 ulp | log1p (10), exp (1) |

The large distances are in the deep tails, at results from 10⁻¹³³ to 10⁻²¹⁰. There the kernel is a few hundred in
size, and exp turns its absolute rounding into the result's relative error. A 101-ulp kernel difference (§8.2)
becomes a thousand ulp of P or Q. These are Rust's operations on correctly rounded primitives, as ADR 009 accepts. T1
(part b4) replaces the prefactor with a double-double one.

**The result.** Every row matches at 0 ulp, both of `gamma_inc`'s outputs included: 6,735 against 1.3.0's own bits
and 36 against `special.cr.tsv`. The replay is nine modules: 126,969 rows ok and 244 accepted (214 under ADR 9, 30
under ADR 7), warning-free, on x86_64 ELF, PE (wine) and aarch64 (qemu). Every `math::` result line is identical on
the three.

**Mutation checks** (the whole corpus replayed with each change; the unmutated run passes):

| Mutant | Rows failed |
|---|---|
| the series used at x ≤ a + 1 | 48 |
| Wilson–Hilferty from a ≥ 10⁹ | 14 |
| Wilson–Hilferty's mean 1 + 1/(9a) | 632 |
| the budget without √a (200 flat) | 614 |
| the series stopped at 2·10⁻¹⁵ | 1,114 |
| the series' term as (del·x)/ap | 785 |
| the continued fraction stopped at 2·10⁻¹⁵ | 563 |
| the continued fraction's an/c as an·(1/c) | 516 |
| the continued fraction's b as x + (1 − a) | 34 |
| the series unclamped | 19 (after the tiny-shape rows; 0 before) |
| the budget's base 100, not 200 | 0 |
| the continued fraction's guard on d dropped | 0 |
| the continued fraction's guard on c dropped | 0 |

The three survivors are code that no reachable input runs:
- **The budget's base:** no row comes near running a loop out. Counted near the mode (x = a + j√a, |j| ≤ 6, a from
  10³ to 10⁹), the loops take at most 8.16√a iterations: 218,875 at a = 10⁹, of the 632,655 allowed.
  `tests/special.tcyr` pins the budget's values directly instead.
- **The two guards:** c starts at 10³⁰⁰, b grows from 2 by 2 each step, and d = an·d + b would need an exact
  cancellation. Neither can approach 10⁻³⁰⁰ for gamma's fraction. They stay because 1.3.0 has them, and `beta_cf`
  (part a4) shares the form.

The tiny-shape rows exist because of the clamp mutant. For a near 0, P just below x = a + 1 is within about 0.2a of 1,
and 1.3.0's series overshoots 1 on 19 of those rows before its clamp.

**Tests.** `tests/special.tcyr` grows to 62 checks:
- 1.3.0's four tests: the known values, the upper tail against 50-digit references to a = 5·10⁵, the
  Wilson–Hilferty switch continuous at 10⁹, and the invalid arguments;
- the budget: 200 for 0, NaN, ±∞ and negative shapes, 220 at 1, 400 at 100, 20,200 at 10⁶, 632,655 at 10⁹, and the
  10⁶ cap at 10¹⁰;
- the edges: x = −0 and −∞ give (0, 1), x = +∞ gives (1, 0), and a = 0 gives (NaN, NaN);
- the wrappers equal gamma_inc's halves bit for bit, on both branches;
- P(1, x) = 1 − e^-x and Q(1, x) = e^-x within 1e-13, for x from 0.05 to 300. My first bound, 1e-14, failed at
  x ≳ 190. MPFR shows the cause is 1.3.0's own algorithm, not the port: Q(1, 300) is 1.4·10⁻¹⁴ off with every
  primitive correctly rounded, because exp(kernel) turns the kernel's absolute rounding into relative error. T1
  (part b4) removes it.

**Cost** (`docs/benchmarks-rust-v-cyrius.md`): 4.5–5.4× Rust, all 24 bench rows' sums agreeing with the twin.

**The harness** now pads function names to 44 columns, since the wrappers' names (41 characters) ran into the class
letter.

### 8.4 Part a4: the regularized incomplete beta, T0 (2026-11-15)

**The module** adds `beta_inc(a, b, x, y)`, which returns (I_x(a, b), 1 − I_x(a, b)). The caller passes y = 1 − x
itself, so a caller that knows it exactly never rounds 1 − x; the value is not checked.

**`beta_cf`,** Lentz's continued fraction, runs on whichever side converges faster:
- **x < (a + 1)/(a + b + 2):** the fraction gives I directly, and 1 − I is 1 − it;
- **otherwise:** through I_x(a, b) = 1 − I_y(b, a).

**Its prefactor,** x^a·y^b/(a·B(a, b)), is taken in logs, in one of two forms:
- **both shapes 10 or more:** TOMS 708's brcomp form on part a2's bd0 and Stirling remainder;
- **otherwise:** a ln x + b ln y − ln B − ln a, each log on the side that keeps precision, ln(1 − y) as ln_1p(−y)
  below ½.

**The loop** alternates even and odd steps, each denominator kept from 0 by 1e-300. It stops when an odd step's
factor is within 1e-15 of 1, or when part a3's budget for max(a, b) runs out, and then clamps to [0, 1].
`regularized_incomplete_beta(x, a, b)` is beta_inc's first value with y = 1 − x.

**The golden rows.** `special` grows to 58,101 rows. `beta_inc` has 5,853, each with two outputs:
- the specials in each argument;
- 17 shapes from 10⁻³ to 10⁵, 10 and its neighbours among them, taken pairwise against eleven x;
- the side switch on 100 seeded pairs;
- both shapes in [10, 10⁶] at the mean ± j standard deviations;
- the callers' forms, with x and y each computed exactly: Student's t (ν/2, ½, ν/(ν + t²), t²/(ν + t²)), and F with
  two numerator degrees of freedom;
- a y that is not 1 − x;
- shapes down to 10⁻³⁰⁰;
- underflowing prefactors;
- 1,000 seeded rows.

The wrapper has 407 rows: seeds, and the out-of-range x.

**The model.** `validate.py` transcribes both functions. All 6,260 rows agree through glibc. With correctly rounded
primitives, 88 rows move, so `special.cr.tsv` has 226:

| Function | Rows corrected | Max | The glibc primitive that moves them |
|---|---|---|---|
| `beta_inc` | 81 of 5,853 | 751 ulp | log1p (75), exp (6) |
| `regularized_incomplete_beta` | 7 of 407 | 235 ulp | log1p |

As in parts a2 and a3, the large distances come from the prefactor. Through bd0 and the kernel-like logs, a 1-ulp
log1p difference becomes an absolute error in the exponent, and exp turns that into relative error.

**What the loop reaches.** I counted the iterations on every row whose result depends on the loop: 4,915 rows, whose
prefactor is finite and not 0. The loop uses at most 14.4% of its budget, and no step's |d| or |c| falls below 10⁻⁷
(the first d below 6·10⁻⁸). So the budget never binds and the five 1e-300 guards never fire. 1.3.0 keeps both as
defensive limits.

**A finding.** On 6 rows the prefactor is NaN rather than 0: an infinite or f64::MAX shape, as in
`beta_inc(3, ∞, 0.7, 0.3)`. 1.3.0 then runs the whole 10⁶-iteration budget before returning NaN, and so does the
port. `tests/special.tcyr` pins the NaN. For T1 (part b5), the arguments could be refused before the loop.

**The result.** Every row matches at 0 ulp: 6,172 against 1.3.0's own bits and 88 against `special.cr.tsv`. The
replay is nine modules: 133,141 rows ok and 332 accepted (302 under ADR 9, 30 under ADR 7), warning-free, on
x86_64 ELF, PE (wine) and aarch64 (qemu). Every `math::` result line is identical on the three.

**Mutation checks** (the whole corpus replayed with each change; the unmutated run passes):

| Mutant | Rows failed |
|---|---|
| the side switch at x ≤ (a + 1)/(a + b + 2) | 221 |
| the switch's denominator as a + (b + 2) | 12 |
| brcomp's form from a > 10 | 43 |
| ln x never taken as ln_1p(−y) | 205 |
| ln y never taken as ln_1p(−x) | 201 |
| brcomp's last terms as t3 − (corr + ln a) | 377 |
| the first d as 1 − (a + b)x/(a + 1), cancelling | 2,508 |
| the even numerator as m((b − m)x) | 666 |
| the odd step's a + (b + m) | 227 |
| the stop at 2·10⁻¹⁵ | 491 |
| the result unclamped | 8 |
| the budget on a, not max(a, b) | 0 |
| the budget's base 100, not 200 | 0 |
| any one of the five 1e-300 guards dropped | 0 |

The seven survivors are the budget and the guards, which (above) no reachable row engages.

**Tests.** `tests/special.tcyr` grows to 75 checks:
- 1.3.0's four tests: the known values, the complement's tail precision, the F(2, d) closed form for d up to 10⁸, and
  the invalid arguments;
- a NaN y gives (NaN, NaN), and y ≤ 0 gives (1, 0);
- the infinite shape's NaN;
- I_x(a, b) = 1 − I_y(b, a) bit for bit on 30 shape pairs;
- the wrapper as beta_inc's first value bit for bit.

**Cost** (`docs/benchmarks-rust-v-cyrius.md`): `beta_inc` at the mean is 5.2× Rust for small shapes and 3.8× for
large ones. The small-shape row's bench sum differs from the twin's, and `validate.py`'s model, run on the same 4,096
arguments, gives Rust's sum through glibc and the port's through correctly rounded primitives: 144 of them differ as
glibc's log1p misrounds.

**T0 is complete.** All of 1.3.0's `src/math.rs` is ported in Rust's operation order. The golden rows cover the
fourteen functions its `oracle` hooks expose, 58,101 rows at 0 ulp on correctly rounded primitives.

### 8.5 Part b1: erf and erfc correctly rounded, T1 (2026-11-16)

T1 begins. erf and erfc are now CORE-MATH's binary64 algorithms, correctly rounded, in place of 1.3.0's fdlibm (D3 =
C, ADR 009's first T1 bound). 1.3.0's fdlibm transcription, which part a1 proved at 0 ulp, is retired from `src/`;
its history is the a1 commit.

**The module.** Probe P9's FMA-free port (§6.7) moves into `src/crerf.cyr`, unchanged except for its names (`_pmn_cm_`,
and kimiya's helpers by their vendored `_pmn_km_` names). Its tables move into `src/crerf_data.cyr`.
- **The tables** are generated by a new script, `scripts/vendor_coremath.py`, from CORE-MATH's `erf.c` and `erfc.c`
  in `build/coremath/`. Those are fetched and never committed, and the script pins their sha256 sums. The output
  equals P9's tables word for word.
- **The inline constants** need checking too. The port writes a few thresholds inline as bit patterns, and
  `--check` verifies each one: it is either an integer word the C sources contain verbatim, or it carries a comment
  citing the C literal it encodes, with the bits matching and the literal present in the sources. Powers of two the
  port uses itself pass on their bits alone. The first run found three hand-typed words without a citation, among
  them erfc's subnormal exception, which C computes as `fma(0x1p-1074, -0.25, 0x1.99ef5883f656cp-1024)`. That rounds
  back to the literal, so the word now cites it.
- **Wiring:** `_pmn_erf` and `_pmn_erfc` in `src/special.cyr` call the port. Φ, 1 − Φ and gamma_inc's
  Wilson–Hilferty branch, which call erfc, keep 1.3.0's formulas and run on the correctly rounded erfc, until part b2.
- **The ledger:** `src/crerf.cyr` carries rows L8 (the special functions' scope) and L12 (the emulated fma), and CI
  runs the vendor check.

**The proof of correct rounding** is `scripts/crverify/` (`run.sh erf`, a new set beside the primitives'), on P9's
2,768,534 arguments: CORE-MATH's 184,595 worst cases in both signs, over 10⁶ seeded arguments per function weighted
to every branch point, the subnormals and the tails, and 10⁵ per function through the accurate path alone.
- **Against P9:** this copy's output equals P9's recorded output byte for byte.
- **Against CORE-MATH's own C build** (hardware FMA): 0 of 2,568,534 rows differ.
- **On three targets:** x86_64, PE and aarch64 give identical outputs (sha256 `3f5d4792044d9e0b`).
- **Against MPFR at 1,024 bits:** every one of the 2,768,534 rows is correctly rounded, max 0.5 ulp and none off, in
  all thirteen sets, the accurate paths alone and the subnormal results (in units of 2⁻¹⁰⁷⁴) included.

**The reference rows.** A new file, `special.ref.tsv`, and the harness's comparator for it (port plan §5 step 7's
deferred `*.ref.tsv` comparator).
- **The model.** `validate.py`'s special model now computes each row three ways:
  - through glibc, which must give 1.3.0's recorded bits;
  - on correctly rounded primitives (T0);
  - with the T1 functions landed so far at their true values (MPFR's erf and erfc), which is what the port now
    computes.
- **Which file a row goes to:** a row whose port value differs from its recorded bits goes to `special.cr.tsv` when
  the two models agree, since only a primitive's rounding moved it. Otherwise it goes to `special.ref.tsv`, since a
  T1 function, or Rust's formula on one, moved it.
- **The harness** (`tests/parity_core.cyr`) loads `<module>.ref.tsv` as a third table, tagged 9. It compares under
  `_par_c_bound`, which for a T1 function will be ADR 009's bound and is 0 for erf and erfc, and reports each file
  apart.

**The rows that move:**

| Function | Rows moved to `special.ref.tsv` | Max distance from 1.3.0 |
|---|---|---|
| `erf` | 226 of 6,246: 1.3.0's fdlibm not correctly rounded | 1 ulp |
| `erfc` | 848 of 6,246 | 2 ulp |
| `normal_cdf` | 1,110 of 7,899: Rust's formula on the true erfc | 2 ulp |
| `normal_sf` | 1,133 of 7,899 | 2 ulp |
| `gamma_inc` | 229 of 5,971: the Wilson–Hilferty rows, through Φ | 2 ulp |

`special.cr.tsv` falls from 226 rows to 221, because 5 of its rows are now reference rows: an erfc, a Φ and a 1 − Φ
row from part a1, and two Wilson–Hilferty `gamma_inc` rows. a1's other 7 exp rows stay there: on those, fdlibm on a
correctly rounded exp already gives the true erfc.

**The result.** Every row matches: erf and erfc at 0 ulp against MPFR on all 12,492 of their rows, and every
downstream row at 0 ulp against Rust's formula on them. The replay is nine modules: 129,600 rows ok and 3,873 accepted
(3,843 under ADR 9, of which 3,546 are reference rows, and 30 under ADR 7), warning-free, on x86_64 ELF, PE (wine)
and aarch64 (qemu). Every `math::` result line is identical on the three.

**Mutation checks:**

| Mutant | Caught by |
|---|---|
| erfc never takes its accurate path | the replay: 343 rows (erfc 82, Φ 124, 1 − Φ 127, gamma_inc 10) |
| the fma as a·b + c, two roundings | the replay: 1,386 rows |
| one bit of one reference row | the replay: that row |
| a reference row no golden row has | the replay: "a correction matches no golden row" |
| erf never takes its accurate path | not the replay (no golden erf row needs it); `crverify`'s comparison with CORE-MATH: 32,658 of its worst-case rows |

The golden grid tests T1 against the truth on M2's arguments. The hard-to-round arguments are `crverify`'s, which
holds CORE-MATH's worst-case lists.

**Tests.** `tests/special.tcyr` grows to 79 checks:
- erf at a worst case its fast path can't round (0x1.0380ebd7a8f91p-61);
- erfc beside 1, at ±0x1.c5bf891b4ef6bp-55;
- erfc's subnormal exception;
- its underflow edge: +0 from 0x1.b39dc41e48bfdp+4, and the smallest subnormal just below.

Two a1 messages that described fdlibm's mechanics now describe the result.

**Cost** (`docs/benchmarks-rust-v-cyrius.md`): erf and erfc cost 542 ns to 2.4 µs a call. That is 28–157× Rust's
fdlibm and 4–16× the T0 port's, and is the price of correctly rounded results without a hardware fma (P9 measured the
same; ledger L12). Φ, 1 − Φ and gamma_inc's Wilson–Hilferty branch inherit it until part b2.
- **The digests:** the bench now prints an FNV-1a digest of each row's results' bits beside its sum, since the sum
  could not see a 1-ulp change. All 26 rows' digests are explained by `validate.py`'s models, Rust's through glibc
  and the port's through the reference.
- **The correction:** the earlier parts' "sums agree" were weaker than they read.

### 8.6 Part b2: Φ and 1 − Φ within 1 ulp, T1 (2026-11-16)

**The module.** `normal_cdf` and `normal_sf` are now probe P9's T1 form (§6.8), on part b1's CORE-MATH erfc:
- **The argument** −z/√2 is carried as a double-double h + l.
- **The correction:** the low part's first-order correction ρ = −l·(2/√π)·e^(−h²)/erfc(h) is applied to CORE-MATH's
  unrounded double-double erfc(h).
- **One rounding:** the ½ is folded into the single rounding, on the subnormal grid too.
- **The bound:** proven within 0.5 + 2⁻⁷ ulp, against ADR 009's 1.
- **1 − Φ(z) is Φ(−z)** bit for bit, as in 1.3.0, with a NaN returned as it came.

**Where the code is:**
- **The helpers** that reach into erfc's internals are in `src/crerf.cyr`, beside the port they extend. That covers
  the double-double core with its 2⁻⁶⁰ fast-path test, the asymptotic path without its final rounding, the scaled
  rounding, and the fixed-result thresholds.
- **`_pmn_normal_cdf` and `_pmn_normal_sf`** in `src/special.cyr` call them.
- **The double-double 1/√2** is pramana's own constant. `scripts/crverify/consts.py`, now also checking
  double-doubles and `src/crerf.cyr`, verifies both words against Python's 90-digit decimal, and
  `vendor_coremath.py --check` leaves those two lines to it.

**The fast bound is 2⁻⁶⁰,** as P9 verified it. P9 found 2⁻⁵⁶ about 30% faster in the core, within 0.5 + 2⁻³ ulp.
2⁻⁶⁰ is kept because it is the configuration verified on 1.2 million rows, it is correctly rounded nearly
everywhere, and speed is measured here, not gated. On `crverify`'s 1,146,809 arguments, a 2⁻⁴⁰ bound changes 5
values, each from the correctly rounded value to the neighbour 0.5001 ulp away. The constant's comment records the
option.

**Verification** (`scripts/crverify/run.sh ncdf`, a new set: P9's nine seeded T1 sets, 1,146,809 arguments):
- **Against P9:** this copy's output equals P9's recorded T1 output byte for byte.
- **On three targets:** the outputs are identical (sha256 `5e37569875c320a4`).
- **Against MPFR's ½·erfc(−z/√2) at 1,024 bits:** max 0.5 ulp in all nine sets, and correctly rounded on all but
  4 rows: one in the core, one in (−4.1, −2.5) and two at the branch points, as P9 found. The bound is 1 ulp.

**The reference rows.** `mpfr_ref.py` gains the true Φ and 1 − Φ: ½·erfc(∓z/√2) at 1,024 bits over MPFR's widest
exponent range, rounded once to binary64. The model's Φ is 1.3.0's formula in its glibc and correctly rounded modes,
and MPFR's true value in its reference mode, and the gamma_inc Wilson–Hilferty branch takes Φ from it too.

`special.ref.tsv` grows to 7,158 rows:

| Function | Reference rows |
|---|---|
| `normal_cdf` | 2,781 of 7,899: 1.3.0's value is not the correctly rounded Φ |
| `normal_sf` | 2,793 of 7,899 |
| `gamma_inc` | 510, the Wilson–Hilferty rows, whose outputs are Φ and 1 − Φ |
| erf, erfc | 226 and 848, unchanged from b1 |

`special.cr.tsv` falls to 213 rows: part a1's last 6 Φ and 1 − Φ rows there, and 2 Wilson–Hilferty `gamma_inc` rows,
are now reference rows.

**The result.** `_par_c_bound` gives Φ and 1 − Φ ADR 009's 1 ulp. Every one of their 15,798 golden rows is at 0 ulp
from the true value: on the golden grid, T1's Φ is correctly rounded. The Wilson–Hilferty rows match at 0. The replay
is nine modules: 125,996 rows ok and 7,477 accepted (7,447 under ADR 9, of which 7,158 are reference rows, and 30
under ADR 7), warning-free, on x86_64 ELF, PE (wine) and aarch64 (qemu). Every `math::` result line is identical on
the three.

**Mutation checks:**

| Mutant | The replay |
|---|---|
| no correction for l (ρ = 0, Rust's rounded argument) | 4,799 rows fail; up to 759 ulp |
| 1/√2's low word dropped | 5,266 rows fail; up to 840 ulp |
| 1 − Φ as 1 − Φ(z), by subtraction | 2,891 rows fail |
| the ½ as a second rounding (§3.4's sketch) | passes: 30 rows at 1 ulp, within ADR 009's bound, which the histogram shows |
| the fast bound 2⁻⁴⁰ for 2⁻⁶⁰ | passes, all at 0 ulp: the bound is the proof's margin; on `crverify`'s rows it misrounds 5 |

The gate is the ADR's bound, so a change that stays within 1 ulp passes. The histogram of ulp distances, printed per
function, shows each move to 1.

**Tests.** `tests/special.tcyr` grows to 87 checks:
- Φ at −1, −6 and −37 to MPFR's correctly rounded values (1.3.0's glibc-based Φ(−1) is 1 ulp off);
- Φ(−38.4), a subnormal 13·2⁻¹⁰⁷⁴ from the folded ½;
- a point where the fast path can't prove the rounding;
- Φ(±0) = ½ exactly;
- 1 − Φ(z) = Φ(−z) bit for bit over 400 points;
- a NaN returned as it came.

1.3.0's own `normal_cdf_tails` test still passes within its tolerances.

**Cost** (`docs/benchmarks-rust-v-cyrius.md`): 1.37–1.40 µs in the core and 2.44–2.53 µs in the tails, 70–106× Rust's
Φ. That is 8–30% over Rust's formula on b1's correctly rounded erfc: the argument's double-double, the correction's
exp, and CORE-MATH's accurate path where its fast bound misses 2⁻⁶⁰. P9 measured the same.

### 8.7 Part b3: lnΓ within 2 ulp, T1 (2026-11-16)

**The module.** `ln_gamma` is now probe P9's T1 form (§6.4), in `src/lgamma.cyr`: fdlibm's `e_lgamma_r` layout,
with kimiya's double-double ln core for every logarithm, double-double products, an exact reflection for x < 0, and
one final rounding.
- **The coefficients** are generated into `src/lgamma_data.cyr` by a new `scripts/vendor_lgamma.py`, from the libm
  crate 0.2.16's `lgamma_r.rs` in the cargo registry, pinned by sha256. Each decimal literal is checked against the
  bits in its comment. `--check`, now in CI, also proves the port's inline words: each 16-digit word is a power of two
  its comment cites, and each high-word threshold occurs in the source, verbatim or as libm's
  `(0x3ff - 70) << 20`.
- **1.3.0's Lanczos form,** which part a2 proved at 0 ulp, is retired from `src/`. lnΓ's edges change as part a2
  recorded they would:
  - −ln x below 2⁻⁷⁰, where 1.3.0 returned +∞ below about 3.2·10⁻³⁰¹;
  - +∞ at the negative integers, where 1.3.0 gave finite values;
  - +∞ at ±∞, where 1.3.0 gave NaN at +∞.

  `tests/special.tcyr`'s three checks of 1.3.0's edges now check T1's.

**A fix to P9's form.** Past lnΓ's overflow, about 2.556·10³⁰⁵, the double-double product x·(ln x − 1) is +∞ and
its error term makes ∞ − ∞, so P9's form returned NaN. P9's sets stopped at 8·2¹⁰¹⁰, short of the overflow; two
golden rows found it (4.19·10³⁰⁵ and f64::MAX). T1 now returns +∞ once that product overflows. It agrees with the
rounded lnΓ except within about 352 of the binary64 overflow tie, a relative window near 10⁻²⁹⁰.

**Verification** (`scripts/crverify/run.sh lgamma`, a new set). P9's six seeded sets have 400,000
arguments, so this set takes 2.5 times as many, 1,000,000, and carries the large set past the overflow to
f64::MAX. Results against MPFR's lgamma at 1,024 bits:
- **x > 0:** max 1.27 ulp, against ADR 009's 2. By set: [2, 8) 1.265, beside the roots 1.146, small x 0.837, large
  x 0.530, with every overflowed row +∞, as MPFR's is.
- **x < 0:** max 1.26 ulp of max(|lnΓ|, 1), against 2 (`_mixed` rows; 1.05 near the poles, 1.263 across the
  zeros). Plain ulps reach 7,955 near lnΓ's zeros, which is why ADR 009's bound is mixed there.
- **On three targets:** identical outputs (sha256 `b9b77d894d78127f`).

**The harness's mixed bound.** A new hook, `_par_c_mixed`, is 1 for an `ln_gamma` row with x < 0. A row past its
bound in plain ulps then passes if |got − want| ≤ bound·2⁻⁵², where |want| < 1. The histogram keeps the plain
distance, so these rows stay visible. `_par_c_bound` gives `ln_gamma` ADR 009's 2.

**Downstream rows: your decision, and ADR 009 amended.** lnΓ is the first T1 function that isn't correctly rounded.
Fed T1's lnΓ, 1.3.0's formulas land up to 1,024 ulp from the same formulas fed the true lnΓ:
- Stirling's remainder: 1,024;
- the kernel and `beta_inc`: 64;
- `gamma_inc`: 12;
- ln B: 8.

Gating those against the true values would have needed bounds that loose. You chose (2026-11-16) to gate a
downstream row on Rust's formula fed the T1 function's own value, at 0 ulp, with each T1 function gated against the
truth at its bound, and ADR 009's Consequences now say so.
- **The model:** `validate.py` carries `t1_ln_gamma`, src/lgamma.cyr in its operation order. It uses exact
  double-double steps (`math.fma`), MPFR's double-double ln (`mpfr_ref.ln_dd`) in place of kimiya's ln core, which
  is within about 2⁻⁸⁰, and libm 0.2.16's coefficients read independently.
- **What it shows:** every downstream row matches at 0 ulp on the first run, Stirling, the kernel, ln B, `gamma_inc`,
  `beta_inc` and the wrappers. So the model reproduces the port's lnΓ bit for bit at every argument they pass it.

**The rows.** `mpfr_ref.py` gains `lgamma` (MPFR's, correctly rounded). `special.ref.tsv` grows to 14,299 rows:

| Rows | Count |
|---|---|
| lnΓ at its true value (1.3.0's Lanczos form is not correctly rounded there) | 2,441 of 3,377 |
| downstream of lnΓ: Stirling 328, the kernel 909, ln B 505, `gamma_inc` 1,515, `beta_inc` 1,571, the three wrappers 382 | 5,210 |
| erf, erfc, Φ and 1 − Φ, from parts b1 and b2 | 6,648 |

`special.cr.tsv` falls to 203.

**The result.** Every lnΓ row is within its bound: 3,250 at 0 ulp, 100 at 1 and 13 at 2, and 14 with x < 0 that pass
on the mixed bound, up to 63 plain ulps where lnΓ is small. Every downstream row is at 0. The replay is nine modules:
118,865 rows ok and 14,608 accepted (14,578 under ADR 9, of which 14,299 are reference rows, and 30 under ADR 7),
warning-free, on x86_64 ELF, PE (wine) and aarch64 (qemu). Every `math::` result line is identical on the three.

**Mutation checks** (the replay):

| Mutant | Rows failed |
|---|---|
| logarithms rounded, without their low parts | 1,667 (lnΓ 37, and every downstream function) |
| −½ dropped from the large-x form | 2,692 (lnΓ 1,566) |
| x < 2: −ln x's low part dropped | 5,213 (lnΓ 156) |
| the reflection's r not reduced | 674 (lnΓ 667) |
| the overflow guard dropped | 2 (the two overflow rows, NaN) |
| the [3, 8) product in plain double | 31, all downstream; every lnΓ row stays within its 2-ulp bound |

The last is the case the exact downstream gate exists for: a change T1's own bound allows still shows, wherever its
callers use the value.

**Tests.** `tests/special.tcyr` grows to 92 checks:
- T1's edges against MPFR: −ln(3·10⁻³⁰¹), the poles, ±∞ and NaN, and lnΓ(1) = lnΓ(2) = +0;
- beside the roots, lnΓ(1 + 2⁻³⁰) and lnΓ(2 − 2⁻³⁰) within 2 ulp (1.3.0: about 10⁶ ulp off);
- lnΓ(½) and lnΓ(10) within 2 ulp;
- lnΓ(−2.5) within the mixed bound (it is 3 plain ulps off, inside 2⁻⁵¹);
- lnΓ(2.55·10³⁰⁵) just under the overflow;
- lnΓ(2.56·10³⁰⁵) and lnΓ(f64::MAX) at +∞.

**Cost** (`docs/benchmarks-rust-v-cyrius.md`): 153–179 ns for x > 0 and 555–562 ns reflected, 5.6–11× Rust's
Lanczos form and 12–39% over T0's, as P9 measured (72–192 and 570 ns). Stirling's remainder and the small-shape kernel
cost 26–28% more downstream, and ln B with a small shape 3–7%. The bench's digests show the port's lnΓ equal to the
T1 model on all 12,288 of its arguments.

### 8.8 Part b4: the incomplete gamma within 4 ulp, T1 (2026-11-16)

**The module.** `gamma_inc` up to a = 10⁹ is now probe P9's T1 form (§6.5), in `src/gammainc.cyr`. It keeps Rust's
series and Legendre's continued fraction, with four changes:
- a double-double prefactor;
- double-double loops;
- the split at x = a;
- a direct Q for small shapes.

Rust's Wilson–Hilferty form past 10⁹ stays, on T1's Φ (ADR 009). 1.3.0's double-precision series and continued
fraction, which part a3 proved at 0 ulp, are retired.
- **The double-double toolkit** (add, multiply, divide, the logarithm, expm1 and the scaling) moved from the two
  modules into `src/dd.cyr`. P9 (§6.5) put it in `src/crmath.cyr`, but that file is kimiya's, vendored and
  regenerated. `src/lgamma.cyr` now uses it, and its `crverify` output is unchanged (sha256 `b9b77d894d78127f`).
- **The ζ(k) table** below a = 10 is `src/zeta_data.cyr`, written by a new `scripts/gen_zeta.py` from MPFR at 320
  bits. All 262 words equal P9's table. Its `--check`, now in CI, also proves the inline hex words of `src/dd.cyr`
  and `src/gammainc.cyr`: each is a power of two its comment cites.
- **Against P9:** the port's output on P9's five seeded sets equalled P9's own T1 rows byte for byte, all 120,000 of
  them, until the first fix below.

**Three fixes to P9's form**, each found past P9's sets, which covered a ∈ [10⁻³, 10⁵] with x up to 50a:
- **The series stops on a bound of its tail.** P9 stopped once a term fell under 2⁻⁶² of the sum. Near x = a the
  terms fall by a ratio r = x/(a + n + 1) close to 1, so the dropped tail is about the last term times 1/(1 − r),
  some √a/9 terms' worth. That is 0.07 ulp at a = 10⁵ but 7 ulp at 10⁹: `crverify`'s new large-shape set measured
  P at 6.96 ulp near a = 9.8·10⁸. The test is now tₙ·x < 2⁻⁶²·S·(a + n + 1 − x), which costs about 9% more terms
  near x = a. It moved 95 of P9's 120,000 rows, each closer to the truth.
- **Below a = 2⁻⁵¹²,** the series' Σ/a is formed as Σ/(a·2⁶⁰⁰), and the 600 is taken back in the final scaling,
  both exactly. Σ/a passes 2⁹⁹⁶ near a = 2⁻⁹⁹⁵, where Dekker's split overflows, so P(10⁻³⁰⁰, 0.5) was NaN.
- **Past x = 2⁹⁹⁰ the result is (1, 0).** From a = 10, the kernel's (x − a)/a and a·φ(u) overflowed the split in the
  same way, so P(30, f64::MAX), a golden row, was NaN. (1, 0) is exactly what the underflowed prefactor gives below
  the guard.

**The reference** (`mpfr_ref.gamma_pq`, and `check.c`'s `gref`, which it follows) is P9's 320-bit series and
continued fraction, with three changes:
- **The series below x = 1 as well as below x = a.** The continued fraction converges too slowly there for small
  shapes, and in practice never for a tiny shape with a small x: `validate.py`'s first run spent an hour on six such
  rows before it was stopped.
- **More precision for tiny shapes.** Below a = 2⁻¹⁰, Q = 1 − P loses about log₂(1/a) bits, so the working precision
  grows by that, plus 64.
- **A NaN result now fails.** `check.c` had skipped a `gref` row whose result was NaN; it now counts it as ∞ ulp, as
  its `cr:` rows do.

A large shape takes up to 10 s a pair through ctypes, so `validate.py` computes the 5,252 golden pairs in a process
pool. It caches them in `build/golden/`, keyed by a hash of the reference's source, and appends each pair as it lands.

**Verification** (`scripts/crverify/run.sh gamma`, a new set): 148,000 rows against `gref`.
- **The sets:** P9's five sets (120,000 rows), then two sets beyond them: 8,000 rows with a ∈ [10⁵, 10⁹], and 20,000
  with a and x down to the least subnormal.
- **Pairs stay together:** a pair's P and Q rows now go to one chunk, so each reference is computed once.
- **The tiny set reaches the overflow:** without the 2⁻⁵¹² scaling, 686 of its rows are NaN.

| Set | P, max (mean) | Q, max (mean) | Not correctly rounded |
|---|---|---|---|
| x near a, a ∈ [10⁻³, 10⁵] | 0.59 (0.250) | 0.60 (0.246) | 0.07%, 0.08% |
| lower tail | 1.00 (0.162) | 0.50 (0.120) | 0.11%, 0% |
| upper tail | 0.50 (0.127) | 0.59 (0.209) | 0.02%, 0.14% |
| small shapes, a ∈ [10⁻³, 1] | 0.50 (0.244) | 0.50 (0.248) | 0%, 0.01% |
| the switches | 0.58 (0.249) | 0.55 (0.251) | 0.09%, 0.30% |
| a ∈ [10⁵, 10⁹] | 0.50 (0.122) | 0.50 (0.126) | 0.05%, 0.05% |
| a and x down to 2⁻¹⁰⁷⁴ | 0.50 (0.012) | 2 units of 2⁻¹⁰⁷⁴ (0.256) | 0%, 1.65% |

- **The worst row** is 1.0 ulp: a P near 10⁻³⁰⁷, where the final scaling rounds a second time, as P9 found.
- **Against ADR 009:** within its 4 ulp on the committed domain, and within 1 ulp beyond it.
- **On three targets:** identical outputs (sha256 `0714f60026574372`).

**The model and the rows.** `validate.py` carries `t1_gamma_inc`, src/gammainc.cyr step for step:
- **Its steps:** the double-double steps are the port's, now on kimiya's Dekker product rather than `math.fma`.
  b3's lnΓ model uses that product too, and every downstream lnΓ row still matches.
- **Its cores:** kimiya's ln and exp cores are MPFR's double-doubles of the same values, with the exp core's
  exponent taken as the port takes it.
- **Its constants:** γ and ζ(k) are MPFR's.
- **The result:** it equals the port bit for bit on all 3,799 golden rows with a ≤ 10⁵, tiny shapes included.

The rows:
- `gamma_inc`'s own rows expect the true P and Q;
- the two wrappers, downstream, expect Rust's projection of T1's value, at 0 ulp (ADR 009 as amended at b3);
- the Wilson–Hilferty rows and the edges keep Rust's values.

`special.ref.tsv` grows to 17,230 rows. `gamma_inc` has 4,179 of them: 3,669 at the true P and Q, and part b2's 510
Wilson–Hilferty rows. The wrappers have 534 (lower 224, upper 310). `special.cr.tsv` falls to 172; its one `gamma_inc`
row is a Wilson–Hilferty row. The ref file's note for a downstream row, and its header, now say "the T1 functions'
values as the port computes them"; b3's amendment had left them saying "true values".

**The harness.**
- **`_par_c_gamma_bound`:** ADR 009's 4 ulp for a ∈ [10⁻³, 10⁵], and 0 for the edges and past a = 10⁹.
- **`_par_c_vs_rust`, a new hook,** implements ADR 009's rule outside the domain, "no row less accurate than Rust",
  row by row. For a below 10⁻³, or a in (10⁵, 10⁹], each output's bound is 1.3.0's own distance from the truth,
  read from the alternate list. The driver's new `_par_has_alt` says when that list holds this row's 1.3.0 outputs.
  A row with no reference entry has bound 0, since 1.3.0 is the truth there.

**The result.** The port is within 1 ulp of the truth on every golden row, and never further from it than 1.3.0:

| Shapes | Rows | Port, max | 1.3.0, max | Port worse than 1.3.0 |
|---|---|---|---|---|
| a ∈ [10⁻³, 10⁵] | 3,697 | 1 ulp | 872,342 ulp | 0 outputs |
| a ∈ (10⁵, 10⁹] | 1,479 | 1 ulp | 72,131 ulp | 0 |
| a < 10⁻³ | 76 | 1 ulp | far off (wrong values) | 0 |

Of `gamma_inc`'s 11,942 outputs, 11,928 are at 0 ulp and 14 at 1; the wrappers' 800 rows are all at 0. The replay is
nine modules: 115,965 rows ok and 17,508 accepted (17,478 under ADR 9, of which 17,230 are reference rows, and 30
under ADR 7), warning-free, on x86_64 ELF, PE (wine) and aarch64 (qemu). Every `math::` result line is identical on the
three.

**Mutation checks** (the replay):

| Mutant | Rows failed |
|---|---|
| the series' terms in plain double | 30 (`gamma_inc` 2, the wrappers 28) |
| Q as 1 − P for small shapes | 22 |
| ζ(k)'s low words dropped | 3, the wrappers only |
| γ's low word dropped | 2, the wrappers only |
| the continued fraction stopped at 2⁻⁴⁰ | 1,481 |
| the tiny-shape scaling dropped | 9 (NaN) |
| the huge-x guard dropped | 3 (NaN) |
| the prefactor's lnΓ low part dropped (a < 10) | 158 |
| Rust's split at x = a + 1 | 0: within the bound, as the double-double complement doesn't cancel |
| ln(1 + u) as ln x − ln a for \|u\| ≤ ¼ too | 0: within the bound |
| P9's last-term stop | 0 in the replay, where 1.3.0 is further off still; `crverify` measures it at 6.96 ulp, and a new unit test fails |

The exact downstream gate catches three of these alone: ζ's and γ's low words, and part of the plain-double series.

**Tests.** `tests/special.tcyr` grows to 105 checks:
- P(1, x) and Q(1, x) against the correctly rounded 1 − e⁻ˣ and e⁻ˣ within 4 ulp, from x = 0.05 to 300. Part a3 had
  checked 10⁻¹³ relative.
- 13 points against MPFR within 4 ulp:
  - the deep lower tail, P(38, 4.6·10⁻⁷) = 2.93·10⁻²⁸⁶;
  - a subnormal P;
  - Q directly at a = 0.01, x = 1.05;
  - erf and erfc of √1.09;
  - a = 10⁻³ at x = 60;
  - a = 10⁵ and 10⁹ at x = a;
  - P9's 7-ulp large-shape point;
  - three tiny and two subnormal shapes.

**Cost** (`docs/benchmarks-rust-v-cyrius.md`): 7.24–7.50 µs a call on small shapes and 5.82–5.97 µs on large ones,
48–61× Rust and 10–13× T0, as P9 measured (10–17×). The bench's digests show the port equal to the T1 model on all
8,192 arguments of its two T1 rows.

### 8.9 Part b5: the incomplete beta within 8 ulp, T1 (2026-11-16)

**The module.** `beta_inc` up to max(a, b) = 10⁹ is now probe P9's T1 form (§6.6), in `src/betainc.cyr`. It keeps
Rust's continued fraction on the side x < (a + 1)/(a + b + 2), with four changes:
- the smaller of x and y is taken as exact, and the other is recomputed as 1 − it in double-double;
- a double-double prefactor;
- a double-double continued fraction, stopped at 2⁻⁶²;
- TOMS 708's power series for a complement below 2⁻⁶.

It reuses `src/dd.cyr`, gammainc's lnΓ below 20 and the ζ table. P9's own Stirling double-double for arguments of 20
and up comes with it. 1.3.0's double-precision fraction, which part a4 proved at 0 ulp, is retired below 10⁹.
- **Past 10⁹, Rust's form stays, as for `gamma_inc`.** Near the switch the fraction's cost grows with the shapes: 5 ms
  a call at a = b = 10⁹, 45 ms at 10¹², 130 ms at 10¹⁵. Beyond about 10¹⁵⁴ its products overflow. 1.3.0 has the same
  limits, which its 10⁶-step budget bounds. Keeping its form there satisfies ADR 009's "no row less accurate than
  Rust" as it stands, and keeps P9's cap, 4000 + 200·⌊√max(a, b)⌋, inside an integer: past about 10³⁸ `f64_to`
  overflows, to INT64_MIN on x86 and to INT64_MAX on aarch64.
- **An infinite shape returns (NaN, NaN) before the loop.** That is 1.3.0's value, which it reached only after running
  its budget on a NaN prefactor (part a4's finding).
- **Against P9:** the port's output on P9's five sets equalled P9's own T1 rows byte for byte, all 103,372 of them,
  until the fixes below. After them, 3 of the 103,372 differ, and all 3 moved closer to the truth: the `near` set's
  share not correctly rounded fell from 0.039% to 0.024%.

**Five fixes to P9's form**, each found past P9's sets (a and b in [10⁻³, 10⁵], y = 1 − x):
- **The power series' ln(a·B(a, b)) cancelled.** P9 formed it as lnΓ(a) + ln a + lnΓ(b) − lnΓ(a + b). With two tiny
  shapes, 1 − I ≈ a/(a + b), and the a/b enters through lnΓ(b) − lnΓ(a + b) ≈ −a/b, which the difference of two
  double-doubles near 662 loses entirely: at a = 3.8·10⁻³²¹, b = 1.6·10⁻²⁸⁸ P9 gave 1 − I = 6.7·10⁻³¹⁹ for
  2.4·10⁻³³. With a large b it loses the logarithms' 2⁻⁸⁰·b ln b instead: 118 ulp at a = 0.0035, b = 6.1·10⁸.
  The port takes ln(aB) = lnΓ(1 + a) − [lnΓ(a + b) − lnΓ(b)], with lnΓ(1 + a) from the ζ series and the difference in
  a form whose every term is proportional to a:
  - from b = 20, Stirling's: (b − ½)·ln(1 + a/b) + a·(ln(a + b) − 1) − [st(b) − st(a + b)], each of Stirling's terms
    c/zⁿ contributing c·a·r·q·Σ rʲqⁿ⁻¹⁻ʲ (r = 1/b, q = 1/(a + b));
  - below 20 (a < 1), a shift by m = ⌊b⌋ − 1 into [1, 3), the ζ series' difference term by term through
    Dₖ = y₂ᵏ − y₁ᵏ = y₂·Dₖ₋₁ + a·y₁ᵏ⁻¹, and ln(1 + a/(b − i)) for each step of the shift.
- **A subnormal numerator kept 27 bits of its quotient.** The double-double a/b takes its correction from the
  remainder a − q·b, near 2⁻⁵³·a, which Dekker's product no longer gives exactly below 2⁻¹⁰²². A numerator below
  2⁻⁵¹² is now scaled by 2⁶⁰⁰ first. Found at a = 9·10⁻³¹⁶, b = 6·10⁻²⁶⁴.
- **The prefactor cancelled for large shapes.** P9's M = a ln x + b ln y − ln B(a, b) − ln a cancels from about a + b
  down to O(1) near the mean, leaving the logarithms' 2⁻⁸⁰ relative error: 2 ulp in I near a = b = 10⁹. For two
  shapes of 20 or more the port takes TOMS 708's brcomp form, which is also 1.3.0's for two shapes of 10 or more,
  here in double-double: with λ = bx − ay, M = −a·φ(λ/a) − b·φ(−λ/b) + ½ ln(b/(a(a + b))) − ln √(2π) − [st(a) +
  st(b) − st(a + b)], φ(u) = u − ln(1 + u) by its series for |u| ≤ ¼.
- **ln B cancelled with one large shape.** With one shape q of 20 or more and p below, ln B = lnΓ(p) − [lnΓ(p + q) −
  lnΓ(q)] by the Stirling difference above, where P9's lnΓ(q) − lnΓ(a + b) cancelled from q ln q: 3 ulp in 1 − I at
  a = 0.0016, b = 7.7·10⁸.
- **The side was chosen on the caller's x.** T1 computes with x = 1 − y when y is the smaller, but P9 tested the
  caller's x against the switch. Wherever y = 1 − x the two agree, so P9's sets never saw it. On 1.3.0's golden rows
  that pass a y that isn't, beta_inc(2, 3, 0.3, 2⁻¹⁰⁷⁴), it ran the fraction at x ≈ 1 on the wrong side and returned
  (0, 1) for (1, 0). The parity replay found it.

**The reference** (`mpfr_ref.beta_ic`, and `check.c`'s `bref`, which it follows) is P9's continued fraction, with three
changes:
- **x and y are held exactly** (1,200 bits): 1 − x needs up to 1,075 bits for a subnormal x.
- **More precision for a tiny complement.** At P9's fixed 320 bits, 1 − V keeps ~270 bits on every row P9 makes, but
  none for a tiny shape, whose complement can be 2⁻¹⁰⁰⁰. When the complement is below 2^-(prec − 330), prec grows by
  its binary exponent plus 64 and the side is recomputed; the stop tightens with it, to 2^-(prec − 10). A complement
  still below 2⁻¹²⁰⁰ at 1,600 bits rounds to 0.
- **A NaN or infinite result now fails.** `check.c` had skipped a `bref` row whose result was NaN.

Against the exact rationals on §3.2's 4,000 integer-shape rows (`p9_beta_exact.py`), `bref` now agrees to
2.3×10⁻⁹⁷ ulp (P9: 4×10⁻⁷⁸). `validate.py` computes the golden inputs' references in a process pool and caches them in
`build/golden/`, as for `gamma_inc`: 5,714 inputs in 50 s.

**Verification** (`scripts/crverify/run.sh beta`, a new set): 140,098 rows against `bref`.
- **The sets:** P9's five, reproduced from its seed and order: §3.2's 4,000 integer-shape rows, drawn as
  `scripts/port-baseline/main.rs` draws them, and its four seeded sets (103,372 rows). Then five sets beyond them:
  - 2,000 draws with the larger shape in [10⁵, 10⁹] (`bigab`);
  - 2,000 with both shapes in [10⁸, 10⁹] at μ ± 3σ, where the prefactor cancels most (`bigboth`);
  - 500 pairs with both shapes in [10⁵, 10⁹] at the switch and its two neighbours, where the fraction converges
    slowest (`bigswitch`);
  - 4,000 with a shape from 2⁻¹⁰⁷⁴ to 10⁻³ (`tiny`);
  - 1,000 pairs over the domain at x = 2⁻¹⁰⁷⁴, 2⁻¹⁰²², 2⁻⁵³ and ½, at y = 2⁻¹⁰⁷⁴ and 2⁻⁵³ given exactly, and at the
    switch and its neighbours (`edges`).
- **An input's two rows stay together,** so `bref` runs once per input.

| Set | I, max (mean) | 1 − I, max (mean) | Not correctly rounded |
|---|---|---|---|
| §3.2's 4,000 integer-shape rows | 0.50 (0.218) | 0.50 (0.221) | 0%, 0.03% |
| x = μ ± 5σ, a, b ∈ [10⁻³, 10⁵] | 0.50 (0.248) | 0.50 (0.250) | 0.02%, 0.02% |
| lower tail | 0.50 (0.150) | 0.50 (0.112) | 0%, 0% |
| upper tail | 0.50 (0.113) | 0.50 (0.156) | 0%, 0% |
| one small shape | 0.50 (0.173) | 0.50 (0.177) | 0%, 0% |
| at the switch | 0.51 (0.250) | 0.50 (0.246) | 0.14%, 0.04% |
| the larger shape in [10⁵, 10⁹] | 0.50 (0.145) | 0.50 (0.137) | 0%, 0% |
| both in [10⁸, 10⁹], μ ± 3σ | 0.50 (0.254) | 0.50 (0.253) | 0.10%, 0% |
| the switch, both in [10⁵, 10⁹] | 0.50 (0.251) | 0.50 (0.251) | 0%, 0% |
| a shape from 2⁻¹⁰⁷⁴ to 10⁻³ | 1 unit of 2⁻¹⁰⁷⁴ (0.139) | 1 unit of 2⁻¹⁰⁷⁴ (0.138) | 0.10%, 0.03% |
| x and y at 2⁻¹⁰⁷⁴, 2⁻¹⁰²², 2⁻⁵³, the switch | 0.50 (0.147) | 0.50 (0.141) | 0.01%, 0.04% |

- **The worst rows** are two subnormal results, one unit of 2⁻¹⁰⁷⁴ each: I at a = 34, b = 9.9·10⁻³¹⁶, and 1 − I at
  a = 2⁻¹⁰⁷⁴, b = 10⁻¹³. Elsewhere the maximum is 0.5064 ulp, P9's own worst row at the switch.
- **Against ADR 009:** within its 8 ulp on the committed domain, and within 1 ulp beyond it, up to 10⁹.
- **On three targets:** identical outputs (sha256 `b76122f13845ebdd`).
- **Against P9:** 3 of P9's 103,372 rows differ from its own T1 rows, and all 3 are now correctly rounded.

**The model and the rows.** `validate.py` carries `t1_beta_inc`, src/betainc.cyr step for step on b4's double-double
helpers, with kimiya's ln and exp cores as MPFR's double-doubles. It equals the port bit for bit on all 6,219 golden
inputs in T1's range: I, 1 − I and the wrapper's value.
- `beta_inc`'s own rows expect the true I and 1 − I: 4,834 rows in `special.ref.tsv`. Two more are in
  `special.cr.tsv`, where the truth happens to equal Rust's form on correctly rounded primitives;
- the wrapper, downstream, expects Rust's projection of T1's value, at 0 ulp: 243 rows;
- the edges, the infinite shapes and the three f64::MAX rows past 10⁹ keep Rust's values.

`special.ref.tsv` grows to 20,621 rows; `special.cr.tsv` falls to 91, as 74 `beta_inc` rows and the wrapper's 7 leave
it for the truth.

**The harness.**
- **`_par_c_beta_bound`:** ADR 009's 8 ulp for a and b in [10⁻³, 10⁵] with x, y > 0, and 0 for the edges, an
  infinite shape and the rows past 10⁹.
- **`_par_c_vs_rust`** now covers `beta_inc` too: for a shape below 10⁻³, or in (10⁵, 10⁹], each output's bound is
  1.3.0's own distance from the truth.
- **It found the fifth fix:** the first replay failed 2 rows, 1.3.0's beta_inc(2, 3, 0.3, y) with y = 2.2·10⁻³⁰⁸ and
  2⁻¹⁰⁷⁴, where the port gave (0, 1).

**The result.** The port is within 1 ulp of the truth on every golden row, and never further from it than 1.3.0:

| Shapes | Rows | Port, max | 1.3.0, max | Port worse than 1.3.0 |
|---|---|---|---|---|
| a, b ∈ [10⁻³, 10⁵] | 5,388 | 1 ulp | 59,059 ulp where y = 1 − x | 0 outputs |
| larger, up to 10⁹ | 258 | 0 | 151,644 ulp | 0 |
| a shape below 10⁻³ | 68 | 0 | wrong values, and 4 NaN outputs | 0 |

On the 15 rows that pass a y other than 1 − x, 1.3.0 answers for the x it is given and T1 for 1 − y, the smaller
being exact; the reference follows T1. Of `beta_inc`'s 11,706 outputs, 11,703 are at 0 ulp and 3 at 1; the wrapper's 407 rows are all at 0. The replay is nine
modules: 112,655 rows ok and 20,818 accepted (20,788 under ADR 9, of which 20,621 are reference rows, and 30 under
ADR 7), warning-free, on x86_64 ELF, PE (wine) and aarch64 (qemu). Every `math::` result line is identical on the three.

**Past 10⁹** (a finding, for 2.1). Rust's form, which stays there by the maintainer's choice, is far from the truth
near the mean, where T1 is exact:

| a, b, x | Rust's form, I and 1 − I | T1 |
|---|---|---|
| 5, 2·10⁹, 2.4·10⁻⁹ | 45, 90 ulp | 0, 0 |
| 2·10⁹, 3·10⁹, 0.40001 | 5,631, 45,046 ulp | 0, 0 |
| 10¹⁰, 10¹⁰, 0.500003 | 65,139, 260,560 ulp | 0, 0 |
| 10¹¹, 3·10¹¹, 0.250001 | 104,619, 836,959 ulp | 0, 0 |

Extending T1 would cost up to 45 ms a call near the switch at 10¹² and 130 ms at 10¹⁵ (ADR 009's rule holds either way).

**Mutation checks** (`build/handoff/mut_b5.py`: the replay, `tests/special.tcyr`, and the `crverify` rows each mutant
moves, measured against MPFR when the first two pass):

| Mutant | Replay rows failed | Unit checks failed | `crverify`: rows moved, worst |
|---|---|---|---|
| the fraction stopped at 2⁻⁴⁰ | 2,460 | 4 | 53,193 |
| the prefactor's low part dropped | 1,227 | 7 | 44,305 |
| x and y as given, neither exact | 1,392 | 9 | 33,915 |
| P9's switch test, on the caller's x | 2 | 1 | 0 |
| the power series dropped | 6 | 1 | 781, all bits lost |
| lnΓ's first-order zl·ψ dropped | 291 | 0 | 12,698 |
| ln(aB) as P9's sum of lnΓ | 0 | 7 | 1,852, all bits lost |
| lnΓ(1 + a) as lnΓ(a) + ln a | 0 | 2 | 207, 2×10¹⁴ ulp |
| the Stirling difference's st terms dropped | 1,574 | 7 | 37,408 |
| (b − ½)·ln(1 + a/b) directly | 0 | 1 | 2, 18 units of 2⁻¹⁰⁷⁴ |
| ln(1 + a/b) dropped for b < 1 | 33 | 3 | 2,315 |
| the quotient's 2⁶⁰⁰ scaling dropped | 0 | 1 | 28, 4.4×10⁷ ulp |
| the brcomp prefactor dropped | 0 | 1 | 3,072, 50,109 ulp |
| the Stirling difference for ln B dropped (one shape ≥ 20) | 0 | 1 | 8, 3 ulp |
| φ's series dropped | 858 | 3 | 22,011 |
| T1 past 10⁹ when one shape is below it | 0 | 1 | 0 |
| Rust's form from 10⁸ | 0 | 6 | 5,816 |
| the power series below 2⁻¹⁰ only | 0 | 0 | 4, 1 ulp |
| Stirling's correction on the high part (P9's third defect) | 0 | 0 | 1, 1 ulp |
| an infinite shape to Rust's loop | 0 | 0 | 0 |
| the power series' stop without j > 2 | 0 | 0 | 0 |

- **Seventeen of twenty-one fail** the replay or a unit check.
- **Several fixes only show past the golden rows.** Each of those mutants first passed both, and moved `crverify` rows by
  the amounts above. So each fix now has a unit check on the `crverify` row where its mutant is worst.
- **The four survivors:** two cost 1 ulp on the rows they move, within the bound (the margin P9 left); the last two are
  equivalent (the same NaN after Rust's budget, and no row stops on its first two terms). P9's third defect is now
  nearly out of reach, as the power series no longer takes lnΓ(a + b).

**Tests.** `tests/special.tcyr` grows to 126 checks:
- §3.2's worst rows, I_x(36, 15) and 1 − I_x(22, 50), where 1.3.0 is 550 and 583 ulp off, within 8 ulp of MPFR;
- P9's three defects, x and y at 2⁻¹⁰⁷⁴, and a = b = 10⁹, within 8 ulp;
- every b5 fix, within 1 ulp, at the row where its mutant is worst;
- the switch on a y that isn't 1 − x: beta_inc(2, 3, 0.3, 2⁻¹⁰⁷⁴) = (1, 0);
- Rust's form past 10⁹, bit for bit, with both shapes past it and with one;
- an infinite shape gives (NaN, NaN) either way round.

**Cost** (`docs/benchmarks-rust-v-cyrius.md`): 17.78–18.46 µs a call on small shapes and 33.41–36.20 µs on large ones,
both at the mean, where the fraction runs longest. That is 72× and 56× Rust, and 14–15× part a4's T0, within the
10–17× P9 measured. The bench's digests show the port equal to the T1 model on all 8,192 arguments of its two beta
rows. TOMS 708's region-specific expansions (`bgrat`, `basym`) are the candidates for the cost, as P9 noted.

### 8.10 Part c: M2's gate (2026-11-16)

Every part ran the per-part gates (build; test and test --bounds, warning-free; bench, recorded against Rust; vet,
lint, fmt --check; distlib; deps --locked; audit; the ledger; the vendor and constant checks; the three targets). The
plan's §7 adds what M2's gate covers across its parts: §3.2's sets re-measured on the T1 functions, and the rows T1
moves remapped once, before M3 records any row against them. M1's gate (§7.15) adds the audit and the soak.

**§3.2's sets, re-measured.** `scripts/port-baseline/run.sh` now measures the port too:
`scripts/port-baseline/port.cyr` reads the Rust baseline's three outputs and writes them back with the port's
results, so both sides are measured on identical rows, through the same `check.c` paths. Its Rust side reproduces
§3.2's table exactly. Both sides are also measured on all 20,000 incomplete gamma pairs by `gref` (P9's re-measure),
and lnΓ for x < 0 in ADR 009's mixed ulps as well as plain ones. Zero and subnormal results are excluded, as in §3.2.

| Function | Domain | n | Rust 1.3.0, max (mean) | The port, max (mean) | Not CR, Rust → port | Bound |
|---|---|---|---|---|---|---|
| `exp`, `ln`, `ln_1p`, `exp_m1` | §3.2's | 100,000 each | 0.75 (0.21) at worst | 0.50 (0.25) | up to 1.19% → 0% | CR |
| `sin`, `cos`, `tan`, `powf` | §3.2's | 100,000 each | 0.53 (0.25) at worst | 0.50 (0.25) | up to 0.24% → 0% | CR |
| `erf` | [−6, 6] and ±[10⁻³⁰⁰, 1] | 100,000 | 0.822 (0.254) | 0.50 (0.25) | 3.1% → 0% | CR |
| `erfc` | [−6, 27] | 98,623 | 2.49 (0.412) | 0.50 (0.25) | 30.6% → 0% | CR |
| `normal_cdf` | z ∈ [−5, 5] | 50,000 | 29.1 (2.2) | 0.50 (0.25) | 42.4% → 0% | 1 |
| `normal_cdf` | z ∈ [−38, 8] | 49,475 | 1,590 (175) | 0.50 (0.249) | 80.5% → 0% | 1 |
| `ln_gamma` | [0.5, 10] | 100,000 | 1.2×10⁶ (65.2) | 1.25 (0.258) | 93.2% → 5.1% | 2 |
| `ln_gamma` | beside 1 and 2 | 25,000 | 4.3×10¹³ (3.8×10¹¹) | 0.98 (0.27) | 99.98% → 11.3% | 2 |
| `ln_gamma` | [10, 10¹⁵] | 25,000 | 4.85 (0.578) | 0.54 (0.251) | 47.2% → 0.04% | 2 |
| `ln_gamma` | (10⁻³⁰⁰, ½) | 25,000 | 8.21 (0.028) | 0.50 (0.013) | 1.7% → 0.004% | 2 |
| `ln_gamma` | (−20, 0), mixed ulps | 25,000 | 1,583 (5.36) | 0.93 (0.25) | 84.9% → 9.0% | 2, mixed |
| `ln_gamma` | (−20, 0), plain ulps | 25,000 | 6.9×10⁴ (17.7) | 985 (0.57) | as above | — |
| `gamma_inc` P | §3.2's sample | 4,000 | 58.1 (4.87) | 0.503 (0.25) | 79.8% → 0.08% | 4 |
| `gamma_inc` Q | §3.2's sample | 4,000 | 246 (12.2) | 0.506 (0.245) | 76.2% → 0.05% | 4 |
| `gamma_inc` P | all 20,000 pairs, `gref` | 20,000 | 97.0 (4.82) | 0.603 (0.249) | 80.4% → 0.08% | 4 |
| `gamma_inc` Q | all 20,000 pairs, `gref` | 20,000 | 485 (12.4) | 0.640 (0.249) | 76.7% → 0.11% | 4 |
| `beta_inc` I | integer shapes, exact rationals | 4,000 | 550 (14.1) | 0.4999 (0.218) | 57.1% → 0% | 8 |
| `beta_inc` 1 − I | same | 4,000 | 583 (12.8) | 0.50 (0.221) | 56.7% → 0% | 8 |

- **Every set is within its bound**, by margins of 1.6× (lnΓ on [0.5, 10]) to 16× (`beta_inc`).
- **lnΓ's 985 ulp at x = −3.1435 is in plain ulps,** beside a zero of lnΓ, where the reflection's O(1) terms cancel
  to near 0 and no fixed working precision bounds the relative error. That is why ADR 009's bound for x < 0 is in
  ulps of max(|lnΓ|, 1), and in that measure the same set is within 0.93.
- **What a user sees moves by up to thirteen orders of magnitude:** lnΓ beside 1 and 2 from 4.3×10¹³ ulp to under 1.

**The rows T1 moves.** Only `special` calls the T1 functions among M2's golden modules: `bridge` and
`combinatorics` take no part of math.rs, and its other callers, the distributions and the hypothesis tests, are M3 and
M4. Each of `special`'s 58,101 rows is gated, with nothing measured or pending:

| Function | Rows | At 1.3.0's bits | `special.cr.tsv` | `special.ref.tsv`, true value | `special.ref.tsv`, downstream |
|---|---|---|---|---|---|
| `erf`, `erfc` | 12,492 | 11,417 | 1 | 1,074 | — |
| `normal_cdf`, `normal_sf` | 15,798 | 10,224 | — | 5,574 | — |
| `ln_gamma` | 3,377 | 936 | — | 2,441 | — |
| `stirling_error`, `bd0`, `ln_gamma_kernel`, `ln_beta` | 13,403 | 11,574 | 87 | — | 1,742 |
| `gamma_inc` | 5,971 | 1,791 | 1 | 3,669 | 510 (Wilson–Hilferty) |
| the two gamma wrappers | 800 | 266 | — | — | 534 |
| `beta_inc` | 5,853 | 1,017 | 2 | 4,834 | — |
| `regularized_incomplete_beta` | 407 | 164 | — | — | 243 |
| **all** | **58,101** | **37,389** | **91** | **17,592** | **3,029** |

M3's rows are recorded against T1 from their first generation: `validate.py`'s reference model (`Prims("ref")`)
carries every T1 function, the ones that aren't correctly rounded as the port computes them (`t1_ln_gamma`,
`t1_gamma_inc`, `t1_beta_inc`), for the distributions' formulas downstream. So nothing is remapped twice.

**The audit of M2's sources** (`special.cyr`, `crerf.cyr`, `lgamma.cyr`, `dd.cyr`, `gammainc.cyr`, `betainc.cyr`
and their data files), against CLAUDE.md's library rules:
- **No aborts:** no `+?`, `-?` or `*?`, no `assert_fatal`, `panic` or `result_unwrap`, no `vec_get`.
- **No allocation at all:** no `alloc`, freelist, builder or vector; every result is a value or a tuple of values.
- **Every transcendental correctly rounded:** none of the stdlib's or ganita's plain transcendentals; the only
  stdlib float functions are `f64_sqrt`, `f64_floor`, `f64_abs` and the conversions, all exact or correctly rounded.
- **Constants generated or proven:** no hex in a `const`; every inline hex word is proven by `vendor_coremath.py`,
  `vendor_lgamma.py` or `gen_zeta.py`, whose `--check` runs in CI.
- **Names:** every top-level name carries the pmn prefix.
- **`#must_use`:** the T1 helpers lacked it on 58 pure functions (12 of CORE-MATH's port, 3 of lnΓ's, and the
  double-double helpers of `dd.cyr`, `gammainc.cyr` and `betainc.cyr`). Each now has it. The five `crverify`
  generators, which reach every T1 path, compile to byte-identical binaries before and after.

**The soak.** `tests/soak.tcyr` gains a group: 10⁴ rounds of all fourteen of math.rs's functions, across their
branches (lnΓ's reflection, the incomplete gamma's series, fraction, small-shape Q and Wilson–Hilferty form, the
incomplete beta's fraction, power series and Rust's form past 10⁹). The bump heap, the freelist's arena, the mapped
size and the pool are unchanged, and every result is finite, on x86_64 ELF, PE and aarch64. The audit says M2
allocates nothing; the soak confirms it at run time. `tests/soak.tcyr` has 13 checks.

**Surface and Debug.** math.rs is crate-private in 1.3.0, and its functions are private here (`_pmn_`), so the
surface check lists nothing new, and M2 has no type to derive Debug for.

**M2's result.** The special functions are ported twice: T0 in Rust's operation order (parts a1–a4), which proved the
transcription at 0 ulp, then T1 (parts b1–b5) to ADR 009's bounds:

| Function | ADR 009's bound | Measured: `crverify` (rows) | Measured: §3.2's sets |
|---|---|---|---|
| `erf`, `erfc` | correctly rounded | correctly rounded (2,768,534) | correctly rounded |
| `normal_cdf`, `normal_sf` | 1 ulp | 0.5 (1,146,809) | 0.5 |
| `ln_gamma` | 2 ulp; mixed for x < 0 | 1.27; 1.26 mixed (1,000,000) | 1.25; 0.93 mixed |
| `gamma_inc` | 4 ulp on its domain | 1.0 (148,000) | 0.64 |
| `beta_inc` | 8 ulp on its domain | 1.0 (140,098) | 0.50 |

The golden `special` module has 58,101 rows. The replay is nine modules: 112,655 rows ok and 20,818 accepted (20,788
under ADR 9, of which 20,621 are reference rows, and 30 under ADR 7), warning-free, on x86_64 ELF, PE (wine) and
aarch64 (qemu), with every `math::` result line identical on the three. `tests/special.tcyr` has 126 checks. The
ledger has 14 rows, all open, with 18 notes in `src/`. The cost is recorded part by part in
`docs/benchmarks-rust-v-cyrius.md`: T1 runs 5–157× Rust, most of it in double-double arithmetic and the emulated FMA
(ledger L12).

## 9. M3: the distributions

M3 ports 1.3.0's `src/distribution.rs`: the `Distribution` trait and its fourteen implementations, each with its
validating constructor, its five methods, serde's codec and the derived Debug. The parts are listed in plan §7. Each
distribution is a `PmnX` struct filled in caller storage by `PmnX_new(out, …)` (D11), with its methods under Rust's
names, `sample` generic over `R: PmnRng` (D7 as amended). Its golden rows are in a new `dist` module, named by Rust's
path (`Normal::pdf`), with its codec in `json` and its Debug in `debug`. The functions are Rust's in its operation
order, on the correctly rounded primitives and the T1 special functions (ADR 009), so a row downstream of a T1 function
is recorded against T1 from the start, and nothing in M3 is remapped twice.

### 9.1 Part a1: Normal, Box–Muller and Debug's f64 form (2026-11-16)

**The modules.**
- **`src/dist.cyr`:** the `PmnDistribution` trait (`pdf`, `cdf`, `mean`, `variance`, and `sample<R: PmnRng>`), Rust's
  three parameter checks (`require_finite`, `require_positive`, `require_probability`), and `standard_normal`,
  Box–Muller's cosine branch √(−2 ln u1)·cos(2π u2) with u1 kept from 0 by f64::MIN_POSITIVE, generic over the
  generator.
- **`src/dist_cont.cyr`:** `PmnNormal { mean, std_dev }`. `PmnNormal_new` refuses a non-finite mean or a std_dev that
  isn't finite and positive (`PMN_ERR_INVALID_PARAMETER`; Rust's message is dropped, D6), writing nothing then. The
  pdf, (1/(σ√(2π)))·exp(−½z²), and the cdf, Φ((x − μ)/σ) on the T1 Φ, are Rust's in its operation order; a draw is
  μ + σ·Z. `to_json` writes serde_json's `{"mean":M,"std_dev":S}`; `PmnNormal_from_json` reads serde's struct rules
  and then the constructor, whose refusal reads as `PMN_ERR_DESERIALIZE`, as serde_json reports `try_from`'s error.
  `debug` writes the derive's two forms.
- **Debug's f64 form** (D12), `_pmn_dbg_f64` in `src/debug.cyr`: the shortest digits that read back, the closest and
  on an exact tie the upper one, as Rust's core printer chooses (impetus's digit core with `tie_even` 0), in
  pravash's form: positional with at least one fractional digit, exponential for a nonzero |x| below 10⁻⁴ or from
  10¹⁶ with no `+`, and `NaN`, `inf`, `-inf`, `-0.0`. D12 expected pravash's form vendored into `src/fmtf.cyr`, but it
  sits on pravash's own copy of the digit core, generalised for f32: vendoring it would bring a second bignum. So the
  form is moved onto impetus's core, about 40 lines in pramana's own module, and pinned by golden rows against Rust's
  `{:?}` instead.

**The golden rows.**
- **`dist`, a new module, 12,882 rows:**
  - `Normal::new` on every pair of a 16-value grid (the specials, ±1e-300, ±1e300, 0.25, −3.5): the fields, or the
    error; 256 rows, class A;
  - `Normal::pdf` and `Normal::cdf`, 6,249 rows each: N(0, 1) at x = k/16 on [−40, 40]; eight more fixed (μ, σ)
    (among them σ = 1e-300 and 1e300, μ = −0.0 and f64::MAX) at z = k/4 on [−40, 40] and at the specials; 40 seeded
    pairs at z = −40, −37.5, …, 40; 1,000 seeded triples. Class C;
  - `Normal::mean` and `Normal::variance` on the 49 pairs, class A;
  - `Normal::sample`, 27 streams: the plan's 10⁴ draws of N(0, 1) from three seeds, and 256 draws of each other fixed
    pair from the same seeds; `standard_normal`, the oracle's hook, 1,024 draws from three seeds. Class C.
- **`json`**, 231 more rows: 187 Normal documents (`struct_docs`'s variants on each field's 22 number forms, serde's
  sequence form, and 88 seeded documents, half with a negative std_dev) and 44 writes.
- **`debug`**, 25,248 more rows: Normal in both forms at 24 pairs, and `debug::f64`, Rust's `{:?}` of `fmt.tsv`'s
  25,200 doubles: every power of two and of ten with its neighbours, the thresholds, and seeded words.

**The models.** `validate.py` re-derives every row without the oracle:
- **`dist`** three ways, as `special`: through glibc (1.3.0's bits on all 12,882), on correctly rounded primitives,
  and on the T1 functions. `dist.cr.tsv` has 14 rows where glibc's exp, ln or cos misrounds (5 pdf values, and 7
  streams and 2 `standard_normal` streams with a moved draw), and `dist.ref.tsv` 2,699, the cdf rows the T1 Φ moves.
- **Normal's codec** in its serde model: the `Repr`'s fields, then the constructor's test. Ten documents serde_json
  reads only by skipping a member unchecked join `json.map.tsv` under ADR 007 row 16, as M1's did.
- **Debug's f64 form:** Python's `repr` gives the shortest length, and an exact tie, which it breaks to even, is redone
  from the exact value with Rust's upper choice. All 25,200 rows match Rust, ties included.

**The result.** Every row passes on the first replay. The replay is ten modules: 148,293 rows ok and 23,541 accepted
(23,501 under ADR 9, 40 under ADR 7), warning-free, on x86_64 ELF, PE (wine) and aarch64 (qemu). Every Normal row is at
0 ulp, against 1.3.0's bits or its correction; every Debug and JSON text matches byte for byte.

**Mutation checks** (`build/handoff/mut_a1.py`: the replay and `tests/dist.tcyr`):

| Mutant | Replay rows failed | Unit checks failed |
|---|---|---|
| the pdf's 1/(σ√(2π)) as (1/σ)/√(2π) | 1,567 | 0 |
| the cdf's (x − μ)/σ as x/σ − μ/σ | 906 | 1 |
| `require_positive` accepting 0 | 28 | 1 |
| the mean unchecked | 21 | 1 |
| Box–Muller's sine branch | 27 | 0 |
| Box–Muller's u1 and u2 swapped | 27 | 0 |
| a draw as σ·(Z + μ/σ) | 6 | 1 |
| Box–Muller without the MIN_POSITIVE guard | 0 | 1 |
| Debug's tie to even (serde_json's) | 41 | 1 |
| Debug's exponential form from 1e-5 | 204 | 1 |
| Debug's exponential form from 1e15 | 247 | 1 |
| Debug without `.0` on an integer | 1,265 | 3 |
| a refused parameter decoded as `INVALID_PARAMETER` | 50 | 1 |
| the fields written in reverse | 44 | 2 |
| the field names read swapped | 103 | 3 |
| the pdf's (−½z)·z as −½(z·z) | 0 | 0 |

- **Fifteen of sixteen fail.** No golden stream draws a 0, so only the unit test, through a consumer's generator whose
  first draw is 0, sees the MIN_POSITIVE guard.
- **The survivor is equivalent:** −½z is exact outside the subnormals, so both orders round once to the same value.

**Tests.** `tests/dist.tcyr` has 61 checks:
- 1.3.0's Normal tests: `normal_pdf_at_mean` (now bit for bit), `normal_cdf_at_mean` (Φ(0) = 0.5 exactly),
  `normal_invalid_std_dev`, `normal_sample_finite`, `serde_roundtrip_normal`, and Normal's lines of
  `constructors_reject_non_finite_parameters`, `deserialize_rejects_invalid_parameters` and
  `normal_cdf_tail_precision`;
- the constructor: a null `out`, and a refusal that writes nothing; −0.0, a subnormal σ and f64::MAX kept;
- the identities, bit for bit: the cdf is the T1 Φ((x − μ)/σ), N(0, 1)'s pdf is even, and a draw is μ + σ·Z on the
  same stream;
- a consumer's own generator through the trait, two uniforms a draw, and the first one 0;
- the codec's text, serde's sequence form, a missing field, a number out of range, −0 and 5e-324;
- Debug's two forms, and the f64 form's edges: the thresholds, the tie 2⁻²⁵, f64::MAX, the specials.

**Surface parity.** `scripts/surface_parity.py` takes `distribution` as a module in progress: its 17 items for the trait
and Normal are mapped, and its other 144 are listed as pending until M3's gate.

**Cost** (`docs/benchmarks-rust-v-cyrius.md`): 86–90 ns a pdf (3.4× Rust: the correctly rounded exp), 1.37–1.41 µs a cdf
in the bulk and 2.51–2.58 µs in the deep tail (51× and 84×: the T1 Φ), 227–235 ns a draw (8×: the correctly rounded ln
and cos), 8.7–9.7 µs a JSON round trip and 6.3–6.4 µs a Debug (48× and 23.5×: the exact shortest-digit printer, L10).
The bench's six digests are all explained by the models.

**`lint` and the trait.** `cyrius lint` compiles a file alone, and a bound or an impl naming another module's trait
doesn't compile there. So `src/dist.cyr` includes `src/rng.cyr`, and `src/dist_cont.cyr` includes `src/dist.cyr`. A
repeated include is skipped, and `cyrius distlib` leaves project includes out of the bundle. CLAUDE.md now names this
as the second exception to "source files need no includes".

### 9.2 Part a2: Uniform, Exponential and Bernoulli (2026-11-16)

**The modules.**
- **`src/dist_cont.cyr`:** `PmnUniform { min, max }` and `PmnExponential { lambda }`, with Normal's methods, codec
  and Debug.
  - `PmnUniform_new` makes 1.3.0's three checks: both bounds finite, min < max (so [0, −0] is refused), and a finite
    width. The pdf is 1/(max − min) on the closed support, the cdf (x − min)/(max − min) between 0 and 1, and a draw
    min + u·(max − min).
  - `PmnExponential_new` takes a finite, positive lambda. The pdf λ·exp(−λx) and the cdf −expm1(−λx) are 0 below 0,
    on the correctly rounded exp and expm1. A draw is −ln(1 − u)/λ, with u kept from 0 by f64::MIN_POSITIVE, as
    Rust's. The guard changes nothing: 1 − MIN_POSITIVE rounds to 1, so a u of 0 draws −0.0 either way.
- **`src/dist_disc.cyr`,** new, for the discrete distributions: `PmnBernoulli { p }`. The constructor takes
  0 ≤ p ≤ 1, −0.0 kept. The mass is 1 − p within f64::EPSILON of 0 and p within f64::EPSILON of 1. The cdf is NaN at
  NaN, then 0, 1 − p and 1. A draw is 1 when u < p.
- **Shared:** `_pmn_dist_fields`, serde's struct rules on a decoder's f64 fields, moves to `src/dist.cyr`.

**The golden rows.**
- **`dist`, 44,739 more rows (57,621 in all):**
  - each constructor on a 20-value grid (part a1's 16, ±1e308, 0.5 and 2), every pair for Uniform's two bounds, and
    Bernoulli's with p = ±ε and 1 ± ε. The fields, or the error; class A;
  - `Uniform::pdf` and `Uniform::cdf`, 3,099 rows each: eleven fixed (min, max) and 40 seeded pairs at
    x = min + w·k/16 for k from −8 to 24, and at each bound and its two neighbours; the fixed pairs at the specials
    too; 1,000 seeded triples. Among the fixed pairs: [0, f64::MAX], [1e308, f64::MAX], [5e-324, 1e-323] and
    [1e6, 1e6 + 1]. Class A;
  - `Exponential::pdf` and `Exponential::cdf`, 17,800 rows each: ten fixed lambdas (from 1e-300 to f64::MAX, and
    MIN_POSITIVE) and 40 seeded ones at λx = k/8 on [−1, 40] and at five tiny x; the fixed ones at the specials too;
    1,000 seeded pairs with λx from 10⁻¹² to 100. Class C: glibc's exp and expm1;
  - `Bernoulli::pdf` and `Bernoulli::cdf`, 1,073 rows each: nine fixed p (0, 1, 1 − 2⁻⁵³ and 5e-324 among them) and
    20 seeded ones, at the specials, at the ε/2 steps around 0 and 1, and at seven more points. Class A;
  - every mean and variance, class A;
  - 90 streams: 10⁴ draws of U(0, 1), Exp(1) and Bernoulli(0.5) from three seeds, and 256 of each other fixed case.
    Uniform's and Bernoulli's are class A, Exponential's class C (glibc's ln).
- **`json`,** 640 more rows (7,270 in all): 511 documents and 129 writes. The documents are `struct_docs`'s variants
  on each field's 22 number forms, serde's sequence form, and seeded documents, half of them refused by the
  constructor. Bernoulli's also take 1 ± an ulp and a p that rounds to 1.
- **`debug`,** 92 more rows (25,400 in all): each type in both forms, at up to 16 parameters.

**The models.** `validate.py` re-derives every row in Rust's operation order, three ways as in part a1. `Prims` gains
expm1.
- `dist.cr.tsv` gains 255 rows where glibc misrounds: 36 Exponential pdf values (three of them 2 ulp apart, after
  the product with λ), 216 cdf values (expm1) and 3 streams (ln). It has 269 rows in all.
- No a2 row is downstream of a T1 function, so `dist.ref.tsv` is unchanged (2,699 rows).
- Each codec's model is its `Repr`'s fields, then the constructor's test. Thirty documents serde_json reads only by
  skipping a member unchecked join `json.map.tsv` (70 rows) under ADR 007 row 16, ten per type, as Normal's did.

**The result.** Every row passed on the first replay. The replay is now ten modules: 193,479 rows ok and 23,826
accepted (23,756 under ADR 9, 70 under ADR 7), warning-free. Every a2 row is at 0 ulp, against 1.3.0's bits or its
correction, and every Debug and JSON text matches byte for byte.

**The harness.** `_par_dist` drives every method through one bounded generic, `_par_dist_call<D: PmnDistribution>`,
as a consumer's generic code calls them. Normal's rows now go through it too, with the same results.

**Mutation checks** (`build/handoff/mut_a2.py`: the replay and `tests/dist.tcyr`):

| Mutant | Replay rows failed | Unit checks failed |
|---|---|---|
| Uniform: min = max accepted | 20 | 3 |
| Uniform: no width check | 4 | 3 |
| Uniform's pdf open at max | 101 | 1 |
| Uniform's cdf as x/w − min/w | 771 | 0 |
| Uniform's mean as min/2 + max/2 | 2 | 2 |
| Uniform's variance as r·(r/12) | 14 | 0 |
| Uniform's draw as max − (1 − u)·w | 18 | 1 |
| Exponential: lambda only finite | 57 | 3 |
| Exponential's pdf 0 at 0 | 71 | 1 |
| Exponential's cdf as 1 − exp | 1,306 | 1 |
| Exponential's variance as (1/λ)/λ | 7 | 0 |
| Exponential's draw as −ln(u)/λ | 30 | 2 |
| Bernoulli: p = 0 refused | 91 | 4 |
| Bernoulli's pmf with ε in the window | 56 | 1 |
| Bernoulli's pmf p at 1 exactly | 28 | 1 |
| Bernoulli's cdf without the NaN arm | 29 | 1 |
| Bernoulli's cdf 1 − p at 1 | 104 | 1 |
| Bernoulli's variance as p − p·p | 6 | 0 |
| Bernoulli's draw on u ≤ p | 0 | 2 |
| Uniform's JSON fields written in reverse | 39 | 1 |
| Uniform's JSON names read swapped | 116 | 3 |
| Bernoulli's refusal decoded as `INVALID_PARAMETER` | 53 | 2 |
| Exponential's Debug field named `rate` | 32 | 2 |
| Uniform's Debug with max as a first field | 14 | 1 |

- **All 24 fail.** One more, the unused MIN_POSITIVE guard in Exponential's draw, is equivalent (above) and wasn't run.
- **Uniform's mean** first survived: min/2 + max/2 and (min + max)/2 differ only when min + max overflows or the bounds
  are subnormal. Two fixed pairs were added for it: [1e308, f64::MAX], whose mean is ∞ in 1.3.0, and
  [5e-324, 1e-323]. They fail it on 2 rows, and a unit test pins both.
- **The draw on u ≤ p** fails unit checks only. No golden stream draws a u equal to p, but a consumer's generator of
  eighths does, at p = 0 and p = 0.5.

**Tests.** `tests/dist.tcyr` has 162 checks, 101 of them new:
- 1.3.0's tests: `uniform_pdf_and_cdf`, `exponential_mean_and_variance` and `bernoulli_pdf`, now exact or bit for
  bit; the three types' lines of `constructors_reject_non_finite_parameters` and
  `deserialize_rejects_invalid_parameters`; `serde_roundtrip_uniform`, `_exponential` and `_bernoulli`; and
  `exponential_and_weibull_cdf_small_x`'s Exponential line, now exactly 1e-20;
- the constructors: a null `out`, a refusal that writes nothing, [0, −0], a width that rounds to f64::MAX against
  one that overflows, a lambda of −0 or 5e-324, p = ±ε, 1 + ε and −0.0;
- the edges: the closed support, NaN, ±∞, the ε windows, the mean's overflow and the subnormal pdf's;
- the draws, bit for bit against their formulas on the same stream; one uniform a draw; a u of 0 drawing −0.0;
- each codec's text and refusals, and each Debug's two forms.

**Surface parity.** 33 more items are mapped (50 of `distribution`'s 161), and 111 are pending until M3's gate.

**Cost** (`docs/benchmarks-rust-v-cyrius.md`): 38–39 ns a Uniform cdf and 27–28 ns a Bernoulli cdf (7.2× and 6.4×
Rust), 79–83 ns an Exponential pdf or cdf (5.9–10.7×: the correctly rounded exp and expm1), and 35–115 ns a draw
(9–11×). Most of a simple row is the constructor's checks, uninlined calls: `Uniform::new` alone is 24 ns over the
loop's 6. The bench's fourteen digests are all explained by the models.

### 9.3 Part a3: Cauchy and Weibull (2026-11-16)

**The modules.** `src/dist_cont.cyr` gains `PmnCauchy { x0, gamma }` and `PmnWeibull { k, lambda }`, with the methods,
codec and Debug of the others.
- **Cauchy:** the constructor takes a finite x0 and a finite, positive gamma. The pdf, 1/(π·γ·(1 + z²)), is plain
  arithmetic, so it matches 1.3.0's bits. The cdf, (1/π)·atan(z) + ½, takes the correctly rounded atan, and a draw,
  x0 + γ·tan(π(u − ½)), the correctly rounded tan. The mean and variance are undefined: NaN.
- **Weibull:** the constructor takes a finite, positive k and lambda.
  - The pdf is (k/λ)·z^(k−1)·exp(−z^k), with 1.3.0's three values at 0: 1/λ for k = 1, 0 for k > 1, +∞ for k < 1.
  - The cdf is −expm1(−(x/λ)^k).
  - A draw is λ·(−ln(1 − u))^(1/k).
  - All three are on the correctly rounded pow, exp, expm1 and ln.
  - The mean, λ·Γ(1 + 1/k), and the variance, (λ·λ)·(Γ(1 + 2/k) − Γ(1 + 1/k)²), take Γ as 1.3.0 does,
    exp(lnΓ(·)), now on the T1 lnΓ.

**The golden rows.**
- **`dist`, 33,360 more rows (90,981 in all):**
  - each constructor on every pair of the 20-value grid; class A;
  - `Cauchy::pdf` and `Cauchy::cdf`, 6,071 rows each: eleven fixed (x0, γ) at z = k/4 on [−40, 40], at
    z = ±10¹⁰, ±10²⁰, ±10¹⁵⁴, ±1.4×10¹⁵⁴ (z² overflows) and ±10³⁰⁰, and at the specials; 40 seeded pairs at
    z = −40, −37.5, …, 40; 1,000 seeded triples with |z| up to 10⁶. Among the fixed pairs: γ = 5e-324,
    (−f64::MAX, f64::MAX) and x0 = f64::MAX. The pdf is class A, the cdf class C (glibc's atan);
  - `Weibull::pdf` and `Weibull::cdf`, 9,180 rows each: fifteen fixed (k, λ) (k from 5e-324 to f64::MAX, λ from
    1e-300 to 1e300) and 40 seeded pairs, at x = λ·j/16 for j from −8 to 80, at λ·2^j for j from −40 to 12, at λ and
    its neighbours and at λ·10⁻³⁰⁰; the fixed pairs at the specials too; 1,000 seeded triples. Class C;
  - the moments: Cauchy's at every pair (NaN, class A); Weibull's mean and variance, 939 rows each, at every pair above
    and at 221 shapes (21 fixed from 5e-324 to f64::MAX, 200 seeded from 10⁻³ to 10⁶) times four scales. Class C;
  - 78 streams: 10⁴ draws of Cauchy(0, 1) and Weibull(1, 1) from three seeds, and 256 of each other fixed pair.
    Class C.
- **`json`,** 461 more rows (7,731 in all), and **`debug`,** 64 more (25,464), as part a2's.

**The models.** `validate.py` models every row in Rust's operation order. `Prims` gains tan, atan and pow, and
`dist_prims` gives each model 1.3.0's Lanczos coefficients, so Weibull's moments go through 1.3.0's lnΓ on glibc and
on the correctly rounded primitives, and through the T1 lnΓ in the reference model.
- **`dist.cr.tsv`** gains 603 rows where a glibc primitive misrounds: 561 Weibull cdf values, 19 pdf values, 13 Cauchy
  and 9 Weibull streams, and one Weibull mean. The far ones are exp amplifying a pow misround: z^k ≈ 120, so 1 ulp of
  z^k is 121 ulp of exp(−z^k).
- **`dist.ref.tsv`** gains 1,100 rows, Weibull moments the T1 lnΓ moves: 686 means and 414 variances.
- Twenty documents serde_json reads only by skipping a member unchecked join `json.map.tsv` (90 rows).

**The result.** Every row passed on the first replay. The replay is ten modules: 225,641 rows ok and 25,549 accepted
(25,459 under ADR 9, 90 under ADR 7), warning-free. Every a3 row is at 0 ulp, against 1.3.0's bits, its correction
or Rust's formula on the T1 lnΓ.

**Weibull's moments against MPFR** (λ·Γ(1 + 1/k) and λ²(Γ(1 + 2/k) − Γ(1 + 1/k)²) at 1,024 bits, rounded once; on
the golden rows):

| Rows | 1.3.0 | The port |
|---|---|---|
| the mean, Γ finite (857) | 134 exact, max 1,268 ulp | 535 exact, max 433 ulp |
| the variance, k ≤ 20 (403) | 180 exact, max 2,412 ulp | 198 exact, max 432 ulp |
| the variance, k > 20 (426) | cancellation: up to 10⁹ ulp, and negative from k = 10¹⁰ | the same |

- **The mean** is never less accurate than 1.3.0's, and Weibull(2, 1)'s is now the correctly rounded √π/2 (1.3.0:
  2 ulp off). What remains is Rust's form: exp turns lnΓ's last-ulp error, relative to |lnΓ| up to about 700, into up
  to 433 ulp of Γ.
- **The variance** cancels for large k: g2 − g1² loses about 2·log₁₀ k digits. At k = 10⁶ and λ = 1, 1.3.0 gives
  1.64502×10⁻¹², the port 1.64491×10⁻¹², and the truth is 1.644934×10⁻¹². From k = 10¹⁰ both are noise: 1.3.0
  gives −1.8×10⁻¹⁵, the port 1.1×10⁻¹⁶, and the truth is 1.6×10⁻²⁰.
- **Where Γ overflows,** both give +∞, though λ·Γ may be finite: Weibull(0.005, 1e-300)'s true mean is about
  10⁶³. At k = 5e-324, 1.3.0's Lanczos lnΓ(∞) is NaN, and the port's T1 lnΓ(∞) = ∞ gives the right mean, +∞.

These are findings for 2.1, where a form on lnΓ differences and logs would remove the cancellation and the overflow.
The port keeps Rust's formulas, as parity requires.

**Mutation checks** (`build/handoff/mut_a3.py`: the replay and `tests/dist.tcyr`):

| Mutant | Replay rows failed | Unit checks failed |
|---|---|---|
| Cauchy: x0 unchecked | 30 | 2 |
| Cauchy: gamma only finite | 170 | 4 |
| Cauchy's pdf as 1/(π·(γ·(1 + z²))) | 1,069 | 0 |
| Cauchy's cdf as atan(z)/π | 723 | 0 |
| Cauchy's cdf with z as x/γ − x0/γ | 804 | 0 |
| Cauchy's mean as x0 | 51 | 2 |
| Cauchy's draw on tan(πu − π/2) | 24 | 1 |
| Cauchy's draw on tan(π(½ − u)) | 30 | 1 |
| Weibull: lambda only finite | 75 | 3 |
| Weibull's pdf without the k = 1 arm at 0 | 6 | 2 |
| Weibull's pdf 0 at 0 for k < 1 | 41 | 1 |
| Weibull's pdf as (k/λ)·(z^(k−1)·exp(−z^k)) | 2,129 | 0 |
| Weibull's pdf with z^k as z^(k−1)·z | 1,435 | 0 |
| Weibull's cdf as 1 − exp | 3,262 | 2 |
| Weibull's cdf with x < 0 for its zero arm | 4 | 1 |
| Weibull's moments with 1 + j/k as (k + j)/k | 160 | 0 |
| Weibull's variance as λ·(λ·(g2 − g1²)) | 34 | 0 |
| Weibull's draw as exp(ln(·)/k) | 38 | 2 |
| Weibull's draw on −ln(u) | 42 | 3 |
| Cauchy's JSON names read swapped | 105 | 1 |
| Weibull's JSON with lambda written first | 43 | 1 |
| Weibull's refusal decoded as `INVALID_PARAMETER` | 55 | 1 |
| Cauchy's Debug field named `x_0` | 32 | 2 |
| Weibull's Debug with lambda as a first field | 16 | 1 |

- **All 24 fail.** Weibull's unused MIN_POSITIVE guard is equivalent, as Exponential's, and wasn't run.
- **The zero arm** first survived both. With x < 0 in place of x ≤ 0, the cdf at x = −0.0 and an odd-integer k is
  −(expm1(−(−0.0)^k)) = −0.0, where 1.3.0 returns +0.0.
  - The golden rows hold these cases, but the harness's class-C comparison measured ulp distance, which puts +0 and −0
    0 ulp apart. `validate.py`'s `same` already compares bits.
  - The harness now compares a zero's sign in classes C and S too (`tests/parity_core.cyr`). No row in the corpus
    changes: a trial run with the check failing every wrong-signed zero failed none.
  - The mutant now fails 4 replay rows, and a unit test pins cdf(−0.0) = +0.0 for k = ½ and 1.

**Tests.** `tests/dist.tcyr` has 256 checks, 94 of them new:
- 1.3.0's tests: `cauchy_pdf_and_cdf` (pdf(0) = 1/π bit for bit), `cauchy_undefined_moments`,
  `cauchy_sample_finite`, `weibull_exponential_case` (the mean exactly 2), `weibull_mean_and_variance` (now the
  correctly rounded √π/2), `weibull_sample_nonnegative`, `serde_roundtrip_cauchy` and `_weibull`, the two types'
  lines of `constructors_reject_non_finite_parameters` and `deserialize_rejects_invalid_parameters`, and
  `exponential_and_weibull_cdf_small_x`'s Weibull line, now exactly 1e-20;
- the constructors: every refusal, a null `out`, a refusal that writes nothing, x0 = −0.0 kept, a subnormal γ or k;
- Cauchy: the pdf even bit for bit, cdf(±∞) exactly 1 and 0, cdf(x0) = ½, a subnormal scale's overflow, the draws bit
  for bit against x0 + γ·tan(π(u − ½));
- Weibull: Weibull(1, 1) is Exp(1) bit for bit, the three values at 0, cdf(−0.0) = +0.0, the moments at k = 5e-324
  and where Γ overflows, the draws bit for bit, and a u of 0 drawing +0.0 for k = 2 and −0.0 for k = 1, as Rust's
  powf gives (−0.0)^½ = +0.0 and (−0.0)^1 = −0.0;
- each codec's text and refusals, and each Debug's two forms.

**Surface parity.** 22 more items are mapped (72 of `distribution`'s 161), and 89 are pending until M3's gate.

**Cost** (`docs/benchmarks-rust-v-cyrius.md`): 44–45 ns a Cauchy pdf (5.8× Rust), 163–167 ns a Cauchy cdf and
233–237 ns a draw (10–11×: the correctly rounded atan and tan), 235–375 ns a Weibull pdf, cdf or draw (7–10×: up to
two correctly rounded pows and an exp), and 186–188 ns a Weibull mean (4.8×: the T1 lnΓ). The bench's 21 digests are
all explained by the models.

### 9.4 Part b1: Gamma, ChiSquared and Marsaglia–Tsang (2026-11-16)

**The modules.**
- **`src/dist.cyr`** gains `_pmn_sample_gamma_standard`, 1.3.0's `sample_gamma_standard`: one Gamma(α, 1) variate by
  Marsaglia and Tsang's method, with the Ahrens–Dieter boost for α < 1 (a Gamma(α + 1, 1) variate times u^(1/α)).
  It is Rust's loop in Rust's operation order: the normal from Box–Muller until v = 1 + c·x is positive, v³, the
  squeeze u < 1 − 0.0331·x⁴, then the full test ln u < ½x² + d·(1 − v + ln v). It sits beside Box–Muller because
  part b2's Beta, Student-t and F samplers and part c2's Binomial draw from it too.
- **`src/dist_gamma.cyr`,** new, for the gamma family: `PmnGamma { alpha, beta }` and `PmnChiSquared { df }`, with
  the methods, codec and Debug of the others.
  - The pdf is exp((α − 1)·ln x − x/β − α·ln β − lnΓ(α)), left to right, on the T1 lnΓ. χ²'s is the same form with
    α = df/2 and β = 2, ln 2 taken at run time as Rust does.
  - The cdf is P(α, x/β), the T1 regularized lower incomplete gamma; χ²'s is P(df/2, x/2).
  - The mean and variance are α·β and α·β·β (df and 2·df); a draw is the standard variate times β (times 2).

**The golden rows.**
- **`dist`, 19,461 more rows (110,442 in all):**
  - each constructor on the 20-value grid, every pair for Gamma; class A;
  - `Gamma::pdf` and `Gamma::cdf`, 4,672 rows each: sixteen fixed (α, β) and 40 seeded pairs. The fixed pairs run
    α from 5e-324 to f64::MAX, with 2e9 past T1's 10⁹ (Rust's Wilson–Hilferty form), and β from 1e-300 to 1e300.
    Each fixed pair is taken across [0, 4·mean] by sixteenths, at β·2^j for j from −40 to 12, at the mean ± up to
    12 standard deviations by halves, at the mean and its neighbours, far out, and at the specials. The seeded pairs
    are taken at the mean ± up to 8 standard deviations. There are 1,000 seeded triples too. Class C;
  - `ChiSquared::pdf` and `ChiSquared::cdf`, 4,672 rows each: sixteen fixed df (5e-324 to 4e9) and 40 seeded ones,
    the same way. Class C;
  - every mean and variance, class A;
  - 96 streams: 10⁴ draws of Gamma(1, 1) and χ²(2) from three seeds, and 256 of each other fixed case; and
    `sample_gamma_standard`, the oracle's hook, 1,024 draws at eleven shapes (10⁻³ to 1e300, either side of the
    boost) from three seeds. Class C.
- **`json`,** 436 more rows (8,167 in all), and **`debug`,** 64 more (25,528), as part a2's.

**The models.** `validate.py` models every row in Rust's operation order: Marsaglia–Tsang's loop, and the pdf and cdf
on 1.3.0's Lanczos lnΓ and incomplete gamma, through glibc and on correctly rounded primitives, or on the T1 functions
as the port computes them (`t1_ln_gamma`, `t1_gamma_inc`). `dist_prims` now carries all of 1.3.0's special-function
constants.
- **`dist.cr.tsv`** gains 25 rows where a glibc primitive misrounds: 21 streams (ln, cos or pow inside the rejection
  loop) and 4 cdf values (897 rows in all).
- **`dist.ref.tsv`** gains 7,847 rows that the T1 functions move: 1,606 Gamma and 1,836 χ² pdf values (lnΓ), and
  2,186 Gamma and 2,219 χ² cdf values (the incomplete gamma). It has 11,646 rows in all.
- Twenty documents serde_json reads only by skipping a member unchecked join `json.map.tsv` (110 rows).

**The result.** Every row passed on the first replay, the Wilson–Hilferty shapes past 10⁹ included. The replay is ten
modules: 237,710 rows ok and 33,441 accepted (33,331 under ADR 9, 110 under ADR 7), warning-free. Every b1 row is at
0 ulp, against 1.3.0's bits, its correction or Rust's formula on the T1 functions.

**The cdf and pdf against MPFR** (the golden rows with 0 < α ≤ 10⁵; the cdf as P(α, X) with X the rounded x/β, by
`mpfr_ref.gamma_pq` at 320 bits or more, and the pdf at 1,024 bits; both rounded once):

| Rows | 1.3.0 | The port |
|---|---|---|
| the cdf, α > 10⁻³ (6,758) | 2,922 exact, max 2.8×10¹³ ulp | 6,742 exact, all within 1 ulp |
| the cdf, α ≤ 10⁻³ (597) | 73 exact; 0 where P is 1 | all 597 exact |
| the pdf, α > 10⁻³ (6,758) | 1,325 exact, max 1.36×10⁶ ulp | 1,712 exact, max 1.36×10⁶ ulp |
| the pdf, α ≤ 10⁻³ (596) | 44 exact; 0 where it is about 1 | 121 exact, max 739 ulp |

- **The cdf** inherits T1's incomplete gamma: within 1 ulp everywhere, where 1.3.0 lost up to 0.4% in the deep left
  tail (Gamma(10, 2) at 2⁻³⁹: 1.3×10¹³ ulp; χ²(30) at 2⁻³⁹: 2.8×10¹³).
- **At α = 5e-324,** 1.3.0's Lanczos lnΓ is +∞ (the reflection's 1e-300 cutoff), so its pdf and cdf are 0. T1's lnΓ is
  744.4, and the port gives the right 1.0 for both at x = 5e-324.
- **The pdf** keeps Rust's form, exp of a sum whose terms reach |α·ln x|. Its error is that sum's rounding: up to
  1.36×10⁶ ulp at α = 10⁵ in both, the port more often exact but on 606 rows further off than 1.3.0 by chance. T1's
  incomplete gamma already holds this prefactor, x^α·e^(−x)/Γ(α), in double-double, so an accurate pdf is near at
  hand: a finding for 2.1.

**Mutation checks** (`build/handoff/mut_b1.py`: the replay and `tests/dist.tcyr`):

| Mutant | Replay rows failed | Unit checks failed |
|---|---|---|
| the sampler without the boost below α = 1 | the run never ends | the run never ends |
| the boost as u^α | 42 | 2 |
| d = α − 0.3333 | 114 | 0 |
| c = 1/(3·√d) | 33 | 0 |
| v² for v³ | 114 | 0 |
| a looser squeeze, 1 − 0.0300·x⁴ | 11 | 0 |
| x² for ½x² in the full test | 76 | 0 |
| v − 1 + ln v in the full test | 114 | 0 |
| no boost factor on the result | 42 | 2 |
| Gamma: beta only finite | 119 | 3 |
| Gamma's pdf with (α − 1)·ln β | 2,223 | 1 |
| Gamma's pdf as t − (x/β + α·ln β) | 524 | 1 |
| Gamma's pdf with x < 0 for its zero arm | 34 | 0 |
| Gamma's cdf at x·(1/β) | 253 | 0 |
| Gamma's cdf as 1 − Q | 1,295 | 3 |
| Gamma's variance as α·(β·β) | 18 | 0 |
| Gamma's draw as Gamma(αβ, 1) | 18 | 1 |
| χ²'s pdf with ln 2 as 0.69314718056 | 2,936 | 1 |
| χ²'s cdf as P(df, x/2) | 3,188 | 3 |
| χ²'s draw as Gamma(df, 1) | 42 | 1 |
| Gamma's JSON names read swapped | 70 | 1 |
| χ²'s refusal decoded as `INVALID_PARAMETER` | 51 | 1 |
| χ²'s JSON key written `dof` | 44 | 3 |
| Gamma's Debug field named `shape` | 32 | 2 |
| a tighter squeeze, 1 − 0.0332·x⁴ | 0 | 0 |
| no squeeze | 0 | 0 |

- **All 24 non-equivalent mutants fail.** Without the boost, α < ⅓ makes d negative and c NaN, and Rust's loop never
  ends: the replay timed out, as a consumer's call would hang.
- **The last two are equivalent.** The squeeze is a fast path inside the acceptance region: a tighter one, or none,
  only sends its points to the full test, which accepts them too. A looser one accepts points the full test refuses,
  and the streams catch it.

**Tests.** `tests/dist.tcyr` has 334 checks, 78 of them new:
- 1.3.0's tests: `gamma_pdf_known` (now bit for bit), `gamma_cdf_known` and `chi_squared_cdf_known` (within 10⁻¹⁵,
  1.3.0's were 10⁻⁶ and 10⁻⁴), `gamma_mean_and_variance`, `gamma_sample_mean`, `gamma_invalid_params`,
  `gamma_small_alpha_sample`, `chi_squared_mean_and_variance`, `chi_squared_sample_mean`, `chi_squared_large_df_cdf`,
  the serde round trips, and the two types' lines of `constructors_reject_non_finite_parameters` and
  `deserialize_rejects_invalid_parameters`;
- the oracle's wiring tests: Gamma(α, 1)'s cdf is P(α, x) bit for bit, χ²(7) is Gamma(3.5, 2) bit for bit, and
  Gamma(α, 1)'s draws are `sample_gamma_standard`'s, bit for bit and draw for draw through a counting generator;
- the edges: 0 at and below 0, NaN at NaN, cdf(∞) = 1, pdf(∞) NaN as 1.3.0's (∞ − ∞); α = 5e-324's pdf and cdf of
  1.0 and its +0.0 draw; the deep tail within 1 ulp of MPFR; α = f64::MAX's draws of f64::MAX;
- each codec's text and refusals, and each Debug's two forms.

**Surface parity.** 22 more items are mapped (94 of `distribution`'s 161), and 67 are pending until M3's gate.

**Cost** (`docs/benchmarks-rust-v-cyrius.md`): 291–296 ns a Gamma pdf (6.2× Rust), 13.0–13.2 µs a Gamma or χ² cdf
(98–101×: the T1 incomplete gamma, 12.9 µs of it alone), and 275–297 ns a draw (7.8–7.9×: Marsaglia–Tsang on the
correctly rounded ln, cos and pow). The bench's 26 digests are all explained by the models.

### 9.5 Part b2: Beta, StudentT, FDistribution and their tails (2026-11-16)

**The modules.** `src/dist_gamma.cyr` gains `PmnBeta { alpha, beta }`, `PmnStudentT { df }` and
`PmnFDistribution { d1, d2 }`, with the methods, codec and Debug of the others, and Rust's two tails, which M4's
hypothesis tests share:
- **`_pmn_student_t_two_tailed(t, df)`:** P(|T| ≥ |t|) = I_x(df/2, ½) at x = 1/(1 + t²/df) and its complement
  y = 1/(1 + df/t²), each formed directly, on the T1 incomplete beta, clamped to [0, 1]; `_pmn_student_t_cdf` takes
  1 − ½·p for t ≥ 0 and ½·p below, so each tail keeps its accuracy.
- **`_pmn_f_cdf_sf(x, d1, d2)`:** (the cdf, the survival) as the T1 incomplete beta's (I_u, 1 − I_u) at
  u = 1/(1 + 1/r) and v = 1/(1 + r), r = d1x/d2, each formed directly.
- **Beta:** the pdf exp((α − 1)·ln x + (β − 1)·ln_1p(−x) − ln B(α, β)) on (0, 1), with ln B on the T1 lnΓ; the cdf
  I_x(α, β) on the T1 incomplete beta; a draw X/(X + Y) from two Marsaglia–Tsang variates, X first.
- **StudentT:** the pdf in Rust's two forms, the ln B one from df = 20 (where the two lnΓ cancel) and the lnΓ one
  below; the cdf `student_t_cdf`; the moments 0 and df/(df − 2), +∞ for 1 < df ≤ 2, NaN below; a draw Z/√(V/df),
  Z from Box–Muller first, then V a χ² variate.
- **FDistribution:** the pdf on ln_1p and ln B; the cdf `f_cdf_sf`'s first value; the moments for d2 > 2 and
  d2 > 4, NaN below; a draw (X1/d1)/(X2/d2), X1 first.

**The golden rows.**
- **`dist`, 36,081 more rows (146,523 in all):**
  - each constructor on the 20-value grid, every pair for Beta and F; class A;
  - `Beta::pdf` and `Beta::cdf`, 5,310 rows each: fifteen fixed (α, β) (from 5e-324 to f64::MAX, (2e9, 3) past T1's
    10⁹) at x = j/64, at 2⁻ʲ and 1 − 2⁻ʲ for j up to 60, at the mean ± up to 8 standard deviations, at 0 and 1 and
    their neighbours, and at the specials; 40 seeded pairs at sixteenths; 1,000 seeded triples. Class C;
  - `StudentT::pdf` and `StudentT::cdf`, 5,578 rows each: eighteen fixed df (5e-324 to 3e9, with 20 and the double
    below it either side of the pdf's switch) at |t| ≤ 20 by quarters, far out to ±1e300 (t² overflows from 1.35e154)
    and at the specials; 40 seeded df; 1,000 seeded pairs. Class C;
  - `FDistribution::pdf` and `FDistribution::cdf`, 4,464 rows each: sixteen fixed pairs (with d2 = 2, 4 and 4.5
    around the moments' edges) at sixteenths to 6, at powers of two and at the specials; 40 seeded pairs; 1,000
    seeded triples. Class C;
  - every mean and variance, class A; 147 streams, 10⁴ draws of Beta(2, 5), t(3) and F(4, 9) from three seeds and
    256 of each other fixed case. Class C;
  - the oracle's tails on their own grids: `student_t_two_tailed` (1,626 rows) and `student_t_cdf` (1,086) at six df
    from 0.5 to 2e9 and the specials, and `f_cdf_sf` (1,360 rows, both values) at five pairs and 500 seeded triples.
    Class C.
- **`json`,** 665 more rows (8,832 in all), and **`debug`,** 96 more (25,624), as part a2's.

**The models.** `validate.py` models every row in Rust's operation order, on 1.3.0's ln B and incomplete beta
through glibc and on correctly rounded primitives, or on the T1 functions as the port computes them.
- **`dist.cr.tsv`** gains 157 rows where glibc misrounds: 120 pdf values, 31 streams and 6 cdf values (1,054 rows in
  all).
- **`dist.ref.tsv`** gains 16,556 rows that the T1 functions move: 5,898 pdf values (ln B and lnΓ), 7,717 cdf values
  and 2,941 tail rows (the incomplete beta). It has 28,202 rows in all.
- Thirty documents serde_json reads only by skipping a member unchecked join `json.map.tsv` (140 rows).

**The model's exp core.** The first replay failed 12 rows by 1 ulp: eleven F(2, 2) cdf values and one Beta cdf
value. F(2, 2)'s cdf is I_u(1, 1), and T1 takes the smaller of u and v as exact, so it returns 1 − v. At
x = 1.8125, v's last bit is set, so 1 − v is an exact tie at double precision, and the double-double's low word
decides the rounding.
- The port's T1 prefactor takes e^M from kimiya's double-double exp core, whose contract is h + l within about 2⁻⁷⁰
  of e^M/2^E, with l unnormalized. `validate.py` stood in for that core with MPFR's exact double-double, which
  agreed with the port's final rounding on every M2 row but not at this tie.
- `t_exp_core` now transcribes `_pmn_km_exp_core` operation for operation, its table and constants read from
  `src/crmath_data.cyr`. All 12 rows then match. `special.ref.tsv` and `special.cr.tsv` are byte-identical
  afterwards, and only `dist.ref.tsv` changes, on 15 lines.
- The port's value at those ties is 1 ulp from MPFR's (within T1's 8 ulp). The model's ln core is still MPFR's
  double-double (within 2⁻⁸⁰ of kimiya's); a tie there would show the same way, as a 1-ulp replay failure, and be
  fixed the same way.

**The result.** Every row passes the replay: ten modules, 257,809 rows ok and 50,184 accepted (50,044 under ADR 9,
140 under ADR 7), warning-free. Every b2 row is at 0 ulp, against 1.3.0's bits, its correction or Rust's formula on
the T1 functions.

**The cdf against MPFR** (Beta's golden rows with 0 < x < 1 and shapes ≤ 10⁵, I_x(α, β) with x exact,
`mpfr_ref.beta_ic`): the port is within 1 ulp on all 4,178 (4,175 exact) and never less accurate than 1.3.0, which
is exact on 2,014. 1.3.0's far misses are its edges: NaN at x = 5e-324 for large shapes (the truth is 0), and 0 at
α = 5e-324 (116 rows; the truth is about 1, but its lnΓ(5e-324) is +∞).

**Mutation checks** (`build/handoff/mut_b2.py`: the replay and `tests/dist.tcyr`):

| Mutant | Replay rows failed | Unit checks failed |
|---|---|---|
| Beta: beta only finite | 120 | 1 |
| Beta's pdf with ln(1 − x) for ln_1p(−x) | 35 | 0 |
| Beta's pdf with x > 1 for its upper arm | 126 | 0 |
| Beta's cdf as 1 − I_(1−x)(β, α) | 1,386 | 1 |
| Beta's variance as (ab·(ab + 1))·ab | 13 | 0 |
| Beta's draw with Y drawn first | 39 | 1 |
| the t tail with y as 1 − x | 2,337 | 1 |
| t's pdf switching at df > 20 | 85 | 0 |
| t's pdf with ln(1 + q) in the ln B form | 1,649 | 1 |
| t's pdf with ln ν + ln π | 174 | 0 |
| t's variance NaN for 1 < df ≤ 2 | 3 | 1 |
| t's mean 0 from df = 1 | 1 | 1 |
| t's draw with V before Z | 54 | 1 |
| the F tail with u as r/(1 + r) | 529 | 1 |
| the F tail with v as 1 − u | 1,406 | 1 |
| F's pdf with ln(1 + q) | 708 | 0 |
| F's pdf with ln d1 − ln d2 | 886 | 0 |
| F's mean from d2 = 2 | 2 | 1 |
| F's variance with d1·(m2·m2) | 3 | 0 |
| F's draw with X2 first | 45 | 0 |
| F's JSON names read swapped | 70 | 1 |
| t's refusal decoded as `INVALID_PARAMETER` | 51 | 1 |
| F's Debug named `F` | 32 | 2 |
| Beta's Debug named `BetaDist` | 32 | 1 |
| the t tail without its clamp | 0 | 0 |

- **All 24 non-equivalent mutants fail.** The cancellation-free forms matter most: y as 1 − x, or v as 1 − u, moves
  1,406–2,337 rows.
- **The clamp is equivalent:** T1's incomplete beta never leaves [0, 1], so Rust's clamp is a guard no input reaches.

**Tests.** `tests/dist.tcyr` has 440 checks, 106 of them new:
- 1.3.0's tests: `beta_mean_and_variance`, `beta_cdf_endpoints`, `beta_uniform_is_beta_1_1` (now exact),
  `beta_sample_in_range`, `beta_small_alpha_sample`, `student_t_symmetric_pdf`, `student_t_mean_and_variance`,
  `student_t_sample_symmetric`, `student_t_cdf_closed_forms` (df = 1 and 2 within 10⁻¹⁵, the far tail, ±∞ and NaN),
  `student_t_large_df_approaches_normal`, `student_t_pdf_matches_gamma_form`, `f_distribution_mean`,
  `f_distribution_variance`, `f_distribution_cdf_nonneg`, `f_distribution_sample_positive`,
  `f_distribution_closed_form_and_tail` (the direct upper tail), the serde round trips, and the three types' lines of
  `constructors_reject_non_finite_parameters` and `deserialize_rejects_invalid_parameters`;
- the oracle's wiring tests: each cdf is its incomplete-beta hook's, bit for bit;
- the edges and identities: the tails at 0, ±∞ and NaN, the two-tailed p-value even in t, the cdf and survival
  summing to 1, and each draw bit for bit against its formula on the same stream, in Rust's draw order;
- each codec's text and refusals, and each Debug's form.

**Surface parity.** 33 more items are mapped (127 of `distribution`'s 161), and 34 are pending until M3's gate.

**Cost** (`docs/benchmarks-rust-v-cyrius.md`): 430–893 ns a pdf (6.2–8.2× Rust), 3.6 µs a Beta cdf and 17.7–21.9 µs
a t or F cdf (23–114×: the T1 incomplete beta), and 490–562 ns a draw (6.9–7.7×). The bench's 34 digests are all
explained by the models.

### 9.6 Part c1: Poisson, Knuth's sampler and PTRS (2026-11-16)

**The modules.** `src/dist_disc.cyr` gains `PmnPoisson { lambda }`, with the methods, codec and Debug of the others,
and `_pmn_sample_poisson_ptrs`, Rust's `sample_poisson_ptrs`:
- **the pmf:** NaN at NaN (Rust's quiet `f64::NAN`, whatever the argument's payload); 0 below 0, off the integers by
  an exact `fract` test, and at ∞; e^-λ at 0; else exp(ln_gamma_kernel(k + 1, λ) − ln λ), Loader's form, O(1) in k;
- **the cdf:** Q(⌊x⌋ + 1, λ) on the T1 upper incomplete gamma, O(√λ) at worst, never O(x);
- **the draws:** Knuth's multiplication up to λ = 30 (a draw of k takes k + 1 uniforms), and above it Hörmann's PTRS
  (1993), transformed rejection with a squeeze, in Rust's operation order on the correctly rounded ln and the
  kernel. Two uniforms a try.

**The golden rows.**
- **`dist`, 8,463 more rows (154,986 in all):**
  - the constructor on the 20-value grid; class A;
  - `Poisson::pdf` (the pmf) and `Poisson::cdf`, 4,128 rows each: sixteen fixed rates (5e-324 to f64::MAX, with 30, 31
    and the double below 30 at the sampler's switch, 1e12 among them) at k = 0 … 60, at ⌊λ ± j·√λ/2⌋ for |j| ≤ 24, off
    the integers (2.5, 3 ± an ulp), at −1, −0.0, λ and ⌊λ⌋, at 1e15, 2⁵³ and 1e300, and at the specials; 40 seeded rates
    around their means; 1,000 seeded pairs, a tenth of them off the integers. Class C;
  - the mean and variance, class A;
  - 48 streams: 10⁴ draws of Poisson(2.5) from three seeds and 256 of each other fixed rate, Knuth's and PTRS's;
    and `sample_poisson_ptrs`, the oracle's hook, 1,024 draws at nine rates from 10 to 1e15 from three seeds.
    Class C.
- **`json`,** 207 more rows (9,039 in all), and **`debug`,** 32 more (25,656), as part a2's.

**The models.** `validate.py` models every row in Rust's operation order, Knuth's loop and PTRS included, on 1.3.0's
kernel and incomplete gamma through glibc and on correctly rounded primitives, or on the T1 functions as the port
computes them.
- **`dist.cr.tsv`** gains 81 rows where glibc misrounds: 77 pmf values and 4 cdf values (1,135 rows in all).
- **`dist.ref.tsv`** gains 1,493 rows that the T1 functions move: 246 pmf values (lnΓ, inside the kernel below
  k + 1 = 10) and 1,247 cdf values (the incomplete gamma). It has 29,695 rows in all.
- Ten documents serde_json reads only by skipping a member unchecked join `json.map.tsv` (150 rows).

**The result.** Every row passed on the first replay. The replay is ten modules: 264,927 rows ok and 51,768 accepted
(51,618 under ADR 9, 150 under ADR 7), warning-free. Every c1 row is at 0 ulp, against 1.3.0's bits, its correction
or Rust's formula on the T1 functions. All 75 streams equal 1.3.0's bit for bit: neither sampler met a glibc
misround that changed a draw.

**The pmf and cdf against MPFR** (the golden rows at integer k with λ and k + 1 ≤ 10⁵; λ^k e^-λ/k! at 1,024 bits, and
Q(k + 1, λ) by `mpfr_ref.gamma_pq`):

| Rows | 1.3.0 | The port |
|---|---|---|
| the cdf (2,937) | 1,832 exact, max 1,525 ulp | 2,935 exact, all within 1 ulp |
| the pmf (2,937) | 1,101 exact, max 1.96×10⁶ ulp | 1,142 exact, max 1.96×10⁶ ulp |

- **The cdf** inherits T1's incomplete gamma: within 1 ulp everywhere, never less accurate than 1.3.0.
- **The pmf** keeps Rust's form. The port is closer on 270 rows and further off on 47. Both versions' worst rows
  (68 over 1,000 ulp) are at λ = 10⁻³ and k from 40 to 57, where Loader's kernel forms u = (λ − a)/a for a = k + 1:
  1 + u ≈ λ/a is tiny, so ln_1p(u) loses about 16 bits before the product with a. The kernel is M2's
  `ln_gamma_kernel`, Rust's form: a finding for 2.1, where the direct form a·ln λ − λ − lnΓ(a) does not cancel.

**Mutation checks** (`build/handoff/mut_c1.py`: the replay and `tests/dist.tcyr`):

| Mutant | Replay rows failed | Unit checks failed |
|---|---|---|
| Poisson: lambda only finite | 58 | 4 |
| the pmf not refusing off-integers | 157 | 1 |
| the pmf without its e^-λ arm at 0 | 294 | 0 |
| the pmf's kernel at k for k + 1 | 2,027 | 3 |
| the cdf unfloored, Q(x + 1, λ) | 115 | 1 |
| the cdf as 1 − P | 1,005 | 0 |
| PTRS from λ = 30 | 3 | 1 |
| Knuth's count k for k − 1 | 30 | 2 |
| PTRS's b = 0.93 + 2.53√λ | 17 | 0 |
| PTRS's k = ⌊… + 0.5⌋ | 42 | 0 |
| PTRS's lhs without ln(1/α) | 42 | 0 |
| PTRS's quick accept from us = 0.05 (looser) | 19 | 0 |
| PTRS's early reject below us = 0.05 (wider) | 42 | 1 |
| the refusal decoded as `INVALID_PARAMETER` | 51 | 1 |
| Debug named `Poison` | 32 | 2 |
| Knuth stopping on p < e^-λ | 0 | 0 |
| PTRS's quick accept from us = 0.08 (tighter) | 0 | 0 |
| PTRS without its early reject | 0 | 0 |

- **All 15 non-equivalent mutants fail.**
- **The last three are equivalent.** Knuth's product never lands exactly on e^-λ. PTRS's two shortcuts sit inside
  the full test's accept and reject regions: a tighter quick accept, or no early reject, only sends points to the full
  test, which decides them the same way. A looser quick accept, or a wider early reject, changes draws, and the
  streams catch both.

**Tests.** `tests/dist.tcyr` has 490 checks, 50 of them new:
- 1.3.0's tests: `poisson_pmf`, `poisson_pmf_and_cdf_reference` (the 50-digit references), `serde_roundtrip_poisson`,
  `poisson_large_lambda_sample`, `poisson_huge_arguments_are_fast_and_bounded` (cdf(1e18), the 1e12 rate in O(1)),
  `poisson_ptrs_sampling_moments`, and Poisson's lines of `constructors_reject_non_finite_parameters` and
  `deserialize_rejects_invalid_parameters`;
- the oracle's wiring test: above λ = 30 a draw is `sample_poisson_ptrs`', bit for bit and draw for draw;
- Knuth's draw count (a draw of k takes k + 1 uniforms), λ = 5e-324 drawing 0, the constructor's refusals;
- the pmf's and cdf's edges: −0.0, off the integers by an ulp, ∞, a negative NaN returning Rust's quiet `f64::NAN`,
  and the cdf constant between the integers;
- the codec's text and refusal, and Debug's two forms.

**Surface parity.** 11 more items are mapped (138 of `distribution`'s 161), and 23 are pending: Binomial (c2) and
MultivariateNormal (d2).

**Cost** (`docs/benchmarks-rust-v-cyrius.md`): 364–370 ns a pmf (8.5× Rust), 2.55–2.62 µs a cdf (29×: the T1
incomplete gamma), 225–230 ns a Knuth draw (4.2×) and 298–306 ns a PTRS draw (8.5×). The bench's 38 digests are all
explained by the models.

### 9.7 Part c2: Binomial (2026-11-16)

**The module.** `src/dist_disc.cyr` gains `PmnBinomial { n, p }`, with the methods, codec and Debug of the others. n is
a u64, held as its 64-bit word, so every comparison, halving and conversion of it is unsigned (`_pmn_u64_lt`, `>>`,
`_pmn_u64_to_f64`, M1's helpers). n = u64::MAX works end to end.
- **The pmf:** NaN at NaN; 0 below 0, off the integers and above n; the point masses at p = 0 and 1; (1 − p)^n and
  p^n at the ends; else Loader's saddle point, exp(lc − ½·ln(2πk(n − k)/n)) with lc of three Stirling remainders and
  two deviances (bd0), O(1) in n, in Rust's order.
- **The cdf:** I_(1−p)(n − k, k + 1) at k = ⌊x⌋, on the T1 incomplete beta with both arguments exact (1 − p and p),
  no O(n) sum; 0 below 0, 1 from n, and the point masses' at p = 0 and 1.
- **The draws:** above 64 trials, Knuth's Beta splitting halves them in O(log n). The a-th smallest of the trials'
  uniforms, a = 1 + ⌊trials/2⌋, is Beta(a, trials − a + 1), drawn as a ratio of two Marsaglia–Tsang variates, and
  the successes lie below it, or all a of them are counted and the rest lie above. The last trials are simulated, a
  success when a uniform is below p.

**The golden rows.**
- **`dist`, 8,070 more rows (163,056 in all):**
  - the constructor on eleven n (0, 1, 2, 10, 64, 65, 1000, 2⁵³, 2⁵³ + 1, 2⁶³ and u64::MAX) by 23 p (the 20-value
    grid, 1 − 2⁻⁵³, 1 + ε and 0.3); class A. n is an `i:` token holding the word, so u64::MAX is `i:-1`;
  - `Binomial::pdf` and `Binomial::cdf`, 3,826 rows each: seventeen fixed (n, p) (n from 1 to u64::MAX either side of
    64 trials, p = 0, 1, 1 − 2⁻⁵³, 1e-300 and 5e-324 among them) at k = 0 … 40 and n − k for k ≤ 10, at the mean
    ± up to 12 standard deviations, at n and n + 1, off the integers, at −1, −0.0 and the specials; 40 seeded pairs
    around their means; 1,000 seeded triples, a tenth off the integers. Class C;
  - the mean and variance, class A;
  - 51 streams: 10⁴ draws of Binomial(10, 0.3) from three seeds and 256 of each other fixed pair, simulated and
    Beta-split, u64::MAX trials among them. Class C.
- **`json`,** 226 more rows (9,265 in all): `struct_docs`'s variants on the u64 field's 14 integer forms (0, −1, 1.0,
  2⁶³, 2⁶⁴ and non-numbers among them) and the f64 field's 25, the sequence forms, and seeded documents; and the
  writes. **`debug`,** 32 more (25,688), n printed as its unsigned decimal.

**The models.** `validate.py` models every row in Rust's operation order, with n as a Python integer modulo 2⁶⁴:
the saddle point on 1.3.0's Stirling remainders and bd0, the cdf on 1.3.0's incomplete beta or T1's, and the
splitting sampler.
- **`dist.cr.tsv`** gains 135 rows where glibc misrounds: 129 pmf values and 6 cdf values (1,270 rows in all).
- **`dist.ref.tsv`** gains 1,742 rows that the T1 functions move: 361 pmf values (lnΓ, inside Stirling's remainder
  below 10) and 1,381 cdf values (the incomplete beta). It has 31,437 rows in all.
- Ten documents serde_json reads only by skipping a member unchecked join `json.map.tsv` (160 rows).

**The result.** Every row passed on the first replay. The replay is ten modules: 271,368 rows ok and 53,655 accepted
(53,495 under ADR 9, 160 under ADR 7), warning-free. Every c2 row is at 0 ulp, against 1.3.0's bits, its correction
or Rust's formula on the T1 functions. All 51 streams equal 1.3.0's bit for bit, the u64::MAX ones included.

**The pmf and cdf against MPFR** (the golden rows with n ≤ 10⁵, 0 < p < 1 and integer k; C(n, k)·p^k·(1 − p)^(n−k)
at 1,024 bits, and I_(1−p)(n − k, k + 1) by `mpfr_ref.beta_ic`):

| Rows | 1.3.0 | The port |
|---|---|---|
| the cdf (2,363) | 1,088 exact; NaN on 13 rows where it is 1 | 2,360 exact, all within 1 ulp |
| the pmf, finite (2,435) | 616 exact, max 4,117 ulp | 618 exact, max 4,877 ulp |
| the pmf at p = 5e-324, k ≥ 1 (40) | NaN | NaN |

- **The cdf** inherits T1's incomplete beta: within 1 ulp everywhere, never less accurate than 1.3.0. 1.3.0's far
  misses are Binomial(30, 5e-324) at k ≥ 9, NaN where the cdf is 1; the port gives 1.
- **The pmf** keeps Rust's saddle point. Its error is exp amplifying lc's rounding in the tails. The port is closer
  on 318 rows and further off on 160: the correctly rounded primitives move a last-ulp rounding either way.
- **At p = 5e-324,** 1.3.0's pmf is NaN for k ≥ 1, where the true value is a tiny subnormal, and the port's is too,
  as parity requires: in bd0(k, n·p) at n·p = 1.5e-322, u = (k − n·p)/(n·p) overflows to ∞, and
  (1 + u)·ln_1p(u) − u is ∞ − ∞. A finding for 2.1.

**Mutation checks** (`build/handoff/mut_c2.py`: the replay and `tests/dist.tcyr`):

| Mutant | Replay rows failed | Unit checks failed |
|---|---|---|
| Binomial: n = 0 accepted | 13 | 2 |
| the pmf with n converted signed | 91 | 0 |
| the pmf without the p = 0 arm | 19 | 1 |
| the pmf without the k = 0 arm | 212 | 2 |
| the pmf with ln(1 − k/n) for ln_1p(−k/n) | 14 | 0 |
| the pmf's lc − (bd0 + bd0) | 323 | 0 |
| the pmf with ln(2π) for 2·ln √(2π) | 186 | 0 |
| the cdf's 1 only past n | 185 | 2 |
| the cdf with y = 1 − (1 − p) | 83 | 1 |
| the cdf with k unfloored | 76 | 1 |
| the variance as n·(p·(1 − p)) | 11 | 1 |
| the draw simulated only to 63 trials | 3 | 2 |
| the split loop's test signed | 3 | 1 |
| a = ⌊trials/2⌋ | 21 | 1 |
| p − x unscaled after a split | 18 | 2 |
| a trial succeeding on u ≤ p | 0 | 2 |
| x > p for the lower half | 0 | 1 |
| the JSON names read swapped | 69 | 5 |
| n written as an f64 | 43 | 4 |
| the refusal decoded as `INVALID_PARAMETER` | 55 | 2 |
| Debug named `Binomia` | 32 | 2 |

- **All 21 fail.** The unsigned handling holds both ways: a signed conversion of n, or a signed test in the split
  loop, fails the u64::MAX and 2⁶³ rows.
- **Two fail through unit tests only:** they differ only at an exact tie, a uniform equal to p or a split at x = p,
  which no seeded stream draws. A consumer's generator of eighths gives the first (u = 0 at p = 0, u = ½ at p = ½).
  A generator of period 3 gives the second: both gamma variates take the same three uniforms, so x is exactly ½, and
  Rust keeps the 32 trials below it at p/x = 1, where the mutant counts 33.

**Tests.** `tests/dist.tcyr` has 550 checks, 60 of them new:
- 1.3.0's tests: `binomial_mean_and_variance`, `binomial_degenerate_p`,
  `binomial_pmf_matches_exact_and_cdf_matches_sum` (against `pmn_combinations` and powi), `serde_roundtrip_binomial`,
  `binomial_large_n_is_fast_and_accurate` (the symmetric cdf, the central pmf, u64::MAX trials in O(log n)),
  `binomial_large_n_sampling_moments`, and Binomial's lines of `constructors_reject_non_finite_parameters` and
  `deserialize_rejects_invalid_parameters`;
- the constructor's refusals, n = 2⁶³ (a negative word) and its unsigned mean;
- the draws: 64 trials simulated with 64 uniforms and bit for bit; one split at 65 with fewer than 65; the two ties;
- the pmf's and cdf's edges, and the cdf as `beta_inc(n − k, k + 1, 1 − p, p)` bit for bit;
- the codec (n an integer, u64::MAX read and written back unsigned, a float n refused) and Debug's two forms.

**Surface parity.** 11 more items are mapped (149 of `distribution`'s 161), and 12 are pending: MultivariateNormal's
(d2).

**Cost** (`docs/benchmarks-rust-v-cyrius.md`): 681–708 ns a pmf (8.3× Rust), 7.15–7.28 µs a cdf (39×: the T1
incomplete beta), 605–622 ns a simulated draw of 50 trials (2.6×) and 7.9–8.1 µs a Beta-split draw of 10⁶ (5.9×). The
bench's 42 digests are all explained by the models.

### 9.8 Part d1: the Cholesky factor and solve (2026-11-16)

**The module.** `src/linalg.cyr` holds hisab 1.4.0's `cholesky` and `cholesky_solve`, the two linear-algebra routines
1.3.0 calls: for MultivariateNormal (d2), the ARIMA Yule–Walker fit (M5) and the logistic regression's Newton step
(M4). It is ledger L4's stand-in, and carries the notes for L5 and L6, whose routines land with their first callers.
- **Term for term:** each sum starts from the matrix entry and subtracts the products k = 0, 1, … in turn. The
  diagonal is the correctly rounded √, and an off-diagonal entry is one division. ganita's factor sums in another
  order (probe P5: up to 40 ulp from 1.4.0), so it would not pass.
- **On caller storage (D11):** a matrix is a flat row-major n × n f64 array, and a vector n f64s. The factor fills
  out, with zeros above the diagonal. The solve's y and x share out, which may be b itself. hisab returns a fresh
  `Vec<Vec<f64>>` per call, and ganita's solve leaves 8n bytes, which an MVN pdf in a loop would leak.
- **The refusals:** hisab's one error, InvalidInput, is `PMN_ERR_INVALID_PARAMETER`. The factor refuses a pivot ≤ 0
  and n = 0. A NaN pivot is accepted, as hisab's is (NaN ≤ 0 is false). The buffer rules add null buffers, n past
  2²⁶, and three overlaps: out over A, out over L, and out partly over b. hisab's shape checks (a ragged row, a b of
  another length) have no counterpart, since a flat array with its n cannot be ragged.

**The golden rows.** `linalg` is a new module, 682 rows, all class A (arithmetic and the correctly rounded √).
- **`linalg::cholesky`, 70 rows:**
  - the empty matrix;
  - 1×1 at every pivot class: 4, 1, 2, ±0, −1, NaN, ±∞, 5e-324 and f64::MAX;
  - nine 2×2: the identity, an SPD, a singular and an indefinite one, a non-symmetric one (its upper triangle
    ignored), NaN and ∞ off the diagonal, 1e-300 and 1e300 on it, and a negative one;
  - seeded B·Bᵀ + n·I and B·Bᵀ + 10⁻³·I for n from 3 to 64, the Hilbert matrices from 3 to 14, and Lehmer's and the
    [2, −1] tridiagonal at n = 3, 7 and 16, the tridiagonal also scaled by 1e300, 1e-300 and −1.

  13 are refused, as hisab refuses them: the empty matrix; the 1×1 pivots ±0, −1 and −∞; the singular, indefinite
  and negative 2×2; the one with ∞ off the diagonal, whose second pivot is 3 − ∞; Hilbert's at n = 14, where rounding
  loses definiteness; and the three negative tridiagonals.
- **`linalg::cholesky_solve`, 612 rows:** every factor above against the unit vectors (up to 8), three seeded right
  sides, a zero one and one with a NaN. Two hand factors: one with a zero diagonal, which divides by zero (±∞ or NaN),
  and one with junk above the diagonal, never read. And n = 0.

**The model.** `validate.py` recomputes every row in hisab's order with Python's floats, whose arithmetic and √ are
IEEE's (`lin_cholesky`, `lin_cholesky_solve`). Every row agrees.

**The result.** Every row passes at 0 ulp, warning-free, on x86_64 ELF, PE and aarch64. The replay is eleven modules:
272,050 rows ok and 53,655 accepted (53,495 under ADR 9, 160 under ADR 7).

**Mutation checks** (`build/handoff/mut_d1.py`: the replay and `tests/linalg.tcyr`):

| Mutant | Replay rows failed | Unit checks failed |
|---|---|---|
| the factor: a zero pivot accepted | 3 | 1 |
| the factor: n = 0 accepted | 1 | 1 |
| the factor: the products in reverse order | 31 | 0 |
| the factor: sum·(1/l_jj) | 34 | 0 |
| the factor: the upper triangle not zeroed | 43 | 3 |
| the factor: A's upper triangle read | 2 | 5 |
| the factor: out over A not refused | 0 | 1 |
| the solve: forward y·(1/l_ii) | 273 | 0 |
| the solve: the forward products in reverse | 218 | 0 |
| the solve: back substitution reading L, not Lᵀ | 450 | 5 |
| the solve: the back products in reverse | 299 | 0 |
| the solve: in place refused | 0 | 2 |
| the solve: out over L not refused | 0 | 2 |

- **All 13 fail.**
- **Five fail through the replay only:** a reordered sum or a reciprocal moves a last bit, which hisab's own tests,
  at a tolerance of 10⁻¹⁰, cannot see.
- **Three fail through unit tests only:** the buffer rules, which no golden row can express.

**Tests.** `tests/linalg.tcyr` has 43 checks:
- hisab 1.4.0's own tests: `cholesky_2x2`, `cholesky_solve_2x2`, `cholesky_3x3_identity`,
  `cholesky_not_positive_definite` and `cholesky_solve_3x3_verify`;
- L·Lᵀ = A, zeros above the diagonal, and the upper triangle never read: junk there gives the same bits, in both;
- the buffer rules: null buffers; n = 0, refused by the factor and nothing to do for the solve; n < 0; out on A, over
  it and just past it; out inside L; out partly over b; and the in-place solve, bit for bit the out-of-place one;
- the 1×1 edges: [4] gives 2, [0] is refused, [NaN] is accepted, [∞] gives ∞, and a zero diagonal's solve gives +∞;
- a 64×64 [2, −1] tridiagonal solve, x_i = i(65 − i)/2 to within 10⁻⁷.

`tests/soak.tcyr` (18 checks) adds 10⁴ factorizations and solves on caller buffers, n from 1 to 24. Nothing is taken:
the bump heap, the freelist arena, the mapped size and the pool are unchanged.

**Surface parity.** Nothing to map. 1.3.0 reaches hisab through `pub use hisab`, a Rust re-export that 2.0 does not
carry (port plan §4.2).

**Cost** (`docs/benchmarks-rust-v-cyrius.md`): 145 ns, 3.7 µs and 181–182 µs a factor at n = 4, 16 and 64 (1.6×, 3.6×
and 5.3× Rust), and 108 ns, 980 ns and 13.8–13.9 µs a solve (1.9×, 3.1× and 4.3×). The bench's 12 sums and digests
are 1.4.0's, bit for bit.

### 9.9 Part d2: MultivariateNormal (2026-11-16)

**The module.** `src/dist_mvn.cyr` holds `PmnMultivariateNormal`, with Rust's `new`, `dim`, `pdf`, `log_pdf`, `sample`,
`mean_vec` and `covariance_matrix`, Clone, the codec and Debug. It is the one owned distribution (D11).
- **The storage:** the struct lives in caller storage, and its buffers in one freelist block sized at construction: the
  mean, the covariance and the lower factor L. `PmnMultivariateNormal_drop` frees the block, and a dropped one refuses
  every call. Clone is `PmnMultivariateNormal_clone`, a block of its own.
- **The covariance:** Rust's `Vec<Vec<f64>>` is a flat row-major array with its rows and columns (port plan §8.3), so
  the constructor still tells a covariance of the wrong shape (DimensionMismatch) from a bad one (InvalidParameter), in
  Rust's order. Only a JSON document can be ragged, and it reads as PMN_ERR_DESERIALIZE, as in Rust.
- **The values:** the factor is part d1's, hisab's bit for bit. log det Σ is 2·Σ ln L_ii, summed from −0.0. The
  densities solve L·Lᵀ·w = x − μ with hisab's solve. A draw is μ + L·z, with z from Box–Muller's cosine and sine
  branches; an odd d draws the last pair whole and keeps its cosine. All are in Rust's operation order, on the
  correctly rounded ln, exp, cos and sin.
- **The calls:** `pdf` and `log_pdf` return a value-form Result (ADR 007 row 1), Ok holding the f64. Their scratch, 2d
  f64s, comes from the work-table pool. A draw fills a caller buffer of d f64s (ADR 007 row 3), computed in place,
  rows from the last up, so each row reads only z values not yet replaced.
- **The codec:** serde_json's text carries the cached fields, as 1.3.0 writes them. Reading takes the two-field repr
  and the constructor, which recomputes the factor, as Rust's `try_from` does: a document's cached fields are ignored.
- **Debug** nests lists, so `src/debug.cyr` gains `_pmn_dbg_vec_f64` and `_pmn_dbg_mat_f64`. Their pretty form
  indents each level by four spaces, as Rust's PadAdapter does.

**The golden rows** (`scripts/golden/src/m_mvn.rs`, through the oracle's `mvn_factor` for L and log det Σ):
- **`dist`, 3,575 more rows (166,631 in all):**
  - `MultivariateNormal::new`, 139 rows: the empty mean; a 1×1 covariance at every pivot class and a mean at every
    special; the wrong shapes, the mean's check first; a NaN or ±∞ at each entry; asymmetries either side of the
    tolerance, at its MIN_POSITIVE floor, at a pair the larger magnitude decides, and on 1.3.0's test matrices;
    matrices hisab refuses, and one it factors with a NaN pivot (the degenerate determinant); seeded SPD matrices for
    d from 1 to 32, scaled by 1e300, 1e-300, 5e-324 and f64::MAX, the Hilbert matrices to 14 and Lehmer's. 84 factor,
    44 are InvalidParameter and 11 DimensionMismatch;
  - `log_pdf` and `pdf`, 1,720 and 1,692 rows: each valid case at its mean, along each axis, at seeded points near and
    far, with a NaN or ±∞ element, and at points of the wrong length (140 DimensionMismatch each). Past d = 8 only the
    mean, one seeded point and a far one, and the pdf at the mean: each row carries the d² covariance;
  - `sample`, 24 rows: 2,000 draws of 1.3.0's own test case and 256 of seven others (d = 1 to 8, −0.0 and f64::MAX
    means, scales of 1e300 and 1e-300), from three seeds.

  All class C: log det Σ and the densities take glibc's ln and exp, the draws its ln, cos and sin.
- **`json`,** 127 more rows (9,392 in all): `struct_docs`'s variants of the two-field repr, the mean in 15 forms and
  the covariance in 22 (ragged, asymmetric, not positive-definite, non-numbers among them), documents with the cached
  fields true, tampered and of the wrong type, the sequence forms, and 12 documents serde_json wrote; 64 are refused.
  And 12 writes. **`debug`,** 24 more (25,712): the codec's 12 cases in both forms.

**The models.** `validate.py` recomputes every row in Rust's order on part d1's `lin_cholesky` and
`lin_cholesky_solve`: the constructor's checks, log det Σ, the densities and the draws, on glibc's primitives and on
correctly rounded ones. The codec's model reads the repr and judges it by the constructor; its writes and Debug compute
the factor and log det Σ, and stop if glibc's ln and the correctly rounded one disagree on a case (none does).
- **`dist.cr.tsv`** gains 12 rows: draws where glibc's ln, cos or sin misrounds (1,282 in all). 7 move by 1 ulp, 3 by
  2, and 2 by 128 and 512, where μ_i + Σ L_ik·z_k cancels.
- **`dist.ref.tsv`** is unchanged: no T1 function is involved.
- **`json.map.tsv`** gains 10 rows: the documents serde_json reads only by skipping an unread member (ADR 007 row 16).

**The result.** Every row passes at 0 ulp, against 1.3.0's bits or their correctly rounded value, warning-free, on
x86_64 ELF, PE and aarch64. The replay is eleven modules: 275,754 rows ok and 53,677 accepted (53,507 under ADR 9,
170 under ADR 7). All 24 streams equal 1.3.0's but where glibc misrounds.

**The densities against exact values** (the 1,266 finite log-density rows and 939 density rows with d ≤ 8: q and
det Σ in exact rationals, Σ the symmetric matrix of the lower triangle hisab factors, then the logarithms and exp in
MPFR at 4,096 bits; κ is the largest squared pivot of L over the smallest, a proxy for the condition number):

| Rows | log_pdf | pdf |
|---|---|---|
| κ ≤ 100 | 853: 473 exact, 780 within 1 ulp, max 25 | 759: 100 exact, max 8,275 |
| 100 < κ ≤ 10⁴ | 66: max 400 | 33: max 141,787 |
| κ > 10⁴ (B·Bᵀ + 10⁻³·I) | 56: max 1.6×10⁸ (relative 2×10⁻⁸, κ = 1.8×10⁸) | 12: max 3.1×10⁹ |

- **The port's values are 1.3.0's** on every density row (no `dist.cr.tsv` row for them), so the table is both's.
- **The error is the factor's:** Cholesky is backward stable, so the Mahalanobis term's error grows with κ. The
  density's is exp amplifying the log density's absolute error.
- **Not a finding against the port:** parity fixes the method. A better-conditioned route, such as a refined solve,
  would be 2.1's to propose.

**Mutation checks** (`build/handoff/mut_d2.py`: the replay of `dist`, `json` and `debug`, and `tests/dist_mvn.tcyr`):

| Mutant | Replay rows failed | Unit checks failed |
|---|---|---|
| new: the shape's check before the mean's | 1 | 1 |
| new: the columns unchecked | 3 | 2 |
| new: asymmetry refused at the tolerance itself | 102 | 0 |
| new: no MIN_POSITIVE floor on the scale | 357 | 0 |
| new: the scale the smaller magnitude | 102 | 0 |
| new: a non-finite log det accepted | 2 | 1 |
| log_pdf: (log det + Mahalanobis) first | 304 | 0 |
| log_pdf: d·ln 2π summed term by term | 83 | 0 |
| log_pdf: no length check | 280 | 3 |
| log_pdf: the solve in place | 2,338 | 2 |
| log_pdf: the dot product with x, not x − μ | 756 | 1 |
| pdf: the log density returned | 1,373 | 5 |
| sample: u1 not kept from 0 | 0 | 1 |
| sample: sine first | 24 | 2 |
| sample: rows from the first down (z overwritten) | 15 | 1 |
| sample: L read transposed | 15 | 0 |
| sample: no cap check | 0 | 1 |
| sample: no overlap check | 0 | 1 |
| debug: a nested list at its parent's depth | 12 | 1 |
| debug: log_det before cholesky_l | 24 | 2 |
| to_json: dim written as an f64 | 12 | 1 |
| from_json: the constructor's code passed through | 18 | 2 |
| from_json: the cached factor's names read as fields | 37 | 2 |
| clone: only the mean copied | 0 | 2 |
| drop: the block kept on the struct | 0 | 3 |

- **All 26 non-equivalent mutants fail.** Three more are equivalent, and survive:
  - log det as Σ 2·ln L_ii: doubling is exact, so fl(2a + 2b) = 2·fl(a + b);
  - the factor from the input rather than its copy: the same bits;
  - ln 2π as 2·ln √(2π): the same bits, as is ln 2 + ln π, for this one constant.
- **The scale of the smaller magnitude** was a survivor until a case was found for it. The verdicts differ only when
  |a − b| lies between 1e-10·|b| and 1e-10·|a|, a window about 10⁻²⁰ wide that about one double in 10⁵ meets. A search
  found (1.7117351981710087, 1.7117351979998352), which Rust accepts, and both orientations are now golden rows.
- **Five fail through unit tests only:** the draw's two buffer rules, the clone and the drop, and u1 = 0, which no
  seeded stream draws and a consumer's generator of eighths does. **Six fail through the replay only:** the symmetry
  check's three, two orders of the log density's sum, and L read transposed, which move bits that tolerance-based tests
  can't see.

**Tests.** `tests/dist_mvn.tcyr` has 67 checks:
- 1.3.0's tests: `mvn_1d_matches_univariate`, `mvn_2d_pdf_at_mean`, `mvn_sample_mean_convergence` (50,000 draws),
  `mvn_invalid_params`, `mvn_dimension_mismatch_pdf`, `mvn_log_pdf_consistency`, `serde_roundtrip_mvn`,
  `mvn_deserialize_recomputes_cached_factor`, `mvn_symmetry_check_is_scale_relative`, and MultivariateNormal's lines
  of `constructors_reject_non_finite_parameters` and `deserialize_rejects_invalid_parameters`;
- the constructor's order and buffer rules, a refusal leaving out untouched, the degenerate determinant, the factor
  from the lower triangle, the covariance and mean kept as given, and out over the mean;
- the block's life: a clone with a block of its own outliving its source, and every call on a dropped one refused;
- the densities: −½·2·ln 2π at N(0, I₂)'s mean bit for bit, pdf as exp(log_pdf) bit for bit, the far point's −∞, NaN,
  the refusals, and the pool left unclaimed;
- the draws: N(0, 1)'s stream equal to `standard_normal`'s, two pairs for d = 3, the buffer rules drawing nothing, and
  a consumer's generator whose first uniform is 0;
- the codec's and Debug's exact text, the tampered cache, the sequence form and the refusals.

`tests/soak.tcyr` (23 checks) adds 10⁴ rounds of the constructor, both densities, a draw, a clone and the codec both
ways, a tenth of them at d = 32, whose block is over 4 KB, mapped and unmapped. Nothing is taken: the bump heap, the
freelist arena, the mapped size and the pool are unchanged, once a warm-up has grown the pool's slots to the largest
document.

**Surface parity.** The 12 pending items are mapped: `distribution`'s 161 are all ported (191 items in all), and the
module moves to the ported list at M3's gate.

**Cost** (`docs/benchmarks-rust-v-cyrius.md`): built once, 314–319 ns a 3-D pdf (4.9× Rust), 250–253 ns a log_pdf
(4.4×) and 559–570 ns a draw (6.2×); 761–788 ns a 3-D construction and drop (4.7×); 1.27–1.31 µs a d = 16 log_pdf
(3.8×); 40–43 µs a JSON round trip (27×, the codec's cost, as Normal's). The bench's 48 digests are all explained by
the models.

### 9.10 Part e: M3's gate (2026-11-16)

Every part ran the per-part gates: build; test and test --bounds, warning-free; bench, recorded against Rust; vet,
lint and fmt --check; distlib; deps --locked; audit; the ledger, surface parity, the vendor and constant checks; and
the three targets. The plan's §7 adds what M3's gate covers across its parts:
- its golden grid's minimum;
- the gates every milestone ends with: surface parity, the soak for anything that allocates, the JSON and Debug rows,
  and the ledger.

M2's gate (§8.10) adds the audit of the milestone's sources. This gate also weighed the findings the parts left for
it.

**The grid's minimum** (plan §7, M3's row), against the corpus:

| The plan asks | The corpus |
|---|---|
| pdf, cdf, mean and variance over each parameter range and its edges | all thirteen distributions' four methods, and MultivariateNormal's two densities: grids across the support and beyond, every edge with its neighbours, the specials, seeded points (parts a1–d2) |
| 10⁴-draw streams per sampler at three seeds | each distribution's first case; and now each further algorithm's (below) |
| every constructor refusal | 3,153 constructor rows, 2,125 of them refusals, on every parameter's specials and its checks' edges |
| JSON with invalid parameters refused | 2,437 documents for the fourteen types, 1,285 refused: invalid parameters, wrong types, syntax faults |

- **Four samplers had no 10⁴-draw stream:** Gamma's Ahrens–Dieter boost below α = 1 and PTRS had 256 draws a seed,
  Beta splitting 256, and MultivariateNormal's draws 2,000 vectors.
- **The gate gives each 10⁴:** Gamma(0.5, 1), Poisson(100), Binomial(1000, 0.5), and N([1, −2], [[1, 0.3],
  [0.3, 2]]) as vectors, from the three seeds. Twelve rows change, and the replay stays at 0 ulp.
- **`dist.cr.tsv` gains a row (1,283):** the longer Gamma and MultivariateNormal streams meet more glibc misrounds,
  moving up to 8 and 32 ulp where a sum cancels.
- **The PTRS and splitting streams** equal 1.3.0's draw for draw, even on T1's lnΓ.

**The audit of M3's sources** (`dist.cyr`, `dist_cont.cyr`, `dist_gamma.cyr`, `dist_disc.cyr`, `dist_mvn.cyr`,
`linalg.cyr`, and `debug.cyr`'s list writers), against CLAUDE.md's library rules:
- **No aborts:** no `+?`, `-?` or `*?`, no `assert_fatal`, `panic` or `result_unwrap`, no `vec_get`, no
  `alloc_reset`.
- **Allocation in one place:** MultivariateNormal's block, on the freelist and freed by its drop, and pool scratch
  for its densities and its decoder. Everything else writes caller storage.
- **Every transcendental correctly rounded:** none of the stdlib's or ganita's plain transcendentals. The only stdlib
  float functions are `f64_sqrt`, `f64_floor`, `f64_abs` and the conversions.
- **Constants generated or proven:** no hex in a `const`. The one inline word, ln 2π, is proven (below).
- **Names:** every top-level name carries the pmn prefix.
- **`#must_use`** is on every function that returns a value, but the JSON decoders passed by pointer (as M1's are)
  and the drop.
- **`#pure`** is now on the seven value-only helpers: the three parameter checks, Weibull's g, and the t and F tails.
  The compiler checks that a `#pure` function calls nothing `#io` or `#alloc`. The library, built with dead-code
  elimination off, is byte-identical before and after (1,214,784 bytes).
- **Null and aliasing:** every function with an error channel checks its pointers. The methods that return a bare f64
  (`pdf`, `cdf`, `mean`, `variance`, `sample`, as M1's `next_f64`) take a non-null object, as Rust's references are.

**The findings the parts left for the gate.**
- **The constructors' checks (part a2):** they were calls, about 24 ns of a simple row. `#inline` is the fix, but the
  compiler declines a body with control flow or a `var`. So the three checks and `_pmn_is_finite` are now one
  expression each, on 0/1 comparisons.
- **ln 2π (part d2):** MultivariateNormal's densities computed a correctly rounded ln on every call, where LLVM folds
  Rust's `(2.0 * PI).ln()`. It is now `_PMN_MVN_LN_2PI`, the ln of the double 2π. `scripts/crverify/consts.py`
  proves it from a 90-digit definition, MPFR agrees, and `tests/dist_mvn.tcyr` checks it against `_pmn_ln`.

Both were measured before adoption: the rows they touch, interleaved, pinned to core 5, three rounds each way. All 26
sums and digests are the same both ways.

| Row | Before | After | Change |
|---|---|---|---|
| `Uniform::cdf`, U(−1, 5) | 40 ns | 32 ns | −20% |
| `Cauchy::pdf`, Cauchy(0, 1) | 45 ns | 37 ns | −18% |
| `Normal::pdf`, N(0, 1) | 90–91 ns | 83 ns | −8% |
| `Exponential::cdf`, Exp(2.5) | 81 ns | 76 ns | −6% |
| `Bernoulli::cdf`, p = 0.3 | 30–31 ns | 29 ns | −4% |
| `Weibull::cdf`, `Gamma::pdf`, `Poisson::pdf` | 234–373 ns | 226–366 ns | −2% to −4% |
| `Beta::pdf`, `Binomial::pdf` | 689–786 ns | 681–787 ns | under 1% |
| `MultivariateNormal::pdf`, 3-D | 310–313 ns | 236–239 ns | −24% |
| `MultivariateNormal::log_pdf`, 3-D | 246–249 ns | 176–178 ns | −28% |
| `MultivariateNormal::new` and drop, 3-D | 742–756 ns | 716–726 ns | −4% |

Five mutants check the change (`build/handoff/mut_e.py`, the replay of `dist` and `json`, and `tests/dist.tcyr`). All
fail:
- `_pmn_is_finite` testing only for NaN: 202 rows and 15 checks;
- `_pmn_require_positive` with `|` for `&`: 1,679 and 46; with v ≥ 0: 278 and 20;
- `_pmn_require_probability` with v < 1: 288 and 6;
- the held ln 2π one ulp up: 402 rows, and `consts.py`.

**The other findings** stay findings:
- **The accuracy ones** are bound by parity in 2.0.0, so they are now the roadmap's (Numerical improvements): Weibull's
  variance (a3), Gamma's pdf (b1), Poisson's pmf at k ≫ λ (c1), Binomial's pmf at p = 5e-324 (c2), and
  MultivariateNormal's ill-conditioned densities (d2).
- **The performance ones** are another project's or another ADR's: the solve's per-term cost is the compiler's
  register use (d1); the JSON round trip is `fmtf.cyr`'s exact digit search (ledger L10); and T1's cost is ADR 009's.

**The soak.** `tests/soak.tcyr` gains a group: 10⁴ rounds of the thirteen caller-storage distributions, each one's
constructor, five methods, JSON both ways and Debug in both forms, the parameters moved round by round. The bump heap,
the freelist's arena, the mapped size and the pool are unchanged, and every pdf and cdf is finite, on x86_64 ELF, PE
and aarch64. With parts d1 and d2's groups, everything M3 ports is soaked. `tests/soak.tcyr` has 28 checks.

**Surface and Debug.** `distribution` moves to surface parity's ported modules: its 161 items are all mapped (191 in
five modules, none pending). Every one of the fourteen types has its Debug rows and both codec directions.

**The ledger.** 14 rows and 21 notes in `src/`. Every one of the 21 upstream requests is still open (re-read
2026-11-16), so nothing converts.

**M3's result.** The fourteen distributions of 1.3.0's `distribution.rs` are ported, with hisab's Cholesky factor
and solve:
- **The corpus:** `dist`, 166,631 rows for the fourteen types and three oracle hooks; 2,993 codec rows and 452 Debug
  rows for them; `linalg`, 682 rows. Every row passes at 0 ulp, against 1.3.0's bits, its correction where glibc
  misrounds (`dist.cr.tsv`, 1,283 rows), or Rust's formula on the T1 functions (`dist.ref.tsv`, 31,437 rows).
- **The replay:** eleven modules, 275,753 rows ok and 53,678 accepted (53,508 under ADR 9, 170 under ADR 7),
  warning-free, on x86_64 ELF, PE (wine) and aarch64 (qemu), with every result line identical on the three.
- **Accuracy, against MPFR:**
  - every cdf on a T1 function is within 1 ulp on its measured rows, and never less accurate than 1.3.0: Gamma's and
    χ²'s, Beta's, StudentT's and F's, Poisson's and Binomial's (parts b1–c2), and Normal's on Φ (a1);
  - Weibull's mean is within 433 ulp (1.3.0: 1,268), on T1's lnΓ;
  - the pdfs keep Rust's formulas, so their errors are 1.3.0's but for the last-ulp moves of the correctly rounded
    primitives. The worst are now the roadmap's.
- **Tests:** `tests/dist.tcyr` (550 checks), `tests/dist_mvn.tcyr` (68), `tests/linalg.tcyr` (43); `tests/soak.tcyr`
  (28) soaks all of M3.
- **Cost** (`docs/benchmarks-rust-v-cyrius.md`, the gate's re-measure of `tests/dist.bcyr`'s 48 rows): a median of
  8.0× Rust, inside §9's expected 5–9×.
  - **Without a T1 function,** densities run 3.1–9.9× and draws 2.6–10×, at or under §9's predicted 6–15× and 5–15×.
  - **The T1 cdfs** run 22–115× (median 81×): the double-double incomplete gamma and beta, whose cost ADR 009
    accepts.
  - **The codec rows** run 24–42×: `fmtf.cyr`'s digit search (ledger L10).

## 10. M4: descriptive statistics and hypothesis tests

M4 ports 1.3.0's `descriptive.rs` and `hypothesis.rs` under ADR 010: the golden rows come from Python's models of
1.3.0 (`scripts/golden/validate.py`), written by `gen.py` and never committed. Its parts are in the plan's §7. Every
row of every part passed on its first replay but where noted, at 0 ulp, on x86_64 ELF, PE (wine) and aarch64 (qemu).

### 10.1 Parts a1–a3: the statistics, KDE, correlation, the Jacobi and the PCA (2026-11-16/17)

- **a1** (`src/sort.cyr`, `src/descriptive.cyr`): 23,180 `descriptive` rows. The order statistics' stable rule (ADR
  007 row 14) is pinned by probe P6's zero-heavy slices: a mutant that takes the quickselect's own zero fails 841 rows.
  The stdlib's byte-wise `memcpy` cost the 10⁴-element median two thirds of its time (ledger L15).
- **a2** (`src/kde.cyr`): 1,212 more rows; 3 where glibc's exp misrounds go to `descriptive.cr.tsv`, and the port
  gives the correctly rounded value.
- **a3** (`src/linalg.cyr`'s Jacobi, ledger L6; `src/pca.cyr`): 123 `linalg` and 78 `descriptive` rows. The Jacobi is
  chaotic in its angle, so a glibc misround of atan, cos or sin anywhere in the sweeps moves the result: 9 `linalg` and
  3 PCA rows go to the `.cr.tsv` files, each matched at 0 ulp. The π/4 threshold at |w_pp − w_qq| = 1e-12 is visible
  only after a single sweep (at convergence both branches give the same bits), so its rows run one sweep.
- **A target dependence, found on aarch64** (ADR 007 row 17): overflowing data makes an eigenvalue NaN through 0·∞,
  with the hardware's default NaN sign, and `total_cmp` ordered PCA's components by it. The port now sorts every NaN
  eigenvalue as x86's default NaN, and the model checks on each row that this is 1.3.0's x86 order.

### 10.2 Parts b1–b2: the tests, the intervals and the quantiles (2026-11-17)

- **The p-values** come from the T1 incomplete beta and gamma, so `hypothesis.ref.tsv` holds 414 rows where 1.3.0's
  value moves: the one-sample t-test by up to 391 ulp, Welch's by 165, the χ² test by 219, ANOVA by 100, the
  one-sample KS test (through Φ) by 53. The port is ADR 009's T1 functions on Rust's formulas, at 0 ulp.
- **The quantiles, against MPFR at 1,024 bits** (the error as (Φ(z) − p)/φ(z), in ulps of z; 6,402 p, log-spaced to
  10⁻³⁰⁰ and 4,000 uniform):
  - `z_quantile`, below ½: within 0.5 ulp on 2,427 of 3,215, within 4 on 3,129, at most 387 (near ½, where Φ(x) − p
    cancels);
  - above ½: at most 9.4×10⁶ ulp (p = 0.999999999999969: Φ's rounding next to 1 is most of the tail 1 − p);
  - `t_quantile` stops its bisection at 1e-12 relative: 1 of 120 (p, df) within 1 ulp, the worst 1.5×10⁶ ulp.

  These are 1.3.0's algorithms, kept for parity (D4); the roadmap holds them as T1 candidates.

### 10.3 Part c: M4's gate (2026-11-17)

**NIST StRD** (plan §3.6; `scripts/golden/strd.py` fetches the sets into `build/strd/`, pinned by sha256, and never
commits them). The sets are golden rows too, so the replay proves the port computes the model's values bit for bit;
`strd.py` then measures those values against NIST's certified ones as LRE, the correct significant digits (15 at most):

| Set | mean | sd (from the population variance) |
|---|---|---|
| PiDigits, Lottery, Lew, NumAcc1 | 15.0 | 14.9–15.0 |
| Mavro | 15.0 | 13.1 |
| Michelso | 15.0 | 13.8 |
| NumAcc2 | 14.0 | 14.6 |
| NumAcc3 | 15.0 | 9.5 |
| NumAcc4 | 14.0 | 8.3 |

| Set (difficulty) | SS between | SS within | F | R² |
|---|---|---|---|---|
| SiRstv (lower) | 12.5 | 13.1 | 12.4 | 12.5 |
| SmLs01–03 (lower) | 12.7–14.4 | 12.7–15.0 | 13.4–14.4 | 13.6–14.6 |
| AtmWtAg (average) | 8.5 | 10.9 | 8.5 | 8.6 |
| SmLs04–06 (average) | 6.5–8.5 | 10.3 | 6.5–8.5 | 6.7–8.8 |
| SmLs07–09 (higher) | −0.9–2.7 | 1.3–4.3 | −0.8–2.7 | 0.1–3.0 |

The univariate sets keep 8.3 digits or more; 1.3.0's one-way ANOVA, its group means from plain sums of values near
10¹², keeps almost none on the higher-difficulty sets. D4 keeps Rust's operation order in 2.0, so these are 2.1
candidates on the roadmap: a shifted or compensated sum for the means and the sums of squares. Lag-1 autocorrelation
is `timeseries`' (M5).

**The soak** gains 10⁴ rounds of every M4 call (the statistics, correlation, a KDE and a PCA built, written, read back
and dropped, the six tests, the intervals and a decoded TestResult's owned name, at up to 300 points): the bump heap,
the freelist's arena, the mapped size and the pool are unchanged, every value finite. `tests/soak.tcyr` has 33 checks.

**Surface.** `descriptive` and `hypothesis` move to surface parity's ported modules (255 items in seven modules, none
pending). **The ledger:** 15 rows, 23 notes in `src/`; L3 (the stable sort) and L6 (the Jacobi) have their stand-ins,
L15 (word copies) is new; nothing converts.

**M4's result.** 1.3.0's `descriptive.rs` and `hypothesis.rs` are ported:
- **The corpus:** `descriptive`, 24,497 rows; `hypothesis`, 1,608; `linalg`'s Jacobi, 123; 816 codec and 142 Debug rows
  for the eight M4 types. Every row passes at 0 ulp against 1.3.0's bits, its correction where glibc misrounds, or
  Rust's formula on the T1 functions.
- **The replay:** thirteen modules, warning-free on x86_64 ELF, PE and aarch64.
- **Tests:** `tests/descriptive.tcyr` (42), `tests/kde.tcyr` (43), `tests/pca.tcyr` (37), `tests/hypothesis.tcyr`
  (113), and the soak.
- **Cost** (`docs/benchmarks-rust-v-cyrius.md`): the statistics 3.8–9.9× Rust; the KDE 11×, the hypothesis tests
  9–12×, the χ² test 70×, each of the last dominated by a correctly rounded exp or a T1 function (ADR 009's price).

## 11. M5: regression, time series and Bayesian inference

M5 ports 1.3.0's `regression.rs`, `timeseries.rs` and `bayesian.rs` under ADR 010, in four parts (the plan's §7).
Every row of every part passed on its first replay but where noted, at 0 ulp, on x86_64 ELF, PE and aarch64.

### 11.1 Parts a–c (2026-11-17)

- **a, regression** (`src/regression.cyr`; hisab's `least_squares_poly` in `src/linalg.cyr`, ledger L5): 571 rows.
  One logistic fit, where glibc's exp misrounds, is in `regression.cr.tsv`. Two mutant survivors were closed with
  rows: hisab's dependence threshold (a column norm between 1e-12 and 1e-6, x = 10⁻⁴·k at degree 2) and the Newton
  steps' 1e-8 stop (60 more seeded fits).
- **b, time series** (`src/timeseries.cyr`): 1,331 rows; c(0)'s 1e-30 floor needed a series with c(0) ≈ 2.5e-25.
  `moving_average` refuses an output that overlaps its input: its slide re-reads x[i − window].
- **c, Bayesian** (`src/bayesian.cyr`): 2,005 rows; naive Bayes' 1e-300 floor is a word proven by
  `scripts/crverify/consts.py`.

### 11.2 Part d: M5's gate (2026-11-17)

**NIST StRD** (plan §3.6, D4; `scripts/golden/strd.py`): the linear least squares sets 1.3.0's fits can take, y on x
alone (NoInt1/2 need a fit without an intercept and Longley a multiple regression, which 1.3.0 doesn't have), and the
univariate sets' lag-1 autocorrelation. Replayed as golden rows (the port computes the models' values bit for bit) and
measured as LRE:

| Set | Degree | Worst coefficient | R² |
|---|---|---|---|
| Norris (lower) | 1 | 12.0 (the linear fit: slope 14.4, intercept 12.8) | 15.0 |
| Pontius (lower) | 2 | 11.8 | 15.0 |
| Wampler1–4 (higher) | 5 | 7.4–10.9 | 15.0 |
| Wampler5 (higher) | 5 | 6.5 | 13.7 |
| Filip (higher) | 10 | 4.0 | 11.0 |

| Univariate set | r(1) |
|---|---|
| Lottery, Lew, NumAcc1 | 15.0 |
| PiDigits, Mavro, Michelso, NumAcc2 | 13.0–13.8 |
| NumAcc3, NumAcc4 | 11.2, 9.0 |

hisab's modified Gram–Schmidt on the raw Vandermonde matrix does fit Filip, where the plan expected a refusal, but
keeps 4.0 digits of its worst coefficient. D4 keeps Rust's operation order in 2.0; a centered, scaled Householder fit is
the 2.1 candidate on the roadmap.

**The soak** gains 10⁴ rounds of every M5 call (the three fits and their codecs and drops, the time series, an ARIMA
fit, forecast and codec, the Bayesian calls) at up to 200 points: nothing taken (`tests/soak.tcyr`, 38 checks).
**Surface:** `regression`, `timeseries` and `bayesian` are ported (300 items in ten modules, none pending).
**The ledger:** 15 rows; L5 has its stand-in; nothing converts.

**M5's result.** 1.3.0's `regression.rs`, `timeseries.rs` and `bayesian.rs` are ported: `regression` 571 rows,
`timeseries` 1,331, `bayesian` 2,005, and 458 codec and 74 Debug rows for their six types, every one at 0 ulp on three
targets. Tests: `tests/regression.tcyr` (46), `tests/timeseries.tcyr` (45), `tests/bayesian.tcyr` (19) and the soak.
Cost: 1.8–8.5× Rust on the arithmetic rows, 17× for naive Bayes (its 44 correctly rounded logarithms).

