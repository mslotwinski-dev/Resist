/// ResistScript v2 Interpreter — walks the AST, builds a Circuit,
/// collects layout metadata, and runs analysis commands.

use std::collections::HashMap;
use resist::{Circuit, NodeId};
use crate::ast::*;
use crate::eval_api::{LayoutEntry, AnalysisConfig};

/// Runtime value.
#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
    Complex { re: f64, im: f64 },
    Str(String),
    Bool(bool),
    /// A component that was just created (carries its name for layout binding).
    Component(String),
    Void,
}

impl Value {
    pub fn as_f64(&self) -> f64 {
        match self {
            Value::Number(n) => *n,
            Value::Bool(b) => if *b { 1.0 } else { 0.0 },
            _ => 0.0,
        }
    }
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Number(n) => *n != 0.0,
            Value::Bool(b) => *b,
            Value::Str(s) => !s.is_empty(),
            _ => false,
        }
    }
}

/// The interpreter's execution environment.
pub struct Environment {
    pub vars: HashMap<String, Value>,
    pub nodes: HashMap<String, NodeId>,
    pub circuit: Circuit,
    pub layout: Vec<LayoutEntry>,
    pub analyses: Vec<AnalysisConfig>,
    pub log: Vec<String>,
    comp_counter: u32,
}

impl Environment {
    pub fn new() -> Self {
        let mut nodes = HashMap::new();
        nodes.insert("gnd".to_string(), NodeId::GROUND);
        nodes.insert("GND".to_string(), NodeId::GROUND);
        let mut vars = HashMap::new();
        vars.insert("PI".to_string(), Value::Number(std::f64::consts::PI));
        
        Self {
            vars,
            nodes,
            circuit: Circuit::new(),
            layout: Vec::new(),
            analyses: Vec::new(),
            log: Vec::new(),
            comp_counter: 0,
        }
    }

    fn get_or_create_node(&mut self, name: &str) -> NodeId {
        // Enforce mathematical ground for gnd/GND/0
        if name == "gnd" || name == "GND" || name == "0" {
            return NodeId::GROUND;
        }
        if let Some(&id) = self.nodes.get(name) { return id; }
        let id = self.circuit.add_node();
        self.nodes.insert(name.to_string(), id);
        id
    }

    fn lookup(&self, name: &str) -> Value {
        self.vars.get(name).cloned().unwrap_or(Value::Number(0.0))
    }

    fn next_comp_name(&mut self, prefix: &str) -> String {
        self.comp_counter += 1;
        format!("{}_{}", prefix, self.comp_counter)
    }
}

/// Execute a program.
pub fn run_program(program: &Program, env: &mut Environment) {
    for stmt in &program.statements {
        exec_statement(env, stmt);
    }
}

fn exec_statement(env: &mut Environment, stmt: &Statement) {
    match stmt {
        Statement::Let { name, value } => {
            let v = eval_expr(env, value);
            env.vars.insert(name.clone(), v);
        }
        Statement::Assign { name, value } => {
            let v = eval_expr(env, value);
            env.vars.insert(name.clone(), v);
        }
        Statement::ForRange { var_name, start, end, body } => {
            let s = eval_expr(env, start).as_f64() as i64;
            let e = eval_expr(env, end).as_f64() as i64;
            for i in s..e {
                env.vars.insert(var_name.clone(), Value::Number(i as f64));
                for stmt in body {
                    exec_statement(env, stmt);
                }
            }
        }
        Statement::If { condition, then_block, else_block } => {
            if eval_expr(env, condition).is_truthy() {
                for s in then_block { exec_statement(env, s); }
            } else if let Some(eb) = else_block {
                for s in eb { exec_statement(env, s); }
            }
        }
        Statement::Analyze { kind, params } => {
            let mut param_map: HashMap<String, f64> = HashMap::new();
            for p in params {
                param_map.insert(p.name.clone(), eval_expr(env, &p.value).as_f64());
            }
            env.analyses.push(AnalysisConfig {
                kind: *kind,
                params: param_map,
            });
            env.log.push(format!("  ▸ Queued {:?} analysis", kind));
        }
        Statement::ExprStmt(expr) => {
            eval_expr(env, expr);
        }
    }
}

