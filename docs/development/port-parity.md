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
