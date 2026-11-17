//! `linalg.tsv` — hisab 1.4.0's dense Cholesky factorization and solve (`hisab::num::cholesky`, `cholesky_solve`), the
//! two linear-algebra routines 1.3.0 calls (MultivariateNormal, ARIMA's Yule–Walker fit, the logistic regression's
//! Newton step). M3 part d1; port plan §8.2, ledger L4. Through pramana's `pub use hisab`, at the version 1.3.0 locks.
//!
//! - `linalg::cholesky`: the empty matrix (refused), 1×1 and 2×2 edges (a zero, negative, NaN, infinite, subnormal or f64::MAX pivot; singular and
//!   indefinite matrices; a non-symmetric one, whose upper triangle is ignored), and n from 3 to 64: seeded SPD
//!   matrices B·Bᵀ + n·I, the Hilbert matrices to n = 14 (refused from where rounding loses definiteness), Lehmer's,
//!   the [2, −1] tridiagonal, scaled by 1e300 and 1e-300, and negative-definite ones.
//! - `linalg::cholesky_solve`: each factor above against the unit vectors, seeded right sides, a zero one and a
//!   NaN-bearing one; hand factors with a zero diagonal and with junk above the diagonal (never read); n = 0.
//!
//! Inputs: `i:n`, the matrix as `v:n²` and its entries row-major, then (for the solve) b as `v:n` and its entries.
//! Outputs: L as `v:n²` row-major, or x as `v:n`; or `E:InvalidParameter`, the code the port returns for hisab's
//! `InvalidInput`. Class A: arithmetic and the correctly rounded √.

use crate::grids::Rng;
use crate::tsv::{Class, Module, Tok, f, i};
use pramana::hisab::num::{cholesky, cholesky_solve};

/// A row-major matrix's tokens: `v:n²` and the entries.
fn mat(m: &[Vec<f64>]) -> Vec<Tok> {
    let n = m.len();
    let mut t = vec![Tok::V(n * n)];
    for row in m {
        t.extend(row.iter().map(|&x| f(x)));
    }
    t
}

fn vec_toks(v: &[f64]) -> Vec<Tok> {
    let mut t = vec![Tok::V(v.len())];
    t.extend(v.iter().map(|&x| f(x)));
    t
}

fn refused() -> Tok {
    Tok::Err("InvalidParameter".to_string())
}

#[allow(clippy::cast_possible_wrap)]
fn factor_row(m: &mut Module, a: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let mut ins = vec![i(a.len() as i64)];
    ins.extend(mat(a));
    match cholesky(a) {
        Ok(l) => {
            m.row("linalg::cholesky", Class::A, &ins, &mat(&l));
            Some(l)
        }
        Err(_) => {
            m.row("linalg::cholesky", Class::A, &ins, &[refused()]);
            None
        }
    }
}

#[allow(clippy::cast_possible_wrap)]
fn solve_row(m: &mut Module, l: &[Vec<f64>], b: &[f64]) {
    let mut ins = vec![i(l.len() as i64)];
    ins.extend(mat(l));
    ins.extend(vec_toks(b));
    match cholesky_solve(l, b) {
        Ok(x) => m.row("linalg::cholesky_solve", Class::A, &ins, &vec_toks(&x)),
        Err(_) => m.row("linalg::cholesky_solve", Class::A, &ins, &[refused()]),
    }
}

/// B·Bᵀ + shift·I for a seeded B with entries in [−1, 1).
fn spd(g: &mut Rng, n: usize, shift: f64) -> Vec<Vec<f64>> {
    let b: Vec<Vec<f64>> = (0..n).map(|_| (0..n).map(|_| g.uniform(-1.0, 1.0)).collect()).collect();
    let mut a = vec![vec![0.0; n]; n];
    for r in 0..n {
        for c in 0..n {
            let mut s = 0.0;
            for k in 0..n {
                s += b[r][k] * b[c][k];
            }
            a[r][c] = s + if r == c { shift } else { 0.0 };
        }
    }
    a
}

#[allow(clippy::cast_precision_loss)]
fn hilbert(n: usize) -> Vec<Vec<f64>> {
    (0..n).map(|r| (0..n).map(|c| 1.0 / (r + c + 1) as f64).collect()).collect()
}

#[allow(clippy::cast_precision_loss)]
fn lehmer(n: usize) -> Vec<Vec<f64>> {
    (0..n).map(|r| (0..n).map(|c| (r.min(c) + 1) as f64 / (r.max(c) + 1) as f64).collect()).collect()
}