fn eval_expr(env: &mut Environment, expr: &Expr) -> Value {
    match expr {
        Expr::Number(n) => Value::Number(*n),
        Expr::Bool(b) => Value::Bool(*b),
        Expr::Imag(n) => Value::Complex { re: 0.0, im: *n },
        Expr::Phasor { mag, phase_deg } => {
            let re = mag * phase_deg.to_radians().cos();
            let im = mag * phase_deg.to_radians().sin();
            Value::Complex { re, im }
        }
        Expr::StringLit(s) => Value::Str(s.clone()),
        Expr::Ident(name) => env.lookup(name),
        Expr::DynIdent { prefix, index_expr } => {
            let idx = eval_expr(env, index_expr).as_f64() as i64;
            let resolved = format!("{}_{}", prefix, idx);
            // Check if it's a variable first, otherwise treat as node name
            if env.vars.contains_key(&resolved) {
                env.lookup(&resolved)
            } else {
                Value::Str(resolved)
            }
        }
        Expr::Neg(inner) => Value::Number(-eval_expr(env, inner).as_f64()),
        Expr::BinOp { left, op, right } => {
            if *op == BinOpKind::And {
                let lf = eval_expr(env, left);
                if !lf.is_truthy() { return Value::Bool(false); }
                let rf = eval_expr(env, right);
                return Value::Bool(rf.is_truthy());
            } else if *op == BinOpKind::Or {
                let lf = eval_expr(env, left);
                if lf.is_truthy() { return Value::Bool(true); }
                let rf = eval_expr(env, right);
                return Value::Bool(rf.is_truthy());
            }

            let lf = eval_expr(env, left).as_f64();
            let rf = eval_expr(env, right).as_f64();
            match op {
                BinOpKind::Add => Value::Number(lf + rf),
                BinOpKind::Sub => Value::Number(lf - rf),
                BinOpKind::Mul => Value::Number(lf * rf),
                BinOpKind::Div => Value::Number(if rf != 0.0 { lf / rf } else { f64::INFINITY }),
                BinOpKind::Rem => Value::Number(lf % rf),
                BinOpKind::Gt => Value::Bool(lf > rf),
                BinOpKind::Lt => Value::Bool(lf < rf),
                BinOpKind::Ge => Value::Bool(lf >= rf),
                BinOpKind::Le => Value::Bool(lf <= rf),
                BinOpKind::Eq => Value::Bool((lf - rf).abs() < 1e-12),
                BinOpKind::Ne => Value::Bool((lf - rf).abs() >= 1e-12),
                BinOpKind::And | BinOpKind::Or => unreachable!(),
            }
        }
        Expr::Lambda { .. } => Value::Void,
        Expr::IfExpr { cond, then_val, else_val } => {
            if eval_expr(env, cond).is_truthy() {
                eval_expr(env, then_val)
            } else {
                eval_expr(env, else_val)
            }
        }
        Expr::ComponentCtor { comp_type, args } => {
            instantiate_component(env, *comp_type, args)
        }
        Expr::MethodChain { receiver, calls } => {
            let base = eval_expr(env, receiver);
            apply_method_chain(env, base, calls)
        }
        Expr::FuncCall { name, args } => {
            eval_builtin(env, name, args)
        }
    }
}

fn resolve_node_arg(env: &mut Environment, arg: &Arg) -> NodeId {
    // CRITICAL: Check for identifier-based node references FIRST.
    // This prevents `gnd` from being evaluated as `0.0` and all unknown
    // idents from collapsing to ground.
    match &arg.value {
        Expr::Ident(name) => {
            return env.get_or_create_node(name);
        }
        Expr::DynIdent { prefix, index_expr } => {
            let idx = eval_expr(env, index_expr).as_f64() as i64;
            let resolved = format!("{}_{}", prefix, idx);
            return env.get_or_create_node(&resolved);
        }
        _ => {}
    }
    // For non-ident expressions (e.g. literal 0), evaluate
    let val = eval_expr(env, &arg.value);
    match val {
        Value::Str(s) => env.get_or_create_node(&s),
        Value::Number(n) if n == 0.0 => NodeId::GROUND,
        _ => env.get_or_create_node("__anon"),
    }
}

