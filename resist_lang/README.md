# ResistScript (`.ohm`)

**ResistScript** is a Turing-complete, modern Domain-Specific Language (DSL) for procedural circuit definition and simulation. Built from the ground up for the `resist` EDA engine, ResistScript is the programmatic alternative to legacy SPICE netlists.

At its core, ResistScript bridges the gap between software engineering and circuit design. It combines standard programming language constructs—like variables, loops, and math operations—with powerful electronic simulation primitives and automatic schematic generation.

Gone are the days of manually typing out rigid node lists or copying and pasting components to create a large array. With ResistScript, you **code your circuit**.

---

## ⚡ Philosophy

1. **Procedural First:** Scale your designs algorithmically. If you need a 100-stage filter, write a 3-line `for` loop instead of a 100-line netlist.
2. **Auto-Routed Schematics:** Define your logical connections and place your components visually (`.pos()`). ResistScript's engine will automatically route Manhattan-style wires to form your schematic.
3. **Developer Experience (DX):** ResistScript is inspired by modern languages like Rust and JavaScript. It features clean syntax, first-class complex numbers, and native engineering suffixes.

---

## 🧮 Basic Syntax & Types

### Variables & Constants

Define mathematical parameters safely via `let` and constants with `const` (where applicable):

```rust
const PI = 3.14159;
let base_res = 10;
let default_voltage = 5.0;
```

### Engineering Suffixes Native Support

ResistScript natively parses standard SI engineering suffixes directly attached to numbers. This makes component values instantly readable:

- **`p`** (`1e-12`), **`n`** (`1e-9`), **`u`** (`1e-6`), **`m`** (`1e-3`)
- **`k`** (`1e3`), **`Meg`** or **`M`** (`1e6`), **`G`** (`1e9`)

```rust
let r_val = 4.7k;   // 4700.0
let c_val = 100n;   // 1e-7
let i_bias = 5m;    // 0.005
```

### Complex Numbers & Phasors

ResistScript treats complex quantities as first-class citizens, essential for AC analysis.
You can declare complex numbers in **Cartesian** or **Polar (Phasor)** form:

```rust
// Cartesian: real + imaginary 'i'
let z = 2 + 3i;

// Polar / Phasor: magnitude @ phase_degrees
let p = 5 @ 45;
```

---

## 🔄 Control Flow & Procedural Generation

ResistScript supports iterative control flow to algorithmically build circuits.

### For-Loops

Build arrays of components effortlessly using Rust-style exclusive ranges (`start..end`):

```rust
for i in 1..5 {
    // Generates elements for i = 1, 2, 3, 4
}
```

### Dynamic Node Interpolation

Inside your loops, you can dynamically bind components to procedurally generated node names using the `node_{expr}` string interpolation syntax.

```rust
for i in 1..5 {
    // Dynamically generates nodes: n_1, n_2, n_3... based on loop index
    let r = Resistor(n_{i}, n_{i+1}, 1k);
}
```

---

## 🧱 Component Instantiation & Layout

ResistScript treats components like functions/objects. The base signature for most passive components is:
`ComponentType(node_A, node_B, value)`

Available components: `Resistor`, `Capacitor`, `Inductor`, `VSource`, `ISource`, `Diode`.

### Method Chaining for Layout

To render the circuit beautifully in the IDE's Schematic Canvas without manual wire routing, simply specify the visual absolute position (`.pos(x, y)`) and the rotation (`.rot(degrees)`) of each component inline:

```rust
let v1 = VSource(input, gnd, 5).pos(80, 150).rot(90);
let r1 = Resistor(input, output, 1k).pos(200, 80);
```

### The Net Auto-Router & Ground

You do **not** need to manually draw wires!

- **Auto-Routing:** Component pins sharing the same logical node string (e.g., `input`, `n_1`) are automatically bridged using a Trunk-and-Branch Manhattan algorithm by the rendering engine.
- **Auto-Ground:** Connecting any pin to the reserved `gnd` or `0` node automatically evaluates it as mathematical `NodeId::GROUND`, generating a downward-facing Earth Ground symbol exactly at that pin's location.

---

## 🔬 Simulation Commands

Once the circuit is modeled procedurally, queue the numerical solvers securely using the `analyze` namespace:

```rust
// Calculates the Non-Linear DC Operating Point
analyze.dc();

// Calculates the Time-Domain Transient response
// (Takes named parameters 'stop' and 'step')
analyze.transient(stop: 1m, step: 1u);

// Calculates the Frequency-Domain AC Sweep
analyze.ac(start: 10, stop: 100k, points: 50);
```

---

## ✨ Showcase Example

The true power of ResistScript is combining equations and loops to build scale mathematically. Here is a procedural 5-stage RC Ladder filter generated cleanly in under 15 lines of code:

```rust
// ═══════════════════════════════════════════════════════════════════════
// ResistScript v2 — 5-Stage RC Ladder Filter
// Wires are auto-routed dynamically from shared nodes!
// ═══════════════════════════════════════════════════════════════════════

// Drive the ladder with a 5V source at the input node
let vsrc = VSource(input, gnd, 5).pos(50, 150).rot(90);

// Tie the source to our dynamic ladder entry point
let rin = Resistor(input, n_1, 10).pos(100, 80);

// Procedurally generate the 5 individual RC filter stages
for i in 1..6 {
    // Dynamically interconnect nodes: n_1->n_2, n_2->n_3, etc.
    let r = Resistor(n_{i}, n_{i + 1}, 1k).pos(100 + i * 80, 80);
    let c = Capacitor(n_{i + 1}, gnd, 10n).pos(100 + i * 80, 180).rot(90);
}

// Extract the signal at the final processed tap
let rout = Resistor(n_6, output, 1).pos(580, 80);

// Queue solvers
analyze.dc();
analyze.transient(stop: 500u, step: 100n);
```
