# Ohm Language Reference Manual (.ohm)

<div align="center">
  <img src="public/ohm.png" alt="Ohm Logo" width="200" />
</div>

## 1. Introduction & Philosophy

Welcome to **Ohm** (`.ohm`), a modern procedural domain-specific language (DSL) for electrical engineering. Ohm brings programmatic workflows to Modified Nodal Analysis (MNA) circuit simulation, acting as a programmable replacement for traditional SPICE netlists.

Ohm code is executed via the `resist` physics engine. Wires are auto-routed implicitly using the mathematical topological identifier of the nodes.

---

## 2. Lexical Structure & Data Types

### Variables
Nodes and numbers are bound dynamically.
- **`let` Statement**: Defines new variables `let x = 10k`. 
- **Assignments**: Mutates existing variables `x = 5`.

*(Note: `const` is currently not supported in the AST).*

### Supported Data Types
Ohm provides native support for engineering mathematics:
- **Numbers**: Floating point precision literals (`1.0`, `5e-3`).
- **Complex Numbers**: Automatically parses purely imaginary literals directly appended with `i` (`3i`).
- **Phasors**: Defined using the `@` magnitude-phase operator (`5 @ 45`).
- **Strings**: Used occasionally as raw parameters (`"node"`).
- **Booleans**: Logical literals matching exactly `true` and `false`.

### Engineering Suffixes
Numeric literals natively support unit-less and unit-aware SI prefix scaling. These prefixes are expanded identically in the parser tree.

| Suffix   | Equivalent | Value (Multiplier) |
|----------|------------|--------------------|
| `Meg`, `meg`, `M` | Mega       | $10^{6}$           |
| `k`, `K` | Kilo       | $10^{3}$           |
| `m`      | Milli      | $10^{-3}$          |
| `u`      | Micro      | $10^{-6}$          |
| `n`      | Nano       | $10^{-9}$          |
| `p`      | Pico       | $10^{-12}$         |
| `f`      | Femto      | $10^{-15}$         |
| `G`      | Giga       | $10^{9}$           |
| `T`      | Tera       | $10^{12}$          |
| `Hz`, `V`, `A` | Base Units | $1.0$ (No scaling) |
| `kHz`    | Kilo       | $10^{3}$           |

---

## 3. Supported Operators

Ohm's `BinOpKind` AST securely implements the following strict mathematical and relational operators.

### Arithmetic Operations
| Operator | Description | Sub-AST Implementation |
|----------|-------------|------------------------|
| `+`      | Addition    | `BinOpKind::Add` |
| `-`      | Subtraction | `BinOpKind::Sub` |
| `*`      | Multiplication | `BinOpKind::Mul` |
| `/`      | Division    | `BinOpKind::Div` |

*(Note: Advanced operators like Modulo (`%`) or Exponentiation (`^`) are omitted by design to keep the evaluation loop deterministic).*

### Relational Operations
| Operator | Description | Sub-AST Implementation |
|----------|-------------|------------------------|
| `==`     | Equal to    | `BinOpKind::Eq` |
| `!=`     | Not equal   | `BinOpKind::Ne` |
| `<`      | Less than   | `BinOpKind::Lt` |
| `>`      | Greater than| `BinOpKind::Gt` |
| `<=`     | Less or Eq  | `BinOpKind::Le` |
| `>=`     | Greater or Eq| `BinOpKind::Ge` |

