//! `bridge.tsv` — 1.3.0's eleven bridge functions (M1 part e3): badal, kimiya, ushma and bodh values into pramana's
//! parameters.
//!
//! Each scalar function gets a grid of special values (±0, ±1, the f64 extremes, NaN, ±∞, and 1e-15 with its
//! neighbours, the functions' own floor) against each other, and seeded values. The slice functions get empty, single,
//! constant, mismatched and NaN-bearing slices and seeded ones; the two fits get records with zero and non-finite
//! entries, an exact power law (T = 60 / N) and an exact Hick line (n = 2^k), equal abscissae (no line), and seeded
//! data. The inputs use no transcendental (README: grids are + − × ÷ and seeded draws). Class B (arithmetic and sqrt, expected
//! exact), except the likelihood (exp) and the two fits (ln, exp, log2): class C, corrected against MPFR in
//! `bridge.cr.tsv` by `validate.py`.
//!
//! A slice is `v:n` and its values; a slice of pairs is `v:n` then each pair's `i:` and `f:` tokens; a tuple is its
//! elements in order.

use crate::grids::Rng;
use crate::tsv::{Class, Module, Tok, f, i};
use pramana::bridge as br;

fn specials() -> Vec<f64> {
    let floor = 1e-15_f64;
    vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        2.5,
        -3.25,
        1.96,
        1e-300,
        1e300,
        f64::MIN_POSITIVE,
        5e-324,
        f64::MAX,
        floor,
        f64::from_bits(floor.to_bits() - 1),
        f64::from_bits(floor.to_bits() + 1),
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ]
}

fn slice(xs: &[f64]) -> Vec<Tok> {
    let mut t = vec![Tok::V(xs.len())];
    t.extend(xs.iter().map(|&x| f(x)));
    t
}

#[allow(clippy::cast_possible_wrap)]
fn pairs<T: Copy + Into<u64>>(xs: &[(T, f64)]) -> Vec<Tok> {
    let mut t = vec![Tok::V(xs.len())];
    for &(w, x) in xs {
        t.push(i(w.into() as i64));
        t.push(f(x));
    }
    t
}

fn seeded(g: &mut Rng, k: usize) -> Vec<f64> {
    (0..k)
        .map(|j| match j % 5 {
            0 => g.uniform(-10.0, 10.0),
            1 => g.log_uniform(-12.0, 12.0),
            2 => -g.log_uniform(-12.0, 12.0),
            3 => g.uniform(0.0, 1.0),
            _ => g.log_uniform(-300.0, 300.0),
        })
        .collect()
}