fn instantiate_component(env: &mut Environment, ct: CompCtorType, args: &[Arg]) -> Value {
    let prefix = match ct {
        CompCtorType::Resistor => "R",
        CompCtorType::Capacitor => "C",
        CompCtorType::Inductor => "L",
        CompCtorType::VSource => "V",
        CompCtorType::ISource => "I",
        CompCtorType::Diode => "D",
        CompCtorType::StepSource => "V_step",
        CompCtorType::SineSource => "V_sine",
        CompCtorType::VCVS => "E",
        CompCtorType::BJT => "Q",
        CompCtorType::MOSFET => "M",
        CompCtorType::FuncVSource => "V_func",
    };
    let comp_name = env.next_comp_name(prefix);

    // Expected args: (node_a, node_b, value?) for most, (node_a, node_b) for Diode
    let na = if args.len() > 0 { resolve_node_arg(env, &args[0]) } else { NodeId::GROUND };
    let nb = if args.len() > 1 { resolve_node_arg(env, &args[1]) } else { NodeId::GROUND };
    let val = if args.len() > 2 { eval_expr(env, &args[2].value).as_f64() } else { 0.0 };
    
    let mut layout_nodes = vec![na, nb];

    match ct {
        CompCtorType::Resistor => {
            let r = if val > 0.0 { val } else { 1000.0 };
            env.circuit.add_resistor(&comp_name, na, nb, r);
            env.log.push(format!("  + R {} = {:.3}", comp_name, r));
        }
        CompCtorType::Capacitor => {
            let c = if val > 0.0 { val } else { 1e-6 };
            env.circuit.add_capacitor(&comp_name, na, nb, c);
            env.log.push(format!("  + C {} = {:.3e}", comp_name, c));
        }
        CompCtorType::Inductor => {
            let l = if val > 0.0 { val } else { 1e-3 };
            env.circuit.add_inductor(&comp_name, na, nb, l);
            env.log.push(format!("  + L {} = {:.3e}", comp_name, l));
        }
        CompCtorType::VSource => {
            let v = val;
            env.circuit.add_voltage_source(&comp_name, na, nb, v);
            env.log.push(format!("  + V {} = {:.3}V", comp_name, v));
        }
        CompCtorType::ISource => {
            env.circuit.add_current_source(&comp_name, na, nb, val);
            env.log.push(format!("  + I {} = {:.3}A", comp_name, val));
        }
        CompCtorType::Diode => {
            let model = resist::components::models::DiodeModel::default();
            env.circuit.add_diode(&comp_name, na, nb, model);
            env.log.push(format!("  + D {}", comp_name));
        }
        CompCtorType::StepSource => {
            let v1 = if args.len() > 2 { eval_expr(env, &args[2].value).as_f64() } else { 0.0 };
            let v2 = if args.len() > 3 { eval_expr(env, &args[3].value).as_f64() } else { 5.0 };
            let delay = if args.len() > 4 { eval_expr(env, &args[4].value).as_f64() } else { 0.0 };
            let waveform = resist::components::transient_voltage_source::Waveform::Step { 
                v1, v2, delay 
            };
            env.circuit.add_transient_voltage_source(&comp_name, na, nb, waveform);
            env.log.push(format!("  + V_step {} = {}V -> {}V @ {}s", comp_name, v1, v2, delay));
        }
        CompCtorType::SineSource => {
            let offset = if args.len() > 2 { eval_expr(env, &args[2].value).as_f64() } else { 0.0 };
            let amplitude = if args.len() > 3 { eval_expr(env, &args[3].value).as_f64() } else { 1.0 };
            let freq = if args.len() > 4 { eval_expr(env, &args[4].value).as_f64() } else { 1000.0 };
            let delay = if args.len() > 5 { eval_expr(env, &args[5].value).as_f64() } else { 0.0 };
            let phase_deg = -delay * freq * 360.0;
            let waveform = resist::components::transient_voltage_source::Waveform::Sine {
                offset, amplitude, freq, phase_deg
            };
            env.circuit.add_transient_voltage_source(&comp_name, na, nb, waveform);
            env.log.push(format!("  + V_sine {} = {}V + {}V*sin(2pi*{}t)", comp_name, offset, amplitude, freq));
        }
        CompCtorType::VCVS => {
            // VCVS(out_p, out_n, in_p, in_n, gain)
            let in_p = if args.len() > 2 { resolve_node_arg(env, &args[2]) } else { NodeId::GROUND };
            let in_n = if args.len() > 3 { resolve_node_arg(env, &args[3]) } else { NodeId::GROUND };
            let gain = if args.len() > 4 { eval_expr(env, &args[4].value).as_f64() } else { 1.0 };
            layout_nodes.push(in_p);
            layout_nodes.push(in_n);
            env.circuit.add_vcvs(&comp_name, na, nb, in_p, in_n, gain);
            env.log.push(format!("  + E (VCVS) {} = {} * V({:?},{:?})", comp_name, gain, in_p, in_n));
        }
        CompCtorType::BJT => {
            // BJT(c, b, e, is_pnp=false)
            let c = na;
            let b = nb;
            let e = if args.len() > 2 { resolve_node_arg(env, &args[2]) } else { NodeId::GROUND };
            layout_nodes.push(e);
            let is_pnp = if args.len() > 3 { eval_expr(env, &args[3].value).is_truthy() } else { false };
            let mut model = resist::components::models::BjtModel::default();
            if is_pnp { model.is_npn = false; }
            env.circuit.add_bjt(&comp_name, c, b, e, model);
            env.log.push(format!("  + Q (BJT) {} ({})", comp_name, if is_pnp { "PNP" } else { "NPN" }));
        }
        CompCtorType::MOSFET => {
            // MOSFET(d, g, s, bulk, is_pmos=false)
            let d = na;
            let g = nb;
            let s = if args.len() > 2 { resolve_node_arg(env, &args[2]) } else { NodeId::GROUND };
            let bulk = if args.len() > 3 { resolve_node_arg(env, &args[3]) } else { NodeId::GROUND };
            layout_nodes.push(s);
            layout_nodes.push(bulk);
            let is_pmos = if args.len() > 4 { eval_expr(env, &args[4].value).is_truthy() } else { false };
            let mut model = resist::components::models::MosfetModel::default();
            if is_pmos { model.is_nmos = false; }
            env.circuit.add_mosfet(&comp_name, d, g, s, bulk, model);
            env.log.push(format!("  + M (MOSFET) {} ({})", comp_name, if is_pmos { "PMOS" } else { "NMOS" }));
        }
        CompCtorType::FuncVSource => {
            let mut param_name = "t".to_string();
            let mut body_expr = Box::new(Expr::Number(0.0));
            if args.len() > 2 {
                if let Expr::Lambda { param, body } = &args[2].value {
                    param_name = param.clone();
                    body_expr = body.clone();
                }
            }
            
            // Capture numerical variables out of the Environment into a localized flat map 
            // so they can be securely Sent across thread boundaries into the physics core.
            let captured_vars = env.vars.iter()
                .map(|(k, v)| (k.clone(), v.as_f64()))
                .collect::<HashMap<_, _>>();
                
            let p_name_clone = param_name.clone();
            let closure = move |t: f64| -> f64 {
                eval_math_only(&body_expr, &captured_vars, &p_name_clone, t)
            };
            
            let waveform = resist::components::transient_voltage_source::Waveform::Custom(std::sync::Arc::new(closure));
            env.circuit.add_transient_voltage_source(&comp_name, na, nb, waveform);
            env.log.push(format!("  + V_func {} = |{}| {{ ... }}", comp_name, param_name));
        }
    }

    // Create a default layout entry
    env.layout.push(LayoutEntry {
        name: comp_name.clone(),
        comp_type: ct,
        nodes: layout_nodes,
        x: 0,
        y: 0,
        rotation: 0,
    });

    Value::Component(comp_name)
}

