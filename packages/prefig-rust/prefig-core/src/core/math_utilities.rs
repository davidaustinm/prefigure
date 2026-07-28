//! Numeric helpers shared by handlers (the f64 side of math_utilities.py;
//! the author-facing functions live in evaluator/builtins.rs).

/// np.linspace(a, b, n+1) — n+1 points with exact endpoints.
pub fn linspace(a: f64, b: f64, n: usize) -> Vec<f64> {
    if n == 0 {
        return vec![a];
    }
    let step = (b - a) / n as f64;
    (0..=n)
        .map(|i| if i == n { b } else { a + i as f64 * step })
        .collect()
}

/// np.logspace(a, b, n+1) — n+1 points from 10^a to 10^b.
pub fn logspace(a: f64, b: f64, n: usize) -> Vec<f64> {
    linspace(a, b, n)
        .into_iter()
        .map(|x| 10f64.powf(x))
        .collect()
}

pub fn length(v: [f64; 2]) -> f64 {
    (v[0] * v[0] + v[1] * v[1]).sqrt()
}

pub fn distance(p: [f64; 2], q: [f64; 2]) -> f64 {
    length([p[0] - q[0], p[1] - q[1]])
}

pub fn normalize(v: [f64; 2]) -> [f64; 2] {
    let len = length(v);
    [v[0] / len, v[1] / len]
}

pub fn midpoint(p: [f64; 2], q: [f64; 2]) -> [f64; 2] {
    [(p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0]
}

pub fn dot(u: [f64; 2], v: [f64; 2]) -> f64 {
    u[0] * v[0] + u[1] * v[1]
}

/// Python '{0:g}' formatting: 6 significant digits, trailing zeros stripped,
/// scientific notation when the exponent is < -4 or >= 6.
pub fn fmt_g(x: f64) -> String {
    if x == 0.0 {
        return "0".to_string();
    }
    if x.is_nan() {
        return "nan".to_string();
    }
    if x.is_infinite() {
        return if x > 0.0 { "inf" } else { "-inf" }.to_string();
    }
    let exp = x.abs().log10().floor() as i32;
    if !(-4..6).contains(&exp) {
        let mantissa = format!("{:.5e}", x);
        let (m, e) = mantissa.split_once('e').expect("exponent");
        let m = m.trim_end_matches('0').trim_end_matches('.');
        let e: i32 = e.parse().unwrap_or(0);
        format!("{}e{}{:02}", m, if e < 0 { '-' } else { '+' }, e.abs())
    } else {
        let decimals = (5 - exp).max(0) as usize;
        let s = format!("{x:.decimals$}");
        if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            s
        }
    }
}
