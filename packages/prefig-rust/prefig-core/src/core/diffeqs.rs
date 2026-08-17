//! Port of prefig/core/diffeqs.py: ODE solving and plotting.
//!
//! The solver reimplements scipy's solve_ivp RK45 (Dormand–Prince 5(4) with
//! adaptive steps and quartic dense output) with the same constants, default
//! tolerances (rtol=1e-3, atol=1e-6), and initial-step selection, so solutions
//! agree with the Python build to visual accuracy.

use crate::core::arrow;
use crate::core::diagram::Diagram;
use crate::core::math_utilities::linspace;
use crate::core::utilities::{self as util, pt2str};
use crate::evaluator::interp_call;
use crate::value::Value;
use crate::xml::{self, El};

const RTOL: f64 = 1e-3;
const ATOL: f64 = 1e-6;
const SAFETY: f64 = 0.9;
const MIN_FACTOR: f64 = 0.2;
const MAX_FACTOR: f64 = 10.0;
const ERROR_EXPONENT: f64 = -0.2; // -1 / (order + 1) with order 4

const C: [f64; 6] = [0.0, 1.0 / 5.0, 3.0 / 10.0, 4.0 / 5.0, 8.0 / 9.0, 1.0];
const A: [[f64; 5]; 6] = [
    [0.0, 0.0, 0.0, 0.0, 0.0],
    [1.0 / 5.0, 0.0, 0.0, 0.0, 0.0],
    [3.0 / 40.0, 9.0 / 40.0, 0.0, 0.0, 0.0],
    [44.0 / 45.0, -56.0 / 15.0, 32.0 / 9.0, 0.0, 0.0],
    [
        19372.0 / 6561.0,
        -25360.0 / 2187.0,
        64448.0 / 6561.0,
        -212.0 / 729.0,
        0.0,
    ],
    [
        9017.0 / 3168.0,
        -355.0 / 33.0,
        46732.0 / 5247.0,
        49.0 / 176.0,
        -5103.0 / 18656.0,
    ],
];
const B: [f64; 6] = [
    35.0 / 384.0,
    0.0,
    500.0 / 1113.0,
    125.0 / 192.0,
    -2187.0 / 6784.0,
    11.0 / 84.0,
];
const E: [f64; 7] = [
    71.0 / 57600.0,
    0.0,
    -71.0 / 16695.0,
    71.0 / 1920.0,
    -17253.0 / 339200.0,
    22.0 / 525.0,
    -1.0 / 40.0,
];
// dense-output polynomial coefficients (scipy rk.py P matrix)
const P: [[f64; 4]; 7] = [
    [
        1.0,
        -8048581381.0 / 2820520608.0,
        8663915743.0 / 2820520608.0,
        -12715105075.0 / 11282082432.0,
    ],
    [0.0, 0.0, 0.0, 0.0],
    [
        0.0,
        131558114200.0 / 32700410799.0,
        -68118460800.0 / 10900136933.0,
        87487479700.0 / 32700410799.0,
    ],
    [
        0.0,
        -1754552775.0 / 470086768.0,
        14199869525.0 / 1410260304.0,
        -10690763975.0 / 1880347072.0,
    ],
    [
        0.0,
        127303824393.0 / 49829197408.0,
        -318862633887.0 / 49829197408.0,
        701980252875.0 / 199316789632.0,
    ],
    [
        0.0,
        -282668133.0 / 205662961.0,
        2019193451.0 / 616988883.0,
        -1453857185.0 / 822651844.0,
    ],
    [
        0.0,
        40617522.0 / 29380423.0,
        -110615467.0 / 29380423.0,
        69997945.0 / 29380423.0,
    ],
];

type OdeFn<'a> = dyn FnMut(f64, &[f64]) -> Result<Vec<f64>, String> + 'a;

fn rms_norm(v: &[f64]) -> f64 {
    (v.iter().map(|x| x * x).sum::<f64>() / v.len() as f64).sqrt()
}