fn apply_method_chain(env: &mut Environment, base: Value, calls: &[MethodCall]) -> Value {
    let comp_name = match &base {
        Value::Component(name) => name.clone(),
        _ => return base,
    };

    for call in calls {
        match call.name.as_str() {
            "pos" => {
                let x = if call.args.len() > 0 { eval_expr(env, &call.args[0].value).as_f64() as i32 } else { 0 };
                let y = if call.args.len() > 1 { eval_expr(env, &call.args[1].value).as_f64() as i32 } else { 0 };
                if let Some(entry) = env.layout.iter_mut().find(|e| e.name == comp_name) {
                    entry.x = x;
                    entry.y = y;
                }
            }
            "rot" => {
                let deg = if call.args.len() > 0 { eval_expr(env, &call.args[0].value).as_f64() } else { 0.0 };
                if let Some(entry) = env.layout.iter_mut().find(|e| e.name == comp_name) {
                    entry.rotation = deg as i32;
                }
            }
            _ => {
                env.log.push(format!("  ⚠ Unknown method: .{}()", call.name));
            }
        }
    }

    base
}

fn eval_builtin(env: &mut Environment, name: &str, args: &[Arg]) -> Value {
    match name {
        "sqrt" => Value::Number(eval_expr(env, &args[0].value).as_f64().sqrt()),
        "abs" => Value::Number(eval_expr(env, &args[0].value).as_f64().abs()),
        "sin" => Value::Number(eval_expr(env, &args[0].value).as_f64().sin()),
        "cos" => Value::Number(eval_expr(env, &args[0].value).as_f64().cos()),
        "exp" => Value::Number(eval_expr(env, &args[0].value).as_f64().exp()),
        "log10" => Value::Number(eval_expr(env, &args[0].value).as_f64().log10()),
        "ln" => Value::Number(eval_expr(env, &args[0].value).as_f64().ln()),
        "print" => {
            let v = eval_expr(env, &args[0].value);
            let msg = match v {
                Value::Number(n) => format!("{}", n),
                Value::Str(s) => s,
                Value::Bool(b) => format!("{}", b),
                Value::Complex { re, im } => format!("{} + {}i", re, im),
                Value::Component(c) => format!("<{}>", c),
                Value::Void => "(void)".to_string(),
            };
            env.log.push(format!("  [print] {}", msg));
            Value::Void
        }
        _ => {
            env.log.push(format!("  ⚠ Unknown function: {}()", name));
            Value::Void
        }
    }
}

