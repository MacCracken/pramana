//! Records the compiler that builds the generator, so every TSV header names it and `--check` refuses a corpus
//! from another one (port plan M0 step 0.3: the recording toolchain is pinned).

use std::process::Command;

fn main() {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let version = Command::new(&rustc)
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.split_whitespace().nth(1).map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=GOLDEN_RUSTC_VERSION={version}");
    println!("cargo:rerun-if-env-changed=RUSTC");
}
