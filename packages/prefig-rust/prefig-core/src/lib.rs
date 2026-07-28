//! Rust port of PreFigure (https://prefigure.org).
//!
//! This crate shadows the reference Python implementation in `prefig/`; see
//! RUST_PORT_OUTLINE.md at the repository root for the architecture and the
//! Python-file → Rust-module mapping.

pub mod core;
pub mod engine;
pub mod evaluator;
pub mod value;
pub mod xml;
