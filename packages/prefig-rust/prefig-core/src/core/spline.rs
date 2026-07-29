//! Cubic spline interpolation matching scipy.interpolate.CubicSpline for the
//! boundary conditions PreFigure uses: not-a-knot, natural, and periodic.
//! The interpolant with a given boundary condition is unique, so any correct
//! construction agrees with scipy to floating-point accuracy.

/// Second-derivative (M) representation of a cubic spline through
/// (t[i], y[i]) for one component.
pub struct CubicSpline1D {
    t: Vec<f64>,
    y: Vec<f64>,
    m: Vec<f64>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BcType {
    NotAKnot,
    Natural,
    Periodic,
}

impl BcType {
    pub fn from_name(name: &str) -> BcType {
        match name {
            "natural" => BcType::Natural,
            "periodic" => BcType::Periodic,
            _ => BcType::NotAKnot,
        }
    }
}

/// Solve a dense linear system by Gaussian elimination with partial pivoting.
/// Knot counts are small, so dense is fine.
fn solve_dense(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Vec<f64> {
    let n = b.len();
    for col in 0..n {
        let pivot = (col..n)
            .max_by(|&i, &j| {
                a[i][col]
                    .abs()
                    .partial_cmp(&a[j][col].abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(col);
        a.swap(col, pivot);
        b.swap(col, pivot);
        let diag = a[col][col];
        if diag.abs() < 1e-300 {
            continue;
        }
        for row in 0..n {
            if row == col {
                continue;
            }
            let factor = a[row][col] / diag;
            if factor == 0.0 {
                continue;
            }
            for k in col..n {
                a[row][k] -= factor * a[col][k];
            }
            b[row] -= factor * b[col];
        }
    }
    (0..n).map(|i| b[i] / a[i][i]).collect()
}

impl CubicSpline1D {
    pub fn new(t: &[f64], y: &[f64], bc: BcType) -> CubicSpline1D {
        let n = t.len() - 1; // number of intervals
        let h: Vec<f64> = (0..n).map(|i| t[i + 1] - t[i]).collect();
        let m = match bc {
            BcType::Periodic => {
                // unknowns M[0..n-1] with M[n] = M[0]
                let mut a = vec![vec![0.0; n]; n];
                let mut b = vec![0.0; n];
                for i in 0..n {
                    let h_prev = if i == 0 { h[n - 1] } else { h[i - 1] };
                    let y_prev = if i == 0 { y[n - 1] } else { y[i - 1] };
                    a[i][(i + n - 1) % n] += h_prev / 6.0;
                    a[i][i] += (h_prev + h[i]) / 3.0;
                    a[i][(i + 1) % n] += h[i] / 6.0;
                    b[i] = (y[i + 1] - y[i]) / h[i] - (y[i] - y_prev) / h_prev;
                }
                let mut m = solve_dense(a, b);
                m.push(m[0]);
                m
            }
            _ => {
                let size = n + 1;
                let mut a = vec![vec![0.0; size]; size];
                let mut b = vec![0.0; size];
                for i in 1..n {
                    a[i][i - 1] = h[i - 1] / 6.0;
                    a[i][i] = (h[i - 1] + h[i]) / 3.0;
                    a[i][i + 1] = h[i] / 6.0;
                    b[i] = (y[i + 1] - y[i]) / h[i] - (y[i] - y[i - 1]) / h[i - 1];
                }
                match bc {
                    BcType::Natural => {
                        a[0][0] = 1.0;
                        a[n][n] = 1.0;
                    }
                    _ => {
                        // not-a-knot: third-derivative continuity at t1, t[n-1]
                        a[0][0] = 1.0 / h[0];
                        a[0][1] = -(1.0 / h[0] + 1.0 / h[1]);
                        a[0][2] = 1.0 / h[1];
                        a[n][n - 2] = 1.0 / h[n - 2];
                        a[n][n - 1] = -(1.0 / h[n - 2] + 1.0 / h[n - 1]);
                        a[n][n] = 1.0 / h[n - 1];
                    }
                }
                solve_dense(a, b)
            }
        };
        CubicSpline1D {
            t: t.to_vec(),
            y: y.to_vec(),
            m,
        }
    }

    pub fn eval(&self, x: f64) -> f64 {
        let n = self.t.len() - 1;
        // find the interval (extrapolate with the end polynomials)
        let mut i = match self
            .t
            .binary_search_by(|probe| probe.partial_cmp(&x).unwrap_or(std::cmp::Ordering::Less))
        {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };
        if i >= n {
            i = n - 1;
        }
        let h = self.t[i + 1] - self.t[i];
        let a = self.t[i + 1] - x;
        let b = x - self.t[i];
        self.m[i] * a * a * a / (6.0 * h)
            + self.m[i + 1] * b * b * b / (6.0 * h)
            + (self.y[i] / h - self.m[i] * h / 6.0) * a
            + (self.y[i + 1] / h - self.m[i + 1] * h / 6.0) * b
    }
}

/// A spline through points that may be scalars or vectors (one 1-D spline per
/// component, like scipy on a 2-D array of values).
pub struct CubicSpline {
    components: Vec<CubicSpline1D>,
}

impl CubicSpline {
    pub fn new(t: &[f64], points: &[Vec<f64>], bc: BcType) -> CubicSpline {
        let dims = points.first().map(|p| p.len()).unwrap_or(0);
        let components = (0..dims)
            .map(|d| {
                let y: Vec<f64> = points.iter().map(|p| p[d]).collect();
                CubicSpline1D::new(t, &y, bc)
            })
            .collect();
        CubicSpline { components }
    }

    pub fn eval(&self, x: f64) -> Vec<f64> {
        self.components.iter().map(|c| c.eval(x)).collect()
    }
}