#[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::too_many_lines)]
pub fn emit() -> Module {
    let mut m = Module::new("bridge");
    let mut g = Rng::new(0x510e_527f_ade6_82d1);
    let sp = specials();
    let mut grid: Vec<(f64, f64)> = Vec::new();
    for &a in &sp {
        for &b in &sp {
            grid.push((a, b));
        }
    }
    let s1 = seeded(&mut g, 300);
    let s2 = seeded(&mut g, 300);
    grid.extend(s1.iter().zip(&s2).map(|(&a, &b)| (a, b)));

    for &(a, b) in &grid {
        m.row("bridge::ensemble_spread_to_confidence", Class::B, &[f(a), f(b)], &[f(br::ensemble_spread_to_confidence(a, b))]);
        m.row("bridge::observation_error_to_likelihood", Class::C, &[f(a), f(b)], &[f(br::observation_error_to_likelihood(a, b))]);
        let (mv, sd) = br::uncertainty_to_gaussian(a, b);
        m.row("bridge::uncertainty_to_gaussian", Class::B, &[f(a), f(b)], &[f(mv), f(sd)]);
        m.row("bridge::sensor_noise_to_std_dev", Class::B, &[f(a), f(b)], &[f(br::sensor_noise_to_std_dev(a, b))]);
        m.row("bridge::z_rates_to_d_prime", Class::B, &[f(a), f(b)], &[f(br::z_rates_to_d_prime(a, b))]);
    }

    let counts: [u64; 8] = [0, 1, 2, 3, 10, 1000, (1 << 53) + 1, u64::MAX];
    let few = [0.0, -0.0, 1.0, 2.5, 1e-300, 1e300, f64::NAN, f64::INFINITY, 1e-15];
    for &k in &counts {
        for &s in &few {
            for &t in &few {
                m.row("bridge::item_variances_to_alpha", Class::B, &[i(k as i64), f(s), f(t)], &[f(br::item_variances_to_alpha(k as usize, s, t))]);
            }
        }
    }
    let floor = 1e-15_f64;
    for t in [floor, f64::from_bits(floor.to_bits() - 1), f64::from_bits(floor.to_bits() + 1)] {
        for k in [2u64, 7] {
            m.row("bridge::item_variances_to_alpha", Class::B, &[i(k as i64), f(t * 0.5), f(t)], &[f(br::item_variances_to_alpha(k as usize, t * 0.5, t))]);
        }
    }
    for _ in 0..200 {
        let k = g.int(0, 40) as u64;
        let s = g.log_uniform(-3.0, 3.0);
        let t = g.log_uniform(-3.0, 3.0);
        m.row("bridge::item_variances_to_alpha", Class::B, &[i(k as i64), f(s), f(t)], &[f(br::item_variances_to_alpha(k as usize, s, t))]);
    }

    let mut reps: Vec<(f64, f64, u64, f64, f64, u64)> = Vec::new();
    for &na in &counts {
        for &nb in &[0u64, 1, 5, u64::MAX] {
            for &(sa, sb) in &[(1.0, 2.0), (0.0, 0.0), (1e-16, 0.0), (1e-15, 0.0), (f64::NAN, 1.0), (f64::INFINITY, 1.0)] {
                reps.push((3.5, sa, na, 1.25, sb, nb));
            }
        }
    }
    for k in 0..60 {
        let se = f64::from_bits(floor.to_bits() - 30 + k);
        reps.push((1.0, se, 1, 0.5, 0.0, 1));
    }
    for _ in 0..300 {
        let na = g.int(1, 50) as u64;
        let nb = g.int(1, 50) as u64;
        reps.push((g.uniform(-5.0, 5.0), g.log_uniform(-3.0, 1.0), na, g.uniform(-5.0, 5.0), g.log_uniform(-3.0, 1.0), nb));
    }
    for &(ma, sa, na, mb, sb, nb) in &reps {
        m.row(
            "bridge::replicates_to_t_statistic",
            Class::B,
            &[f(ma), f(sa), i(na as i64), f(mb), f(sb), i(nb as i64)],
            &[f(br::replicates_to_t_statistic(ma, sa, na as usize, mb, sb, nb as usize))],
        );
    }

    let mut slices: Vec<Vec<f64>> = vec![vec![], vec![2.5], vec![f64::NAN], vec![f64::INFINITY], vec![-0.0], vec![1e300, -1e300], vec![3.0; 7]];
    slices.extend(sp.iter().map(|&x| vec![x, 1.0, 2.0]));
    for k in 0..200 {
        let len = g.int(2, 20) as usize;
        let mut v: Vec<f64> = if k % 3 == 0 {
            (0..len).map(|_| 1e8 + g.uniform(-1.0, 1.0)).collect()
        } else {
            seeded(&mut g, len)
        };
        if k % 25 == 0 {
            v[0] = f64::NAN;
        }
        slices.push(v);
    }
    for v in &slices {
        let (mean, var) = br::yields_to_mean_variance(v);
        m.row("bridge::yields_to_mean_variance", Class::B, &slice(v), &[f(mean), f(var)]);
    }
    let mut twos: Vec<(Vec<f64>, Vec<f64>)> = vec![(vec![], vec![]), (vec![1.0], vec![2.0]), (vec![1.0, 2.0], vec![3.0]), (vec![], vec![1.0])];
    for k in 0..200 {
        let len = g.int(2, 15) as usize;
        let a = seeded(&mut g, len);
        let mut b: Vec<f64> = a.iter().map(|&x| 2.0 * x + g.uniform(-1.0, 1.0)).collect();
        if k % 20 == 0 {
            b[len - 1] = f64::NAN;
        }
        twos.push((a, b));
    }
    for (a, b) in &twos {
        let (ma, mb, c, va, vb) = br::trait_scores_to_correlation(a, b);
        let mut ins = slice(a);
        ins.extend(slice(b));
        m.row("bridge::trait_scores_to_correlation", Class::B, &ins, &[f(ma), f(mb), f(c), f(va), f(vb)]);
    }

    let mut trials: Vec<Vec<(u32, f64)>> = vec![
        vec![],
        vec![(1, 10.0)],
        vec![(3, 5.0), (3, 4.0), (3, 6.0)],
        (1..=12).map(|n| (n, 60.0 / f64::from(n))).collect(),
        vec![(0, 1.0), (1, 0.0), (2, -1.0), (3, f64::NAN), (4, f64::INFINITY), (5, 2.0), (6, 1.5)],
        vec![(1, 2.0), (u32::MAX, 1e-300), (2, 1e300)],
    ];
    for _ in 0..200 {
        let len = g.int(0, 15) as usize;
        let v: Vec<(u32, f64)> = (0..len)
            .map(|k| {
                let n = g.int(0, 1000) as u32;
                let t = match k % 9 {
                    0 => -1.0,
                    1 => 0.0,
                    _ => 40.0 / (1.0 + 0.01 * f64::from(n)) + g.uniform(0.0, 3.0),
                };
                (n, t)
            })
            .collect();
        trials.push(v);
    }
    for v in &trials {
        let (a, b) = br::learning_trials_to_power_law(v);
        m.row("bridge::learning_trials_to_power_law", Class::C, &pairs(v), &[f(a), f(b)]);
    }
    let mut hick: Vec<Vec<(u64, f64)>> = vec![
        vec![],
        vec![(2, 0.3)],
        vec![(4, 0.5), (4, 0.6)],
        (0..10).map(|k| (1u64 << k, 0.2 + 0.15 * f64::from(k))).collect(),
        vec![(0, 0.1), (1, f64::NAN), (2, f64::INFINITY), (3, -0.5), (8, 0.6), (16, 0.7)],
        vec![(1, 0.2), ((1 << 53) + 1, 0.9), (u64::MAX, 1.1)],
    ];
    for _ in 0..200 {
        let len = g.int(0, 15) as usize;
        let v: Vec<(u64, f64)> = (0..len)
            .map(|k| {
                let e = g.int(0, 12);
                let n = if k % 7 == 0 { 0 } else { 1u64 << e };
                let t = if k % 11 == 0 { f64::NAN } else { 0.18 + 0.12 * e as f64 + g.uniform(0.0, 0.05) };
                (n, t)
            })
            .collect();
        hick.push(v);
    }
    for v in &hick {
        let data: Vec<(usize, f64)> = v.iter().map(|&(n, t)| (n as usize, t)).collect();
        let (a, b) = br::response_times_to_hick_params(&data);
        m.row("bridge::response_times_to_hick_params", Class::C, &pairs(v), &[f(a), f(b)]);
    }
    m
}
