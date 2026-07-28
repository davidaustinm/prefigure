//! Built-in functions available in author expressions.
//!
//! Mirrors what Python's user_namespace assembles from `math` plus
//! prefig/core/math_utilities.py. Functions that need the diagram (intersect,
//! proj_2d, …) arrive later with the EvalEnv handle (outline §4.1).

use super::interp::call_value;
use super::{EvalError, ExpressionContext};
use crate::core::calculus;
use crate::value::Value;

fn err(m: impl Into<String>) -> EvalError {
    EvalError::new(m)
}

fn arity(name: &str, args: &[Value], n: usize) -> Result<(), EvalError> {
    if args.len() != n {
        return Err(err(format!("{name}() takes {n} arguments, got {}", args.len())));
    }
    Ok(())
}

fn num(name: &str, args: &[Value], i: usize) -> Result<f64, EvalError> {
    args.get(i)
        .ok_or_else(|| err(format!("{name}(): missing argument {i}")))?
        .as_num()
}

fn one_num(name: &str, args: &[Value], f: impl Fn(f64) -> f64) -> Result<Value, EvalError> {
    arity(name, args, 1)?;
    Ok(Value::Num(f(num(name, args, 0)?)))
}

fn two_num(name: &str, args: &[Value], f: impl Fn(f64, f64) -> f64) -> Result<Value, EvalError> {
    arity(name, args, 2)?;
    Ok(Value::Num(f(num(name, args, 0)?, num(name, args, 1)?)))
}

const NAMES: &[&str] = &[
    // from math
    "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "sinh", "cosh", "tanh",
    "asinh", "acosh", "atanh", "exp", "log", "log2", "log10", "sqrt", "floor",
    "ceil", "degrees", "radians", "factorial", "comb", "fabs", "hypot", "copysign",
    "trunc", "isclose", "gcd", "pow",
    // python builtins whitelisted in user_namespace
    "max", "min", "round", "abs",
    // math_utilities
    "ln", "sec", "csc", "cot", "dot", "distance", "length", "normalize",
    "midpoint", "angle", "roll", "choose", "append", "chi_oo", "chi_oc", "chi_co",
    "chi_cc", "rotate", "deriv", "grad", "zip_lists", "evaluate_bezier", "eulers_method",
    "filter", "proj_2d", "line_intersection", "intersect", "solve", "delta",
];

pub fn is_builtin(name: &str) -> bool {
    NAMES.contains(&name)
}

