//! Parser tests: the §16.5 corner-case table from RUST_PORT_OUTLINE.md.
//! These pin the grammar to Python's expression semantics for the whitelisted
//! subset (validate_node in prefig/core/user_namespace.py).

use prefig_core::evaluator::ast::{BinOp::*, Expr, Expr::*, UnaryOp::*};
use prefig_core::evaluator::parse_expression;

fn p(src: &str) -> Expr {
    parse_expression(src).unwrap_or_else(|e| panic!("failed to parse {src:?}: {e}"))
}

fn num(v: f64) -> Expr {
    Num(v)
}
fn bin(op: prefig_core::evaluator::ast::BinOp, l: Expr, r: Expr) -> Expr {
    BinOp(op, Box::new(l), Box::new(r))
}
fn neg(e: Expr) -> Expr {
    UnaryOp(Neg, Box::new(e))
}
fn name(s: &str) -> Expr {
    Name(s.to_string())
}

#[test]
fn power_binds_tighter_than_unary_on_the_left() {
    // -2**2 == -(2**2)
    assert_eq!(p("-2**2"), neg(bin(Pow, num(2.0), num(2.0))));
}

#[test]
fn power_rhs_admits_unary() {
    // 2**-3 parses
    assert_eq!(p("2**-3"), bin(Pow, num(2.0), neg(num(3.0))));
}

#[test]
fn power_is_right_associative() {
    // 2**3**2 == 2**(3**2)
    assert_eq!(
        p("2**3**2"),
        bin(Pow, num(2.0), bin(Pow, num(3.0), num(2.0)))
    );
}

#[test]
fn unary_chains() {
    assert_eq!(p("--3"), neg(neg(num(3.0))));
    assert_eq!(p("+-3"), UnaryOp(Pos, Box::new(neg(num(3.0)))));
}

#[test]
fn add_mul_precedence_and_associativity() {
    // 1 - 2 - 3 == (1-2)-3
    assert_eq!(
        p("1 - 2 - 3"),
        bin(Sub, bin(Sub, num(1.0), num(2.0)), num(3.0))
    );
    // 1 + 2 * 3 == 1 + (2*3)
    assert_eq!(
        p("1 + 2 * 3"),
        bin(Add, num(1.0), bin(Mult, num(2.0), num(3.0)))
    );
}

#[test]
fn floor_div_and_mod() {
    assert_eq!(p("7//2"), bin(FloorDiv, num(7.0), num(2.0)));
    assert_eq!(p("7 % 3"), bin(Mod, num(7.0), num(3.0)));
}

#[test]
fn tuple_forms() {
    assert_eq!(p("(1,)"), Tuple(vec![num(1.0)]));
    assert_eq!(p("(1)"), num(1.0)); // grouping, not a tuple
    assert_eq!(p("()"), Tuple(vec![]));
    assert_eq!(p("1, 2"), Tuple(vec![num(1.0), num(2.0)])); // bare top-level tuple
    assert_eq!(p("(1, 2,)"), Tuple(vec![num(1.0), num(2.0)])); // trailing comma
}

#[test]
fn nested_displays() {
    assert_eq!(
        p("[ (1,2), (3,4) ]"),
        List(vec![
            Tuple(vec![num(1.0), num(2.0)]),
            Tuple(vec![num(3.0), num(4.0)])
        ])
    );
    assert_eq!(p("[1, 2,]"), List(vec![num(1.0), num(2.0)]));
    assert_eq!(p("[]"), List(vec![]));
}

#[test]
fn subscripts() {
    assert_eq!(
        p("a[i, j]"),
        Subscript(
            Box::new(name("a")),
            Box::new(Tuple(vec![name("i"), name("j")]))
        )
    );
    assert_eq!(
        p("a[-1]"),
        Subscript(Box::new(name("a")), Box::new(neg(num(1.0))))
    );
    assert_eq!(
        p("m[1][0]"),
        Subscript(
            Box::new(Subscript(Box::new(name("m")), Box::new(num(1.0)))),
            Box::new(num(0.0))
        )
    );
}

#[test]
fn starred_in_displays_and_calls() {
    assert_eq!(
        p("(*p, 1)"),
        Tuple(vec![Starred(Box::new(name("p"))), num(1.0)])
    );
    assert_eq!(
        p("f(*args)"),
        Call("f".to_string(), vec![Starred(Box::new(name("args")))])
    );
}

#[test]
fn dict_literals() {
    assert_eq!(p("{}"), Dict(vec![]));
    assert_eq!(
        p("{'a': 'x', 3: 'y', }"),
        Dict(vec![
            (Str("a".to_string()), Str("x".to_string())),
            (num(3.0), Str("y".to_string())),
        ])
    );
}

#[test]
fn number_literal_forms() {
    assert_eq!(p(".5"), num(0.5));
    assert_eq!(p("5."), num(5.0));
    assert_eq!(p("1.e5"), num(1.0e5));
    assert_eq!(p("1_000"), num(1000.0));
    assert_eq!(p("2.5E-3"), num(2.5e-3));
    assert_eq!(p("1e-6"), num(1.0e-6));
}

#[test]
fn string_escapes() {
    assert_eq!(p(r"'it\'s'"), Str("it's".to_string()));
    assert_eq!(p(r#""a\nb""#), Str("a\nb".to_string()));
    // Python keeps unknown escapes verbatim: '\q' == '\\q'
    assert_eq!(p(r"'a\qb'"), Str("a\\qb".to_string()));
}

#[test]
fn raw_strings() {
    // r'...' / R"..." keep backslashes literal (no escape processing). The
    // corpus writes LaTeX math labels this way, e.g. labels=[r'\omega^3', ...].
    assert_eq!(p(r"r'\omega'"), Str("\\omega".to_string()));
    assert_eq!(p(r#"R"\tau""#), Str("\\tau".to_string()));
    // The distinguishing case: a normal string turns \t into a tab; a raw
    // string keeps it as the two characters backslash-t.
    assert_eq!(p(r"'\t'"), Str("\t".to_string()));
    assert_eq!(p(r"r'\t'"), Str("\\t".to_string()));
    // A leading r/R that is not a string prefix is still an identifier.
    assert_eq!(p("radius"), name("radius"));
}

#[test]
fn keywords() {
    assert_eq!(p("True"), Bool(true));
    assert_eq!(p("False"), Bool(false));
    assert_eq!(p("None"), Expr::None);
    // ...but only as whole words
    assert_eq!(p("Truex"), name("Truex"));
}

#[test]
fn calls_compose_with_power() {
    assert_eq!(
        p("sin(pi/4)**2"),
        bin(
            Pow,
            Call("sin".to_string(), vec![bin(Div, name("pi"), num(4.0))]),
            num(2.0)
        )
    );
    assert_eq!(
        p("atan2(y, x)"),
        Call("atan2".to_string(), vec![name("y"), name("x")])
    );
}

#[test]
fn whitespace_tolerance() {
    assert_eq!(p("  ( 1 ,\n  2 )  "), Tuple(vec![num(1.0), num(2.0)]));
}

#[test]
fn rejections() {
    for src in [
        "x < 3",
        "x == 3",
        "a.b",
        "f(x=1)",
        "a[1:3]",
        "lambda x: x",
        "[i for i in (1,2)]",
        "1 | 2",
        "not x",
        "x and y",
        "3 +",
        "'unclosed",
        "f(x)(y)", // call of non-name: Python's validator reads node.func.id
        "(*p)",    // lone starred expression in parens
    ] {
        assert!(
            parse_expression(src).is_err(),
            "expected parse error for {src:?}"
        );
    }
}
