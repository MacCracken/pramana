//! p7_json — the Rust twin of probe P7, JSON (port plan §5 step 9c, D5). Run from scripts/probes/rust:
//!   cargo run --release --locked --features roundtrip --bin p7_json -- corpus > ../../../build/p7_rust_corpus.tsv
//!   cargo run --release --locked --features roundtrip --bin p7_json -- floats > ../../../build/p7_rust_floats.tsv
//!   cargo run --release --locked [--features roundtrip] --bin p7_json -- read ../../../build/p7_cyrius.tsv
//!
//! - `corpus`: serde_json 1.0.154's documents for nine types that carry every shape pramana serializes (two
//!   validated f64s, u64s, a unit enum, a newtype-variant enum, strings, bools, vectors, matrices, MVN's ignored
//!   cache). Each type's base values, and about 80 variants of its first one: whitespace, order, unknown,
//!   missing, duplicate and null fields, each number and string form in each field, enum and sequence forms,
//!   syntax faults and invalid parameters. One row each: id, type, the document (hex), serde_json's verdict,
//!   the value it read as tokens, its canonical text for that value (hex), and its message.
//! - `floats`: bits, serde_json's text and `Display`'s text for the printer comparison.
//! - `read`: serde_json's reading of each Cyrius encoding in p7_json.cyr's `D` rows.
//!
//! `--features roundtrip` turns on serde_json's `float_roundtrip` (a correctly rounded parse). Without it the
//! parse is serde_json's default best-effort one, which `read` measures.

use pramana::descriptive::{Kernel, KernelDensity};
use pramana::distribution::{Binomial, MultivariateNormal, Normal};
use pramana::hypothesis::TestResult;
use pramana::integration::soorat::HistogramData;
use pramana::markov::MarkovChain;
use pramana::{PramanaError, Rng, SimpleRng};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use serde_json::error::Category;
use std::fmt::Write as _;
use std::io::{BufWriter, Write as _};

const TYPES: [&str; 9] = [
    "normal", "binomial", "rng", "kde", "error", "test", "markov", "mvn", "hist",
];

fn fields(ty: &str) -> &'static [&'static str] {
    match ty {
        "normal" => &["mean", "std_dev"],
        "binomial" => &["n", "p"],
        "rng" => &["state"],
        "kde" => &["data", "bandwidth", "kernel"],
        "test" => &[
            "test_name",
            "statistic",
            "p_value",
            "degrees_of_freedom",
            "reject_at_alpha",
            "reject",
        ],
        "markov" => &["transition_matrix", "state"],
        "mvn" => &["mean", "covariance", "cholesky_l", "log_det", "dim"],
        "hist" => &["bin_edges", "counts", "total"],
        _ => &[],
    }
}

/// The twin's xorshift64* (P5's): uniform in [0, 1) from the top 53 bits.
struct G(u64);

