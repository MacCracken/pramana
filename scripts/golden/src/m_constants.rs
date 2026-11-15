//! `constants.tsv` — every constant `src/constants.cyr` defines, one row each: the Rust path and the value's bits
//! (port plan §5 step 6). `probe_tables.py` compiles the generated Cyrius file and compares each value with its row;
//! `validate.py` re-reads each literal from `rust-old/src` independently.

use crate::consts::{Const, Val};
use crate::tsv::{Class, Module, f, i, s};

pub fn emit(consts: &[Const]) -> Module {
    let mut m = Module::new("constants");
    for c in consts {
        let out = match c.val {
            Val::F(x) => f(x),
            Val::I(n) => i(n),
        };
        m.row("const", Class::A, &[s(c.path.clone())], &[out]);
    }
    m
}
