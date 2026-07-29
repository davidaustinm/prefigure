//! AST for the whitelisted expression language (RUST_PORT_OUTLINE.md §16.2).
//!
//! This mirrors exactly the node types admitted by `validate_node` in
//! prefig/core/user_namespace.py — nothing more.

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(f64),
    Str(String),
    Bool(bool),
    None,
    Name(String),
    List(Vec<Expr>),
    Tuple(Vec<Expr>),
    Dict(Vec<(Expr, Expr)>),
    BinOp(BinOp, Box<Expr>, Box<Expr>),
    UnaryOp(UnaryOp, Box<Expr>),
    /// Callee is always a bare name: Python's validator reads `node.func.id`.
    Call(String, Vec<Expr>),
    Subscript(Box<Expr>, Box<Expr>),
    Starred(Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mult,
    MatMul,
    Div,
    FloorDiv,
    Mod,
    Pow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Pos,
}
