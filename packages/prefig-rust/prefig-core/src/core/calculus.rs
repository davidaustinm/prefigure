//! Port of prefig/core/calculus.py: Richardson-extrapolated numerical derivative.

/// Mirror of calculus.derivative: right-sided by default with h = 0.1 and
/// four levels of Richardson extrapolation.
pub fn derivative<E>(
    mut f: impl FnMut(f64) -> Result<f64, E>,
    a: f64,
    right: bool,
) -> Result<f64, E> {
    let h = if right { 0.1 } else { -0.1 };
    richardson(&mut f, a, h, 4)
}

fn richardson<E>(
    f: &mut impl FnMut(f64) -> Result<f64, E>,
    a: f64,
    h: f64,
    k: u32,
) -> Result<f64, E> {
    let fa = f(a)?;
    let mut estimates = Vec::with_capacity(k as usize);
    for i in 0..k {
        let delta = h / f64::from(2u32.pow(i));
        estimates.push((f(a + delta)? - fa) / delta);
    }

    let mut j = 1u32;
    while estimates.len() > 1 {
        let mut next = Vec::with_capacity(estimates.len() - 1);
        for i in 0..estimates.len() - 1 {
            let (e0, e1) = (estimates[i], estimates[i + 1]);
            next.push(e1 + (e1 - e0) / (2f64.powi(j as i32) - 1.0));
        }
        estimates = next;
        j += 1;
    }
    Ok(estimates[0])
}
