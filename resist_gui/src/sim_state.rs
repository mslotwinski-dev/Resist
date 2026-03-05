use std::collections::HashMap;

use resist::analysis::nonlinear::NonLinearDcResult;
use resist::analysis::transient::TransientResult;
use resist::NodeId;

/// Explicit coordinates on the schematic canvas.
#[derive(Clone, Copy, Debug)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

impl Position {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

// Equality based on a small epsilon since it's float
impl PartialEq for Position {
    fn eq(&self, other: &Self) -> bool {
        (self.x - other.x).abs() < 1e-3 && (self.y - other.y).abs() < 1e-3
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
    pub node_a: NodeId,
    pub node_b: NodeId,
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
        }
    }
}
