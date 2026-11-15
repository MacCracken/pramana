//! P6 (port plan §5 step 9): does Rust's unstable selection leak into a public result? `median` and `percentile`
//! select with `select_nth_unstable_by`, and two equal floats differ only when they are ±0. So the inputs are
//! zero-heavy: each element is −0, +0, −1, 1 or 2. Prints `case <TAB> input-bits... <TAB> result-bits`;
//! scripts/probes/p6_model.py compares each result with selection from a stable sort.

use pramana::descriptive::{median, percentile};

struct G(u64);
impl G {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
}

fn main() {
    let pool = [-0.0f64, 0.0, -1.0, 1.0, 2.0];
    let mut g = G(0x5e1e_c7);
    for case in 0..20_000 {
        let len = 1 + (g.next() % 24) as usize;
        // Zero-heavy: three draws in five are a zero.
        let data: Vec<f64> = (0..len)
            .map(|_| {
                let r = g.next() % 10;
                if r < 6 { pool[(r % 2) as usize] } else { pool[2 + (r % 3) as usize] }
            })
            .collect();
        let bits: Vec<String> = data.iter().map(|x| format!("{:016x}", x.to_bits())).collect();
        let m = median(&data).unwrap();
        println!("{case}\tmedian\t-\t{}\t{:016x}", bits.join(" "), m.to_bits());
        for p in [0.0, 10.0, 25.0, 33.3, 50.0, 75.0, 90.0, 100.0] {
            let v = percentile(&data, p).unwrap();
            println!("{case}\tpercentile\t{p}\t{}\t{:016x}", bits.join(" "), v.to_bits());
        }
    }
}