/// scipy's _select_initial_step for order-5(4) methods.
fn select_initial_step(
    f: &mut OdeFn,
    t0: f64,
    y0: &[f64],
    f0: &[f64],
    t_bound: f64,
) -> Result<f64, String> {
    let scale: Vec<f64> = y0.iter().map(|y| ATOL + y.abs() * RTOL).collect();
    let d0 = rms_norm(
        &y0.iter()
            .zip(&scale)
            .map(|(y, s)| y / s)
            .collect::<Vec<_>>(),
    );
    let d1 = rms_norm(
        &f0.iter()
            .zip(&scale)
            .map(|(f, s)| f / s)
            .collect::<Vec<_>>(),
    );
    let h0 = if d0 < 1e-5 || d1 < 1e-5 {
        1e-6
    } else {
        0.01 * d0 / d1
    };
    let y1: Vec<f64> = y0.iter().zip(f0).map(|(y, f)| y + h0 * f).collect();
    let f1 = f(t0 + h0, &y1)?;
    let d2 = rms_norm(
        &f1.iter()
            .zip(f0)
            .zip(&scale)
            .map(|((f1, f0), s)| (f1 - f0) / s)
            .collect::<Vec<_>>(),
    ) / h0;
    let h1 = if d1 <= 1e-15 && d2 <= 1e-15 {
        (h0 * 1e-3).max(1e-6)
    } else {
        (0.01 / d1.max(d2)).powf(0.2)
    };
    Ok(h1.min(100.0 * h0).min((t_bound - t0).abs()))
}

/// One accepted step of RK45; returns (t_new, y_new, f_new, stage matrix K).
#[allow(clippy::type_complexity)]
fn rk_step(
    f: &mut OdeFn,
    t: f64,
    y: &[f64],
    f_cur: &[f64],
    h: f64,
) -> Result<(Vec<f64>, Vec<f64>, Vec<Vec<f64>>), String> {
    let n = y.len();
    let mut k: Vec<Vec<f64>> = vec![f_cur.to_vec()];
    for s in 1..6 {
        let mut y_stage = y.to_vec();
        for (j, k_j) in k.iter().enumerate() {
            let a = A[s][j];
            if a != 0.0 {
                for i in 0..n {
                    y_stage[i] += h * a * k_j[i];
                }
            }
        }
        k.push(f(t + C[s] * h, &y_stage)?);
    }
    let mut y_new = y.to_vec();
    for (j, k_j) in k.iter().enumerate() {
        for i in 0..n {
            y_new[i] += h * B[j] * k_j[i];
        }
    }
    let f_new = f(t + h, &y_new)?;
    k.push(f_new.clone());
    Ok((y_new, f_new, k))
}

fn error_norm(k: &[Vec<f64>], h: f64, y: &[f64], y_new: &[f64]) -> f64 {
    let n = y.len();
    let mut err = vec![0.0; n];
    for (j, k_j) in k.iter().enumerate() {
        for i in 0..n {
            err[i] += E[j] * k_j[i];
        }
    }
    let scaled: Vec<f64> = (0..n)
        .map(|i| {
            let scale = ATOL + y[i].abs().max(y_new[i].abs()) * RTOL;
            err[i] * h / scale
        })
        .collect();
    rms_norm(&scaled)
}

/// solve_ivp(f, (t0, t1), y0, t_eval=..., method='RK45'): returns the values
/// at the requested times via quartic dense output.
pub fn solve_ivp_rk45(
    f: &mut OdeFn,
    t0: f64,
    t1: f64,
    y0: &[f64],
    t_eval: &[f64],
    max_step: Option<f64>,
) -> Result<Vec<Vec<f64>>, String> {
    let n = y0.len();
    let mut t = t0;
    let mut y = y0.to_vec();
    let mut f_cur = f(t, &y)?;
    let max_step = max_step.unwrap_or(f64::INFINITY);
    let mut h = select_initial_step(f, t0, &y, &f_cur, t1)?.min(max_step);

    let mut outputs: Vec<Vec<f64>> = Vec::with_capacity(t_eval.len());
    let mut eval_index = 0;

    // emit any t_eval points at or before t0
    while eval_index < t_eval.len() && t_eval[eval_index] <= t {
        outputs.push(y.clone());
        eval_index += 1;
    }

    let mut steps = 0;
    while t < t1 && eval_index < t_eval.len() {
        steps += 1;
        if steps > 100_000 {
            return Err("ODE solver failed to converge".to_string());
        }
        h = h.min(max_step).min(t1 - t);
        let min_step = 10.0 * (t.abs().max(1.0)) * f64::EPSILON;
        if h < min_step {
            h = min_step;
        }

        let (y_new, f_new, k) = rk_step(f, t, &y, &f_cur, h)?;
        let err = error_norm(&k, h, &y, &y_new);
        if err < 1.0 {
            let t_new = t + h;
            // dense output: Q = K^T P; y(t + x h) = y + h Q [x, x², x³, x⁴]
            while eval_index < t_eval.len() && t_eval[eval_index] <= t_new {
                let x = (t_eval[eval_index] - t) / h;
                let powers = [x, x * x, x * x * x, x * x * x * x];
                let mut y_out = y.clone();
                for (j, k_j) in k.iter().enumerate() {
                    let q: f64 = (0..4).map(|c| P[j][c] * powers[c]).sum();
                    for i in 0..n {
                        y_out[i] += h * q * k_j[i];
                    }
                }
                outputs.push(y_out);
                eval_index += 1;
            }
            t = t_new;
            y = y_new;
            f_cur = f_new;
            let factor = if err == 0.0 {
                MAX_FACTOR
            } else {
                (SAFETY * err.powf(ERROR_EXPONENT)).min(MAX_FACTOR)
            };
            h *= factor;
        } else {
            h *= (SAFETY * err.powf(ERROR_EXPONENT)).max(MIN_FACTOR);
        }
    }
    // any leftover points (numerical edge at t1)
    while eval_index < t_eval.len() {
        outputs.push(y.clone());
        eval_index += 1;
    }
    Ok(outputs)
}

