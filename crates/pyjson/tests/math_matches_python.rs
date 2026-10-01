//! Ticket 211: Rust `math` and the probe inputs match the hashes `record_math.py` took from CPython.

use hydra_pyjson::math::{exp, fsum_like_python};
use hydra_pyjson::math_probe::{
    PROBE_BLOCK_SIZE, PROBE_SUM_LISTS, PROBE_UNARY_COUNT, all_sum_lists, all_unary_inputs,
    unary_log, unary_log1p,
};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/math")
}

fn read_lines(name: &str) -> Vec<String> {
    let path = fixtures().join(name);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    text.lines().map(str::to_string).collect()
}

fn pack_f64(x: f64) -> [u8; 8] {
    x.to_bits().to_be_bytes()
}

fn digest_hex(hasher: Sha256) -> String {
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

fn block_hashes<F>(xs: &[f64], block_size: usize, transform: F) -> Vec<String>
where
    F: Fn(f64) -> f64,
{
    let mut blocks = Vec::new();
    for start in (0..xs.len()).step_by(block_size) {
        let end = (start + block_size).min(xs.len());
        let mut hasher = Sha256::new();
        for &x in &xs[start..end] {
            hasher.update(pack_f64(transform(x)));
        }
        blocks.push(digest_hex(hasher));
    }
    blocks
}

fn assert_hashes(name: &str, got: Vec<String>) {
    let expected = read_lines(&format!("{name}_hashes.txt"));
    assert_eq!(got.len(), expected.len(), "{name}: block count mismatch");
    for (block, (exp, got_hash)) in expected.iter().zip(got.iter()).enumerate() {
        if exp == got_hash {
            continue;
        }
        panic!("{name} block {block} hash differs (expected {exp}, got {got_hash})",);
    }
}

#[test]
fn math_unary_and_sum_match_cpython_hashes() {
    let _ = std::fs::read_to_string(fixtures().join("version.txt"))
        .expect("run tests/record_math.py to write fixtures/math/");
    assert_eq!(PROBE_UNARY_COUNT, 1_000_000);
    assert_eq!(PROBE_SUM_LISTS, 10_000);

    let xs = all_unary_inputs();
    assert_hashes("input", block_hashes(&xs, PROBE_BLOCK_SIZE, |x| x));
    assert_hashes("exp", block_hashes(&xs, PROBE_BLOCK_SIZE, exp));
    assert_hashes("log", block_hashes(&xs, PROBE_BLOCK_SIZE, unary_log));
    assert_hashes("log1p", block_hashes(&xs, PROBE_BLOCK_SIZE, unary_log1p));

    let expected_sums = read_lines("sum_hashes.txt");
    assert_eq!(expected_sums.len(), PROBE_SUM_LISTS);
    let lists = all_sum_lists();
    for (i, (values, exp_hash)) in lists.iter().zip(expected_sums.iter()).enumerate() {
        let total = fsum_like_python(values);
        let mut hasher = Sha256::new();
        hasher.update(pack_f64(total));
        let got = digest_hex(hasher);
        if exp_hash != &got {
            panic!(
                "first sum mismatch at list {i}: fsum_like_python bits {:016x}",
                total.to_bits()
            );
        }
    }
}
