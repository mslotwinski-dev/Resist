//! ResistScript v2 — A modern, Turing-complete DSL for procedural circuit generation.
//!
//! This crate provides a PEG parser (via `pest`), an AST, an interpreter,
//! and a public `eval_script()` API that compiles `.res` scripts down to
//! `resist::Circuit` builder calls with layout metadata.

use pest_derive::Parser;

pub mod ast;
pub mod eval_api;
pub mod interpreter;
pub mod parser;

/// The pest-generated parser for `.res` files.
#[derive(Parser)]
#[grammar = "resist.pest"]
pub struct ResistParser;
