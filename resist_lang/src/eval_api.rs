/// Public evaluation API for ResistScript v2.
/// 
/// The GUI calls `eval_script(source)` and gets back everything it needs:
/// a built Circuit, layout metadata, analysis configs, and a log.

use std::collections::HashMap;
use resist::{Circuit, NodeId};
use pest::Parser;

use crate::ast::*;
use crate::{ResistParser, Rule};
use crate::parser::parse_program;
use crate::interpreter::{Environment, run_program};

/// Layout metadata for a single component (produced by `.pos()` / `.rot()` chains).
#[derive(Debug, Clone)]
pub struct LayoutEntry {
    pub name: String,
    pub comp_type: CompCtorType,
    pub nodes: Vec<NodeId>,
    pub x: i32,
    pub y: i32,
    pub rotation: i32,
}

/// An analysis command queued by the script.
#[derive(Debug, Clone)]
pub struct AnalysisConfig {
    pub kind: AnalysisKind,
    pub params: HashMap<String, f64>,
}

/// Successful result of evaluating a script.
pub struct ScriptResult {
    pub circuit: Circuit,
    pub layout: Vec<LayoutEntry>,
    pub analyses: Vec<AnalysisConfig>,
    pub nodes: HashMap<String, NodeId>,
    pub log: Vec<String>,
}

/// Error from parsing or evaluating a script.
#[derive(Debug)]
pub struct ScriptError {
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let (Some(line), Some(col)) = (self.line, self.column) {
            write!(f, "[{}:{}] {}", line, col, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

/// Evaluate a ResistScript v2 source string.
///
/// Returns a `ScriptResult` containing the built circuit, layout info,
/// queued analyses, resolved nodes, and log output.
pub fn eval_script(source: &str) -> Result<ScriptResult, ScriptError> {
    // 1. Parse
    let pairs = ResistParser::parse(Rule::program, source).map_err(|e| {
        let (line, col) = match e.line_col {
            pest::error::LineColLocation::Pos((l, c)) => (Some(l), Some(c)),
            pest::error::LineColLocation::Span((l, c), _) => (Some(l), Some(c)),
        };
        ScriptError {
            message: format!("{}", e),
            line,
            column: col,
        }
    })?;

    let program = parse_program(pairs.into_iter().next().ok_or_else(|| ScriptError {
        message: "Empty program".to_string(),
        line: None,
        column: None,
    })?);

    // 2. Interpret
    let mut env = Environment::new();
    run_program(&program, &mut env);

    Ok(ScriptResult {
        circuit: env.circuit,
        layout: env.layout,
        analyses: env.analyses,
        nodes: env.nodes,
        log: env.log,
    })
}
