//! Tree-walking interpreter over Value (RUST_PORT_OUTLINE.md §7).
//!
//! List/tuple displays evaluate to Value::Array — this is Python's
//! TransformList AST rewrite (lists/tuples become np.array) folded into
//! evaluation. Name resolution at call time mirrors Python's lambdas over
//! globals(): a function body sees definitions made after the function was.

use super::builtins;
use super::ast::Expr;
use super::{EvalError, ExpressionContext};
use crate::value::{self, Function, Value};

pub fn eval_expr(
    expr: &Expr,
    ctx: &mut ExpressionContext,
    locals: &[(String, Value)],
) -> Result<Value, EvalError> {
    match expr {
        Expr::Num(n) => Ok(Value::Num(*n)),
        Expr::Str(s) => Ok(Value::Str(s.clone())),
        Expr::Bool(b) => Ok(Value::Bool(*b)),
        Expr::None => Err(EvalError::new("None is not a usable value")),
        Expr::Name(id) => lookup(id, ctx, locals),
        Expr::List(items) | Expr::Tuple(items) => {
            Ok(Value::Array(eval_items(items, ctx, locals)?))
        }
        Expr::Dict(pairs) => {
            let mut map = indexmap::IndexMap::new();
            for (k, v) in pairs {
                let key = eval_expr(k, ctx, locals)?.as_dict_key()?;
                let val = eval_expr(v, ctx, locals)?;
                map.insert(key, val);
            }
            Ok(Value::Dict(map))
        }
        Expr::BinOp(op, l, r) => {
            let l = eval_expr(l, ctx, locals)?;
            let r = eval_expr(r, ctx, locals)?;
            value::binop(*op, &l, &r)
        }
        Expr::UnaryOp(op, e) => {
            let v = eval_expr(e, ctx, locals)?;
            value::unop(*op, &v)
        }
        Expr::Subscript(target, index) => {
            let target = eval_expr(target, ctx, locals)?;
            let index = eval_expr(index, ctx, locals)?;
            value::subscript(&target, &index)
        }
        Expr::Call(name, args) => {
            let args = eval_items(args, ctx, locals)?;
            // user definitions shadow built-ins, as in Python
            if let Some(v) = ctx.vars.get(name).cloned() {
                call_value(&v, &args, ctx)
            } else if builtins::is_builtin(name) {
                builtins::call(name, &args, ctx)
            } else {
                Err(EvalError::new(format!("Unknown function in evaluation: {name}")))
            }
        }
        Expr::Starred(_) => Err(EvalError::new(
            "cannot use a starred expression here",
        )),
    }
}

/// Evaluate display/argument items, splicing `*expr` (Starred) elements.
fn eval_items(
    items: &[Expr],
    ctx: &mut ExpressionContext,
    locals: &[(String, Value)],
) -> Result<Vec<Value>, EvalError> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        match item {
            Expr::Starred(inner) => match eval_expr(inner, ctx, locals)? {
                Value::Array(vs) => out.extend(vs),
                other => {
                    return Err(EvalError::new(format!(
                        "cannot unpack non-sequence {other:?}"
                    )))
                }
            },
            _ => out.push(eval_expr(item, ctx, locals)?),
        }
    }
    Ok(out)
}

fn lookup(
    id: &str,
    ctx: &ExpressionContext,
    locals: &[(String, Value)],
) -> Result<Value, EvalError> {
    if let Some((_, v)) = locals.iter().rev().find(|(name, _)| name == id) {
        return Ok(v.clone());
    }
    if let Some(v) = ctx.vars.get(id) {
        return Ok(v.clone());
    }
    Err(EvalError::new(format!("Unrecognized name: {id}")))
}

/// Call a function value (user-defined or native) with evaluated arguments.
pub fn call_value(
    func: &Value,
    args: &[Value],
    ctx: &mut ExpressionContext,
) -> Result<Value, EvalError> {
    match func {
        Value::Function(f) => match f.as_ref() {
            Function::User { params, body } => {
                if params.len() != args.len() {
                    return Err(EvalError::new(format!(
                        "function takes {} arguments but {} were given",
                        params.len(),
                        args.len()
                    )));
                }
                let locals: Vec<(String, Value)> = params
                    .iter()
                    .cloned()
                    .zip(args.iter().cloned())
                    .collect();
                eval_expr(body, ctx, &locals)
            }
            Function::Native(name) => builtins::call(name, args, ctx),
            Function::Closure(f) => f(args, ctx),
        },
        other => Err(EvalError::new(format!("{other:?} is not callable"))),
    }
}
