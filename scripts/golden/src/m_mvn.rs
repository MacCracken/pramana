//! `dist.tsv`'s `MultivariateNormal` rows (M3 part d2; port plan §7, D11): 1.3.0's constructor, density, log density
//! and draws, with the private factor and log det through the oracle's `mvn_factor`. Called by `m_dist::emit`.
//!
//! - `MultivariateNormal::new`: the empty mean; a 1×1 covariance at every pivot class and a mean at every special;
//!   covariances of the wrong shape (fewer or more rows or columns, the mean's own check first); a NaN or ±∞ at each
//!   entry; asymmetries either side of Rust's relative tolerance, at its MIN_POSITIVE floor, where the larger magnitude
//!   decides it, and on 1.3.0's own test matrices; matrices hisab refuses, and one it factors with a NaN pivot (the degenerate determinant); seeded SPD
//!   matrices for d from 1 to 32, scaled by 1e300, 1e-300, 5e-324 and f64::MAX, the Hilbert matrices to 14 and
//!   Lehmer's. Output: L as `v:d²` row-major and log det Σ, or the error. Class C: log det takes glibc's ln.
//! - `MultivariateNormal::log_pdf`, `MultivariateNormal::pdf`: each valid case at its mean, along each axis, at seeded
//!   points around the mean and far from it, with a NaN or ±∞ element, and with a point of the wrong length (the
//!   error). Past d = 8 only the mean, one seeded point and a far one, and the pdf at the mean. Class C: glibc's ln
//!   and exp.
//! - `MultivariateNormal::sample`: 2,000 draws of 1.3.0's own test case and 256 of each other fixed case, from three
//!   seeds, the vectors' elements in order. Class C: Box–Muller's ln, cos and sin.
//!
//! Inputs: d, the mean as `v:d`, the covariance's rows and columns, its entries as `v:rows·cols` row-major; then the
//! point as `v:n`, or the seed and the count.

use std::hint::black_box;

use crate::grids::Rng as Seeds;
use crate::tsv::{Class, Module, Tok, err, f, i};
use pramana::distribution::MultivariateNormal;
use pramana::{SimpleRng, oracle};

const SEEDS: [u64; 3] = [42, 0x9e37_79b9_7f4a_7c15, 12345];

/// A case: the mean, and the covariance as rows × cols row-major.
struct Case {
    mean: Vec<f64>,
    rows: usize,
    cols: usize,
    cov: Vec<f64>,
}

impl Case {
    fn square(mean: Vec<f64>, cov: Vec<f64>) -> Self {
        let d = mean.len();
        Case { mean, rows: d, cols: d, cov }
    }

    fn of(mean: &[f64], m: &[Vec<f64>]) -> Self {
        Case::square(mean.to_vec(), m.iter().flatten().copied().collect())
    }

    fn matrix(&self) -> Vec<Vec<f64>> {
        (0..self.rows).map(|r| self.cov[r * self.cols..(r + 1) * self.cols].to_vec()).collect()
    }

    #[allow(clippy::cast_possible_wrap)]
    fn toks(&self) -> Vec<Tok> {
        let mut t = vec![i(self.mean.len() as i64), Tok::V(self.mean.len())];
        t.extend(self.mean.iter().map(|&x| f(x)));
        t.push(i(self.rows as i64));
        t.push(i(self.cols as i64));
        t.push(Tok::V(self.cov.len()));
        t.extend(self.cov.iter().map(|&x| f(x)));
        t
    }

    fn build(&self) -> Result<MultivariateNormal, pramana::PramanaError> {
        MultivariateNormal::new(black_box(self.mean.clone()), black_box(self.matrix()))
    }
}

fn vec_toks(v: &[f64]) -> Vec<Tok> {
    let mut t = vec![Tok::V(v.len())];
    t.extend(v.iter().map(|&x| f(x)));
    t
}

