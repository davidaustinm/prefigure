//! PEG parser for the whitelisted expression language.
//!
//! Grammar per RUST_PORT_OUTLINE.md §16.3 — Python's reference-grammar shape,
//! including the u_expr/power mutual recursion that encodes `**`'s corner cases
//! (-2**2 == -(2**2), 2**-3 parses, right associativity).
//!
//! By the time text reaches this parser, `valid_eval` has already substituted
//! `^` → `**` and short-circuited color literals; we only ever see a single
//! expression.

use super::ast::{BinOp, Expr, UnaryOp};

#[derive(Debug, Clone)]
pub struct ParseError {
    pub offset: usize,
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "parse error at offset {}: {}", self.offset, self.message)
    }
}

impl std::error::Error for ParseError {}

pub fn parse_expression(src: &str) -> Result<Expr, ParseError> {
    pyexpr::expression(src).map_err(|e| ParseError {
        offset: e.location.offset,
        message: format!("expected {}", e.expected),
    })
}

enum Suffix {
    Call(Vec<Expr>),
    Subscript(Expr),
}

fn apply_suffixes(atom: Expr, suffixes: Vec<Suffix>) -> Result<Expr, &'static str> {
    let mut acc = atom;
    for suffix in suffixes {
        acc = match suffix {
            Suffix::Call(args) => match acc {
                Expr::Name(name) => Expr::Call(name, args),
                _ => return Err("only named functions can be called"),
            },
            Suffix::Subscript(index) => Expr::Subscript(Box::new(acc), Box::new(index)),
        };
    }
    Ok(acc)
}

fn fold_binops(first: Expr, rest: Vec<(BinOp, Expr)>) -> Expr {
    rest.into_iter().fold(first, |l, (op, r)| {
        Expr::BinOp(op, Box::new(l), Box::new(r))
    })
}

/// Collapse a comma-separated sequence: `(items, saw_trailing_comma)` into
/// either a Tuple or, for a single un-trailed item, the item itself.
/// A lone Starred without a comma is an error, as in Python.
fn seq_to_expr(mut items: Vec<Expr>, trailing: bool) -> Result<Expr, &'static str> {
    if items.len() > 1 || trailing {
        Ok(Expr::Tuple(items))
    } else {
        let only = items.pop().expect("item_seq is non-empty");
        if matches!(only, Expr::Starred(_)) {
            Err("cannot use a starred expression here")
        } else {
            Ok(only)
        }
    }
}

