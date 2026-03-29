use std::collections::HashMap;

use resist::NodeId;
use resist::analysis::nonlinear::NonLinearDcResult;
use resist::analysis::transient::TransientResult;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rotation {
    Deg0,
    Deg90,
    Deg180,
    Deg270,
}

impl Rotation {
    pub fn next(self) -> Self {
        match self {
            Self::Deg0 => Self::Deg90,
            Self::Deg90 => Self::Deg180,
            Self::Deg180 => Self::Deg270,
            Self::Deg270 => Self::Deg0,
        }
    }
}

impl Default for Rotation {
    fn default() -> Self {
        Self::Deg0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComponentKind {
    Resistor,
    Capacitor,
    Inductor,
    VoltageSource,
    CurrentSource,
    FunctionalVoltageSource,
    FunctionalCurrentSource,
    Ground,
}

impl ComponentKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Resistor => "Resistor",
            Self::Capacitor => "Capacitor",
            Self::Inductor => "Inductor",
            Self::VoltageSource => "VSource",
            Self::CurrentSource => "ISource",
            Self::FunctionalVoltageSource => "V_func",
            Self::FunctionalCurrentSource => "I_func",
            Self::Ground => "Ground",
        }
    }

    pub fn default_value(self) -> f64 {
        match self {
            Self::Resistor => 1_000.0,
            Self::Capacitor => 100e-9,
            Self::Inductor => 1e-3,
            Self::VoltageSource => 5.0,
            Self::CurrentSource => 1e-3,
            Self::FunctionalVoltageSource => 5.0,
            Self::FunctionalCurrentSource => 1e-3,
            Self::Ground => 0.0,
        }
    }

    pub fn pin_count(self) -> usize {
        match self {
            Self::Ground => 1,
            _ => 2,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PinRef {
    pub component_id: String,
    pub pin_index: usize,
}

#[derive(Clone, Debug)]
pub struct Wire {
    pub from: PinRef,
    pub to: PinRef,
}

#[derive(Clone, Debug)]
pub struct ComponentInfo {
    pub id: String,
    pub name: String,
    pub kind: ComponentKind,
    pub value: f64,
    pub pos: Position,
    pub rotation: Rotation,
    pub expression: Option<String>,
}

#[derive(Clone, Default)]
pub struct CircuitLayout {
    pub components: Vec<ComponentInfo>,
    pub wires: Vec<Wire>,
}

#[derive(Clone)]
pub struct IvPoint {
    pub v: f64,
    pub i: f64,
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorMode {
    Select,
    Wire,
}

impl Default for EditorMode {
    fn default() -> Self {
        Self::Select
    }
}

pub struct SimState {
    pub dc: Option<NonLinearDcResult>,
    pub transient: Option<Result<TransientResult, String>>,
    pub bode: Vec<(f64, resist::analysis::ac::AcAnalysisResult)>,
    pub iv_sweeps: HashMap<String, Vec<IvPoint>>,
    pub layout: CircuitLayout,
    pub selection: SelectedEntity,
    pub active_tab: PlotTab,
    pub console_output: Vec<ConsoleLine>,
    pub editor_mode: EditorMode,
    pub pending_wire: Option<PinRef>,
    pub dragging_component: Option<String>,
    pub last_component_nodes: HashMap<String, Vec<NodeId>>,
    pub transient_stop: f64,
    pub transient_step: f64,
    pub ac_start: f64,
    pub ac_stop: f64,
    pub ac_points: usize,
}

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
            layout: default_layout(),
            selection: SelectedEntity::None,
            active_tab: PlotTab::Transient,
            console_output: vec![ConsoleLine {
                text: "Canvas mode: drag blocks, connect pins with wires, run simulation buttons."
                    .to_string(),
                is_error: false,
            }],
            editor_mode: EditorMode::Select,
            pending_wire: None,
            dragging_component: None,
            last_component_nodes: HashMap::new(),
            transient_stop: 2e-3,
            transient_step: 1e-6,
            ac_start: 10.0,
            ac_stop: 1e6,
            ac_points: 80,
        }
    }
}

fn default_layout() -> CircuitLayout {
    CircuitLayout {
        components: vec![
            ComponentInfo {
                id: "V1".to_string(),
                name: "V1".to_string(),
                kind: ComponentKind::VoltageSource,
                value: 5.0,
                pos: Position::new(3, 3),
                rotation: Rotation::Deg90,
                expression: None,
            },
            ComponentInfo {
                id: "R1".to_string(),
                name: "R1".to_string(),
                kind: ComponentKind::Resistor,
                value: 1_000.0,
                pos: Position::new(7, 2),
                rotation: Rotation::Deg0,
                expression: None,
            },
            ComponentInfo {
                id: "C1".to_string(),
                name: "C1".to_string(),
                kind: ComponentKind::Capacitor,
                value: 100e-9,
                pos: Position::new(7, 5),
                rotation: Rotation::Deg90,
                expression: None,
            },
            ComponentInfo {
                id: "GND".to_string(),
                name: "GND".to_string(),
                kind: ComponentKind::Ground,
                value: 0.0,
                pos: Position::new(3, 6),
                rotation: Rotation::Deg0,
                expression: None,
            },
        ],
        wires: vec![],
    }
}