/// The constructor's row: L and log det, or the error. The valid ones are returned for the density and draw rows.
fn new_row(m: &mut Module, c: &Case) -> Option<MultivariateNormal> {
    match c.build() {
        Ok(mvn) => {
            let (l, log_det) = oracle::mvn_factor(&mvn);
            let flat: Vec<f64> = l.iter().flatten().copied().collect();
            let mut out = vec_toks(&flat);
            out.push(f(log_det));
            m.row("MultivariateNormal::new", Class::C, &c.toks(), &out);
            Some(mvn)
        }
        Err(e) => {
            m.row("MultivariateNormal::new", Class::C, &c.toks(), &[err(&e)]);
            None
        }
    }
}

fn density_rows(m: &mut Module, c: &Case, mvn: &MultivariateNormal, x: &[f64], with_pdf: bool) {
    let mut ins = c.toks();
    ins.extend(vec_toks(x));
    let lp = match mvn.log_pdf(black_box(x)) {
        Ok(v) => f(v),
        Err(e) => err(&e),
    };
    m.row("MultivariateNormal::log_pdf", Class::C, &ins, &[lp]);
    if !with_pdf {
        return;
    }
    let p = match mvn.pdf(black_box(x)) {
        Ok(v) => f(v),
        Err(e) => err(&e),
    };
    m.row("MultivariateNormal::pdf", Class::C, &ins, &[p]);
}

/// The points a valid case's density is evaluated at. Past d = 8 (each row carries the d² covariance) only the mean,
/// one seeded point and a far one: the specials and the wrong lengths are the small cases'.
fn points(g: &mut Seeds, c: &Case) -> Vec<Vec<f64>> {
    let d = c.mean.len();
    let mu = &c.mean;
    let scale: Vec<f64> = (0..d).map(|k| c.cov[k * d + k].sqrt()).collect();
    let mut ps = vec![mu.clone()];
    let seeded = if d <= 4 { 12 } else if d <= 8 { 4 } else { 1 };
    for _ in 0..seeded {
        ps.push((0..d).map(|k| mu[k] + scale[k] * g.uniform(-4.0, 4.0)).collect());
    }
    if d <= 8 {
        for k in 0..d.min(3) {
            for t in [-3.0, 1.0, 40.0] {
                let mut x = mu.clone();
                x[k] += t * scale[k];
                ps.push(x);
            }
        }
    }
    let mut far = mu.clone();
    far[0] += 1e200;
    ps.push(far);
    if d > 8 {
        return ps;
    }
    for special in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut x = mu.clone();
        x[d / 2] = special;
        ps.push(x);
    }
    ps.push(mu[..d - 1].to_vec());
    let mut longer = mu.clone();
    longer.push(0.0);
    ps.push(longer);
    ps
}

/// B·Bᵀ + shift·I for a seeded B with entries in [−1, 1): symmetric bit for bit.
fn spd(g: &mut Seeds, d: usize, shift: f64) -> Vec<f64> {
    let b: Vec<f64> = (0..d * d).map(|_| g.uniform(-1.0, 1.0)).collect();
    let mut a = vec![0.0; d * d];
    for r in 0..d {
        for c in 0..d {
            let mut s = 0.0;
            for k in 0..d {
                s += b[r * d + k] * b[c * d + k];
            }
            a[r * d + c] = s + if r == c { shift } else { 0.0 };
        }
    }
    a
}

#[allow(clippy::cast_precision_loss)]
fn identity(d: usize, s: f64) -> Vec<f64> {
    (0..d * d).map(|k| if k / d == k % d { s } else { 0.0 }).collect()
}

/// The asymmetric 2×2 [[1, a], [b, 1]], or [[s, a], [b, s]].
fn pair(s: f64, a: f64, b: f64) -> Case {
    Case::square(vec![0.0, 0.0], vec![s, a, b, s])
}

