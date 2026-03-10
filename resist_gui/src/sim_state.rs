use std::collections::HashMap;

use resist::analysis::nonlinear::NonLinearDcResult;
use resist::analysis::transient::TransientResult;
use resist::NodeId;

/// Explicit grid coordinates on the schematic canvas.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Orientation of a component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rotation {
    Deg0,
    Deg90,
    Deg180,
    Deg270,
}

impl Default for Rotation {
    fn default() -> Self {
        Self::Deg0
    }
}

/// Metadata for a component drawn on the schematic.
#[derive(Clone)]
pub struct ComponentInfo {
    pub id: String,
    pub name: String,
    pub kind: ComponentKind,
    pub pins: Vec<NodeId>,
    /// Explicit center position on the canvas.
    pub pos: Position,
    pub rotation: Rotation,
}

#[derive(Clone, Debug)]
pub enum ComponentKind {
    Resistor(f64),
    Capacitor(f64),
    Inductor(f64),
    VoltageSource(f64),
    CurrentSource(f64),
    Diode,
    Bjt { is_npn: bool },
    Mosfet { is_nmos: bool },
    TransientSource,
    OpAmp,
}

/// An explicit orthongonal wire segment connecting two absolute positions.
#[derive(Clone, Debug)]
pub struct WireSegment {
    pub start: Position,
    pub end: Position,
}

impl WireSegment {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }
}

/// Layout descriptor for the schematic canvas.
#[derive(Clone, Default)]
pub struct CircuitLayout {
    pub components: Vec<ComponentInfo>,
    /// Node label → absolute position (used for tooltips/hit testing).
    pub node_positions: HashMap<NodeId, Position>,
    /// Explicit wires connecting nodes and component pins.
    pub wires: Vec<WireSegment>,
    /// Explicit junction points to draw connection dots.
    pub junctions: Vec<Position>,
}

/// I-V sweep data point.
#[derive(Clone)]
pub struct IvPoint {
    pub v: f64,
    pub i: f64,
}

/// Current interactive selection in the GUI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectedEntity {
    None,
    Node(NodeId),
    Component(String),
    NodePair(NodeId, NodeId),
}

impl Default for SelectedEntity {
    fn default() -> Self {
        Self::None
    }
}

/// All simulation data needed by the GUI.
pub struct SimState {
    pub dc: Option<NonLinearDcResult>,
    pub transient: Option<Result<TransientResult, String>>,
    pub bode: Vec<(f64, resist::analysis::ac::AcAnalysisResult)>,
    pub iv_sweeps: HashMap<String, Vec<IvPoint>>,
    pub layout: CircuitLayout,
    /// Which nodes are currently selected for plotting (legacy, to be removed)
    pub selected_nodes: Vec<(NodeId, String)>,
    /// Global interactive selection.
    pub selection: SelectedEntity,
    /// Active plot tab.
    pub active_tab: PlotTab,
    /// Source code in the editor.
    pub source_code: String,
    /// Console output lines.
    pub console_output: Vec<ConsoleLine>,
}

/// A line of console output.
#[derive(Clone)]
pub struct ConsoleLine {
    pub text: String,
    pub is_error: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PlotTab {
    Transient,
    Bode,
    IvCurve,
}

impl Default for SimState {
    fn default() -> Self {
        Self {
            dc: None,
            transient: None,
            bode: Vec::new(),
            iv_sweeps: HashMap::new(),
            layout: CircuitLayout::default(),
            selected_nodes: Vec::new(),
            selection: SelectedEntity::None,
            active_tab: PlotTab::Transient,
            source_code: DEFAULT_SCRIPT.to_string(),
            console_output: Vec::new(),
        }
    }
}

/// The default script shown when the IDE opens.
pub const DEFAULT_SCRIPT: &str = r#"// ResistScript v2 — RC Low-Pass Filter
// Click ▶ Compile & Run to simulate!
// Wires are auto-routed from shared nodes.

let vsrc = StepSource(input, gnd, 0, 5, 10u).pos(80, 150).rot(90)
let r1 = Resistor(input, output, 1k).pos(200, 80)
let c1 = Capacitor(output, gnd, 100n).pos(320, 150).rot(90)

analyze.dc()
analyze.transient(stop: 1m, step: 1u)
"#;
