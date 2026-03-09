/// Abstract Syntax Tree for ResistScript v2.

/// A complete program is a list of statements.
#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Statement>,
}

/// Top-level statements.
#[derive(Debug, Clone)]
pub enum Statement {
    /// `let x = expr;`
    Let { name: String, value: Expr },
    /// `x = expr;`
    Assign { name: String, value: Expr },
    /// `for i in start..end { ... }`
    ForRange {
        var_name: String,
        start: Expr,
        end: Expr,
        body: Vec<Statement>,
    },
    /// `if expr { ... } else { ... }`
    If {
        condition: Expr,
        then_block: Vec<Statement>,
        else_block: Option<Vec<Statement>>,
    },
    /// `analyze.dc();` or `analyze.transient(stop: 10m, step: 1u);`
    Analyze {
        kind: AnalysisKind,
        params: Vec<NamedParam>,
    },
    /// Bare expression statement.
    ExprStmt(Expr),
}

/// Analysis kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisKind {
    Dc,
    Ac,
    Transient,
}

/// A named parameter: `stop: 10m`.
#[derive(Debug, Clone)]
pub struct NamedParam {
    pub name: String,
    pub value: Expr,
}

/// Component types usable as constructors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompCtorType {
    Resistor,
    Capacitor,
    Inductor,
    VSource,
    ISource,
    Diode,
    StepSource,
    SineSource,
    VCVS,
    BJT,
    MOSFET,
    FuncVSource,
}

/// A method call in a chain: `.pos(100, 200)`.
#[derive(Debug, Clone)]
pub struct MethodCall {
    pub name: String,
    pub args: Vec<Arg>,
}

/// A function/method argument, optionally named.
#[derive(Debug, Clone)]
pub struct Arg {
    pub name: Option<String>,
    pub value: Expr,
}

/// Expression nodes.
#[derive(Debug, Clone)]
pub enum Expr {
    /// A real number literal (already scaled by engineering suffix).
    Number(f64),
    /// An imaginary literal: `3i`.
    Imag(f64),
    /// A phasor: `5 @ 45`.
    Phasor { mag: f64, phase_deg: f64 },
    /// A string literal.
    StringLit(String),
    /// Variable / identifier reference.
    Ident(String),
    /// Dynamic identifier: `node_{i+1}`.
    DynIdent { prefix: String, index_expr: Box<Expr> },
    /// Binary operation.
    BinOp { left: Box<Expr>, op: BinOpKind, right: Box<Expr> },
    /// Unary negation.
    Neg(Box<Expr>),
    /// Component constructor: `Resistor(a, b, 47k)`.
    ComponentCtor {
        comp_type: CompCtorType,
        args: Vec<Arg>,
    },
    /// Method chain: `expr.pos(100, 200).rot(90)`.
    MethodChain {
        receiver: Box<Expr>,
        calls: Vec<MethodCall>,
    },
    /// Function call: `sqrt(x)`.
    FuncCall { name: String, args: Vec<Arg> },
    /// A boolean literal (`true` or `false`)
    Bool(bool),
    /// A lambda expression: `|t| { expr }`
    Lambda { param: String, body: Box<Expr> },
    /// An inline if expression: `if cond { then_val } else { else_val }`
    IfExpr { cond: Box<Expr>, then_val: Box<Expr>, else_val: Box<Expr> },
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOpKind {
    Add, Sub, Mul, Div, Rem,
    Gt, Lt, Ge, Le, Eq, Ne,
}
