use std::collections::HashMap;

use resist::analysis::nonlinear::NonLinearDcResult;
use resist::analysis::transient::TransientResult;
use resist::NodeId;

/// Integer coordinates on the schematic grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GridPoint {
    pub x: i32,
    pub y: i32,
}

impl GridPoint {
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
    pub node_a: NodeId,
    pub node_b: NodeId,
    /// Grid position of the component's Anchor (usually its center).
    pub pos: GridPoint,
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
}

/// An explicit orthogonal wire segment connecting two grid points.
#[derive(Clone, Debug)]
pub struct WireSegment {
    pub a: GridPoint,
    pub b: GridPoint,
}

impl WireSegment {
    pub fn new(a: GridPoint, b: GridPoint) -> Self {
        Self { a, b }
    }
}

/// Layout descriptor for the schematic canvas.
#[derive(Clone, Default)]
pub struct CircuitLayout {
    pub components: Vec<ComponentInfo>,
    /// Node label → grid position (used for junction dots and tooltips).
    pub node_positions: HashMap<NodeId, GridPoint>,
    /// Explicit wires connecting nodes and component pins.
    pub wires: Vec<WireSegment>,
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
    pub transient: Option<TransientResult>,
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
