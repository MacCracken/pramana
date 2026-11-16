//! `json.tsv` — serde_json 1.0.154's reading of a document as each value type 1.3.0's serialized types are made of
//! (M1 part d3a; port plan D5, §8.3): `f64`, `u64`, `usize`, `bool`, `String`, `Vec<f64>`, `Vec<u64>`,
//! `Vec<String>` and `Vec<Vec<f64>>`. Their fields hold nothing else but two enums, which arrive with serde's struct
//! rules in part d3b. Each row is `serde_json::from_slice::<T>(document)` with `float_roundtrip` on (a correctly
//! rounded parse, as probe P7 found a Rust consumer needs): the value as tokens, or `E:Deserialize` for any refusal.
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
//! Class A: the bits, the integer, the bytes, or the refusal.

use crate::grids::Rng;
use crate::tsv::{Class, Module, Tok, b, deser, f, i, s, sb};

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
    m
}