pub fn call(name: &str, args: &[Value], ctx: &mut ExpressionContext) -> Result<Value, EvalError> {
    match name {
        "sin" => one_num(name, args, f64::sin),
        "cos" => one_num(name, args, f64::cos),
        "tan" => one_num(name, args, f64::tan),
        // Python's math module raises on domain errors (unlike IEEE NaN);
        // graphing code relies on these being errors to find domain edges
        "asin" | "acos" => {
            arity(name, args, 1)?;
            let x = num(name, args, 0)?;
            if !(-1.0..=1.0).contains(&x) {
                return Err(err(format!("math domain error: {name}({x})")));
            }
            Ok(Value::Num(if name == "asin" { x.asin() } else { x.acos() }))
        }
        "atan" => one_num(name, args, f64::atan),
        "atan2" => two_num(name, args, f64::atan2),
        "sinh" => one_num(name, args, f64::sinh),
        "cosh" => one_num(name, args, f64::cosh),
        "tanh" => one_num(name, args, f64::tanh),
        "asinh" => one_num(name, args, f64::asinh),
        "acosh" => one_num(name, args, f64::acosh),
        "atanh" => one_num(name, args, f64::atanh),
        "exp" => one_num(name, args, f64::exp),
        "ln" => {
            arity(name, args, 1)?;
            checked_log(num(name, args, 0)?, f64::ln)
        }
        "log" => match args.len() {
            1 => checked_log(num(name, args, 0)?, f64::ln),
            2 => {
                let base = num(name, args, 1)?;
                checked_log(num(name, args, 0)?, move |x| x.log(base))
            }
            n => Err(err(format!("log() takes 1 or 2 arguments, got {n}"))),
        },
        "log2" => {
            arity(name, args, 1)?;
            checked_log(num(name, args, 0)?, f64::log2)
        }
        "log10" => {
            arity(name, args, 1)?;
            checked_log(num(name, args, 0)?, f64::log10)
        }
        "sqrt" => {
            arity(name, args, 1)?;
            let x = num(name, args, 0)?;
            if x < 0.0 {
                return Err(err(format!("math domain error: sqrt({x})")));
            }
            Ok(Value::Num(x.sqrt()))
        }
        "floor" => one_num(name, args, f64::floor),
        "ceil" => one_num(name, args, f64::ceil),
        "degrees" => one_num(name, args, f64::to_degrees),
        "radians" => one_num(name, args, f64::to_radians),
        "fabs" => one_num(name, args, f64::abs),
        "trunc" => one_num(name, args, f64::trunc),
        "hypot" => two_num(name, args, f64::hypot),
        "copysign" => two_num(name, args, f64::copysign),
        "pow" => two_num(name, args, f64::powf),
        "factorial" => one_num(name, args, |n| (2..=(n as u64)).product::<u64>() as f64),
        "comb" | "choose" => {
            arity(name, args, 2)?;
            let (n, k) = (num(name, args, 0)? as u64, num(name, args, 1)? as u64);
            Ok(Value::Num(binomial(n, k)))
        }
        "gcd" => two_num(name, args, |a, b| {
            let (mut a, mut b) = ((a as i64).unsigned_abs(), (b as i64).unsigned_abs());
            while b != 0 {
                (a, b) = (b, a % b);
            }
            a as f64
        }),
        "isclose" => {
            arity(name, args, 2)?;
            let (a, b) = (num(name, args, 0)?, num(name, args, 1)?);
            // math.isclose defaults: rel_tol=1e-9, abs_tol=0
            Ok(Value::Bool((a - b).abs() <= 1e-9 * a.abs().max(b.abs())))
        }
        "sec" => one_num(name, args, |x| 1.0 / x.cos()),
        "csc" => one_num(name, args, |x| 1.0 / x.sin()),
        "cot" => one_num(name, args, |x| 1.0 / x.tan()),

        "abs" => {
            arity(name, args, 1)?;
            abs_value(&args[0])
        }
        "max" | "min" => {
            if args.is_empty() {
                return Err(err(format!("{name}() needs at least one argument")));
            }
            // max((1,2,3)) over a single vector, or max(1, 2, 3) over scalars
            let nums: Vec<f64> = if args.len() == 1 {
                args[0].as_vec_f64()?
            } else {
                args.iter().map(|v| v.as_num()).collect::<Result<_, _>>()?
            };
            let init = nums[0];
            let folded = nums.into_iter().fold(init, |acc, x| {
                if name == "max" {
                    acc.max(x)
                } else {
                    acc.min(x)
                }
            });
            Ok(Value::Num(folded))
        }
        "round" => match args.len() {
            // Python round() is banker's rounding
            1 => one_num(name, args, f64::round_ties_even),
            2 => two_num(name, args, |x, nd| {
                if nd >= 0.0 {
                    // Python rounds the true decimal value (half-to-even); Rust's
                    // float formatter does exactly that, while multiply-by-10^n
                    // would introduce binary error (round(2.675, 2) must be 2.67)
                    format!("{x:.*}", nd as usize).parse().unwrap_or(x)
                } else {
                    let scale = 10f64.powi(nd as i32);
                    (x * scale).round_ties_even() / scale
                }
            }),
            n => Err(err(format!("round() takes 1 or 2 arguments, got {n}"))),
        },

        "dot" => {
            arity(name, args, 2)?;
            let (u, v) = (args[0].as_vec_f64()?, args[1].as_vec_f64()?);
            if u.len() != v.len() {
                return Err(err("dot(): vectors have different lengths"));
            }
            Ok(Value::Num(u.iter().zip(&v).map(|(a, b)| a * b).sum()))
        }
        "length" => {
            arity(name, args, 1)?;
            Ok(Value::Num(norm(&args[0].as_vec_f64()?)))
        }
        "distance" => {
            arity(name, args, 2)?;
            let (p, q) = (args[0].as_vec_f64()?, args[1].as_vec_f64()?);
            if p.len() != q.len() {
                return Err(err("distance(): points have different dimensions"));
            }
            let diff: Vec<f64> = p.iter().zip(&q).map(|(a, b)| a - b).collect();
            Ok(Value::Num(norm(&diff)))
        }
        "normalize" => {
            arity(name, args, 1)?;
            let u = args[0].as_vec_f64()?;
            let n = norm(&u);
            if n == 0.0 {
                return Err(err("normalize(): zero vector"));
            }
            Ok(nums_to_value(u.iter().map(|x| x / n)))
        }
        "midpoint" => {
            arity(name, args, 2)?;
            let (u, v) = (args[0].as_vec_f64()?, args[1].as_vec_f64()?);
            Ok(nums_to_value(u.iter().zip(&v).map(|(a, b)| 0.5 * (a + b))))
        }
        "angle" => {
            let p = args
                .first()
                .ok_or_else(|| err("angle() needs a point"))?
                .as_vec_f64()?;
            let radians = p[1].atan2(p[0]);
            let degrees_wanted = match args.get(1) {
                None => true,
                Some(Value::Str(s)) => s == "deg",
                Some(other) => return Err(err(format!("angle(): bad units {other:?}"))),
            };
            Ok(Value::Num(if degrees_wanted {
                radians.to_degrees()
            } else {
                radians
            }))
        }
        "rotate" => {
            arity(name, args, 2)?;
            let v = args[0].as_vec_f64()?;
            let theta = num(name, args, 1)?;
            let (c, s) = (theta.cos(), theta.sin());
            Ok(nums_to_value([c * v[0] - s * v[1], s * v[0] + c * v[1]].into_iter()))
        }
        "roll" => {
            arity(name, args, 1)?;
            match &args[0] {
                Value::Array(items) if !items.is_empty() => {
                    let mut rolled = items.clone();
                    rolled.rotate_right(1);
                    Ok(Value::Array(rolled))
                }
                other => Err(err(format!("roll(): expected an array, found {other:?}"))),
            }
        }
        "append" => {
            arity(name, args, 2)?;
            match &args[0] {
                Value::Array(items) => {
                    let mut out = items.clone();
                    out.push(args[1].clone());
                    Ok(Value::Array(out))
                }
                other => Err(err(format!("append(): expected an array, found {other:?}"))),
            }
        }
        "zip_lists" => {
            arity(name, args, 2)?;
            match (&args[0], &args[1]) {
                (Value::Array(a), Value::Array(b)) => Ok(Value::Array(
                    a.iter()
                        .zip(b)
                        .map(|(x, y)| Value::Array(vec![x.clone(), y.clone()]))
                        .collect(),
                )),
                _ => Err(err("zip_lists(): expected two arrays")),
            }
        }
        "chi_oo" | "chi_oc" | "chi_co" | "chi_cc" => {
            arity(name, args, 3)?;
            let (a, b, t) = (num(name, args, 0)?, num(name, args, 1)?, num(name, args, 2)?);
            let lower = if name.as_bytes()[4] == b'o' { t > a } else { t >= a };
            let upper = if name.as_bytes()[5] == b'o' { t < b } else { t <= b };
            Ok(Value::Num(if lower && upper { 1.0 } else { 0.0 }))
        }
        "evaluate_bezier" => {
            arity(name, args, 2)?;
            evaluate_bezier(&args[0], num(name, args, 1)?)
        }
        "deriv" => {
            arity(name, args, 2)?;
            let f = args[0].clone();
            let a = num(name, args, 1)?;
            value_derivative(&f, a, ctx)
        }
        "grad" => {
            arity(name, args, 2)?;
            let f = args[0].clone();
            let a = args[1].as_vec_f64()?;
            let mut grad = Vec::with_capacity(a.len());
            for j in 0..a.len() {
                let mut b: Vec<Value> = a.iter().map(|&x| Value::Num(x)).collect();
                let d = calculus::derivative(
                    |x| {
                        b[j] = Value::Num(x);
                        call_value(&f, &b, ctx)?.as_num()
                    },
                    a[j],
                    true,
                )?;
                grad.push(Value::Num(d));
            }
            Ok(Value::Array(grad))
        }
        "eulers_method" => {
            arity(name, args, 5)?;
            eulers_method(
                &args[0],
                num(name, args, 1)?,
                args[2].clone(),
                num(name, args, 3)?,
                num(name, args, 4)? as usize,
                ctx,
            )
        }
        // delta(t, a): Dirac delta for ODE forcing. In break-collection mode it
        // records the location a; when measuring a jump it fires at t≈a.
        "delta" => {
            arity(name, args, 2)?;
            let t = num(name, args, 0)?;
            let a = num(name, args, 1)?;
            if let Some(breaks) = ctx.delta_breaks.as_mut() {
                breaks.push(a);
                return Ok(Value::Num(0.0));
            }
            // np.isclose defaults: atol=1e-8, rtol=1e-5
            if ctx.delta_on && (t - a).abs() <= 1e-8 + 1e-5 * a.abs() {
                return Ok(Value::Num(1.0));
            }
            Ok(Value::Num(0.0))
        }
        // proj_2d(point): project a 3-D point to the screen via the current
        // 3-D transform (math_utilities.proj_2d).
        "proj_2d" => {
            let p = match args {
                [single] => {
                    let v = single.as_vec_f64()?;
                    if v.len() == 3 {
                        [v[0], v[1], v[2], 1.0]
                    } else if v.len() == 4 {
                        [v[0], v[1], v[2], v[3]]
                    } else {
                        return Err(err("proj_2d(): point must be 3- or 4-dimensional"));
                    }
                }
                [a, b, c] => [a.as_num()?, b.as_num()?, c.as_num()?, 1.0],
                [a, b, c, d] => [a.as_num()?, b.as_num()?, c.as_num()?, d.as_num()?],
                _ => return Err(err("proj_2d(): point must be 3- or 4-dimensional")),
            };
            let (ctm_3d, eye) = ctx
                .env_ctm3d
                .ok_or_else(|| err("proj_2d(): no diagram in context"))?;
            // permute to [x,y,z,1] and project (CTM::project_to_screen)
            let pp = [p[1], p[2], p[0], p[3]];
            let mut out = [0.0f64; 4];
            for (i, row) in ctm_3d.iter().enumerate() {
                out[i] = (0..4).map(|k| row[k] * pp[k]).sum();
            }
            Ok(Value::Array(vec![
                Value::Num(out[0] - eye[0] * out[2]),
                Value::Num(out[1] - eye[1] * out[2]),
            ]))
        }
        // line_intersection([[p1,p2],[q1,q2]]): where two lines meet.
        "line_intersection" => {
            arity(name, args, 1)?;
            line_intersection(&args[0], ctx)
        }
        // intersect(f, seed[, interval]): a zero of f (or intersection of two
        // graphs / two lines) near seed. solve(f, y, seed) = intersect(f-y).
        "intersect" => intersect(args, ctx),
        "solve" => {
            arity(name, args, 3)?;
            let f = args[0].clone();
            let y = num(name, args, 1)?;
            let seed = num(name, args, 2)?;
            // intersect(lambda x: f(x) - y, seed)
            intersect_fn(
                &mut |x, ctx| Ok(call_value(&f, &[Value::Num(x)], ctx)?.as_num()? - y),
                seed,
                None,
                ctx,
            )
        }
        // math_utilities.filter(df, a, b, value): boolean-mask a CSV column.
        // mask = df[b] == value; return df[a][mask].
        "filter" => {
            arity(name, args, 4)?;
            let Value::Dict(df) = &args[0] else {
                return Err(err("filter(): first argument must be a data table"));
            };
            let col_a = args[1].as_dict_key()?;
            let col_b = args[2].as_dict_key()?;
            let value = &args[3];
            let Some(Value::Array(mask_col)) = df.get(&col_b) else {
                return Err(err(format!("filter(): no column {col_b:?}")));
            };
            let Some(Value::Array(data_col)) = df.get(&col_a) else {
                return Err(err(format!("filter(): no column {col_a:?}")));
            };
            let selected: Vec<Value> = data_col
                .iter()
                .zip(mask_col)
                .filter(|(_, m)| values_equal(m, value))
                .map(|(d, _)| d.clone())
                .collect();
            Ok(Value::Array(selected))
        }
        _ => Err(err(format!("Unknown function in evaluation: {name}"))),
    }
}

