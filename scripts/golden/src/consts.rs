//! Every constant pramana 1.3.0's library code names, read from `rust-old/src` (port plan §5 step 6).
//!
//! pramana's constants are private: module consts, the fdlibm table inside `mod fdlibm`, and tables local to a
//! function (Lanczos in `ln_gamma`, Acklam in `z_quantile`). No API exposes them, so their values come from the
//! source text, through Rust's own parser: `str::parse::<f64>` rounds a decimal exactly as rustc rounds the
//! literal. `p_constants.rs` then proves each table by recomputing the oracle's functions with the extracted values,
//! bit for bit.
//!
//! Completeness: every `const` item in the library code (outside `#[cfg(test)] mod tests`) must be extracted, and the
//! two counts must agree. A form this extractor doesn't know is refused, never skipped.
//!
//! The Rust std float constants the code names (`f64::NAN`, `consts::PI`, …) come from Rust itself, not from text.

use std::fs;
use std::path::Path;

/// One constant's value.
#[derive(Clone, Copy, Debug)]
pub enum Val {
    F(f64),
    I(i64),
}

/// One constant: its Rust path, its value, and whether it is public API.
#[derive(Clone, Debug)]
pub struct Const {
    /// `math::LN_SQRT_2PI`, `math::fdlibm::PP0`, `math::ln_gamma::COEFFICIENTS[3]`, `f64::NAN`, `f64::consts::PI`.
    pub path: String,
    pub val: Val,
    /// `pub const` in a public module (only `ai`'s two bounds). Everything else is private to the crate.
    pub public: bool,
}

/// The library part of a source file: everything before `#[cfg(test)]` + `mod tests`.
fn library_part(text: &str) -> &str {
    let mut at = text.len();
    let mut search = 0;
    while let Some(k) = text[search..].find("#[cfg(test)]\nmod tests") {
        at = search + k;
        search = at + 1;
    }
    &text[..at]
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// The name of the nearest enclosing `fn` or `mod`, by indentation; None at module level.
fn scope(lines: &[&str], at: usize) -> Option<String> {
    let ind = indent(lines[at]);
    if ind == 0 {
        return None;
    }
    for l in lines[..at].iter().rev() {
        if l.trim().is_empty() || indent(l) >= ind {
            continue;
        }
        let t = l.trim_start();
        let t = t
            .strip_prefix("pub(crate) ")
            .or_else(|| t.strip_prefix("pub(super) "))
            .or_else(|| t.strip_prefix("pub "))
            .unwrap_or(t);
        for kw in ["fn ", "mod "] {
            if let Some(rest) = t.strip_prefix(kw) {
                let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                return Some(name);
            }
        }
    }
    None
}

/// A Rust numeric literal (underscores, exponent, optional `f64` / integer suffix) as Rust parses it.
fn literal(text: &str, float: bool) -> Result<Val, String> {
    let t: String = text.trim().chars().filter(|&c| c != '_').collect();
    let t = t
        .trim_end_matches("f64")
        .trim_end_matches("usize")
        .trim_end_matches("u64");
    if float {
        t.parse::<f64>()
            .map(Val::F)
            .map_err(|e| format!("float literal {text:?}: {e}"))
    } else {
        t.parse::<i64>()
            .map(Val::I)
            .map_err(|e| format!("integer literal {text:?}: {e}"))
    }
}

/// The value of a const's right-hand side: a literal, `<int> << <int>`, or `<float> - <NAME in scope>`. Rust
/// evaluates these in IEEE f64 / integer arithmetic at compile time; so does this.
fn value(expr: &str, float: bool, scope_vals: &[(String, Val)]) -> Result<Val, String> {
    let e = expr.trim();
    if let Some((a, b)) = e.split_once("<<") {
        if let (Val::I(x), Val::I(y)) = (literal(a, false)?, literal(b, false)?) {
            return Ok(Val::I(x << y));
        }
    }
    if let Some((a, b)) = e.split_once(" - ") {
        let name = b.trim();
        let rhs = scope_vals
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| *v)
            .ok_or_else(|| format!("`{e}`: {name} is not a constant in scope"))?;
        return match (literal(a, true)?, rhs) {
            (Val::F(x), Val::F(y)) => Ok(Val::F(x - y)),
            _ => Err(format!("`{e}`: not an f64 difference")),
        };
    }
    literal(e, float)
}

/// Every `const` item of one file's library code.
fn extract_file(module: &str, text: &str, out: &mut Vec<Const>) -> Result<usize, String> {
    let lib = library_part(text);
    let lines: Vec<&str> = lib.lines().collect();
    let mut seen = 0;
    let mut scope_vals: Vec<(String, Val)> = Vec::new();
    let mut k = 0;
    while k < lines.len() {
        let line = lines[k];
        let t = line.trim_start();
        let public = t.starts_with("pub const ");
        let body = t
            .strip_prefix("pub(crate) const ")
            .or_else(|| t.strip_prefix("pub(super) const "))
            .or_else(|| t.strip_prefix("pub const "))
            .or_else(|| t.strip_prefix("const "));
        let Some(body) = body else {
            k += 1;
            continue;
        };
        if body.starts_with("fn ") {
            k += 1;
            continue;
        }
        seen += 1;
        let (name, rest) = body
            .split_once(':')
            .ok_or_else(|| format!("{module}:{}: no type in `{t}`", k + 1))?;
        let name = name.trim().to_string();
        let (ty, rhs) = rest
            .split_once('=')
            .ok_or_else(|| format!("{module}:{}: no value in `{t}`", k + 1))?;
        let ty = ty.trim();
        let mut path = module.to_string();
        if let Some(s) = scope(&lines, k) {
            path = format!("{path}::{s}");
        }
        let rhs = rhs.trim();
        if ty.starts_with("[f64;") {
            // An array: its elements, one per line, until `];`.
            let mut idx = 0;
            k += 1;
            while k < lines.len() && lines[k].trim() != "];" {
                let item = lines[k].trim().trim_end_matches(',');
                let v = literal(item, true).map_err(|e| format!("{module}:{}: {e}", k + 1))?;
                out.push(Const { path: format!("{path}::{name}[{idx}]"), val: v, public });
                idx += 1;
                k += 1;
            }
            if rhs != "[" {
                return Err(format!("{module}: array {name} must open with `[` on its own line"));
            }
            k += 1;
            continue;
        }
        let float = match ty {
            "f64" => true,
            "usize" | "u64" | "u32" | "i64" | "i32" => false,
            _ => return Err(format!("{module}:{}: type {ty} not handled", k + 1)),
        };
        let expr = rhs
            .strip_suffix(';')
            .ok_or_else(|| format!("{module}:{}: `{t}` is not one line", k + 1))?;
        let v = value(expr, float, &scope_vals).map_err(|e| format!("{module}:{}: {e}", k + 1))?;
        scope_vals.push((name.clone(), v));
        out.push(Const { path: format!("{path}::{name}"), val: v, public });
        k += 1;
    }
    Ok(seen)
}