#[allow(clippy::too_many_lines, clippy::cast_precision_loss, clippy::cast_possible_wrap)]
pub fn emit(m: &mut Module, g: &mut Seeds) {
    let mut cases: Vec<Case> = Vec::new();
    // The empty mean; 1×1 at every pivot class; a mean at every special.
    cases.push(Case::square(vec![], vec![]));
    for v in [4.0, 1.0, 2.0, 0.25, 0.0, -0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY, f64::from_bits(1), 1e-300, 1e300, f64::MAX] {
        cases.push(Case::square(vec![0.0], vec![v]));
    }
    for mu in [-0.0, 1.5, -3.0, 1e300, f64::MAX, -f64::MAX, f64::from_bits(1), f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        cases.push(Case::square(vec![mu], vec![4.0]));
    }
    // The wrong shapes, and the mean's check before the shape's, the shape's before the entries'.
    for (rows, cols) in [(0, 0), (1, 1), (1, 2), (2, 1), (3, 3), (2, 3), (3, 2), (4, 1)] {
        cases.push(Case { mean: vec![0.0, 0.0], rows, cols, cov: (0..rows * cols).map(|k| f64::from(u8::try_from(k % 3).unwrap_or(0))).collect() });
    }
    cases.push(Case { mean: vec![0.0], rows: 1, cols: 2, cov: vec![1.0, 0.0] });
    cases.push(Case { mean: vec![0.0], rows: 0, cols: 0, cov: vec![] });
    cases.push(Case { mean: vec![f64::NAN, 0.0], rows: 1, cols: 1, cov: vec![1.0] });
    cases.push(Case { mean: vec![0.0, 0.0], rows: 3, cols: 3, cov: vec![f64::NAN; 9] });
    // A NaN or ±∞ at each entry of an SPD 2×2.
    for k in 0..4 {
        for v in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut cov = vec![4.0, 1.0, 1.0, 9.0];
            cov[k] = v;
            cases.push(Case::square(vec![1.0, 2.0], cov));
        }
    }
    // The symmetry check: either side of 1e-10 relative, at its MIN_POSITIVE floor, and 1.3.0's test matrices.
    let lim: f64 = 0.5 * (1.0 + 1e-10);
    for k in -3i64..=3 {
        let b = f64::from_bits(lim.to_bits().wrapping_add_signed(k));
        cases.push(pair(1.0, 0.5, b));
        cases.push(pair(1.0, b, 0.5));
    }
    let floor = 1e-10 * f64::MIN_POSITIVE;
    for k in -2i64..=2 {
        let b = f64::from_bits(floor.to_bits().wrapping_add_signed(k));
        cases.push(pair(1.0, 0.0, b));
        cases.push(pair(1.0, -b, 0.0));
    }
    cases.push(pair(1.0, 0.0, -0.0));
    cases.push(pair(1.0, 0.0, f64::from_bits(1)));
    cases.push(pair(1.0, 0.5, 0.0));
    cases.push(pair(1e-6, 1e-7, 2e-7));
    cases.push(pair(1e12, 0.5e12, 0.5e12 + 1e-3));
    cases.push(pair(4.0, 1.0, 1.0 + 1e-12));
    cases.push(pair(1.0, f64::MAX, -f64::MAX));
    // A pair whose verdict turns on the scale being the larger magnitude: |a − b| lies between 1e-10·|b| and
    // 1e-10·|a|, a window about 1e-20 wide that one double in about 10^5 meets.
    cases.push(pair(4.0, 1.711_735_198_171_008_7, 1.711_735_197_999_835_2));
    cases.push(pair(4.0, 1.711_735_197_999_835_2, 1.711_735_198_171_008_7));
    // Matrices hisab refuses, and one it factors with a NaN pivot: log det Σ is NaN, the degenerate determinant.
    for cov in [vec![1.0, 1.0, 1.0, 1.0], vec![1.0, 2.0, 2.0, 1.0], vec![-4.0, 0.0, 0.0, 3.0], vec![0.0, 0.0, 0.0, 1.0]] {
        cases.push(Case::square(vec![0.0, 0.0], cov));
    }
    let tiny = f64::from_bits(1);
    cases.push(Case::square(vec![0.0; 3], vec![tiny, 0.0, 1e200, 0.0, 1.0, 0.0, 1e200, 0.0, 1.0]));
    // 1.3.0's own test cases.
    cases.push(Case::of(&[2.0], &[vec![4.0]]));
    cases.push(Case::of(&[0.0, 0.0], &[vec![1.0, 0.0], vec![0.0, 1.0]]));
    cases.push(Case::of(&[1.0, -2.0], &[vec![1.0, 0.3], vec![0.3, 2.0]]));
    cases.push(Case::of(&[1.0, -1.0], &[vec![2.0, 0.5], vec![0.5, 3.0]]));
    cases.push(Case::of(&[1.0, 2.0], &[vec![4.0, 1.0], vec![1.0, 9.0]]));
    cases.push(Case::of(&[1.0, 2.0], &[vec![2.0, 0.5], vec![0.5, 1.0]]));
    cases.push(Case::of(&[0.0, 0.0, 0.0], &[vec![1.0, 0.0, 0.0], vec![0.0, 1.0, 0.0], vec![0.0, 0.0, 1.0]]));
    // Seeded SPD matrices, scales, Hilbert's and Lehmer's.
    for d in [1usize, 2, 3, 4, 5, 6, 8, 12, 16, 24, 32] {
        let mean: Vec<f64> = (0..d).map(|_| g.signed_log_uniform(-3.0, 3.0)).collect();
        cases.push(Case::square(mean.clone(), spd(g, d, d as f64)));
        cases.push(Case::square(mean, spd(g, d, 1e-3)));
    }
    for (d, s) in [(3usize, 1e300), (3, 1e-300), (3, tiny), (3, f64::MAX), (5, 1e-300), (2, f64::MIN_POSITIVE)] {
        let mean: Vec<f64> = (0..d).map(|_| g.signed_log_uniform(-3.0, 3.0)).collect();
        cases.push(Case::square(mean, identity(d, s)));
    }
    let scaled_spd = spd(g, 4, 4.0);
    for s in [1e300, 1e-300] {
        cases.push(Case::square(vec![0.0, -0.0, 1.0, -1.0], scaled_spd.iter().map(|x| x * s / 8.0).collect()));
    }
    for d in 3..=14 {
        cases.push(Case::square(vec![0.0; d], (0..d * d).map(|k| 1.0 / (k / d + k % d + 1) as f64).collect()));
    }
    for d in [3usize, 7, 16] {
        let lehmer = (0..d * d).map(|k| ((k / d).min(k % d) + 1) as f64 / ((k / d).max(k % d) + 1) as f64).collect();
        cases.push(Case::square((0..d).map(|k| k as f64 - 1.0).collect(), lehmer));
    }
    let mut valid: Vec<(usize, MultivariateNormal)> = Vec::new();
    for (k, c) in cases.iter().enumerate() {
        if let Some(mvn) = new_row(m, c) {
            valid.push((k, mvn));
        }
    }
    for (k, mvn) in &valid {
        let d = cases[*k].mean.len();
        for (j, x) in points(g, &cases[*k]).iter().enumerate() {
            density_rows(m, &cases[*k], mvn, x, d <= 8 || j == 0);
        }
    }
    // The draws: 1.3.0's own test case first (2,000 vectors a seed), then 256 of each other.
    let draw_cases = [
        Case::of(&[1.0, -2.0], &[vec![1.0, 0.3], vec![0.3, 2.0]]),
        Case::of(&[2.0], &[vec![4.0]]),
        Case::of(&[1.0, 2.0], &[vec![4.0, 1.0], vec![1.0, 9.0]]),
        Case::square(vec![0.5, -0.0, 3.0], spd(g, 3, 3.0)),
        Case::square(vec![-1.0, 0.0, 1.0, 2.0, -0.0], spd(g, 5, 1e-3)),
        Case::square((0..8).map(|k| f64::from(k) * 0.5).collect(), spd(g, 8, 8.0)),
        Case::square(vec![0.0; 3], identity(3, 1e300)),
        Case::square(vec![f64::MAX, -0.0], identity(2, 1e-300)),
    ];
    for &seed in &SEEDS {
        for (k, c) in draw_cases.iter().enumerate() {
            let count = if k == 0 { 2000 } else { 256 };
            let mvn = c.build().expect("a valid case");
            let mut r = SimpleRng::new(seed);
            let xs: Vec<f64> = (0..count).flat_map(|_| mvn.sample(&mut r)).collect();
            let mut ins = c.toks();
            ins.push(i(seed as i64));
            ins.push(i(count as i64));
            m.row("MultivariateNormal::sample", Class::C, &ins, &vec_toks(&xs));
        }
    }
}