/// math_utilities.line_intersection: intersection of segments p1p2 and q1q2,
/// falling back to the bbox center when the lines are parallel.
fn line_intersection(lines: &Value, ctx: &mut ExpressionContext) -> Result<Value, EvalError> {
    let Value::Array(pair) = lines else {
        return Err(err("line_intersection(): expected two lines"));
    };
    let (Value::Array(l0), Value::Array(l1)) = (&pair[0], &pair[1]) else {
        return Err(err("line_intersection(): expected two lines"));
    };
    let p1 = l0[0].as_vec_f64()?;
    let p2 = l0[1].as_vec_f64()?;
    let q1 = l1[0].as_vec_f64()?;
    let q2 = l1[1].as_vec_f64()?;

    let diff = [p2[0] - p1[0], p2[1] - p1[1]];
    let normal = [-diff[1], diff[0]];
    let v = [q2[0] - q1[0], q2[1] - q1[1]];
    let denom = normal[0] * v[0] + normal[1] * v[1];
    if denom.abs() < 1e-10 {
        let bbox = ctx.env_bbox.unwrap_or([0.0, 0.0, 0.0, 0.0]);
        return Ok(Value::Array(vec![
            Value::Num((bbox[0] + bbox[2]) / 2.0),
            Value::Num((bbox[1] + bbox[3]) / 2.0),
        ]));
    }
    let t = (normal[0] * (q1[0] - p1[0]) + normal[1] * (q1[1] - p1[1])) / denom;
    Ok(Value::Array(vec![
        Value::Num(q1[0] - t * v[0]),
        Value::Num(q1[1] - t * v[1]),
    ]))
}