fn tridiagonal(n: usize) -> Vec<Vec<f64>> {
    (0..n)
        .map(|r| {
            (0..n)
                .map(|c| {
                    if r == c {
                        2.0
                    } else if r.abs_diff(c) == 1 {
                        -1.0
                    } else {
                        0.0
                    }
                })
                .collect()
        })
        .collect()
}

fn scaled(a: &[Vec<f64>], s: f64) -> Vec<Vec<f64>> {
    a.iter().map(|row| row.iter().map(|&x| x * s).collect()).collect()
}

/// The right sides a factor of order n is solved against: the unit vectors (up to 8), three seeded ones, zero, and one
/// with a NaN.
fn rights(g: &mut Rng, n: usize) -> Vec<Vec<f64>> {
    let mut bs: Vec<Vec<f64>> = (0..n.min(8)).map(|k| (0..n).map(|j| if j == k { 1.0 } else { 0.0 }).collect()).collect();
    bs.extend((0..3).map(|_| (0..n).map(|_| g.uniform(-10.0, 10.0)).collect::<Vec<f64>>()));
    bs.push(vec![0.0; n]);
    let mut nb: Vec<f64> = (0..n).map(|j| f64::from(u8::try_from(j % 7).unwrap_or(0)) - 3.0).collect();
    nb[n / 2] = f64::NAN;
    bs.push(nb);
    bs
}

pub fn emit() -> Module {
    let mut m = Module::new("linalg");
    let mut g = Rng::new(0x9b05_688c_2b3e_6c1f);
    let mut factors: Vec<Vec<Vec<f64>>> = Vec::new();
    // The empty matrix, which hisab refuses.
    factor_row(&mut m, &[]);
    // 1×1: every pivot class.
    for x in [4.0, 1.0, 2.0, 0.0, -0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY, f64::from_bits(1), f64::MAX] {
        if let Some(l) = factor_row(&mut m, &[vec![x]]) {
            factors.push(l);
        }
    }
    // 2×2: the identity, an SPD, singular, indefinite, non-symmetric (upper ignored), NaN and ∞ off the diagonal.
    let twos: Vec<Vec<Vec<f64>>> = vec![
        vec![vec![1.0, 0.0], vec![0.0, 1.0]],
        vec![vec![4.0, 2.0], vec![2.0, 3.0]],
        vec![vec![1.0, 1.0], vec![1.0, 1.0]],
        vec![vec![1.0, 2.0], vec![2.0, 1.0]],
        vec![vec![4.0, 99.0], vec![2.0, 3.0]],
        vec![vec![4.0, f64::NAN], vec![f64::NAN, 3.0]],
        vec![vec![4.0, 0.0], vec![f64::INFINITY, 3.0]],
        vec![vec![1e-300, 0.0], vec![0.0, 1e300]],
        vec![vec![-4.0, 0.0], vec![0.0, 3.0]],
    ];
    for a in &twos {
        if let Some(l) = factor_row(&mut m, a) {
            factors.push(l);
        }
    }
    // n from 3 to 64.
    let mut bigs: Vec<Vec<Vec<f64>>> = Vec::new();
    for n in [3, 4, 5, 6, 8, 10, 12, 16, 24, 32, 64] {
        bigs.push(spd(&mut g, n, n as f64));
        bigs.push(spd(&mut g, n, 1e-3));
    }
    for n in 3..=14 {
        bigs.push(hilbert(n));
    }
    for n in [3, 7, 16] {
        bigs.push(lehmer(n));
        bigs.push(tridiagonal(n));
        bigs.push(scaled(&tridiagonal(n), 1e300));
        bigs.push(scaled(&tridiagonal(n), 1e-300));
        bigs.push(scaled(&tridiagonal(n), -1.0));
    }
    for a in &bigs {
        if let Some(l) = factor_row(&mut m, a) {
            factors.push(l);
        }
    }
    // The solves: every factor against its right sides.
    for l in &factors {
        for b in rights(&mut g, l.len()) {
            solve_row(&mut m, l, &b);
        }
    }
    // Hand factors: a zero diagonal (division by zero), junk above the diagonal (never read), and n = 0.
    let zero_diag = vec![vec![2.0, 0.0, 0.0], vec![1.0, 0.0, 0.0], vec![0.5, 1.0, 3.0]];
    let junk = vec![vec![2.0, 7.0, -9.0], vec![1.0, 3.0, f64::NAN], vec![0.5, 1.0, 3.0]];
    for l in [&zero_diag, &junk] {
        for b in rights(&mut g, 3) {
            solve_row(&mut m, l, &b);
        }
    }
    solve_row(&mut m, &[], &[]);
    m
}