peg::parser! {
    grammar pyexpr() for str {
        rule _() = quiet!{ [' ' | '\t' | '\r' | '\n']* }

        pub rule expression() -> Expr
            = _ e:tuple_top() _ ![_] { e }

        // bare top-level tuple: Python eval("1, 2") yields a tuple
        rule tuple_top() -> Expr
            = s:item_seq() {? seq_to_expr(s.0, s.1) }

        // comma-separated items; returns (items, saw_trailing_comma)
        rule item_seq() -> (Vec<Expr>, bool)
            = first:item() rest:(_ "," _ i:item() { i })* t:(_ ",")?
              { (std::iter::once(first).chain(rest).collect(), t.is_some()) }

        rule item() -> Expr
            = "*" _ e:add() { Expr::Starred(Box::new(e)) }
            / add()

        // ---- precedence ladder (Python reference-grammar shape) ----
        rule add() -> Expr
            = first:mul() rest:(_ op:addop() _ r:mul() { (op, r) })*
              { fold_binops(first, rest) }
        rule addop() -> BinOp
            = "+" { BinOp::Add } / "-" { BinOp::Sub }

        rule mul() -> Expr
            = first:u_expr() rest:(_ op:mulop() _ r:u_expr() { (op, r) })*
              { fold_binops(first, rest) }
        rule mulop() -> BinOp
            = "//" { BinOp::FloorDiv }
            / "/" { BinOp::Div }
            / "%" { BinOp::Mod }
            / "@" { BinOp::MatMul }
            / "*" !"*" { BinOp::Mult }

        rule u_expr() -> Expr
            = "-" _ e:u_expr() { Expr::UnaryOp(UnaryOp::Neg, Box::new(e)) }
            / "+" _ e:u_expr() { Expr::UnaryOp(UnaryOp::Pos, Box::new(e)) }
            / power()

        rule power() -> Expr
            = base:postfix() exp:(_ "**" _ e:u_expr() { e })?
              { match exp {
                    Some(e) => Expr::BinOp(BinOp::Pow, Box::new(base), Box::new(e)),
                    None => base,
              } }

        rule postfix() -> Expr
            = a:atom() sufs:suffix()* {? apply_suffixes(a, sufs) }

        rule suffix() -> Suffix
            = _ "(" _ ")" { Suffix::Call(vec![]) }
            / _ "(" _ s:item_seq() _ ")" { Suffix::Call(s.0) }
            / _ "[" _ s:item_seq() _ "]" {? seq_to_expr(s.0, s.1).map(Suffix::Subscript) }

        // ---- atoms ----
        rule atom() -> Expr
            = number()
            / string()
            / kw_const()
            / n:ident() { Expr::Name(n) }
            / paren()
            / list()
            / dict()

        rule kw_const() -> Expr
            = "True" !idchar() { Expr::Bool(true) }
            / "False" !idchar() { Expr::Bool(false) }
            / "None" !idchar() { Expr::None }

        rule paren() -> Expr
            = "(" _ ")" { Expr::Tuple(vec![]) }
            / "(" _ s:item_seq() _ ")" {? seq_to_expr(s.0, s.1) }

        rule list() -> Expr
            = "[" _ "]" { Expr::List(vec![]) }
            / "[" _ s:item_seq() _ "]" { Expr::List(s.0) }

        rule dict() -> Expr
            = "{" _ "}" { Expr::Dict(vec![]) }
            / "{" _ first:pair() rest:(_ "," _ p:pair() { p })* (_ ",")? _ "}"
              { Expr::Dict(std::iter::once(first).chain(rest).collect()) }

        rule pair() -> (Expr, Expr)
            = k:add() _ ":" _ v:add() { (k, v) }

        // ---- lexical rules ----
        rule idchar() = ['A'..='Z' | 'a'..='z' | '0'..='9' | '_']

        rule ident() -> String
            = quiet!{ n:$(['A'..='Z' | 'a'..='z' | '_'] idchar()*) { n.to_string() } }
            / expected!("identifier")

        rule number() -> Expr
            = n:$(numlit()) {? n.replace('_', "").parse::<f64>().map(Expr::Num).or(Err("number")) }

        rule numlit()
            = digits() "." digits()? exponent()?   // 1.5   5.   1.e5
            / "." digits() exponent()?             // .5
            / digits() exponent()                  // 1e-6
            / digits()                             // 42

        // greedy PEG repetitions are possessive, so allow '_' only as a
        // digit-pair separator instead of lookahead trickery
        rule digits()
            = ['0'..='9'] ("_"? ['0'..='9'])*

        rule exponent()
            = ['e' | 'E'] ['+' | '-']? ['0'..='9']+

        rule string() -> Expr
            = "'" parts:sq_piece()* "'" { Expr::Str(parts.concat()) }
            / "\"" parts:dq_piece()* "\"" { Expr::Str(parts.concat()) }
            // Raw strings (r'...' / R"..."): backslashes are literal, no escape
            // processing. Common in LaTeX math labels, e.g. r'\omega^3'. A
            // backslash still keeps the following quote from terminating the
            // string, and is preserved, matching Python (r'\'' == "\\'").
            / ['r' | 'R'] "'" parts:raw_sq_piece()* "'" { Expr::Str(parts.concat()) }
            / ['r' | 'R'] "\"" parts:raw_dq_piece()* "\"" { Expr::Str(parts.concat()) }

        rule sq_piece() -> String
            = escape()
            / c:$([^ '\'' | '\\']) { c.to_string() }

        rule dq_piece() -> String
            = escape()
            / c:$([^ '"' | '\\']) { c.to_string() }

        rule raw_sq_piece() -> String
            = "\\" c:$([_]) { format!("\\{c}") }
            / c:$([^ '\'' | '\\']) { c.to_string() }

        rule raw_dq_piece() -> String
            = "\\" c:$([_]) { format!("\\{c}") }
            / c:$([^ '"' | '\\']) { c.to_string() }

        rule escape() -> String
            = "\\\\" { "\\".to_string() }
            / "\\'" { "'".to_string() }
            / "\\\"" { "\"".to_string() }
            / "\\n" { "\n".to_string() }
            / "\\t" { "\t".to_string() }
            / "\\r" { "\r".to_string() }
            // Python keeps unknown escapes verbatim: '\q' == '\\q'
            / "\\" c:$([_]) { format!("\\{c}") }
    }
}