/// math_utilities.intersect: dispatch the various call shapes to the
/// scan-and-bisect root finder.
fn intersect(args: &[Value], ctx: &mut ExpressionContext) -> Result<Value, EvalError> {
    let seed = args.get(1).and_then(|v| v.as_num().ok());
    let interval = args.get(2).and_then(|v| v.as_vec_f64().ok());
    let interval = interval.map(|v| [v[0], v[1]]);

    // two lines: intersect((p1,p2),(q1,q2))
    if let Value::Array(items) = &args[0] {
        if items.first().map(|i| i.rank() >= 2).unwrap_or(false) {
            return line_intersection(&args[0], ctx);
        }
        // (f, y_value) or (f, g)
        if items.len() == 2 {
            let seed = seed.ok_or_else(|| err("intersect(): needs a seed"))?;
            if let Ok(y) = items[1].as_num() {
                let f = items[0].clone();
                return intersect_fn(
                    &mut |x, ctx| Ok(call_value(&f, &[Value::Num(x)], ctx)?.as_num()? - y),
                    seed,
                    interval,
                    ctx,
                );
            }
            let f = items[0].clone();
            let g = items[1].clone();
            return intersect_fn(
                &mut |x, ctx| {
                    Ok(call_value(&f, &[Value::Num(x)], ctx)?.as_num()?
                        - call_value(&g, &[Value::Num(x)], ctx)?.as_num()?)
                },
                seed,
                interval,
                ctx,
            );
        }
    }
    // zero of a single function
    let seed = seed.ok_or_else(|| err("intersect(): needs a seed"))?;
    let f = args[0].clone();
    intersect_fn(
        &mut |x, ctx| call_value(&f, &[Value::Num(x)], ctx)?.as_num(),
        seed,
        interval,
        ctx,
    )
}

