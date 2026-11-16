//! `json.tsv` — serde_json 1.0.154's reading of a document as each shape 1.3.0's serialized types are made of (M1
//! parts d3a and d3b; port plan D5, §8.3):
//! - the value types (d3a): `f64`, `u64`, `usize`, `bool`, `String`, `Vec<f64>`, `Vec<u64>`, `Vec<String>` and
//!   `Vec<Vec<f64>>`;
//! - serde derive's struct rules (d3b), on two test structs: `Pair`, `Normal`'s two f64 fields, and `Shapes`, one
//!   field of every kind 1.3.0's types hold, `Kernel` included;
//! - the two enum forms (d3b), on 1.3.0's own enums: `Kernel` (unit variants) and `PramanaError` (newtype variants
//!   holding a String).
//!
//! Each row is `serde_json::from_slice::<T>(document)` with `float_roundtrip` on (a correctly rounded parse, as
//! probe P7 found a Rust consumer needs): the value as tokens, or `E:Deserialize` for any refusal.
//!
//! The documents:
//! - for every type: the other JSON types, and syntax faults (truncation, trailing content, a byte-order mark,
//!   comments, whitespace inside and around);
//! - number forms: every integer boundary (2^53, i64, u64), `-0`, the f64 range edges, subnormals, exponent forms
//!   (and exponents past 2^64), long digit strings, and the malformed ones (`01`, `+1`, `.5`, `1.`, `NaN`);
//! - floats as text: serde_json's own for 600 seeded doubles, Rust's `{:e}` and `{:.25e}`, random decimals, and the
//!   exact midpoint between two adjacent doubles (a tie: to even), just below it and just above it, for 120 pairs;
//! - u64s: 200 seeded words;
//! - strings: every escape, surrogate pairs and lone surrogates, raw control bytes, invalid UTF-8 (overlong,
//!   encoded surrogates, past U+10FFFF, truncated), and 500 seeded strings, raw and escaped;
//! - vectors and matrices: each form above as an element, and seeded ones, ragged matrices included.
//!
//! - for the structs: each field missing, named twice, and set to each form of its kind; unknown members of every
//!   kind (nested, holding a duplicate key, nested past 128, holding a lone surrogate, a control byte, invalid UTF-8,
//!   numbers out of range), a key that is not UTF-8, escaped keys, the sequence form at every length, syntax faults,
//!   and seeded values;
//! - for the enums: every variant in both forms, the wrong form, other cases, extra members and every string form as
//!   the payload.
//!
//! Class A: the bits, the integer, the bytes, or the refusal. Some documents serde_json reads only because it skips a
//! member without checking it, where bayan checks the whole document: `validate.py` finds them and writes their
//! refusals to `json.map.tsv` (ADR 007 row 16).

use crate::grids::Rng;
use crate::tsv::{Class, Module, Tok, b, deser, f, i, s, sb};
use pramana::PramanaError;
use pramana::descriptive::Kernel;
use serde::{Deserialize, Serialize};

/// `Normal`'s fields: serde derive's rules on the smallest struct pramana serializes.
#[derive(Serialize, Deserialize)]
struct Pair {
    mean: f64,
    std_dev: f64,
}

/// One field of every kind 1.3.0's serialized types hold.
#[derive(Serialize, Deserialize)]
struct Shapes {
    x: f64,
    n: u64,
    k: usize,
    flag: bool,
    name: String,
    xs: Vec<f64>,
    ns: Vec<u64>,
    labels: Vec<String>,
    m: Vec<Vec<f64>>,
    kernel: Kernel,
}

const KERNELS: [Kernel; 4] = [Kernel::Gaussian, Kernel::Epanechnikov, Kernel::Uniform, Kernel::Triangular];

/// The documents every type gets: the other JSON types and syntax faults.
fn common() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = [
        "", " ", "\n", "null", "true", "false", "0", "1", "-1", "1.5", "\"x\"", "\"\"", "[]", "[1]", "[\"a\"]",
        "[[1]]", "{}", "{\"a\":1}", "nul", "nulll", "tru", "fals", "[", "]", "{", "}", "\"", ",", ":", "x", "1 x",
        "1,", "1 1", " 1 ", "\t1\r\n", "/*c*/1", "1//c", "[1]]", "[[1]", "\"x\"\"y\"",
    ]
    .iter()
    .map(|t| t.as_bytes().to_vec())
    .collect();
    v.push(b"\xef\xbb\xbf1".to_vec());
    v.push(b"1\x00".to_vec());
    v.push(b"\x0c1".to_vec());
    v
}

