//! Expression-evaluation tests checked against the reference Python implementation.
//!
//! The corpus is the shared `tests/expressions/expression_tests.json` at the
//! repository root (regenerate with tests/helpers/generate_expressions.py). Each session runs in a
//! fresh ExpressionContext (mirroring Python's importlib.reload of
//! user_namespace); steps run in order so definitions persist within a session.

use prefig_core::evaluator::ExpressionContext;
use prefig_core::value::Value;
use serde::Deserialize;

#[derive(Deserialize)]
struct TestFile {
    sessions: Vec<Session>,
}

#[derive(Deserialize)]
struct Session {
    name: String,
    steps: Vec<Step>,
}

#[derive(Deserialize)]
struct Step {
    op: String,
    input: String,
    #[serde(default)]
    expect: Option<serde_json::Value>,
    #[serde(default)]
    tol: Option<f64>,
}

fn num_matches(actual: f64, expected: &serde_json::Value, tol: Option<f64>) -> bool {
    let expected = match expected {
        serde_json::Value::String(s) if s == "inf" => f64::INFINITY,
        serde_json::Value::String(s) if s == "-inf" => f64::NEG_INFINITY,
        v => v.as_f64().expect("numeric expectation"),
    };
    if expected.is_infinite() {
        return actual == expected;
    }
    match tol {
        None => actual == expected,
        Some(t) => (actual - expected).abs() <= t + t * expected.abs(),
    }
}

fn value_matches(actual: &Value, expect: &serde_json::Value, tol: Option<f64>) -> bool {
    let t = expect["t"].as_str().expect("expectation has a type tag");
    let v = &expect["v"];
    match (t, actual) {
        ("num", Value::Num(n)) => num_matches(*n, v, tol),
        ("bool", Value::Bool(b)) => v.as_bool() == Some(*b),
        ("str", Value::Str(s)) => v.as_str() == Some(s.as_str()),
        ("array", Value::Array(items)) => {
            let exp_items = v.as_array().expect("array expectation");
            items.len() == exp_items.len()
                && items
                    .iter()
                    .zip(exp_items)
                    .all(|(a, e)| value_matches(a, e, tol))
        }
        ("dict", Value::Dict(map)) => {
            let exp = v.as_object().expect("dict expectation");
            map.len() == exp.len()
                && exp.iter().all(|(k, e)| {
                    map.get(k).map(|a| value_matches(a, e, tol)).unwrap_or(false)
                })
        }
        ("function", Value::Function(_)) => true,
        _ => false,
    }
}

#[test]
fn expressions_match_python_reference() {
    let json = include_str!("../../../tests/expressions/expression_tests.json");
    let tests: TestFile = serde_json::from_str(json).expect("valid test JSON");

    let mut failures = Vec::new();
    for session in &tests.sessions {
        let mut ctx = ExpressionContext::new();
        for step in &session.steps {
            let label = format!("[{}] {} {:?}", session.name, step.op, step.input);
            match step.op.as_str() {
                "define" => {
                    if let Err(e) = ctx.define(&step.input) {
                        failures.push(format!("{label}: define failed: {e}"));
                    }
                }
                "eval" => match ctx.valid_eval(&step.input) {
                    Ok(actual) => {
                        let expect = step.expect.as_ref().expect("eval step has expect");
                        if !value_matches(&actual, expect, step.tol) {
                            failures.push(format!(
                                "{label}: got {actual:?}, expected {expect}"
                            ));
                        }
                    }
                    Err(e) => failures.push(format!("{label}: eval failed: {e}")),
                },
                "error" => {
                    if let Ok(v) = ctx.valid_eval(&step.input) {
                        failures.push(format!("{label}: expected error, got {v:?}"));
                    }
                }
                other => panic!("unknown op {other:?}"),
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "{} failures:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }
}
