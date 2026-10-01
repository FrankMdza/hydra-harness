//! Inputs for ticket 211: the same sequence `tests/record_math.py` writes hashes for.
//! Keep the two generators in lockstep.

const SEED: u64 = 21_100_943_147;
const UNARY_COUNT: usize = 1_000_000;
const BLOCK_SIZE: usize = 10_000;
const SUM_LISTS: usize = 10_000;
const LOG_EPS: f64 = 1e-12;

/// Build 213: the counts `baseline.py` takes `log1p` of. Every one below `COUNT_DENSE`, then
/// `COUNT_SPARSE` of them below `COUNT_MAX`.
const COUNT_SEED: u64 = SEED + 213;
const COUNT_DENSE: u64 = 200_000;
const COUNT_SPARSE: usize = 100_000;
const COUNT_MAX: u64 = 100_000_000;

/// Fixed seed for the unary and sum probes.
pub const PROBE_SEED: u64 = SEED;

pub const PROBE_UNARY_COUNT: usize = UNARY_COUNT;
pub const PROBE_BLOCK_SIZE: usize = BLOCK_SIZE;
pub const PROBE_SUM_LISTS: usize = SUM_LISTS;

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_u53(&mut self) -> u64 {
        self.next_u64() >> 11
    }
}

fn ldexp(mant: f64, exp: i32) -> f64 {
    mant * 2f64.powi(exp)
}

fn unary_input(i: usize, rng: &mut SplitMix64) -> f64 {
    let slot = i % 11;
    match slot {
        0 => ldexp(1.0, -1022),
        1 => f64::from_bits(0x8000_0000_0000_0001),
        2 => 700.0,
        3 => -709.0,
        4 => {
            let u = rng.next_u53() as f64 / (1u64 << 53) as f64;
            u.max(LOG_EPS)
        }
        5 => {
            let u = rng.next_u53() as f64 / (1u64 << 53) as f64;
            (1.0 - u).max(LOG_EPS)
        }
        6 => (rng.next_u64() % 708) as f64,
        7 => {
            let z = rng.next_u53() as f64 / (1u64 << 53) as f64 * 40.0;
            (-z).exp()
        }
        8 => {
            let z = -(rng.next_u53() as f64) / (1u64 << 53) as f64 * 40.0;
            z.exp()
        }
        9 => -(rng.next_u53() as f64 / (1u64 << 53) as f64),
        _ => {
            let u = rng.next_u53() as f64 / (1u64 << 53) as f64;
            u * 80.0 - 40.0
        }
    }
}

/// `log` on the probe input: CPython's domain rule as a quiet NaN bit pattern when `x <= 0`.
pub fn unary_log(x: f64) -> f64 {
    if x > 0.0 { x.ln() } else { f64::NAN }
}

/// `log1p` on the probe input: quiet NaN when `x <= -1`.
pub fn unary_log1p(x: f64) -> f64 {
    if x > -1.0 { x.ln_1p() } else { f64::NAN }
}

fn sum_list(_i: usize, rng: &mut SplitMix64) -> Vec<f64> {
    let n = 2 + (rng.next_u64() as usize % 199);
    (0..n)
        .map(|j| {
            let base = (rng.next_u64() % 2000) as f64 / 100.0 - 10.0;
            base * ((j % 3) + 1) as f64
        })
        .collect()
}

fn advance_unary_rng(i: usize, rng: &mut SplitMix64) {
    for n in 0..i {
        unary_input(n, rng);
    }
}

/// All unary inputs in order, using `PROBE_SEED`.
pub fn all_unary_inputs() -> Vec<f64> {
    let mut rng = SplitMix64::new(SEED);
    (0..UNARY_COUNT).map(|i| unary_input(i, &mut rng)).collect()
}

/// All sum lists in order after the unary stream.
pub fn all_sum_lists() -> Vec<Vec<f64>> {
    let mut rng = SplitMix64::new(SEED);
    advance_unary_rng(UNARY_COUNT, &mut rng);
    (0..SUM_LISTS).map(|i| sum_list(i, &mut rng)).collect()
}

/// The task lengths of build 213 as floats, in the order `record_math.py` hashes them.
pub fn all_count_inputs() -> Vec<f64> {
    let mut rng = SplitMix64::new(COUNT_SEED);
    let mut counts: Vec<f64> = (0..COUNT_DENSE).map(|n| n as f64).collect();
    counts.extend((0..COUNT_SPARSE).map(|_| (rng.next_u64() % COUNT_MAX) as f64));
    counts
}
