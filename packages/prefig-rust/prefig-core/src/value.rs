//! Dynamically-typed value with numpy-like semantics (RUST_PORT_OUTLINE.md §6.2).
//!
//! Author expressions in Python evaluate over numpy arrays; this type reproduces
//! the behaviors PreFigure actually uses: elementwise arithmetic with
//! trailing-dimension broadcasting, negative and "fancy" (index-array) indexing,
//! and ragged nested arrays (Python's inhomogeneous-array fallback).

use crate::evaluator::ast::{BinOp, Expr, UnaryOp};
use crate::evaluator::EvalError;
use indexmap::IndexMap;
use std::rc::Rc;

#[derive(Clone)]
pub enum Value {
    Num(f64),
    Bool(bool),
    Str(String),
    Array(Vec<Value>),
    Dict(IndexMap<String, Value>),
    Function(Rc<Function>),
}

pub enum Function {
    /// Author-defined `f(x) = …`: the body re-resolves names at call time,
    /// exactly like the Python lambda over globals().
    User { params: Vec<String>, body: Expr },
    /// A built-in registered by name (see evaluator/builtins.rs).
    Native(&'static str),
    /// A function registered from Rust code (derivatives, tangent lines,
    /// splines, ODE solutions, ...).
    Closure(ClosureFn),
}

pub type ClosureFn =
    Box<dyn Fn(&[Value], &mut crate::evaluator::ExpressionContext) -> Result<Value, EvalError>>;

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Num(n) => write!(f, "{n:?}"),
            Value::Bool(b) => write!(f, "{b:?}"),
            Value::Str(s) => write!(f, "{s:?}"),
            Value::Array(items) => f.debug_list().entries(items).finish(),
            Value::Dict(map) => f.debug_map().entries(map.iter()).finish(),
            Value::Function(func) => match func.as_ref() {
                Function::User { params, .. } => write!(f, "<function({})>", params.join(", ")),
                Function::Native(name) => write!(f, "<builtin {name}>"),
                Function::Closure(_) => write!(f, "<function>"),
            },
        }
    }
}

fn err(message: impl Into<String>) -> EvalError {
    EvalError::new(message)
}

impl Value {
    pub fn as_num(&self) -> Result<f64, EvalError> {
        match self {
            Value::Num(n) => Ok(*n),
            // Python bools are ints; True + 1 == 2
            Value::Bool(b) => Ok(*b as u8 as f64),
            other => Err(err(format!("expected a number, found {other:?}"))),
        }
    }

    pub fn as_index(&self, len: usize) -> Result<usize, EvalError> {
        let n = self.as_num()?;
        if n.fract() != 0.0 {
            return Err(err(format!("index {n} is not an integer")));
        }
        let i = n as i64;
        let wrapped = if i < 0 { i + len as i64 } else { i };
        if wrapped < 0 || wrapped >= len as i64 {
            return Err(err(format!("index {i} out of range for length {len}")));
        }
        Ok(wrapped as usize)
    }

    pub fn as_vec_f64(&self) -> Result<Vec<f64>, EvalError> {
        match self {
            Value::Array(items) => items.iter().map(|v| v.as_num()).collect(),
            other => Err(err(format!("expected a vector, found {other:?}"))),
        }
    }

    /// Nesting depth: 0 for scalars, 1 for vectors, 2 for lists of points, …
    /// (numpy's ndim; ragged arrays use the maximum over items).
    pub fn rank(&self) -> usize {
        match self {
            Value::Array(items) => 1 + items.iter().map(Value::rank).max().unwrap_or(0),
            _ => 0,
        }
    }

    /// Python's str() of a dict key (int keys print without a decimal point).
    pub fn as_dict_key(&self) -> Result<String, EvalError> {
        match self {
            Value::Str(s) => Ok(s.clone()),
            Value::Num(n) if n.fract() == 0.0 && n.is_finite() => Ok(format!("{}", *n as i64)),
            Value::Num(n) => Ok(format!("{n}")),
            Value::Bool(b) => Ok(if *b { "True" } else { "False" }.to_string()),
            other => Err(err(format!("invalid dict key: {other:?}"))),
        }
    }
}