/// Number forms, well-formed and not.
const NUMBERS: &[&str] = &[
    "0",
    "1",
    "-1",
    "-0",
    "-0.0",
    "0.0",
    "00",
    "01",
    "-01",
    "+1",
    ".5",
    "1.",
    "1.e3",
    "1e",
    "1e+",
    "-",
    "--1",
    "1.5.5",
    "1e3e3",
    "0x10",
    "1_000",
    "NaN",
    "Infinity",
    "-Infinity",
    "1.5",
    "1e3",
    "1E3",
    "1e+3",
    "1e-3",
    "1.0",
    "-0e5",
    "0e999",
    "1.5e300",
    "1e308",
    "1.7976931348623157e308",
    "1.7976931348623158e308",
    "1.7976931348623159e308",
    "2e308",
    "1e400",
    "-1e400",
    "1e-400",
    "4.9e-324",
    "5e-324",
    "2.4703282292062327e-324",
    "2.4703282292062328e-324",
    "2.2250738585072011e-308",
    "2.2250738585072012e-308",
    "2.2250738585072014e-308",
    "9007199254740992",
    "9007199254740993",
    "9007199254740995",
    "-9007199254740993",
    "9223372036854775807",
    "9223372036854775808",
    "9223372036854775809",
    "-9223372036854775807",
    "-9223372036854775808",
    "-9223372036854775809",
    "18446744073709551615",
    "18446744073709551616",
    "18446744073709551617",
    "-18446744073709551615",
    "-18446744073709551616",
    "100000000000000000000000",
    "123456789012345678901234567890e-10",
    "0.1000000000000000055511151231257827021181583404541015625",
    "1.00000000000000011102230246251565404236316680908203125",
    "1.00000000000000011102230246251565404236316680908203124",
    "1.00000000000000011102230246251565404236316680908203126",
    "0e18446744073709551616",
    "1e18446744073709551616",
    "1e-18446744073709551616",
    "-1e18446744073709551616",
    "1e9223372036854775807",
    "1e-9223372036854775808",
    "0.000000000000000000000000000000000000000001e42",
];

/// String documents: escapes, surrogates, control bytes, invalid UTF-8, unterminated forms.
fn strings() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = [
        r#""abc""#,
        r#""\b\f\n\r\t\"\\\/""#,
        r#""é""#,
        r#""É""#,
        "\"é\"",
        "\"😀\"",
        r#""😀""#,
        r#""😀""#,
        r#""\ud800""#,
        r#""\udc00""#,
        r#""\ud800A""#,
        r#""\ud800x""#,
        r#""\udc00\ud800""#,
        r#""\ud800\ud800""#,
        r#""\u0000""#,
        r#""\u001f""#,
        r#""\u007f""#,
        r#""￿""#,
        r#""\x""#,
        r#""\u12""#,
        r#""\u12G4""#,
        r#""\U00e9""#,
        r#""\""#,
        r#""a"#,
        r#"" ""#,
        "\"\u{7f}\"",
        "\"\u{10ffff}\"",
    ]
    .iter()
    .map(|t| t.as_bytes().to_vec())
    .collect();
    for raw in [
        &b"\"a\x01b\""[..],
        b"\"a\tb\"",
        b"\"a\nb\"",
        b"\"\x1f\"",
        b"\"\\\x01\"",
        b"\"a\xffb\"",
        b"\"\xc0\xaf\"",
        b"\"\xed\xa0\x80\"",
        b"\"\xf4\x90\x80\x80\"",
        b"\"\xe2\x82\"",
        b"\"\xf8\x88\x80\x80\x80\"",
        b"\"\x80\"",
        b"\"\xc2\"",
        b"\"\xe0\x80\xaf\"",
    ] {
        v.push(raw.to_vec());
    }
    v
}

/// A seeded string: printable ASCII, quotes, backslashes, control characters and wide code points.
fn random_string(g: &mut Rng) -> String {
    const POOL: &[char] = &[
        'a', 'Z', '0', ' ', '"', '\\', '/', '\n', '\t', '\u{1}', '\u{1f}', '\u{7f}', 'é', '€', '😀', '\u{ffff}',
        '\u{10ffff}', '\u{2028}', '\u{0}', '~', '%',
    ];
    let n = g.int(0, 12) as usize;
    (0..n)
        .map(|_| POOL[g.int(0, POOL.len() as i64 - 1) as usize])
        .collect()
}