/// The scan-and-bisect root finder from math_utilities.intersect.
fn intersect_fn(
    f: &mut dyn FnMut(f64, &mut ExpressionContext) -> Result<f64, EvalError>,
    seed: f64,
    interval: Option<[f64; 2]>,
    ctx: &mut ExpressionContext,
) -> Result<Value, EvalError> {
    let bbox = ctx.env_bbox.unwrap_or([0.0, 0.0, 1.0, 1.0]);
    let width = bbox[2] - bbox[0];
    let height = bbox[3] - bbox[1];
    let tolerance = 1e-6 * height;
    let upper = bbox[3] + height;
    let lower = bbox[1] - height;
    let interval = interval.unwrap_or([bbox[0], bbox[2]]);

    let x0 = seed;
    let y0 = f(x0, ctx)?;
    if y0.abs() < tolerance {
        return Ok(Value::Num(x0));
    }

    let dx = 0.002 * width;
    let mut x = x0;
    let mut x_left = f64::NEG_INFINITY;
    while x >= interval[0] {
        x -= dx;
        let y = match f(x, ctx) {
            Ok(y) => y,
            Err(_) => break,
        };
        if y > upper || y < lower {
            break;
        }
        if y.abs() < tolerance {
            x_left = x;
            break;
        }
        if y * y0 < 0.0 {
            x_left = x;
            break;
        }
    }
    if x_left != f64::NEG_INFINITY
        && (f(x_left, ctx)? - f(x_left + dx, ctx)?).abs() > height
    {
        x_left = f64::NEG_INFINITY;
    }

    let mut x = x0;
    let mut x_right = f64::INFINITY;
    while x <= interval[1] {
        x += dx;
        let y = match f(x, ctx) {
            Ok(y) => y,
            Err(_) => break,
        };
        if y > upper || y < lower {
            break;
        }
        if y.abs() < tolerance {
            x_right = x;
            break;
        }
        if y * y0 < 0.0 {
            x_right = x;
            break;
        }
    }
    if x_right != f64::INFINITY && (f(x_right, ctx)? - f(x_right - dx, ctx)?).abs() > height {
        x_right = f64::INFINITY;
    }

    if x_left < interval[0] && x_right > interval[1] {
        return Ok(Value::Num(x0));
    }

    let (mut x1, mut x2);
    if x_left < interval[0] {
        x2 = x_right;
        x1 = x_right - dx;
    } else if x_right > interval[1] {
        x2 = x_left + dx;
        x1 = x_left;
    } else if (x0 - x_right).abs() < (x0 - x_left).abs() {
        x1 = x_right - dx;
        x2 = x_right;
    } else {
        x1 = x_left;
        x2 = x_left + dx;
    }

    for _ in 0..8 {
        let mid = (x1 + x2) / 2.0;
        if f(mid, ctx)? * f(x1, ctx)? < 0.0 {
            x2 = mid;
        } else {
            x1 = mid;
        }
    }
    Ok(Value::Num((x1 + x2) / 2.0))
}

