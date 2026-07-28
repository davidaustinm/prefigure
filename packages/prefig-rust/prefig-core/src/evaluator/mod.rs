//! Port of prefig/core/user_namespace.py: safe evaluation of author expressions.
//!
//! Python stores author definitions in module globals reset via
//! importlib.reload(); here each Diagram owns an ExpressionContext instance
//! (deliberate divergence, RUST_PORT_OUTLINE.md §4.1).

pub mod ast;
mod builtins;
mod interp;
mod parse;

pub use interp::call_value as interp_call;
pub use parse::{parse_expression, ParseError};

use crate::value::{Function, Value};
use ast::Expr;
use indexmap::IndexMap;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct EvalError {
    message: String,
}

impl EvalError {
    pub fn new(message: impl Into<String>) -> Self {
        EvalError {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for EvalError {}

impl From<ParseError> for EvalError {
    fn from(e: ParseError) -> Self {
        EvalError::new(e.to_string())
    }
}

pub struct ExpressionContext {
    pub(crate) vars: IndexMap<String, Value>,
    /// The current bounding box, kept in sync by the Diagram so that
    /// intersect(), line_intersection(), ... can see it.
    pub env_bbox: Option<[f64; 4]>,
    /// The current 3-D transform and eye point, for proj_2d().
    pub env_ctm3d: Option<(crate::core::ctm::Mat4, [f64; 2])>,
    /// While collecting ODE break points, delta(t, a) records `a` here and
    /// returns 0 (user_namespace.find_breaks).
    pub delta_breaks: Option<Vec<f64>>,
    /// While measuring an ODE jump, delta(t, a) returns 1 at t≈a
    /// (user_namespace.measure_de_jump).
    pub delta_on: bool,
}

impl Default for ExpressionContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ExpressionContext {
    pub fn new() -> Self {
        let mut vars = IndexMap::new();
        vars.insert("e".to_string(), Value::Num(std::f64::consts::E));
        vars.insert("pi".to_string(), Value::Num(std::f64::consts::PI));
        vars.insert("inf".to_string(), Value::Num(f64::INFINITY));
        ExpressionContext {
            vars,
            env_bbox: None,
            env_ctm3d: None,
            delta_breaks: None,
            delta_on: false,
        }
    }

    /// Port of user_namespace.derivative: register `name` as the numerical
    /// derivative of function `f`.
    pub fn register_derivative(&mut self, name: &str, f: Value) {
        let closure = move |args: &[Value], ctx: &mut ExpressionContext| {
            let x = args
                .first()
                .ok_or_else(|| EvalError::new("missing argument"))?
                .as_num()?;
            builtins::value_derivative(&f, x, ctx)
        };
        self.vars.insert(
            name.to_string(),
            Value::Function(Rc::new(Function::Closure(Box::new(closure)))),
        );
    }

    /// Port of user_namespace.valid_eval(s).
    pub fn valid_eval(&mut self, s: &str) -> Result<Value, EvalError> {
        self.valid_eval_named(s, None, true)
    }

    /// Port of user_namespace.valid_eval(s, name, substitution).
    pub fn valid_eval_named(
        &mut self,
        s: &str,
        name: Option<&str>,
        substitution: bool,
    ) -> Result<Value, EvalError> {
        // authors write ^ for exponentiation
        let s = if substitution {
            s.replace('^', "**")
        } else {
            s.to_string()
        };
        let stripped = s.trim();
        if stripped.is_empty() {
            return Err(EvalError::new(
                "Evaluating an empty object. Perhaps a required attribute is missing",
            ));
        }

        // color literals pass through unevaluated
        if stripped.starts_with('#') {
            return Ok(Value::Str(s));
        }
        if let Some(colors) = stripped.strip_prefix("rgb") {
            let components = self.eval_str(colors)?.as_vec_f64().map_err(|_| {
                EvalError::new(format!("Unsafe evaluation in rgb: {s}"))
            })?;
            if components.len() != 3 {
                return Err(EvalError::new(format!("Unsafe evaluation in rgb: {s}")));
            }
            // int() truncates toward zero
            let (r, g, b) = (
                components[0] as i64,
                components[1] as i64,
                components[2] as i64,
            );
            return Ok(Value::Str(format!("rgb({r},{g},{b})")));
        }

        // "f(x, y) = expr" defines a function
        if s.contains('=') {
            return self.define_function(&s);
        }

        let value = self.eval_str(&s)?;
        if let Some(name) = name {
            self.vars.insert(name.to_string(), value.clone());
        }
        Ok(value)
    }

    /// Port of user_namespace.define: "a = 5" or "f(x) = x**2".
    pub fn define(&mut self, expression: &str) -> Result<(), EvalError> {
        self.define_with_substitution(expression, true)
    }

    pub fn define_with_substitution(
        &mut self,
        expression: &str,
        substitution: bool,
    ) -> Result<(), EvalError> {
        let parts: Vec<&str> = expression.split('=').collect();
        let [left, right] = parts.as_slice() else {
            return Err(EvalError::new(format!(
                "Unrecognized definition: {expression}"
            )));
        };
        let (left, right) = (left.trim(), right.trim());
        if left.find('(').is_some_and(|i| i > 0) {
            self.valid_eval_named(expression, None, substitution)?;
        } else {
            self.valid_eval_named(right, Some(left), substitution)?;
        }
        Ok(())
    }

    /// Register a value under a name (user_namespace.enter_namespace).
    pub fn enter_namespace(&mut self, name: &str, value: Value) {
        self.vars.insert(name.to_string(), value);
    }

    /// Look up a name (user_namespace.retrieve).
    pub fn retrieve(&self, name: &str) -> Option<&Value> {
        self.vars.get(name)
    }

    fn eval_str(&mut self, s: &str) -> Result<Value, EvalError> {
        let expr = parse_expression(s)?;
        interp::eval_expr(&expr, self, &[])
    }

    fn define_function(&mut self, s: &str) -> Result<Value, EvalError> {
        let parts: Vec<&str> = s.split('=').collect();
        let [lhs, body_src] = parts.as_slice() else {
            return Err(EvalError::new(format!("Unsafe function definition: {s}")));
        };
        let (lhs, body_src) = (lhs.trim(), body_src.trim());
        let (Some(open), Some(close)) = (lhs.find('('), lhs.find(')')) else {
            return Err(EvalError::new(format!("Unsafe function definition: {s}")));
        };
        let fname = lhs[..open].trim().to_string();
        let args_src = lhs[open + 1..close].trim();
        let params: Vec<String> = if args_src.is_empty() {
            vec![]
        } else {
            args_src.split(',').map(|p| p.trim().to_string()).collect()
        };

        let body = parse_expression(body_src)?;
        // Python validates the body against known names at definition time
        self.validate_names(&body, &params)?;

        let func = Value::Function(Rc::new(Function::User { params, body }));
        self.vars.insert(fname, func.clone());
        Ok(func)
    }

    /// Mirror of validate_node's name checks: every Name must be a known
    /// variable or a bound parameter; every callee a known function.
    fn validate_names(&self, expr: &Expr, params: &[String]) -> Result<(), EvalError> {
        match expr {
            Expr::Name(id) => {
                if params.iter().any(|p| p == id) || self.vars.contains_key(id) {
                    Ok(())
                } else {
                    Err(EvalError::new(format!("Unrecognized name: {id}")))
                }
            }
            Expr::Call(name, args) => {
                if !self.vars.contains_key(name) && !builtins::is_builtin(name) {
                    return Err(EvalError::new(format!(
                        "Unknown function in evaluation: {name}"
                    )));
                }
                args.iter().try_for_each(|a| self.validate_names(a, params))
            }
            Expr::List(items) | Expr::Tuple(items) => {
                items.iter().try_for_each(|i| self.validate_names(i, params))
            }
            Expr::Dict(pairs) => pairs.iter().try_for_each(|(k, v)| {
                self.validate_names(k, params)?;
                self.validate_names(v, params)
            }),
            Expr::BinOp(_, l, r) => {
                self.validate_names(l, params)?;
                self.validate_names(r, params)
            }
            Expr::UnaryOp(_, e) | Expr::Starred(e) => self.validate_names(e, params),
            Expr::Subscript(t, i) => {
                self.validate_names(t, params)?;
                self.validate_names(i, params)
            }
            Expr::Num(_) | Expr::Str(_) | Expr::Bool(_) | Expr::None => Ok(()),
        }
    }
}
