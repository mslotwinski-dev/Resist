/// Parser: Converts pest Pairs into the ResistScript v2 AST.

use pest::iterators::Pair;
use crate::ast::*;
use crate::Rule;

/// Parse a program from the top-level pest pair.
pub fn parse_program(pair: Pair<'_, Rule>) -> Program {
    let mut statements = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::statement {
            if let Some(stmt) = parse_statement(inner) {
                statements.push(stmt);
            }
        }
    }
    Program { statements }
}

fn parse_statement(pair: Pair<'_, Rule>) -> Option<Statement> {
    let inner = pair.into_inner().next()?;
    match inner.as_rule() {
        Rule::let_stmt => {
            let mut it = inner.into_inner();
            let name = it.next()?.as_str().to_string();
            let value = parse_expr(it.next()?);
            Some(Statement::Let { name, value })
        }
        Rule::assign_stmt => {
            let mut it = inner.into_inner();
            let name = it.next()?.as_str().to_string();
            let value = parse_expr(it.next()?);
            Some(Statement::Assign { name, value })
        }
        Rule::for_stmt => {
            let mut it = inner.into_inner();
            let var_name = it.next()?.as_str().to_string();
            let start = parse_expr(it.next()?);
            let end = parse_expr(it.next()?);
            let body = parse_block(it.next()?);
            Some(Statement::ForRange { var_name, start, end, body })
        }
        Rule::if_stmt => {
            let mut it = inner.into_inner();
            let condition = parse_expr(it.next()?);
            let then_block = parse_block(it.next()?);
            let else_block = it.next().map(parse_block);
            Some(Statement::If { condition, then_block, else_block })
        }
        Rule::analyze_stmt => {
            let mut it = inner.into_inner();
            let kind = match it.next()?.as_str() {
                "transient" => AnalysisKind::Transient,
                "ac" => AnalysisKind::Ac,
                "dc" => AnalysisKind::Dc,
                _ => return None,
            };
            let params = if let Some(params_pair) = it.next() {
                if params_pair.as_rule() == Rule::sim_params {
                    params_pair.into_inner().map(|p| {
                        let mut pit = p.into_inner();
                        let name = pit.next().unwrap().as_str().to_string();
                        let value = parse_expr(pit.next().unwrap());
                        NamedParam { name, value }
                    }).collect()
                } else { Vec::new() }
            } else { Vec::new() };
            Some(Statement::Analyze { kind, params })
        }
        Rule::expr_stmt => {
            let expr = parse_expr(inner.into_inner().next()?);
            Some(Statement::ExprStmt(expr))
        }
        _ => None,
    }
}

fn parse_block(pair: Pair<'_, Rule>) -> Vec<Statement> {
    pair.into_inner()
        .filter_map(|p| if p.as_rule() == Rule::statement { parse_statement(p) } else { None })
        .collect()
}