/// Python `==` for the values that appear in CSV columns (numbers, strings).
fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Num(x), Value::Num(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        _ => false,
    }
}

/// Richardson-extrapolated derivative over Values, so vector-valued functions
/// (splines, curves) differentiate componentwise like numpy arrays do.
pub fn value_derivative(
    f: &Value,
    a: f64,
    ctx: &mut ExpressionContext,
) -> Result<Value, EvalError> {
    use crate::evaluator::ast::BinOp;
    let h = 0.1;
    let fa = call_value(f, &[Value::Num(a)], ctx)?;
    let mut estimates: Vec<Value> = Vec::with_capacity(4);
    for i in 0..4u32 {
        let delta = h / f64::from(2u32.pow(i));
        let f_ahead = call_value(f, &[Value::Num(a + delta)], ctx)?;
        let diff = crate::value::binop(BinOp::Sub, &f_ahead, &fa)?;
        estimates.push(crate::value::binop(BinOp::Div, &diff, &Value::Num(delta))?);
    }
    let mut j = 1i32;
    while estimates.len() > 1 {
        let mut next = Vec::with_capacity(estimates.len() - 1);
        for i in 0..estimates.len() - 1 {
            let delta = crate::value::binop(BinOp::Sub, &estimates[i + 1], &estimates[i])?;
            let correction = crate::value::binop(
                BinOp::Div,
                &delta,
                &Value::Num(2f64.powi(j) - 1.0),
            )?;
            next.push(crate::value::binop(
                BinOp::Add,
                &estimates[i + 1],
                &correction,
            )?);
        }
        estimates = next;
        j += 1;
    }
    Ok(estimates.remove(0))
}