*(Note: `&&` and `||` logical operators are currently intrinsically unsupported by Ohm's standard compiler suite).*

---

## 4. Control Flow

Ohm uses scoped braces `{ ... }` for block topologies and evaluates strictly procedure-oriented topologies. Due to changes in the grammar tree, semicolons `;` are completely optional at the end of statements.

### If / Else Conditionals
Parenthesis enclosing the condition are omitted by convention. `else if` chains are unsupported; nest `else { if ... }` instead if multiple evaluations are needed.
```ohm
if x > 10 {
    let r1 = Resistor(in, out, 1k)
} else {
    let r1 = Resistor(in, out, 100)
}
```

### For Range Iteration
Loop limits express an exclusive integer bound range evaluating via standard Rust `start..end`.
```ohm
for i in 1..5 {
    // Generates 4 sequential iterations
    let x = x + i
}
```

### String Interpolation (Dynamic Nodes)
To build cascading circuits recursively, define nodes dynamically using the evaluation brace `_{expr}`. The identifier is cast string-wise at runtime.
```ohm
for i in 1..10 {
    let resistor = Resistor(n_{i}, n_{i+1}, 100)
}
```

---

## 5. Built-in Abstract Components

Ohm strictly implements component allocation as internal function dispatches toward standard physics engines. Any unrecognized component triggers a `Void` drop failure in the analyzer.

### Passives
| Signature | Description |
|-----------|-------------|
| `Resistor(n1, n2, value)` | Linear fixed resistance mapping. Defaults to `1000.0` if omitted. |
| `Capacitor(n1, n2, value)`| Linear capacitance mapping resolving via Backward Euler implicit models. Defaults to `1e-6`. |
| `Inductor(n1, n2, value)` | Linear inductance mapping. Defaults to `1e-3`. |

### Sources
| Signature | Description |
|-----------|-------------|
| `VSource(n_pos, n_neg, value)` | Ideal internal Direct Current voltage reference. |
| `ISource(n_pos, n_neg, value)` | Ideal internal Direct Current current reference. |
| `StepSource(n_pos, n_neg, v_init, v_step, delay_time)` | Transient time-varying step function resolving as a continuous `Waveform::Step`. |

### Non-Linear Semiconductors
| Signature | Description |
|-----------|-------------|
| `Diode(anode, cathode)` | Standard static PN Junction diode implementing the default Newton-Raphson approximation. |

*(Note: Transistors like BJT and MOSFET are supported in the core `resist` crate's mathematical structures, but are currently not mapped into the AST component loader module).*

---

## 6. Visual Component Representation

Schematic properties are assigned sequentially via evaluation-chain modifiers.

### Physical Auto-Routing Metadata
- **`.pos(x, y)`**: Injects `[X, Y]` geometric floats representing the local pixel center of the visual asset onto the schematic grid.
- **`.rot(degrees)`**: Evaluates absolute rotation. Should typically be normalized in standard intervals (`0, 90, 180, 270`). 

### The Master Ground Node
Nodes are implicitly wired together sequentially based on matching identifier strings. If a topological node evaluates exclusively to the literal string `"gnd"`, `"GND"`, or `"0"`, its matrix evaluation converges locally onto `NodeId::GROUND` representing a mathematical zero volt identity sink.

---

## 7. Mathematical Directives (Commands)

All mathematical instructions dispatch explicitly via the top-level `analyze.` keyword context modifier.

### `analyze.dc()`
Fires the baseline `DcAnalyzer`, attempting to resolve static quiescent operating points via internal iterative linear bounds estimation algorithm matrices.

### `analyze.transient(stop: f64, step: f64, uic: bool)`
Performs active mathematical integrations representing physical elapsed time vectors within the network.
- `stop`: Upper theoretical boundary to evaluate to in fractional seconds.
- `step`: Discretized $\Delta t$ bounds sampling resolution.
- `uic`: Boolean mapping evaluating directly to the strict Use Initial Conditions directive `use_ic`. Bypasses automatic $t=0$ DC solving and locks internal physics structures securely to `0.0V` to stabilize Step Responses.

### `analyze.ac()`
Bootstraps the internal frequency distribution analyzer logic. Supports general dictionary parameters internally mapping frequency arrays.

---

## 8. Working Component Demonstration

This comprehensive program reflects completely exact supported logic utilizing variables, integer range loops, string interpolators, the step source modifier, and valid commands inside the Ohm specification.

```ohm
// ResistScript Engine Evaluation: Ohm Pipeline 
let vsrc = StepSource(input, gnd, 0, 5, 10u).pos(50, 150).rot(90)

// Cascading RC Sequence Allocation Pipeline
for i in 1..6 {
    let r = Resistor(n_{i}, n_{i + 1}, 1k).pos(100 + i * 80, 80)
    let c = Capacitor(n_{i + 1}, gnd, 100n).pos(100 + i * 80, 180).rot(90)
}

// Peripheral Terminations
let rin = Resistor(input, n_1, 10).pos(100, 80)
let rout = Resistor(n_6, output, 1).pos(580, 80)

analyze.dc()
analyze.transient(stop: 500u, step: 100n, uic: true)
```