/// Python str() of a number: integers print without a decimal point, floats
/// print with shortest round-trip digits, switching to scientific notation
/// when the exponent is < -4 or >= 16 (e.g. "1e-07", "1e+20").
pub fn py_str(x: f64) -> String {
    if x.is_nan() {
        return "nan".to_string();
    }
    if x.is_infinite() {
        return if x > 0.0 { "inf" } else { "-inf" }.to_string();
    }
    if x == 0.0 {
        return "0".to_string();
    }
    if x.fract() == 0.0 && x.abs() < 1e16 {
        return format!("{}", x as i64);
    }
    // {:e} gives shortest round-trip digits as d[.ddd]e[-]E
    let sci = format!("{x:e}");
    let (mantissa, exp) = sci.split_once('e').expect("exponent in {:e} output");
    let exp: i32 = exp.parse().expect("numeric exponent");
    let negative = mantissa.starts_with('-');
    let digits: String = mantissa.chars().filter(|c| c.is_ascii_digit()).collect();
    let sign = if negative { "-" } else { "" };

    let body = if (-4..16).contains(&exp) {
        if exp >= 0 {
            let exp = exp as usize;
            if digits.len() > exp + 1 {
                format!("{}.{}", &digits[..=exp], &digits[exp + 1..])
            } else {
                format!("{}{}.0", digits, "0".repeat(exp + 1 - digits.len()))
            }
        } else {
            format!("0.{}{}", "0".repeat((-exp - 1) as usize), digits)
        }
    } else {
        let mantissa = if digits.len() > 1 {
            format!("{}.{}", &digits[..1], &digits[1..])
        } else {
            digits
        };
        format!(
            "{}e{}{:02}",
            mantissa,
            if exp < 0 { '-' } else { '+' },
            exp.abs()
        )
    };
    format!("{sign}{body}")
}

impl Value {
    /// Python str() of this value, as when it lands in an SVG attribute.
    pub fn to_py_str(&self) -> String {
        match self {
            Value::Num(n) => py_str(*n),
            Value::Bool(b) => if *b { "True" } else { "False" }.to_string(),
            Value::Str(s) => s.clone(),
            Value::Array(items) => {
                // numpy legacy-1.25 printing: [1 2 3] for ints, [1. 2.5] for floats
                let parts: Vec<String> = items.iter().map(|v| v.to_py_str()).collect();
                format!("[{}]", parts.join(" "))
            }
            Value::Dict(_) | Value::Function(_) => format!("{self:?}"),
        }
    }
}

fn scalar_binop(op: BinOp, a: f64, b: f64) -> Result<f64, EvalError> {
    match op {
        BinOp::Add => Ok(a + b),
        BinOp::Sub => Ok(a - b),
        BinOp::Mult => Ok(a * b),
        // IEEE semantics on zero divisors (inf/nan), matching the numpy
        // floats that author functions evaluate over in Python
        BinOp::Div => Ok(a / b),
        BinOp::FloorDiv => Ok((a / b).floor()),
        // Python/numpy semantics: result has the sign of the divisor
        BinOp::Mod => Ok(a - b * (a / b).floor()),
        BinOp::Pow => Ok(a.powf(b)),
        BinOp::MatMul => Err(err(
            "matmul: operands must be arrays (numpy rejects scalars)".to_string(),
        )),
    }
}

/// numpy `@`: 1-D @ 1-D is a dot product, 2-D @ 1-D maps rows, 1-D @ 2-D dots
/// with columns, and 2-D @ 2-D is the matrix product.
fn matmul(a: &Value, b: &Value) -> Result<Value, EvalError> {
    fn dot(x: &[Value], y: &[Value]) -> Result<Value, EvalError> {
        if x.len() != y.len() {
            return Err(err(format!(
                "matmul: mismatched lengths {} and {}",
                x.len(),
                y.len()
            )));
        }
        let mut sum = Value::Num(0.0);
        for (xi, yi) in x.iter().zip(y) {
            sum = binop(BinOp::Add, &sum, &binop(BinOp::Mult, xi, yi)?)?;
        }
        Ok(sum)
    }
    fn column(m: &[Value], j: usize) -> Result<Vec<Value>, EvalError> {
        m.iter()
            .map(|row| match row {
                Value::Array(r) if j < r.len() => Ok(r[j].clone()),
                _ => Err(err("matmul: ragged matrix".to_string())),
            })
            .collect()
    }
    match (a, b) {
        (Value::Array(xs), Value::Array(ys)) => match (a.rank(), b.rank()) {
            (1, 1) => dot(xs, ys),
            (2, 1) => {
                let rows: Result<Vec<_>, _> = xs
                    .iter()
                    .map(|row| match row {
                        Value::Array(r) => dot(r, ys),
                        _ => Err(err("matmul: ragged matrix".to_string())),
                    })
                    .collect();
                Ok(Value::Array(rows?))
            }
            (1, 2) => {
                let ncols = match &ys[0] {
                    Value::Array(r) => r.len(),
                    _ => 0,
                };
                let cols: Result<Vec<_>, _> =
                    (0..ncols).map(|j| dot(xs, &column(ys, j)?)).collect();
                Ok(Value::Array(cols?))
            }
            (2, 2) => {
                let rows: Result<Vec<_>, _> = xs
                    .iter()
                    .map(|row| {
                        let Value::Array(r) = row else {
                            return Err(err("matmul: ragged matrix".to_string()));
                        };
                        matmul(&Value::Array(r.clone()), b)
                    })
                    .collect();
                Ok(Value::Array(rows?))
            }
            (ra, rb) => Err(err(format!("matmul: unsupported ranks {ra} and {rb}"))),
        },
        _ => Err(err(
            "matmul: operands must be arrays (numpy rejects scalars)".to_string(),
        )),
    }
}