pub fn de_solve(element: &El, diagram: &mut Diagram, _parent: &El, _outline_group: Option<&El>) {
    let function_attr = element.borrow().get("function").unwrap_or_default();
    let Ok(f) = diagram.ctx.valid_eval(&function_attr) else {
        log::error!("Error in ODE solver: cannot retrieve function={function_attr}");
        return;
    };

    let eval_num = |diagram: &mut Diagram, attr: &str| -> Option<f64> {
        diagram.ctx.valid_eval(attr).ok()?.as_num().ok()
    };
    let t0_attr = element.borrow().get("t0").unwrap_or_default();
    let Some(t0) = eval_num(diagram, &t0_attr) else {
        log::error!("Error in ODE solver: cannot retrieve t0={t0_attr}");
        return;
    };
    let y0_attr = element.borrow().get("y0").unwrap_or_default();
    let Ok(y0_value) = diagram.ctx.valid_eval(&y0_attr) else {
        log::error!("Error in ODE solver: cannot retrieve y0={y0_attr}");
        return;
    };
    let y0: Vec<f64> = match &y0_value {
        Value::Array(_) => y0_value.as_vec_f64().unwrap_or_default(),
        v => v.as_num().map(|n| vec![n]).unwrap_or_default(),
    };

    let bbox = diagram.bbox();
    let t1_attr = element
        .borrow()
        .get_or("t1", &crate::value::py_str(bbox[2]));
    let t1 = eval_num(diagram, &t1_attr).unwrap_or(bbox[2]);
    let n_attr = element.borrow().get_or("N", "100");
    let n = eval_num(diagram, &n_attr).unwrap_or(100.0) as usize;
    let max_step_attr = element.borrow().get("max-step");
    let max_step = max_step_attr.and_then(|a| eval_num(diagram, &a));

    // Python forwards @method to scipy's solve_ivp (RK23, DOP853, LSODA, ...);
    // only RK45 is implemented here. Warn rather than silently substituting.
    let method = element.borrow().get_or("method", "RK45");
    if method != "RK45" {
        log::warn!(
            "<de-solve> method=\"{method}\" is not implemented in the Rust port; using RK45"
        );
    }

    let ctx = &mut diagram.ctx;

    // If f contains delta functions, find where they occur and integrate
    // piecewise, adding the jump at each break (diffeqs.py).
    let mut breaks: Vec<f64> = find_breaks(&f, t0, &y0, ctx)
        .into_iter()
        .filter(|&b| b >= t0 && b < t1)
        .collect();
    breaks.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    breaks.push(t1);

    let mut cur_t0 = t0;
    let mut cur_y0 = y0.clone();
    let mut solution_t: Vec<f64> = Vec::new();
    let mut solution: Vec<Vec<f64>> = Vec::new();

    if !breaks.is_empty() && (cur_t0 - breaks[0]).abs() <= 1e-8 {
        let jump = measure_de_jump(&f, cur_t0, &cur_y0, ctx);
        for (yi, j) in cur_y0.iter_mut().zip(&jump) {
            *yi += j;
        }
        breaks.remove(0);
    }

    while !breaks.is_empty() {
        let next_t = breaks.remove(0);
        // Python: np.linspace(t0, next_t, N) gives N points; helper gives m+1
        let t_eval = linspace(cur_t0, next_t, n.saturating_sub(1));
        let f_ref = &f;
        let mut ode = |t: f64, y: &[f64]| eval_ode(f_ref, t, y, ctx).map_err(|e| e.to_string());
        let segment = match solve_ivp_rk45(&mut ode, cur_t0, next_t, &cur_y0, &t_eval, max_step) {
            Ok(s) => s,
            Err(e) => {
                log::error!("Error in ODE solver: {e}");
                return;
            }
        };
        cur_t0 = next_t;
        cur_y0 = segment.last().cloned().unwrap_or(cur_y0);
        let jump = measure_de_jump(&f, cur_t0, &cur_y0, ctx);
        for (yi, j) in cur_y0.iter_mut().zip(&jump) {
            *yi += j;
        }
        solution_t.extend(t_eval);
        solution.extend(segment);
    }

    let Some(name) = element.borrow().get("name") else {
        log::error!("Error in ODE solver setting name");
        return;
    };

    // rows: [t values, y0 values, y1 values, ...] like np.stack((t, *y))
    let mut rows: Vec<Value> = Vec::with_capacity(1 + y0.len());
    rows.push(Value::Array(
        solution_t.iter().map(|&t| Value::Num(t)).collect(),
    ));
    for dim in 0..y0.len() {
        rows.push(Value::Array(
            solution.iter().map(|y| Value::Num(y[dim])).collect(),
        ));
    }
    diagram.ctx.enter_namespace(&name, Value::Array(rows));
}