fn checked_log(x: f64, f: impl Fn(f64) -> f64) -> Result<Value, EvalError> {
    if x <= 0.0 {
        return Err(err(format!("math domain error: log({x})")));
    }
    Ok(Value::Num(f(x)))
}

fn norm(v: &[f64]) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
}

fn nums_to_value(iter: impl Iterator<Item = f64>) -> Value {
    Value::Array(iter.map(Value::Num).collect())
}

fn binomial(n: u64, k: u64) -> f64 {
    if k > n {
        return 0.0;
    }
    let k = k.min(n - k);
    let mut result = 1f64;
    for i in 0..k {
        result = result * (n - i) as f64 / (i + 1) as f64;
    }
    result.round()
}

fn abs_value(v: &Value) -> Result<Value, EvalError> {
    // Python's abs() on an ndarray is elementwise
    match v {
        Value::Array(items) => {
            let items: Result<Vec<_>, _> = items.iter().map(abs_value).collect();
            Ok(Value::Array(items?))
        }
        _ => Ok(Value::Num(v.as_num()?.abs())),
    }
}

/// Port of math_utilities.evaluate_bezier (quadratic and cubic).
fn evaluate_bezier(controls: &Value, t: f64) -> Result<Value, EvalError> {
    let controls = match controls {
        Value::Array(items) => items,
        other => return Err(err(format!("evaluate_bezier(): bad controls {other:?}"))),
    };
    let n = controls.len();
    let coefficients: &[f64] = match n {
        3 => &[1.0, 2.0, 1.0],
        4 => &[1.0, 3.0, 3.0, 1.0],
        _ => return Err(err("evaluate_bezier(): need 3 or 4 control points")),
    };
    let dim = controls[0].as_vec_f64()?.len();
    let mut sum = vec![0.0; dim];
    for (j, control) in controls.iter().enumerate() {
        let point = control.as_vec_f64()?;
        let weight =
            coefficients[j] * (1.0 - t).powi((n - j - 1) as i32) * t.powi(j as i32);
        for (acc, c) in sum.iter_mut().zip(&point) {
            *acc += weight * c;
        }
    }
    Ok(nums_to_value(sum.into_iter()))
}

/// Port of math_utilities.eulers_method: rows are [t, *y].
fn eulers_method(
    f: &Value,
    t0: f64,
    y0: Value,
    t1: f64,
    n: usize,
    ctx: &mut ExpressionContext,
) -> Result<Value, EvalError> {
    use crate::evaluator::ast::BinOp;
    let h = (t1 - t0) / n as f64;
    let row = |t: f64, y: &Value| -> Value {
        let mut cells = vec![Value::Num(t)];
        match y {
            Value::Array(items) => cells.extend(items.iter().cloned()),
            other => cells.push(other.clone()),
        }
        Value::Array(cells)
    };

    let mut t = t0;
    let mut y = y0;
    let mut points = vec![row(t, &y)];
    for _ in 0..n {
        let dy = call_value(f, &[Value::Num(t), y.clone()], ctx)?;
        let step = crate::value::binop(BinOp::Mult, &dy, &Value::Num(h))?;
        y = crate::value::binop(BinOp::Add, &y, &step)?;
        t += h;
        points.push(row(t, &y));
    }
    Ok(Value::Array(points))
}