/// Elementwise arithmetic with numpy-style trailing-dimension broadcasting:
/// the higher-rank operand maps the lower-rank one over its items, so
/// `[[1,2],[3,4]] + [10,20]` adds `[10,20]` to each row.
pub fn binop(op: BinOp, a: &Value, b: &Value) -> Result<Value, EvalError> {
    if op == BinOp::MatMul {
        return matmul(a, b);
    }
    match (a, b) {
        (Value::Array(xs), Value::Array(ys)) => {
            let (ra, rb) = (a.rank(), b.rank());
            if ra > rb {
                let items: Result<Vec<_>, _> = xs.iter().map(|x| binop(op, x, b)).collect();
                Ok(Value::Array(items?))
            } else if rb > ra {
                let items: Result<Vec<_>, _> = ys.iter().map(|y| binop(op, a, y)).collect();
                Ok(Value::Array(items?))
            } else if xs.len() == ys.len() {
                let items: Result<Vec<_>, _> =
                    xs.iter().zip(ys).map(|(x, y)| binop(op, x, y)).collect();
                Ok(Value::Array(items?))
            } else {
                Err(err(format!(
                    "operands have mismatched lengths {} and {}",
                    xs.len(),
                    ys.len()
                )))
            }
        }
        (Value::Array(xs), _) => {
            let items: Result<Vec<_>, _> = xs.iter().map(|x| binop(op, x, b)).collect();
            Ok(Value::Array(items?))
        }
        (_, Value::Array(ys)) => {
            let items: Result<Vec<_>, _> = ys.iter().map(|y| binop(op, a, y)).collect();
            Ok(Value::Array(items?))
        }
        _ => Ok(Value::Num(scalar_binop(op, a.as_num()?, b.as_num()?)?)),
    }
}

pub fn unop(op: UnaryOp, v: &Value) -> Result<Value, EvalError> {
    match v {
        Value::Array(items) => {
            let items: Result<Vec<_>, _> = items.iter().map(|x| unop(op, x)).collect();
            Ok(Value::Array(items?))
        }
        _ => {
            let n = v.as_num()?;
            Ok(Value::Num(match op {
                UnaryOp::Neg => -n,
                UnaryOp::Pos => n,
            }))
        }
    }
}

/// Indexing. A scalar index selects one element (negative wraps, as in Python).
/// An array index does numpy "fancy" indexing — it selects per element — which
/// is what Python-PreFigure's `m[i, j]` actually does: the AST rewrite wraps the
/// index tuple in np.array, so `m[1, 0]` yields rows 1 and 0, not element [1][0].
pub fn subscript(target: &Value, index: &Value) -> Result<Value, EvalError> {
    match target {
        Value::Array(items) => match index {
            Value::Array(idxs) => {
                let selected: Result<Vec<_>, _> =
                    idxs.iter().map(|i| subscript(target, i)).collect();
                Ok(Value::Array(selected?))
            }
            _ => Ok(items[index.as_index(items.len())?].clone()),
        },
        Value::Dict(map) => {
            let key = index.as_dict_key()?;
            map.get(&key)
                .cloned()
                .ok_or_else(|| err(format!("key {key:?} not found")))
        }
        other => Err(err(format!("{other:?} is not subscriptable"))),
    }
}