/// Evaluate the author's f(t, y), returning the derivative vector. The state is
/// passed as an array even for a single equation (like Python).
fn eval_ode(
    f: &Value,
    t: f64,
    y: &[f64],
    ctx: &mut crate::evaluator::ExpressionContext,
) -> Result<Vec<f64>, crate::evaluator::EvalError> {
    let y_value = Value::Array(y.iter().map(|&v| Value::Num(v)).collect());
    let result = interp_call(f, &[Value::Num(t), y_value], ctx)?;
    match &result {
        Value::Array(_) => result.as_vec_f64(),
        v => v.as_num().map(|n| vec![n]),
    }
}

/// user_namespace.find_breaks: collect the locations of delta() calls in f.
fn find_breaks(
    f: &Value,
    t0: f64,
    y0: &[f64],
    ctx: &mut crate::evaluator::ExpressionContext,
) -> Vec<f64> {
    ctx.delta_breaks = Some(Vec::new());
    let _ = eval_ode(f, t0, y0, ctx);
    ctx.delta_breaks.take().unwrap_or_default()
}

/// user_namespace.measure_de_jump: the jump f(t,y)|delta_on - f(t,y)|delta_off.
fn measure_de_jump(
    f: &Value,
    t: f64,
    y: &[f64],
    ctx: &mut crate::evaluator::ExpressionContext,
) -> Vec<f64> {
    ctx.delta_on = true;
    let f1 = eval_ode(f, t, y, ctx).unwrap_or_default();
    ctx.delta_on = false;
    let f0 = eval_ode(f, t, y, ctx).unwrap_or_default();
    f1.iter().zip(&f0).map(|(a, b)| a - b).collect()
}

