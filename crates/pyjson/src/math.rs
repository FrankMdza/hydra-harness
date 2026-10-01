//! `math.exp`, `math.log`, `math.log1p` and float `sum()` as CPython 3.14 uses them in the grader.

/// `math.exp`.
#[must_use]
pub fn exp(x: f64) -> f64 {
    x.exp()
}

/// `math.log` on positive `x`; callers that mirror the probe use `math_probe::unary_log`.
#[must_use]
pub fn log(x: f64) -> f64 {
    x.ln()
}

/// `math.log1p`.
#[must_use]
pub fn log1p(x: f64) -> f64 {
    x.ln_1p()
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
