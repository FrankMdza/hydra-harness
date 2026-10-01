//! `math.exp`, `math.log`, `math.log1p`, `math.sqrt`, `**` and float `sum()` as CPython 3.14 uses them in the grader.

/// `math.exp`.
#[must_use]
pub fn exp(x: f64) -> f64 {
    x.exp()
}

/// `math.log` on positive `x`; the probe applies CPython's domain rule before calling this.
#[must_use]
pub fn log(x: f64) -> f64 {
    x.ln()
}

/// `math.log1p`.
#[must_use]
pub fn log1p(x: f64) -> f64 {
    x.ln_1p()
}

/// `x ** y` on floats: the C library's `pow`. For `y` of 2 that is not `x * x`. On arm64 macOS
/// the two differ on about one input in a thousand (build 213 measured 2,683 of 2,000,000),
/// and the compiler rewrites a `powf` whose exponent it can see is 2 into the multiplication.
/// The exponent goes through `black_box` so it cannot see it, and
/// `tests/math_matches_python.rs` fails if it ever does.
#[must_use]
#[inline(never)]
pub fn pow(x: f64, y: f64) -> f64 {
    x.powf(std::hint::black_box(y))
}

/// `math.sqrt` on an `x` that is not negative.
#[must_use]
pub fn sqrt(x: f64) -> f64 {
    x.sqrt()
}

/// Float `sum()` since Python 3.12: Neumaier compensated addition, then one correction term.
#[must_use]
pub fn fsum_like_python(values: &[f64]) -> f64 {
    let (mut sum, mut lost) = (0.0f64, 0.0f64);
    for &term in values {
        let next = sum + term;
        lost += if sum.abs() >= term.abs() {
            (sum - next) + term
        } else {
            (term - next) + sum
        };
        sum = next;
    }
    if lost != 0.0 && lost.is_finite() {
        sum += lost;
    }
    sum
}