pub fn plot_de_solution(
    element: &El,
    diagram: &mut Diagram,
    parent: &El,
    outline_group: Option<&El>,
) {
    let solution = if element.borrow().get("function").is_some() {
        element.borrow_mut().set("name", "__de_solution");
        de_solve(element, diagram, parent, None);
        match diagram.ctx.valid_eval("__de_solution") {
            Ok(s) => s,
            Err(_) => return,
        }
    } else {
        let solution_attr = element.borrow().get("solution").unwrap_or_default();
        match diagram.ctx.valid_eval(&solution_attr) {
            Ok(s) => s,
            Err(_) => {
                log::error!("Error in <plot-de-solution> finding solution={solution_attr}");
                return;
            }
        }
    };
    let Value::Array(rows) = &solution else {
        return;
    };
    let rows: Vec<Vec<f64>> = rows.iter().filter_map(|r| r.as_vec_f64().ok()).collect();

    // which quantities go on the axes: default (t, y)
    let axes_attr = element.borrow().get_or("axes", "(t,y)");
    let trimmed = axes_attr.trim();
    let inner = &trimmed[1..trimmed.len() - 1];
    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
    if parts.len() != 2 {
        log::error!("Error in <plot-de-solution> setting axes={axes_attr}");
        return;
    }
    let index_of = |axis: &str, default: usize| -> usize {
        if let Some(num) = axis.strip_prefix('y') {
            if let Ok(i) = num.parse::<usize>() {
                return i + 1;
            }
        }
        default
    };
    let axis0 = &rows[if parts[0].starts_with('y') {
        index_of(parts[0], 0)
    } else {
        0
    }];
    let axis1 = &rows[if parts[1] == "y" {
        1
    } else {
        index_of(parts[1], 1)
    }];

    let mut cmds: Vec<String> = Vec::new();
    for (i, (&x, &y)) in axis0.iter().zip(axis1).enumerate() {
        let p = diagram.transform([x, y]);
        let cmd = if i == 0 { "M" } else { "L" };
        cmds.push(format!("{cmd} {}", pt2str(p, " ")));
    }

    if diagram.output_format() == "tactile" {
        element.borrow_mut().set("stroke", "black");
    } else {
        util::set_attr(element, "stroke", "blue", &mut diagram.ctx);
        util::set_attr(element, "fill", "none", &mut diagram.ctx);
    }
    util::set_attr(element, "thickness", "2", &mut diagram.ctx);

    let path = xml::new_element("path");
    let id = element.borrow().get("id");
    diagram.add_id(&path, id.as_deref());
    diagram.register_svg_element(element, &path);
    let attrs = util::get_2d_attr(element, diagram);
    util::add_attr(&path, attrs);

    if element.borrow().get_or("arrow", "no") == "yes" {
        let arrow_width = element.borrow().get("arrow-width");
        let arrow_angles = element.borrow().get("arrow-angles");
        arrow::add_arrowhead_to_path(
            diagram,
            "marker-end",
            &path,
            arrow_width.as_deref(),
            arrow_angles.as_deref(),
        );

        // optionally an arrow in the middle of the trajectory
        let location_attr = element.borrow().get("arrow-location");
        if let Some(attr) = location_attr {
            if let Some(arrow_location) = diagram
                .ctx
                .valid_eval(&attr)
                .ok()
                .and_then(|v| v.as_num().ok())
            {
                let t_vals = &rows[0];
                if arrow_location > t_vals[0] && arrow_location < t_vals[t_vals.len() - 1] {
                    let mut index = t_vals.len() - 1;
                    for (i, &t) in t_vals.iter().enumerate() {
                        if arrow_location > t {
                            continue;
                        }
                        index = i;
                        break;
                    }
                    let start = index.saturating_sub(5);
                    let p = diagram.transform([axis0[start], axis1[start]]);
                    cmds.push(format!("M {}", pt2str(p, " ")));
                    for i in start..=index {
                        let p = diagram.transform([axis0[i], axis1[i]]);
                        cmds.push(format!("L {}", pt2str(p, " ")));
                    }
                }
            }
        }
    }
    path.borrow_mut().set("d", &cmds.join(" "));

    let clip = element.borrow().get_or("cliptobbox", "yes");
    element.borrow_mut().set("cliptobbox", &clip);
    util::cliptobbox(&path, element, diagram);

    if let Some(outline_group) = outline_group {
        diagram.add_outline(element, &path, outline_group, None, None);
        finish_outline(element, diagram, parent);
    } else if element.borrow().get_or("outline", "no") == "yes"
        || diagram.output_format() == "tactile"
    {
        diagram.add_outline(element, &path, parent, None, None);
        finish_outline(element, diagram, parent);
    } else {
        xml::append(parent, &path);
    }
}

fn finish_outline(element: &El, diagram: &mut Diagram, parent: &El) {
    let stroke = element.borrow().get("stroke");
    let thickness = element.borrow().get("thickness");
    let fill = element.borrow().get_or("fill", "none");
    diagram.finish_outline(element, stroke, thickness, &fill, parent, None);
}