/// A seeded string written with `\u` escapes where it can be: each character raw, escaped, or (past the BMP) a
/// surrogate pair.
fn escaped_string(g: &mut Rng) -> Vec<u8> {
    let mut d = b"\"".to_vec();
    for _ in 0..g.int(1, 8) {
        let c = loop {
            let cp = match g.int(0, 3) {
                0 => g.int(0x20, 0x7e) as u32,
                1 => g.int(0x80, 0xd7ff) as u32,
                2 => g.int(0xe000, 0xffff) as u32,
                _ => g.int(0x10000, 0x10ffff) as u32,
            };
            if let Some(c) = char::from_u32(cp) {
                break c;
            }
        };
        match g.int(0, 2) {
            0 if c != '"' && c != '\\' => {
                let mut buf = [0u8; 4];
                d.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
            1 if g.int(0, 1) == 0 => {
                let mut u = [0u16; 2];
                for w in c.encode_utf16(&mut u) {
                    d.extend_from_slice(format!("\\u{w:04X}").as_bytes());
                }
            }
            _ => {
                let mut u = [0u16; 2];
                for w in c.encode_utf16(&mut u) {
                    d.extend_from_slice(format!("\\u{w:04x}").as_bytes());
                }
            }
        }
    }
    d.push(b'"');
    d
}

/// A finite double from seeded bits.
fn finite(g: &mut Rng) -> f64 {
    loop {
        let x = f64::from_bits(g.word());
        if x.is_finite() {
            return x;
        }
    }
}

// ── the exact midpoint between two adjacent doubles, in decimal ──

/// A big unsigned integer in base 10^9, least significant limb first.
struct Big(Vec<u32>);

impl Big {
    fn from_u64(mut x: u64) -> Self {
        let mut v = Vec::new();
        while x > 0 {
            v.push((x % 1_000_000_000) as u32);
            x /= 1_000_000_000;
        }
        Big(v)
    }

    fn mul_small(&mut self, k: u32) {
        let mut carry = 0u64;
        for limb in &mut self.0 {
            let t = u64::from(*limb) * u64::from(k) + carry;
            *limb = (t % 1_000_000_000) as u32;
            carry = t / 1_000_000_000;
        }
        while carry > 0 {
            self.0.push((carry % 1_000_000_000) as u32);
            carry /= 1_000_000_000;
        }
    }

    fn digits(&self) -> String {
        let mut out = String::new();
        for (k, limb) in self.0.iter().rev().enumerate() {
            if k == 0 {
                out.push_str(&limb.to_string());
            } else {
                out.push_str(&format!("{limb:09}"));
            }
        }
        if out.is_empty() { "0".into() } else { out }
    }
}

/// The exact decimal of the midpoint between positive finite x and its successor, (2M + 1)·2^(E−1) for x = M·2^E,
/// as its significant digits and the power of ten of the first: d1.d2d3… × 10^e10. The last digit is 5 whenever
/// E − 1 < 0, which is every finite double below 2^53.
fn midpoint(x: f64) -> (String, i32) {
    let bits = x.to_bits();
    let exp = ((bits >> 52) & 0x7ff) as i32;
    let frac = bits & ((1u64 << 52) - 1);
    let (m, e) = if exp == 0 { (frac, -1074) } else { (frac | (1u64 << 52), exp - 1075) };
    let mut n = Big::from_u64(2 * m + 1);
    let k = e - 1;
    let point;
    if k >= 0 {
        for _ in 0..k {
            n.mul_small(2);
        }
        point = 0;
    } else {
        for _ in 0..-k {
            n.mul_small(5);
        }
        point = -k;
    }
    let d = n.digits();
    let e10 = d.len() as i32 - 1 - point;
    (d.trim_end_matches('0').to_string(), e10)
}

/// d1.d2d3…e±x from significant digits and the power of ten of the first.
fn sci(digits: &str, e10: i32) -> String {
    if digits.len() == 1 {
        format!("{digits}e{e10}")
    } else {
        format!("{}.{}e{e10}", &digits[..1], &digits[1..])
    }
}

/// The float texts: serde_json's, Rust's `{:e}` and `{:.25e}`, random decimals and the midpoints.
fn floats() -> Vec<Vec<u8>> {
    let mut g = Rng::new(0x6a09_e667_f3bc_c908);
    let mut v = Vec::new();
    for _ in 0..600 {
        let x = finite(&mut g);
        v.push(serde_json::to_vec(&x).expect("a finite double"));
    }
    for _ in 0..200 {
        let x = finite(&mut g);
        v.push(format!("{x:e}").into_bytes());
        let y = finite(&mut g);
        v.push(format!("{y:.25e}").into_bytes());
    }
    for _ in 0..400 {
        let nd = g.int(1, 30);
        let mut t = String::new();
        if g.int(0, 3) == 0 {
            t.push('-');
        }
        t.push(char::from(b'1' + g.int(0, 8) as u8));
        if nd > 1 {
            t.push('.');
            for _ in 1..nd {
                t.push(char::from(b'0' + g.int(0, 9) as u8));
            }
        }
        t.push_str(&format!("e{}", g.int(-345, 312)));
        v.push(t.into_bytes());
    }
    for k in 0..120 {
        let x = if k % 6 == 0 {
            f64::from_bits(g.int(1, (1i64 << 52) - 1) as u64)
        } else {
            let b = (g.word() & 0x7fef_ffff_ffff_ffff) | 0x0010_0000_0000_0000;
            f64::from_bits(b)
        };
        let (digits, e10) = midpoint(x);
        v.push(sci(&digits, e10).into_bytes());
        if let Some(stem) = digits.strip_suffix('5') {
            v.push(sci(&format!("{stem}49999"), e10).into_bytes());
        }
        v.push(sci(&format!("{digits}000001"), e10).into_bytes());
    }
    v
}

// ── rows ──

fn row<T>(m: &mut Module, func: &str, doc: &[u8], r: serde_json::Result<T>, toks: impl Fn(&T) -> Vec<Tok>) {
    match r {
        Ok(x) => m.row(func, Class::A, &[sb(doc)], &toks(&x)),
        Err(_) => m.row(func, Class::A, &[sb(doc)], &[deser()]),
    }
}

fn wrap(open: &str, elems: &[&[u8]], close: &str) -> Vec<u8> {
    let mut d = open.as_bytes().to_vec();
    for (k, e) in elems.iter().enumerate() {
        if k > 0 {
            d.push(b',');
        }
        d.extend_from_slice(e);
    }
    d.extend_from_slice(close.as_bytes());
    d
}

fn f64s(xs: &[f64]) -> Vec<Tok> {
    let mut t = vec![Tok::V(xs.len())];
    t.extend(xs.iter().map(|&x| f(x)));
    t
}

#[allow(clippy::cast_possible_wrap)]
fn u64s(xs: &[u64]) -> Vec<Tok> {
    let mut t = vec![Tok::V(xs.len())];
    t.extend(xs.iter().map(|&x| i(x as i64)));
    t
}

#[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn emit() -> Module {
    let mut m = Module::new("json");
    let mut g = Rng::new(0xbb67_ae85_84ca_a73b);
    let numbers: Vec<Vec<u8>> = NUMBERS.iter().map(|t| t.as_bytes().to_vec()).collect();
    let floats = floats();
    let strings = strings();

    // f64
    for d in common().iter().chain(&numbers).chain(&floats) {
        row(&mut m, "json::f64", d, serde_json::from_slice::<f64>(d), |&x| vec![f(x)]);
    }

    // u64 and usize
    let mut words: Vec<Vec<u8>> = Vec::new();
    for k in 0..200 {
        let w = g.word() >> (k % 64);
        words.push(w.to_string().into_bytes());
    }
    for d in common().iter().chain(&numbers).chain(&words) {
        row(&mut m, "json::u64", d, serde_json::from_slice::<u64>(d), |&x| vec![i(x as i64)]);
    }
    for d in common().iter().chain(&numbers) {
        row(&mut m, "json::usize", d, serde_json::from_slice::<usize>(d), |&x| vec![i(x as i64)]);
    }

    // bool
    let bools: Vec<Vec<u8>> = ["true", "false", " true ", "True", "TRUE", "tru", "truee", "true false", "\"true\"", "t"]
        .iter()
        .map(|t| t.as_bytes().to_vec())
        .collect();
    for d in common().iter().chain(&bools) {
        row(&mut m, "json::bool", d, serde_json::from_slice::<bool>(d), |&x| vec![b(x)]);
    }

    // String
    let mut texts: Vec<Vec<u8>> = Vec::new();
    for _ in 0..300 {
        let t = random_string(&mut g);
        texts.push(serde_json::to_vec(&t).expect("a string"));
    }
    for _ in 0..200 {
        texts.push(escaped_string(&mut g));
    }
    for d in common().iter().chain(&strings).chain(&texts) {
        row(&mut m, "json::String", d, serde_json::from_slice::<String>(d), |x| vec![sb(x.as_bytes())]);
    }

    // Vec<f64>
    let mut vecs: Vec<Vec<u8>> = [
        "[ ]", "[1,2,3]", " [ 1 , 2 ] ", "[1,]", "[,]", "[,1]", "[1 2]", "[1,,2]", "[1", "[null]", "[1,null]",
        "[\"1\"]", "[true]", "[{}]", "[[]]",
    ]
    .iter()
    .map(|t| t.as_bytes().to_vec())
    .collect();
    for n in &numbers {
        vecs.push(wrap("[", &[n], "]"));
        vecs.push(wrap("[", &[b"1", n], "]"));
    }
    for k in 0..150 {
        let len = g.int(0, 12) as usize;
        let xs: Vec<f64> = (0..len).map(|_| finite(&mut g)).collect();
        if k % 5 == 0 {
            vecs.push(serde_json::to_vec_pretty(&xs).expect("a vector"));
        } else {
            vecs.push(serde_json::to_vec(&xs).expect("a vector"));
        }
    }
    for d in common().iter().chain(&vecs) {
        row(&mut m, "json::Vec<f64>", d, serde_json::from_slice::<Vec<f64>>(d), |x| f64s(x));
    }

    // Vec<u64>
    let mut uvecs: Vec<Vec<u8>> = ["[ ]", "[0,1,2]", "[1,]", "[1.0]", "[-0]", "[-1]", "[null]", "[\"1\"]", "[[1]]"]
        .iter()
        .map(|t| t.as_bytes().to_vec())
        .collect();
    for n in &numbers {
        uvecs.push(wrap("[", &[n], "]"));
    }
    for _ in 0..100 {
        let len = g.int(0, 10) as usize;
        let xs: Vec<u64> = (0..len).map(|k| g.word() >> (k * 7 % 64)).collect();
        uvecs.push(serde_json::to_vec(&xs).expect("a vector"));
    }
    for d in common().iter().chain(&uvecs) {
        row(&mut m, "json::Vec<u64>", d, serde_json::from_slice::<Vec<u64>>(d), |x| u64s(x));
    }

    // Vec<String>
    let mut svecs: Vec<Vec<u8>> = ["[ ]", "[\"a\",\"b\"]", "[\"a\",1]", "[null]", "[\"a\",]", "[[\"a\"]]"]
        .iter()
        .map(|t| t.as_bytes().to_vec())
        .collect();
    for t in &strings {
        svecs.push(wrap("[", &[t], "]"));
        svecs.push(wrap("[", &[b"\"ok\"", t], "]"));
    }
    for _ in 0..100 {
        let len = g.int(0, 6) as usize;
        let xs: Vec<String> = (0..len).map(|_| random_string(&mut g)).collect();
        svecs.push(serde_json::to_vec(&xs).expect("a vector"));
    }
    for d in common().iter().chain(&svecs) {
        row(&mut m, "json::Vec<String>", d, serde_json::from_slice::<Vec<String>>(d), |x| {
            let mut t = vec![Tok::V(x.len())];
            t.extend(x.iter().map(|e| s(e.clone())));
            t
        });
    }

    // Vec<Vec<f64>>
    let mut mats: Vec<Vec<u8>> = [
        "[ ]", "[[]]", "[[],[]]", "[[1],[2,3]]", "[[1,2],[3,4]]", "[1]", "[[1],1]", "[[1],null]", "[null]",
        "[[\"x\"]]", "[[[1]]]", "[[1e400]]", "[[-0]]", "[[18446744073709551615]]", "[[1],]", "[[1,]]",
        " [ [ 1 , 2 ] , [ 3 ] ] ",
    ]
    .iter()
    .map(|t| t.as_bytes().to_vec())
    .collect();
    for depth in [3usize, 100, 128, 129, 200] {
        mats.push([b"[".repeat(depth), b"]".repeat(depth)].concat());
    }
    for k in 0..100 {
        let rows = g.int(0, 5) as usize;
        let ragged = k % 4 == 0;
        let cols = g.int(0, 5) as usize;
        let mat: Vec<Vec<f64>> = (0..rows)
            .map(|_| {
                let c = if ragged { g.int(0, 5) as usize } else { cols };
                (0..c).map(|_| finite(&mut g)).collect()
            })
            .collect();
        mats.push(serde_json::to_vec(&mat).expect("a matrix"));
    }
    for d in common().iter().chain(&mats) {
        row(&mut m, "json::Vec<Vec<f64>>", d, serde_json::from_slice::<Vec<Vec<f64>>>(d), |x| {
            let mut t = vec![Tok::V(x.len())];
            for r in x {
                t.extend(f64s(r));
            }
            t
        });
    }
    structs(&mut m, &mut g, &numbers, &strings);
    enums(&mut m, &strings);
    m
}