/// A specialized, high-performance AST fast-path evaluator designed EXCLUSIVELY to execute 
/// raw mathematical `Expr` structures without taking a lock on the heavy `Environment`. 
/// Used natively within the MNA `TransientAnalyzer` time-step hot loop.
fn eval_math_only(expr: &Expr, vars: &HashMap<String, f64>, t_param: &str, t_val: f64) -> f64 {
    match expr {
        Expr::Number(n) => *n,
        Expr::Bool(b) => if *b { 1.0 } else { 0.0 },
        Expr::Ident(name) => {
            if name == t_param {
                t_val
            } else {
                vars.get(name).copied().unwrap_or(0.0)
            }
        }
        Expr::Neg(inner) => -eval_math_only(inner, vars, t_param, t_val),
        Expr::BinOp { left, op, right } => {
            if *op == BinOpKind::And {
                let l = eval_math_only(left, vars, t_param, t_val);
                if l == 0.0 { return 0.0; }
                let r = eval_math_only(right, vars, t_param, t_val);
                return if r != 0.0 { 1.0 } else { 0.0 };
            } else if *op == BinOpKind::Or {
                let l = eval_math_only(left, vars, t_param, t_val);
                if l != 0.0 { return 1.0; }
                let r = eval_math_only(right, vars, t_param, t_val);
                return if r != 0.0 { 1.0 } else { 0.0 };
            }

            let l = eval_math_only(left, vars, t_param, t_val);
            let r = eval_math_only(right, vars, t_param, t_val);
            match op {
                BinOpKind::Add => l + r,
                BinOpKind::Sub => l - r,
                BinOpKind::Mul => l * r,
                BinOpKind::Div => if r != 0.0 { l / r } else { f64::INFINITY },
                BinOpKind::Rem => l % r,
                BinOpKind::Gt => if l > r { 1.0 } else { 0.0 },
                BinOpKind::Lt => if l < r { 1.0 } else { 0.0 },
                BinOpKind::Ge => if l >= r { 1.0 } else { 0.0 },
                BinOpKind::Le => if l <= r { 1.0 } else { 0.0 },
                BinOpKind::Eq => if (l - r).abs() < 1e-12 { 1.0 } else { 0.0 },
                BinOpKind::Ne => if (l - r).abs() >= 1e-12 { 1.0 } else { 0.0 },
                BinOpKind::And | BinOpKind::Or => unreachable!(),
            }
        }
        Expr::IfExpr { cond, then_val, else_val } => {
            if eval_math_only(cond, vars, t_param, t_val) != 0.0 {
                eval_math_only(then_val, vars, t_param, t_val)
            } else {
                eval_math_only(else_val, vars, t_param, t_val)
            }
        }
        Expr::FuncCall { name, args } => {
            if args.is_empty() { return 0.0; }
            let v = eval_math_only(&args[0].value, vars, t_param, t_val);
            match name.as_str() {
                "sqrt" => v.sqrt(),
                "abs" => v.abs(),
                "sin" => v.sin(),
                "cos" => v.cos(),
                "exp" => v.exp(),
                "log10" => v.log10(),
                "ln" => v.ln(),
                _ => 0.0,
            }
        }
        _ => 0.0,
    }
}