/// The Rust std float constants the library code names, with their values from Rust itself.
fn std_constants(texts: &[String], out: &mut Vec<Const>) {
    use std::f64::consts;
    let named: [(&str, &[&str], f64); 10] = [
        ("f64::EPSILON", &["f64::EPSILON"], f64::EPSILON),
        ("f64::MIN_POSITIVE", &["f64::MIN_POSITIVE"], f64::MIN_POSITIVE),
        ("f64::MAX", &["f64::MAX"], f64::MAX),
        ("f64::INFINITY", &["f64::INFINITY"], f64::INFINITY),
        ("f64::NEG_INFINITY", &["f64::NEG_INFINITY"], f64::NEG_INFINITY),
        ("f64::NAN", &["f64::NAN"], f64::NAN),
        ("f64::consts::PI", &["consts::PI", "consts::{PI"], consts::PI),
        ("f64::consts::E", &["consts::E"], consts::E),
        ("f64::consts::TAU", &["consts::TAU"], consts::TAU),
        ("f64::consts::SQRT_2", &["SQRT_2"], consts::SQRT_2),
    ];
    for (path, needles, v) in named {
        let used = texts.iter().any(|t| {
            needles.iter().any(|n| {
                t.match_indices(n).any(|(i, _)| {
                    // A whole name: not followed by another identifier character (`f64::MAX` vs `f64::MAX_EXP`).
                    !t[i + n.len()..].starts_with(|c: char| c.is_alphanumeric() || c == '_')
                })
            })
        });
        if used {
            out.push(Const { path: path.to_string(), val: Val::F(v), public: false });
        }
    }
}

/// Every constant, with the completeness check. `nudge`: the path of an f64 to move by one ulp (the mutation test).
pub fn extract(root: &Path, nudge: Option<&str>) -> Result<Vec<Const>, String> {
    let src = root.join("rust-old/src");
    let mut files: Vec<(String, std::path::PathBuf)> = Vec::new();
    for entry in fs::read_dir(&src).map_err(|e| format!("{}: {e}", src.display()))?.flatten() {
        let p = entry.path();
        if p.extension().is_some_and(|x| x == "rs") {
            let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            files.push((stem, p));
        }
    }
    files.push(("integration::soorat".to_string(), src.join("integration/soorat.rs")));
    files.sort();
    let mut out = Vec::new();
    let mut texts = Vec::new();
    let mut items = 0;
    for (module, path) in &files {
        let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        items += extract_file(module, &text, &mut out)?;
        texts.push(library_part(&text).to_string());
    }
    // Completeness: the const items found by a plain count must all have been extracted (arrays count once).
    let counted: usize = texts
        .iter()
        .map(|t| {
            t.lines()
                .filter(|l| {
                    let s = l.trim_start();
                    (s.starts_with("const ") || s.contains(" const ") && !s.starts_with("//"))
                        && !s.contains("const fn")
                })
                .count()
        })
        .sum();
    if counted != items {
        return Err(format!("{counted} const items in rust-old/src, {items} extracted"));
    }
    std_constants(&texts, &mut out);
    if let Some(n) = nudge {
        let c = out
            .iter_mut()
            .find(|c| c.path == n)
            .ok_or_else(|| format!("GOLDEN_NUDGE: no constant {n}"))?;
        if let Val::F(x) = c.val {
            c.val = Val::F(f64::from_bits(x.to_bits() + 1));
        }
    }
    Ok(out)
}

/// The value of `path` (an f64), for the proofs.
pub fn f(consts: &[Const], path: &str) -> f64 {
    match consts.iter().find(|c| c.path == path).map(|c| c.val) {
        Some(Val::F(x)) => x,
        _ => f64::NAN, // a missing constant makes every proof that reads it fail
    }
}

/// The Cyrius name: `PMN_` for public API, `_PMN_` otherwise, then the path upper-cased with `_` between parts.
/// `math::ln_gamma::COEFFICIENTS[3]` → `_PMN_MATH_LN_GAMMA_COEFFICIENTS_3`; `ai::MAX_FORECAST_STEPS` →
/// `PMN_AI_MAX_FORECAST_STEPS`; `f64::consts::PI` → `_PMN_F64_PI`.
pub fn cyr_name(c: &Const) -> String {
    let path = c.path.replace("::consts::", "::");
    let body: String = path
        .replace("::", "_")
        .replace('[', "_")
        .replace(']', "")
        .to_uppercase();
    if c.public { format!("PMN_{body}") } else { format!("_PMN_{body}") }
}