// ── d3b: serde derive's struct rules and the two enum forms ──

#[allow(clippy::cast_possible_wrap)]
fn shapes_toks(x: &Shapes) -> Vec<Tok> {
    let mut t = vec![f(x.x), i(x.n as i64), i(x.k as i64), b(x.flag), s(x.name.clone())];
    t.extend(f64s(&x.xs));
    t.extend(u64s(&x.ns));
    t.push(Tok::V(x.labels.len()));
    t.extend(x.labels.iter().map(|e| s(e.clone())));
    t.push(Tok::V(x.m.len()));
    for r in &x.m {
        t.extend(f64s(r));
    }
    t.push(Tok::E("Kernel", format!("{:?}", x.kernel)));
    t
}

fn bytes(list: &[&str]) -> Vec<Vec<u8>> {
    list.iter().map(|t| t.as_bytes().to_vec()).collect()
}

fn obj(members: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut d = b"{".to_vec();
    for (k, (key, val)) in members.iter().enumerate() {
        if k > 0 {
            d.push(b',');
        }
        d.extend_from_slice(format!("\"{key}\":").as_bytes());
        d.extend_from_slice(val);
    }
    d.push(b'}');
    d
}

/// A struct's documents: P7's variants of one base value, given its fields' names and each field's forms.
fn struct_docs(base: &serde_json::Value, fields: &[&str], forms: &[Vec<Vec<u8>>]) -> Vec<Vec<u8>> {
    let members: Vec<(String, Vec<u8>)> = fields
        .iter()
        .map(|f| (f.to_string(), serde_json::to_vec(&base[*f]).expect("a field")))
        .collect();
    let with = |extra: &[(&str, &[u8])]| {
        let mut ms = members.clone();
        for (k, v) in extra {
            ms.push((k.to_string(), v.to_vec()));
        }
        obj(&ms)
    };
    let mut out = vec![obj(&members)];
    let mut rev = members.clone();
    rev.reverse();
    out.push(obj(&rev));
    let mut ws = b" {".to_vec();
    for (k, (key, val)) in members.iter().enumerate() {
        if k > 0 {
            ws.push(b',');
        }
        ws.extend_from_slice(format!("\n\t\"{key}\" : ").as_bytes());
        ws.extend_from_slice(val);
        ws.push(b' ');
    }
    ws.extend_from_slice(b"\r\n} ");
    out.push(ws);
    let mut first = vec![("zz".to_string(), b"1".to_vec())];
    first.extend(members.iter().cloned());
    out.push(obj(&first));
    let deep = |d: usize| [b"[".repeat(d), b"]".repeat(d)].concat();
    let unknown: Vec<Vec<u8>> = vec![
        br#"{"a":[1,2.5,{"b":null}],"c":"d"}"#.to_vec(),
        br#"{"a":1,"a":2}"#.to_vec(),
        br#"{"a":1,"\u0061":2}"#.to_vec(),
        deep(100),
        deep(128),
        deep(129),
        deep(130),
        b"\"a\x01b\"".to_vec(),
        b"\"a\xffb\"".to_vec(),
        br#""\ud800""#.to_vec(),
        br#""\udc00""#.to_vec(),
        br#"{"\ud800":1}"#.to_vec(),
        br#""\x""#.to_vec(),
        b"1e400".to_vec(),
        b"18446744073709551616".to_vec(),
        b"-0".to_vec(),
        b"01".to_vec(),
        b"[1,]".to_vec(),
        b"nul".to_vec(),
    ];
    for val in &unknown {
        out.push(with(&[("zz", val)]));
    }
    out.push(with(&[("zz", b"1"), ("zz", b"2")]));
    out.push(with(&[("zz", b"1"), ("\\u007a\\u007a", b"2")]));
    let mut badkey = members.clone();
    badkey.push(("z\u{fffd}".to_string(), b"1".to_vec()));
    let mut bk = obj(&badkey);
    let at = bk.windows(3).position(|w| w == "\u{fffd}".as_bytes()).expect("the marker");
    bk.splice(at..at + 3, [0xffu8]);
    out.push(bk);
    out.push(with(&[("\\udc00", b"1")]));
    let mut esc = members.clone();
    let k0 = esc[0].0.clone();
    esc[0].0 = format!("\\u{:04x}{}", k0.as_bytes()[0], &k0[1..]);
    out.push(obj(&esc));
    let mut upper = members.clone();
    upper[0].0 = upper[0].0.to_uppercase();
    out.push(obj(&upper));
    let mut spaced = members.clone();
    spaced[0].0 = format!("{} ", spaced[0].0);
    out.push(obj(&spaced));
    for (j, field) in fields.iter().enumerate() {
        let mut miss = members.clone();
        miss.remove(j);
        out.push(obj(&miss));
        out.push(with(&[(field, &members[j].1)]));
        let mut nul = members.clone();
        nul[j].1 = b"null".to_vec();
        out.push(obj(&nul));
        for form in &forms[j] {
            let mut ms = members.clone();
            ms[j].1 = form.clone();
            out.push(obj(&ms));
        }
    }
    let vals: Vec<&[u8]> = members.iter().map(|(_, v)| v.as_slice()).collect();
    out.push(wrap("[", &vals, "]"));
    out.push(wrap(" [ ", &vals, " ] "));
    out.push(wrap("[", &vals[..vals.len() - 1], "]"));
    let mut long = vals.clone();
    long.push(b"0");
    out.push(wrap("[", &long, "]"));
    out.push(wrap("[", &vals, ",]"));
    out.push(b"[]".to_vec());
    let doc = obj(&members);
    let n = doc.len();
    let mut tc = doc[..n - 1].to_vec();
    tc.extend_from_slice(b",}");
    out.push(tc);
    out.push(doc[..n - 1].to_vec());
    out.push(doc[..n / 2].to_vec());
    out.push([doc.as_slice(), b" x"].concat());
    out.push([doc.as_slice(), doc.as_slice()].concat());
    out.push([b"\xef\xbb\xbf".as_slice(), doc.as_slice()].concat());
    let nocolon = doc.iter().position(|&c| c == b':').expect("a colon");
    let mut nc = doc.clone();
    nc.remove(nocolon);
    out.push(nc);
    out.push(br#"{1:2}"#.to_vec());
    out.push(b"{}".to_vec());
    out
}

fn random_shapes(g: &mut Rng) -> Shapes {
    let n = g.int(0, 4) as usize;
    Shapes {
        x: finite(g),
        n: g.word(),
        k: (g.word() >> g.int(0, 63)) as usize,
        flag: g.int(0, 1) == 1,
        name: random_string(g),
        xs: (0..g.int(0, 6)).map(|_| finite(g)).collect(),
        ns: (0..g.int(0, 6)).map(|_| g.word()).collect(),
        labels: (0..g.int(0, 4)).map(|_| random_string(g)).collect(),
        m: (0..n).map(|_| (0..g.int(0, 3)).map(|_| finite(g)).collect()).collect(),
        kernel: KERNELS[g.int(0, 3) as usize],
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn structs(m: &mut Module, g: &mut Rng, numbers: &[Vec<u8>], strings: &[Vec<u8>]) {
    // Pair: every number form in each field.
    let pair = serde_json::to_value(Pair { mean: 1.5, std_dev: 0.25 }).expect("a pair");
    let mut docs = struct_docs(&pair, &["mean", "std_dev"], &[numbers.to_vec(), numbers.to_vec()]);
    for k in 0..150 {
        let p = Pair { mean: finite(g), std_dev: finite(g) };
        docs.push(if k % 5 == 0 {
            serde_json::to_vec_pretty(&p).expect("a pair")
        } else {
            serde_json::to_vec(&p).expect("a pair")
        });
    }
    for d in &docs {
        row(m, "json::Pair", d, serde_json::from_slice::<Pair>(d), |p| vec![f(p.mean), f(p.std_dev)]);
    }

    // Shapes: each field set to each form of its kind.
    let ints = bytes(&[
        "0", "1", "-1", "-0", "1.0", "1e3", "9223372036854775808", "18446744073709551615", "18446744073709551616",
        "\"1\"", "true", "[]",
    ]);
    let bools = bytes(&["true", "false", "1", "\"true\"", "True"]);
    let mut texts = strings.to_vec();
    texts.extend(bytes(&["1", "[]", "{}"]));
    let vf = bytes(&["[]", "[1,2]", "[null]", "[\"1\"]", "[[1]]", "1", "{}", "[-0]", "[1e400]", "[1,]", "[18446744073709551615]"]);
    let vu = bytes(&["[]", "[1]", "[-1]", "[1.0]", "[18446744073709551615]", "[18446744073709551616]", "[\"1\"]", "1"]);
    let vs = bytes(&["[]", "[\"a\"]", "[1]", "[null]", r#"["\ud800"]"#, "\"a\"", "[\"a\",[]]"]);
    let mut vm = bytes(&["[]", "[[]]", "[[1],[2,3]]", "[1]", "[[null]]", "[[1e400]]", "[[-0]]", "[[1],[]]", "{}"]);
    vm.push([b"[".repeat(129), b"]".repeat(129)].concat());
    let kern = bytes(&[
        "\"Gaussian\"", "\"Triangular\"", "{\"Uniform\":null}", "\"gaussian\"", "\"Cosine\"", "0", "{\"Gaussian\":0}",
        "{}", "[\"Gaussian\"]", "null",
    ]);
    let base = Shapes {
        x: 0.5,
        n: 7,
        k: 3,
        flag: true,
        name: "t-test".into(),
        xs: vec![1.0, -0.0, 2.5],
        ns: vec![0, 1 << 63],
        labels: vec!["a".into(), "é".into()],
        m: vec![vec![1.0, 2.0], vec![3.0]],
        kernel: Kernel::Epanechnikov,
    };
    let value = serde_json::to_value(&base).expect("shapes");
    let fields = ["x", "n", "k", "flag", "name", "xs", "ns", "labels", "m", "kernel"];
    let forms = vec![numbers.to_vec(), ints.clone(), ints, bools, texts, vf, vu, vs, vm, kern];
    let mut docs = struct_docs(&value, &fields, &forms);
    for k in 0..40 {
        let x = random_shapes(g);
        docs.push(if k % 4 == 0 {
            serde_json::to_vec_pretty(&x).expect("shapes")
        } else {
            serde_json::to_vec(&x).expect("shapes")
        });
    }
    for d in &docs {
        row(m, "json::Shapes", d, serde_json::from_slice::<Shapes>(d), shapes_toks);
    }
}

fn enums(m: &mut Module, strings: &[Vec<u8>]) {
    let mut docs = bytes(&[
        "\"Gaussian\"",
        "\"Epanechnikov\"",
        "\"Uniform\"",
        "\"Triangular\"",
        " \"Uniform\" ",
        "{\"Gaussian\":null}",
        "{\"Epanechnikov\":null}",
        "{\"Uniform\":null}",
        "{\"Triangular\":null}",
        "{ \"Triangular\" : null }",
        "\"gaussian\"",
        "\"GAUSSIAN\"",
        "\"Cosine\"",
        "\"\"",
        "\"\\u0047aussian\"",
        "{\"\\u0047aussian\":null}",
        "{\"Gaussian\":[]}",
        "{\"Gaussian\":0}",
        "{\"Gaussian\":{}}",
        "{\"Gaussian\":nul}",
        "{\"Gaussian\" null}",
        "{\"Gaussian\":null,}",
        "{\"Gaussian\":null,\"Uniform\":null}",
        "{\"Gaussian\":null,\"Gaussian\":null}",
        "{}",
        "[\"Gaussian\"]",
        "0",
        "null",
        "\"Gaussian",
        "\"Gaussian\" x",
        "{\"Gaussian\":null} ",
    ]);
    docs.extend(common());
    for d in &docs {
        row(m, "json::Kernel", d, serde_json::from_slice::<Kernel>(d), |k| vec![Tok::E("Kernel", format!("{k:?}"))]);
    }

    let mut docs = bytes(&[
        "{\"InvalidParameter\":\"m\"}",
        "{\"InvalidSample\":\"s\"}",
        "{\"ConvergenceFailure\":\"c\"}",
        "{\"DimensionMismatch\":\"d\"}",
        "{\"ComputationError\":\"e\"}",
        " { \"InvalidParameter\" :\n\"m\" } ",
        "\"InvalidParameter\"",
        "{\"InvalidParameter\":null}",
        "{\"InvalidParameter\":5}",
        "{\"InvalidParameter\":[\"m\"]}",
        "{\"Nope\":\"m\"}",
        "{\"InvalidParameter\":\"m\",\"InvalidSample\":\"n\"}",
        "{\"invalidParameter\":\"m\"}",
        "{\"\\u0049nvalidParameter\":\"m\"}",
        "{\"InvalidParameter\":\"m\",}",
        "{}",
        "[]",
        "[\"m\"]",
    ]);
    for t in strings {
        docs.push([b"{\"InvalidParameter\":".as_slice(), t, b"}"].concat());
    }
    docs.extend(common());
    for d in &docs {
        row(m, "json::PramanaError", d, serde_json::from_slice::<PramanaError>(d), |e| {
            let (v, msg) = match e {
                PramanaError::InvalidParameter(x) => ("InvalidParameter", x),
                PramanaError::InvalidSample(x) => ("InvalidSample", x),
                PramanaError::ConvergenceFailure(x) => ("ConvergenceFailure", x),
                PramanaError::DimensionMismatch(x) => ("DimensionMismatch", x),
                PramanaError::ComputationError(x) => ("ComputationError", x),
                _ => unreachable!("1.3.0 has five variants"),
            };
            vec![Tok::E("PramanaError", v.to_string()), sb(msg.as_bytes())]
        });
    }
}