impl G {
    fn word(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    fn unit(&mut self) -> f64 {
        (self.word() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn finite(&mut self) -> f64 {
        loop {
            let b = self.word();
            if (b >> 52) & 0x7ff != 0x7ff {
                return f64::from_bits(b);
            }
        }
    }
}

fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        write!(s, "{x:02x}").unwrap();
    }
    s
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

/// Tokens: `f`+bits, `u`+decimal, `s`+hex bytes, `b0`/`b1`, `n`, and `[`…`]`, `{`…`}` around containers.
fn flat_value(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Null => out.push("n".into()),
        Value::Bool(b) => out.push(if *b { "b1" } else { "b0" }.into()),
        Value::Number(n) => {
            if n.is_f64() {
                out.push(format!("f{:016x}", n.as_f64().unwrap().to_bits()));
            } else if let Some(u) = n.as_u64() {
                out.push(format!("u{u}"));
            } else {
                out.push(format!("i{}", n.as_i64().unwrap()));
            }
        }
        Value::String(s) => out.push(format!("s{}", hex(s.as_bytes()))),
        Value::Array(a) => {
            out.push("[".into());
            for e in a {
                flat_value(e, out);
            }
            out.push("]".into());
        }
        Value::Object(m) => {
            out.push("{".into());
            for (k, e) in m {
                out.push(format!("s{}", hex(k.as_bytes())));
                flat_value(e, out);
            }
            out.push("}".into());
        }
    }
}

/// A struct's fields in declaration order; the error enum as its one-member object.
fn flatten(ty: &str, v: &Value) -> String {
    let mut out = Vec::new();
    if ty == "error" {
        flat_value(v, &mut out);
    } else {
        for f in fields(ty) {
            flat_value(&v[*f], &mut out);
        }
    }
    out.join(" ")
}

/// `ok`, `data:<PramanaError variant>` for a validating constructor's refusal, or serde_json's category.
fn verdict(e: &serde_json::Error) -> String {
    let m = e.to_string();
    for (p, n) in [
        ("invalid parameter: ", "InvalidParameter"),
        ("invalid sample: ", "InvalidSample"),
        ("convergence failure: ", "ConvergenceFailure"),
        ("dimension mismatch: ", "DimensionMismatch"),
        ("computation error: ", "ComputationError"),
    ] {
        if m.starts_with(p) {
            return format!("data:{n}");
        }
    }
    match e.classify() {
        Category::Io => "io",
        Category::Syntax => "syntax",
        Category::Data => "data",
        Category::Eof => "eof",
    }
    .into()
}

struct Outcome {
    verdict: String,
    tokens: String,
    canon: Vec<u8>,
    msg: String,
}

fn run<T: DeserializeOwned + Serialize>(ty: &str, doc: &[u8]) -> Outcome {
    match serde_json::from_slice::<T>(doc) {
        Ok(x) => Outcome {
            verdict: "ok".into(),
            tokens: flatten(ty, &serde_json::to_value(&x).unwrap()),
            canon: serde_json::to_vec(&x).unwrap(),
            msg: String::new(),
        },
        Err(e) => Outcome {
            verdict: verdict(&e),
            tokens: String::new(),
            canon: Vec::new(),
            msg: e.to_string().replace(['\t', '\n', '\r'], " "),
        },
    }
}

fn decode(ty: &str, doc: &[u8]) -> Outcome {
    match ty {
        "normal" => run::<Normal>(ty, doc),
        "binomial" => run::<Binomial>(ty, doc),
        "rng" => run::<SimpleRng>(ty, doc),
        "kde" => run::<KernelDensity>(ty, doc),
        "error" => run::<PramanaError>(ty, doc),
        "test" => run::<TestResult>(ty, doc),
        "markov" => run::<MarkovChain>(ty, doc),
        "mvn" => run::<MultivariateNormal>(ty, doc),
        "hist" => run::<HistogramData>(ty, doc),
        _ => unreachable!(),
    }
}

/// A base value: its canonical document, its pretty document and its tree.
struct Base {
    doc: Vec<u8>,
    pretty: Vec<u8>,
    value: Value,
}

fn base<T: Serialize>(x: &T) -> Base {
    Base {
        doc: serde_json::to_vec(x).unwrap(),
        pretty: serde_json::to_vec_pretty(x).unwrap(),
        value: serde_json::to_value(x).unwrap(),
    }
}

fn spd(g: &mut G, n: usize) -> Vec<Vec<f64>> {
    let b: Vec<Vec<f64>> = (0..n).map(|_| (0..n).map(|_| 2.0 * g.unit() - 1.0).collect()).collect();
    (0..n)
        .map(|i| {
            (0..n)
                .map(|j| {
                    let mut s = 0.0;
                    for row in &b {
                        s += row[i] * row[j];
                    }
                    if i == j { s + n as f64 } else { s }
                })
                .collect()
        })
        .collect()
}

fn stochastic(g: &mut G, n: usize) -> Vec<Vec<f64>> {
    (0..n)
        .map(|_| {
            let w: Vec<f64> = (0..n).map(|_| g.unit() + 0.01).collect();
            let s: f64 = w.iter().sum();
            w.iter().map(|x| x / s).collect()
        })
        .collect()
}

fn bases(ty: &str) -> Vec<Base> {
    let mut g = G(0x9e37_79b9_7f4a_7c15 ^ ty.len() as u64);
    let mut v = Vec::new();
    match ty {
        "normal" => {
            for (m, s) in [
                (0.0, 1.0),
                (-3.5, 0.25),
                (1e300, 1e-300),
                (5e-324, f64::MAX),
                (-0.0, 1.0),
                (0.1, 0.2),
            ] {
                v.push(base(&Normal::new(m, s).unwrap()));
            }
            for _ in 0..20 {
                let m = g.finite();
                let s = g.finite().abs().max(f64::MIN_POSITIVE);
                v.push(base(&Normal::new(m, s).unwrap()));
            }
        }
        "binomial" => {
            let ns = [10, 1, 1, (1u64 << 53) + 1, 1u64 << 63, u64::MAX];
            let ps = [0.5, 0.0, 1.0, 0.1, 0.3, 0.999];
            for (n, p) in ns.iter().zip(ps) {
                v.push(base(&Binomial::new(*n, p).unwrap()));
            }
            for _ in 0..10 {
                let n = g.word() | 1;
                v.push(base(&Binomial::new(n, g.unit()).unwrap()));
            }
        }
        "rng" => {
            for s in [42u64, 0, 1, u64::MAX, 1 << 63] {
                v.push(base(&SimpleRng::new(s)));
            }
            for i in 0..30u64 {
                let mut r = SimpleRng::new(i * 7919 + 3);
                for _ in 0..(i * 7) {
                    r.next_f64();
                }
                v.push(base(&r));
            }
        }
        "kde" => {
            let ks = [
                Kernel::Gaussian,
                Kernel::Epanechnikov,
                Kernel::Uniform,
                Kernel::Triangular,
            ];
            v.push(base(&KernelDensity::new(vec![1.0], 0.5, Kernel::Gaussian).unwrap()));
            v.push(base(
                &KernelDensity::new(vec![0.5, -2.25, 1e-10], 1e-3, Kernel::Epanechnikov).unwrap(),
            ));
            for i in 0..12usize {
                let data: Vec<f64> = (0..1 + i % 8).map(|_| g.finite()).collect();
                let bw = g.finite().abs().max(f64::MIN_POSITIVE);
                v.push(base(&KernelDensity::new(data, bw, ks[i % 4]).unwrap()));
            }
        }
        "error" => {
            let msgs = [
                "m",
                "",
                "a\"b\\c/d",
                "line\nbreak\ttab\u{1}\u{1f}\u{7f}",
                "é😀ü",
                &"x".repeat(300),
            ];
            for m in msgs {
                v.push(base(&PramanaError::InvalidParameter(m.into())));
            }
            v.push(base(&PramanaError::InvalidSample("s".into())));
            v.push(base(&PramanaError::ConvergenceFailure("c".into())));
            v.push(base(&PramanaError::DimensionMismatch("d".into())));
            v.push(base(&PramanaError::ComputationError("e".into())));
        }
        "test" => {
            let mk = |n: &str, s: f64, p: f64, d: f64, a: f64, r: bool| TestResult {
                test_name: n.into(),
                statistic: s,
                p_value: p,
                degrees_of_freedom: d,
                reject_at_alpha: a,
                reject: r,
            };
            v.push(base(&mk("t-test", 2.5, 0.0123, 9.0, 0.05, true)));
            v.push(base(&mk("chi\"sq", -0.0, 1.0, 1.5, 0.01, false)));
            v.push(base(&mk("é", 1e-7, 5e-324, 1e16, 0.1, false)));
            v.push(base(&mk("nan", f64::NAN, f64::NAN, 3.0, 0.05, false)));
            v.push(base(&mk("inf", f64::INFINITY, 0.0, f64::NEG_INFINITY, 0.05, true)));
            for i in 0..10 {
                v.push(base(&mk(
                    &format!("r{i}"),
                    g.finite(),
                    g.unit(),
                    g.finite(),
                    g.unit(),
                    i % 2 == 0,
                )));
            }
        }
        "markov" => {
            v.push(base(
                &MarkovChain::new(vec![vec![0.5, 0.5], vec![0.25, 0.75]], 1).unwrap(),
            ));
            v.push(base(&MarkovChain::new(vec![vec![1.0]], 0).unwrap()));
            for n in 2..=5 {
                let st = g.word() as usize % n;
                v.push(base(&MarkovChain::new(stochastic(&mut g, n), st).unwrap()));
            }
        }
        "mvn" => {
            let diag = |d: &[f64]| -> Vec<Vec<f64>> {
                (0..d.len())
                    .map(|i| (0..d.len()).map(|j| if i == j { d[i] } else { 0.0 }).collect())
                    .collect()
            };
            v.push(base(
                &MultivariateNormal::new(vec![0.0, 1.0], diag(&[4.0, 9.0])).unwrap(),
            ));
            v.push(base(
                &MultivariateNormal::new(vec![0.5; 3], diag(&[1.0, 1.0, 1.0])).unwrap(),
            ));
            v.push(base(
                &MultivariateNormal::new(vec![-1.0, 2.0], diag(&[0.25, 16.0])).unwrap(),
            ));
            v.push(base(
                &MultivariateNormal::new(vec![0.0, 0.0], vec![vec![2.0, 1.0], vec![1.0, 2.0]]).unwrap(),
            ));
            for n in 2..=4 {
                let mean: Vec<f64> = (0..n).map(|_| 2.0 * g.unit() - 1.0).collect();
                v.push(base(&MultivariateNormal::new(mean, spd(&mut g, n)).unwrap()));
            }
        }
        "hist" => {
            v.push(base(&HistogramData {
                bin_edges: vec![0.0, 0.5, 1.0],
                counts: vec![3, 0],
                total: 3,
            }));
            v.push(base(&HistogramData {
                bin_edges: vec![-1.5, 1e-300],
                counts: vec![1 << 63, u64::MAX],
                total: u64::MAX,
            }));
            v.push(base(&HistogramData {
                bin_edges: vec![],
                counts: vec![],
                total: 0,
            }));
        }
        _ => unreachable!(),
    }
    v
}

/// One variant: its kind and its document.
type Variant = (String, Vec<u8>);

fn obj(members: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut d = b"{".to_vec();
    for (i, (k, val)) in members.iter().enumerate() {
        if i > 0 {
            d.push(b',');
        }
        d.extend_from_slice(format!("\"{k}\":").as_bytes());
        d.extend_from_slice(val);
    }
    d.push(b'}');
    d
}

fn seq(vals: &[Vec<u8>]) -> Vec<u8> {
    let mut d = b"[".to_vec();
    for (i, val) in vals.iter().enumerate() {
        if i > 0 {
            d.push(b',');
        }
        d.extend_from_slice(val);
    }
    d.push(b']');
    d
}

/// Every number form, and the other JSON types, as a field's value.
const NUMBERS: &[(&str, &[u8])] = &[
    ("str", b"\"x\""),
    ("numstr", b"\"1\""),
    ("true", b"true"),
    ("arr", b"[]"),
    ("obj", b"{}"),
    ("int0", b"0"),
    ("int1", b"1"),
    ("neg1", b"-1"),
    ("negzero", b"-0"),
    ("negzerof", b"-0.0"),
    ("f15", b"1.5"),
    ("e3", b"1e3"),
    ("onef", b"1.0"),
    ("big63", b"9223372036854775808"),
    ("u64max", b"18446744073709551615"),
    ("u64max1", b"18446744073709551616"),
    ("below_i64", b"-9223372036854775809"),
    ("e400", b"1e400"),
    ("ne400", b"-1e400"),
    ("em400", b"1e-400"),
    ("e_huge", b"1e18446744073709551616"),
    (
        "long_digits",
        b"0.1000000000000000055511151231257827021181583404541015625",
    ),
    ("lead0", b"01"),
    ("plus", b"+1"),
    ("dot5", b".5"),
    ("onedot", b"1."),
    ("nan", b"NaN"),
    ("inf", b"Infinity"),
    ("arr_f", b"[1.5]"),
    ("arr_neg", b"[-1]"),
    ("arr_negzero", b"[-0]"),
    ("arr_big", b"[18446744073709551615]"),
    ("mat", b"[[0.5,0.5],[0.5,0.5]]"),
];

/// String forms: escapes, raw UTF-8, surrogates, raw control bytes, invalid UTF-8.
const STRINGS: &[(&str, &[u8])] = &[
    ("esc_all", b"\"\\b\\f\\n\\r\\t\\\"\\\\\\/\""),
    ("u_e9", b"\"\\u00e9\""),
    ("u_upper", b"\"\\u00C9\""),
    ("raw_e9", "\"é\"".as_bytes()),
    ("surr", b"\"\\ud83d\\ude00\""),
    ("lone", b"\"\\ud800\""),
    ("lone_low", b"\"\\udc00\""),
    ("u0000", b"\"\\u0000\""),
    ("ctl", b"\"a\x01b\""),
    ("tab", b"\"a\tb\""),
    ("nl", b"\"a\nb\""),
    ("del", b"\"\x7f\""),
    ("badutf8", b"\"a\xffb\""),
    ("overlong", b"\"\xc0\xaf\""),
    ("bad_esc", b"\"\\x\""),
];

const KERNELS: &[(&str, &[u8])] = &[
    ("Gaussian", b"\"Gaussian\""),
    ("Epanechnikov", b"\"Epanechnikov\""),
    ("Uniform", b"\"Uniform\""),
    ("Triangular", b"\"Triangular\""),
    ("lower", b"\"gaussian\""),
    ("map_null", b"{\"Gaussian\":null}"),
    ("map_arr", b"{\"Gaussian\":[]}"),
    ("map_two", b"{\"Gaussian\":null,\"Uniform\":null}"),
    ("map_empty", b"{}"),
    ("unknown", b"\"Cosine\""),
    ("idx", b"0"),
];

/// Values a validating constructor refuses or accepts at its edges: (field, name, value).
fn semantic(ty: &str) -> Vec<(&'static str, &'static str, &'static [u8])> {
    match ty {
        "normal" => vec![
            ("std_dev", "sd0", b"0.0"),
            ("std_dev", "sd_neg", b"-1.0"),
            ("std_dev", "sd_min", b"5e-324"),
            ("mean", "mean_max", b"1.7976931348623157e308"),
        ],
        "binomial" => vec![
            ("n", "n0", b"0"),
            ("p", "p_over", b"1.5"),
            ("p", "p_negzero", b"-0.0"),
            ("p", "p_next", b"1.0000000000000002"),
        ],
        "kde" => vec![
            ("data", "empty", b"[]"),
            ("bandwidth", "bw0", b"0.0"),
            ("data", "data_e400", b"[1e400]"),
        ],
        "markov" => vec![
            ("transition_matrix", "empty", b"[]"),
            ("transition_matrix", "ragged", b"[[0.5,0.4]]"),
            ("transition_matrix", "sum", b"[[0.5,0.5],[0.5,0.6]]"),
            ("transition_matrix", "within", b"[[1.0000005,0.0],[0.0,1.0]]"),
            ("transition_matrix", "negzero", b"[[-0.0,1.0],[1.0,0.0]]"),
            ("transition_matrix", "neg", b"[[-0.5,1.5],[1.0,0.0]]"),
            ("state", "state_out", b"2"),
            ("state", "state_max", b"18446744073709551615"),
        ],
        "mvn" => vec![
            ("covariance", "ragged", b"[[4.0,0.0]]"),
            ("covariance", "asym", b"[[2.0,1.0],[1.1,2.0]]"),
            ("covariance", "not_pd", b"[[1.0,2.0],[2.0,1.0]]"),
            ("mean", "mean_empty", b"[]"),
            ("mean", "mean_dim3", b"[0.0,0.0,0.0]"),
            ("cholesky_l", "chol_str", b"\"x\""),
            ("log_det", "logdet_null", b"null"),
            ("log_det", "logdet_e400", b"1e400"),
            ("dim", "dim5", b"5"),
            ("dim", "dim_neg", b"-1"),
        ],
        "hist" => vec![("counts", "counts_neg", b"[-1]"), ("total", "total_neg", b"-1")],
        _ => vec![],
    }
}

fn syntax(base: &[u8]) -> Vec<Variant> {
    let n = base.len();
    let mut tc = base[..n - 1].to_vec();
    tc.extend_from_slice(b",}");
    let mut bom = vec![0xef, 0xbb, 0xbf];
    bom.extend_from_slice(base);
    vec![
        ("trunc1".into(), base[..n - 1].to_vec()),
        ("trunc_half".into(), base[..n / 2].to_vec()),
        ("garbage".into(), [base, b" x"].concat()),
        ("two_docs".into(), [base, base].concat()),
        ("ws_after".into(), [base, b" \n"].concat()),
        ("trailing_comma".into(), tc),
        ("bom".into(), bom),
        ("empty".into(), b"".to_vec()),
        ("ws_only".into(), b" ".to_vec()),
        ("null_doc".into(), b"null".to_vec()),
        ("num_doc".into(), b"0".to_vec()),
        ("str_doc".into(), b"\"x\"".to_vec()),
        ("obj_empty".into(), b"{}".to_vec()),
    ]
}

fn struct_variants(ty: &str, b: &Base) -> Vec<Variant> {
    let fs = fields(ty);
    let members: Vec<(String, Vec<u8>)> = fs
        .iter()
        .map(|f| (f.to_string(), serde_json::to_vec(&b.value[*f]).unwrap()))
        .collect();
    let mut out: Vec<Variant> = Vec::new();
    let with = |extra: &[(&str, &[u8])]| {
        let mut m = members.clone();
        for (k, val) in extra {
            m.push((k.to_string(), val.to_vec()));
        }
        obj(&m)
    };
    let mut rev = members.clone();
    rev.reverse();
    out.push(("rev".into(), obj(&rev)));
    let mut ws = b" {".to_vec();
    for (i, (k, val)) in members.iter().enumerate() {
        if i > 0 {
            ws.push(b',');
        }
        ws.extend_from_slice(format!("\n\t\"{k}\" : ").as_bytes());
        ws.extend_from_slice(val);
        ws.push(b' ');
    }
    ws.extend_from_slice(b"\r\n} ");
    out.push(("ws".into(), ws));
    let mut first = vec![("zz".to_string(), b"1".to_vec())];
    first.extend(members.iter().cloned());
    out.push(("unk_first".into(), obj(&first)));
    let deep = |d: usize| [b"[".repeat(d), b"]".repeat(d)].concat();
    let unknown: Vec<(&str, Vec<u8>)> = vec![
        ("unk_nested", br#"{"a":[1,2.5,{"b":null}],"c":"d"}"#.to_vec()),
        ("unk_dup_inner", br#"{"a":1,"a":2}"#.to_vec()),
        ("unk_deep100", deep(100)),
        ("unk_deep130", deep(130)),
        ("unk_ctl", b"\"a\x01b\"".to_vec()),
        ("unk_badutf8", b"\"a\xffb\"".to_vec()),
        ("unk_lone", b"\"\\ud800\"".to_vec()),
        ("unk_e400", b"1e400".to_vec()),
        ("unk_big", b"18446744073709551616".to_vec()),
        ("unk_negzero", b"-0".to_vec()),
    ];
    for (kind, val) in &unknown {
        out.push((kind.to_string(), with(&[("zz", val)])));
    }
    out.push(("unk_dup".into(), with(&[("zz", b"1"), ("zz", b"2")])));
    let mut esc = members.clone();
    let k0 = esc[0].0.clone();
    esc[0].0 = format!("\\u{:04x}{}", k0.as_bytes()[0], &k0[1..]);
    out.push(("key_escaped".into(), obj(&esc)));
    for (j, f) in fs.iter().enumerate() {
        let mut miss = members.clone();
        miss.remove(j);
        out.push((format!("missing:{f}"), obj(&miss)));
        out.push((format!("dup:{f}"), with(&[(f, &members[j].1)])));
        let mut forms: Vec<(&str, &[u8])> = vec![("null", b"null")];
        forms.extend_from_slice(NUMBERS);
        if b.value[*f].is_string() {
            forms.extend_from_slice(STRINGS);
        }
        if ty == "kde" && *f == "kernel" {
            forms.extend_from_slice(KERNELS);
        }
        for (name, val) in forms {
            let mut m = members.clone();
            m[j].1 = val.to_vec();
            out.push((format!("set:{f}:{name}"), obj(&m)));
        }
    }
    for (f, name, val) in semantic(ty) {
        let j = fs.iter().position(|x| *x == f).unwrap();
        let mut m = members.clone();
        m[j].1 = val.to_vec();
        out.push((format!("sem:{name}"), obj(&m)));
    }
    let vals: Vec<Vec<u8>> = members.iter().map(|(_, val)| val.clone()).collect();
    out.push(("seq".into(), seq(&vals)));
    out.push(("seq_short".into(), seq(&vals[..vals.len() - 1])));
    let mut long = vals.clone();
    long.push(b"0".to_vec());
    out.push(("seq_long".into(), seq(&long)));
    out.push(("seq_empty".into(), b"[]".to_vec()));
    out.extend(syntax(&b.doc));
    out
}

fn error_variants(b: &Base) -> Vec<Variant> {
    let forms: &[(&str, &[u8])] = &[
        ("unit", b"\"InvalidParameter\""),
        ("map_null", b"{\"InvalidParameter\":null}"),
        ("map_num", b"{\"InvalidParameter\":5}"),
        ("map_arr", b"{\"InvalidParameter\":[\"m\"]}"),
        ("unknown", b"{\"Nope\":\"m\"}"),
        ("two", b"{\"InvalidParameter\":\"m\",\"InvalidSample\":\"n\"}"),
        ("arr", b"[]"),
        ("arr1", b"[\"m\"]"),
        ("ws", b" { \"InvalidParameter\" :\n\"m\" } "),
        ("lower", b"{\"invalidParameter\":\"m\"}"),
        ("key_escaped", b"{\"\\u0049nvalidParameter\":\"m\"}"),
    ];
    let mut out: Vec<Variant> = forms.iter().map(|(k, d)| (k.to_string(), d.to_vec())).collect();
    for (name, val) in STRINGS {
        out.push((
            format!("msg:{name}"),
            [b"{\"InvalidParameter\":".as_slice(), val, b"}"].concat(),
        ));
    }
    out.extend(syntax(&b.doc));
    out
}

fn emit(w: &mut impl std::io::Write, id: &str, ty: &str, doc: &[u8]) {
    let o = decode(ty, doc);
    writeln!(
        w,
        "{id}\t{ty}\t{}\t{}\t{}\t{}\t{}",
        hex(doc),
        o.verdict,
        o.tokens,
        hex(&o.canon),
        o.msg
    )
    .unwrap();
}

fn corpus() {
    let mut w = BufWriter::new(std::io::stdout().lock());
    for ty in TYPES {
        let bs = bases(ty);
        for (i, b) in bs.iter().enumerate() {
            emit(&mut w, &format!("{ty}/{i}/base"), ty, &b.doc);
            emit(&mut w, &format!("{ty}/{i}/pretty"), ty, &b.pretty);
        }
        let vs = if ty == "error" {
            error_variants(&bs[0])
        } else {
            struct_variants(ty, &bs[0])
        };
        for (kind, doc) in vs {
            emit(&mut w, &format!("{ty}/0/{kind}"), ty, &doc);
        }
    }
}

fn collect_floats(v: &Value, out: &mut Vec<u64>) {
    match v {
        Value::Number(n) if n.is_f64() => out.push(n.as_f64().unwrap().to_bits()),
        Value::Array(a) => a.iter().for_each(|e| collect_floats(e, out)),
        Value::Object(m) => m.values().for_each(|e| collect_floats(e, out)),
        _ => {}
    }
}

fn floats() {
    let mut set: Vec<u64> = Vec::new();
    for ty in TYPES {
        for b in bases(ty) {
            collect_floats(&b.value, &mut set);
        }
    }
    for e in 1..2047u64 {
        set.push(e << 52);
    }
    for k in 0..52 {
        set.push(1u64 << k);
    }
    for k in -325..=308 {
        let x: f64 = format!("1e{k}").parse().unwrap();
        let b = x.to_bits();
        set.extend([b, b.wrapping_sub(1), b + 1]);
    }
    for i in 0..=1000u64 {
        set.push((i as f64).to_bits());
    }
    let mut g = G(0x5eed_0007);
    for _ in 0..100_000 {
        set.push(g.finite().to_bits());
    }
    for _ in 0..50_000 {
        let digits = 1 + g.word() % 6;
        let d = g.word() % 10u64.pow(digits as u32);
        let e = (g.word() % 61) as i64 - 30;
        let x: f64 = format!("{d}e{e}").parse().unwrap();
        set.push(x.to_bits());
    }
    set.retain(|b| (b >> 52) & 0x7ff != 0x7ff);
    set.extend(set.clone().iter().map(|b| b ^ (1u64 << 63)));
    set.sort_unstable();
    set.dedup();
    let mut w = BufWriter::new(std::io::stdout().lock());
    for b in set {
        let x = f64::from_bits(b);
        writeln!(w, "{b:016x}\t{}\t{x}", serde_json::to_string(&x).unwrap()).unwrap();
    }
}

fn read(path: &str) {
    let text = std::fs::read_to_string(path).unwrap();
    let mut w = BufWriter::new(std::io::stdout().lock());
    for line in text.lines() {
        let p: Vec<&str> = line.split('\t').collect();
        if p[0] != "D" {
            continue;
        }
        for (which, col) in [("fmtf", 6), ("grisu", 7)] {
            if p[col].is_empty() {
                continue;
            }
            let o = decode(p[2], &unhex(p[col]));
            writeln!(w, "{}\t{which}\t{}\t{}", p[1], o.verdict, o.tokens).unwrap();
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("corpus") => corpus(),
        Some("floats") => floats(),
        Some("read") => read(&args[2]),
        _ => eprintln!("usage: p7_json corpus | floats | read <p7_cyrius.tsv>"),
    }
}