fn parse_expr(pair: Pair<'_, Rule>) -> Expr {
    match pair.as_rule() {
        Rule::expr => parse_expr(pair.into_inner().next().unwrap()),
        Rule::comparison => parse_binop_chain(pair, parse_comp_op),
        Rule::add_expr => parse_binop_chain(pair, parse_add_op),
        Rule::mul_expr => parse_binop_chain(pair, parse_mul_op),
        Rule::unary_expr => {
            let mut it = pair.into_inner();
            let first = it.next().unwrap();
            if first.as_rule() == Rule::unary_op {
                Expr::Neg(Box::new(parse_expr(it.next().unwrap())))
            } else {
                parse_expr(first)
            }
        }
        Rule::postfix_expr => {
            let mut it = pair.into_inner();
            let base = parse_expr(it.next().unwrap());
            let calls: Vec<MethodCall> = it.map(|mc| {
                let mut mit = mc.into_inner();
                let name = mit.next().unwrap().as_str().to_string();
                let args = mit.next()
                    .map(|al| parse_arg_list(al))
                    .unwrap_or_default();
                MethodCall { name, args }
            }).collect();
            if calls.is_empty() { base }
            else { Expr::MethodChain { receiver: Box::new(base), calls } }
        }
        Rule::atom => parse_expr(pair.into_inner().next().unwrap()),
        Rule::component_ctor => {
            let mut it = pair.into_inner();
            let comp_type = match it.next().unwrap().as_str() {
                "Resistor" => CompCtorType::Resistor,
                "Capacitor" => CompCtorType::Capacitor,
                "Inductor" => CompCtorType::Inductor,
                "VSource" => CompCtorType::VSource,
                "ISource" => CompCtorType::ISource,
                "Diode" => CompCtorType::Diode,
                "StepSource" => CompCtorType::StepSource,
                "SineSource" => CompCtorType::SineSource,
                "VCVS" => CompCtorType::VCVS,
                "BJT" => CompCtorType::BJT,
                "MOSFET" => CompCtorType::MOSFET,
                "FuncVSource" => CompCtorType::FuncVSource,
                _ => CompCtorType::Resistor,
            };
            let args = it.next().map(|al| parse_arg_list(al)).unwrap_or_default();
            Expr::ComponentCtor { comp_type, args }
        }
        Rule::phasor => {
            let mut it = pair.into_inner();
            let mag = parse_num_token(it.next().unwrap());
            let phase = parse_num_token(it.next().unwrap());
            Expr::Phasor { mag, phase_deg: phase }
        }
        Rule::lambda_expr => {
            let mut it = pair.into_inner();
            let param = it.next().unwrap().as_str().to_string();
            // inner could be block or expr. Both wrap logic.
            let inner_pair = it.next().unwrap();
            let body = if inner_pair.as_rule() == Rule::block_or_expr {
                // Dig into block_or_expr
                let innerest = inner_pair.into_inner().next().unwrap();
                if innerest.as_rule() == Rule::block {
                    // It's a block. If block just has an expr_stmt, use it.
                    let mut stmts = innerest.into_inner();
                    if let Some(stmt_pair) = stmts.next() {
                        if stmt_pair.as_rule() == Rule::statement {
                             let stmt_inner = stmt_pair.into_inner().next().unwrap();
                             if stmt_inner.as_rule() == Rule::expr_stmt {
                                 parse_expr(stmt_inner.into_inner().next().unwrap())
                             } else {
                                 Expr::Number(0.0) // Fallback for complex blocks
                             }
                        } else {
                            Expr::Number(0.0)
                        }
                    } else {
                        Expr::Number(0.0)
                    }
                } else {
                    parse_expr(innerest)
                }
            } else {
                parse_expr(inner_pair)
            };
            Expr::Lambda { param, body: Box::new(body) }
        }
        Rule::if_expr => {
            let mut it = pair.into_inner();
            let cond = Box::new(parse_expr(it.next().unwrap()));
            
            let parse_block_or_expr = |p: pest::iterators::Pair<'_, Rule>| {
                let innerest = p.into_inner().next().unwrap();
                if innerest.as_rule() == Rule::block {
                    let mut stmts = innerest.into_inner();
                    if let Some(stmt_pair) = stmts.next() {
                        if stmt_pair.as_rule() == Rule::statement {
                             let stmt_inner = stmt_pair.into_inner().next().unwrap();
                             if stmt_inner.as_rule() == Rule::expr_stmt {
                                 parse_expr(stmt_inner.into_inner().next().unwrap())
                             } else { Expr::Number(0.0) }
                        } else { Expr::Number(0.0) }
                    } else { Expr::Number(0.0) }
                } else {
                    parse_expr(innerest)
                }
            };
            
            let then_val = Box::new(parse_block_or_expr(it.next().unwrap()));
            let else_val = Box::new(parse_block_or_expr(it.next().unwrap()));
            Expr::IfExpr { cond, then_val, else_val }
        }
        Rule::func_call => {
            let mut it = pair.into_inner();
            let name = it.next().unwrap().as_str().to_string();
            let args = it.next().map(|al| parse_arg_list(al)).unwrap_or_default();
            Expr::FuncCall { name, args }
        }
        Rule::bool_lit => Expr::Bool(pair.as_str() == "true"),
        Rule::eng_number => Expr::Number(parse_eng_number_str(pair.as_str())),
        Rule::imag_number => {
            let s = pair.as_str();
            Expr::Imag(s[..s.len() - 1].parse().unwrap_or(0.0))
        }
        Rule::number => Expr::Number(pair.as_str().parse().unwrap_or(0.0)),
        Rule::string_lit => {
            let s = pair.as_str();
            Expr::StringLit(s[1..s.len() - 1].to_string())
        }
        Rule::dyn_ident => {
            let s = pair.as_str();
            if let Some(brace_start) = s.find("_{") {
                let prefix = &s[..brace_start];
                let expr_str = &s[brace_start + 2..s.len() - 1];
                Expr::DynIdent {
                    prefix: prefix.to_string(),
                    index_expr: Box::new(parse_inline_expr(expr_str)),
                }
            } else {
                Expr::Ident(s.to_string())
            }
        }
        Rule::ident => Expr::Ident(pair.as_str().to_string()),
        _ => Expr::Number(0.0),
    }
}

fn parse_arg_list(pair: Pair<'_, Rule>) -> Vec<Arg> {
    pair.into_inner().map(|arg| {
        let mut parts: Vec<Pair<'_, Rule>> = arg.into_inner().collect();
        if parts.len() >= 2 && parts[0].as_rule() == Rule::ident {
            // Could be named: check if there's a second element that is an expr
            let maybe_name = parts[0].as_str().to_string();
            let maybe_val = parts.remove(1);
            // If first was consumed as name label
            Arg { name: Some(maybe_name), value: parse_expr(maybe_val) }
        } else {
            Arg { name: None, value: parse_expr(parts.remove(0)) }
        }
    }).collect()
}

fn parse_binop_chain(pair: Pair<'_, Rule>, op_parser: fn(&str) -> Option<BinOpKind>) -> Expr {
    let mut it = pair.into_inner();
    let mut left = parse_expr(it.next().unwrap());
    while let Some(op_pair) = it.next() {
        let op = op_parser(op_pair.as_str()).unwrap();
        let right = parse_expr(it.next().unwrap());
        left = Expr::BinOp { left: Box::new(left), op, right: Box::new(right) };
    }
    left
}

fn parse_comp_op(s: &str) -> Option<BinOpKind> {
    match s {
        ">=" => Some(BinOpKind::Ge), "<=" => Some(BinOpKind::Le),
        "!=" => Some(BinOpKind::Ne), "==" => Some(BinOpKind::Eq),
        ">" => Some(BinOpKind::Gt), "<" => Some(BinOpKind::Lt),
        _ => None,
    }
}
fn parse_add_op(s: &str) -> Option<BinOpKind> {
    match s { "+" => Some(BinOpKind::Add), "-" => Some(BinOpKind::Sub), _ => None }
}
fn parse_mul_op(s: &str) -> Option<BinOpKind> {
    match s { "*" => Some(BinOpKind::Mul), "/" => Some(BinOpKind::Div), "%" => Some(BinOpKind::Rem), _ => None }
}

fn parse_num_token(pair: Pair<'_, Rule>) -> f64 {
    match pair.as_rule() {
        Rule::eng_number => parse_eng_number_str(pair.as_str()),
        Rule::number => pair.as_str().parse().unwrap_or(0.0),
        _ => 0.0,
    }
}

fn parse_eng_number_str(s: &str) -> f64 {
    let suffixes = [
        ("Meg", 1e6), ("meg", 1e6), ("kHz", 1e3), ("Hz", 1.0),
        ("k", 1e3), ("K", 1e3), ("G", 1e9), ("M", 1e6), ("T", 1e12),
        ("m", 1e-3), ("u", 1e-6), ("n", 1e-9), ("p", 1e-12), ("f", 1e-15),
        ("V", 1.0), ("A", 1.0),
    ];
    for (sfx, mult) in &suffixes {
        if s.ends_with(sfx) {
            let num = &s[..s.len() - sfx.len()];
            return num.parse::<f64>().unwrap_or(0.0) * mult;
        }
    }
    s.parse().unwrap_or(0.0)
}

/// Parse a simple inline expression from brace content.
fn parse_inline_expr(s: &str) -> Expr {
    let s = s.trim();
    if let Ok(n) = s.parse::<f64>() { return Expr::Number(n); }
    for op_char in ['+', '-', '*', '/', '%'] {
        if let Some(pos) = s.find(op_char) {
            if pos > 0 {
                let left = s[..pos].trim();
                let right = s[pos + 1..].trim();
                let left_expr = left.parse::<f64>().map(Expr::Number)
                    .unwrap_or_else(|_| Expr::Ident(left.to_string()));
                let right_expr = right.parse::<f64>().map(Expr::Number)
                    .unwrap_or_else(|_| Expr::Ident(right.to_string()));
                let op = match op_char {
                    '+' => BinOpKind::Add, '-' => BinOpKind::Sub,
                    '*' => BinOpKind::Mul, '/' => BinOpKind::Div,
                    '%' => BinOpKind::Rem,
                    _ => BinOpKind::Add,
                };
                return Expr::BinOp { left: Box::new(left_expr), op, right: Box::new(right_expr) };
            }
        }
    }
    Expr::Ident(s.to_string())
}
