//! # Interactive Diode Bridge Rectifier
//!
//! Visualizes a full-wave diode bridge with a smoothing capacitor using the
//! `resist_gui` interface. Showcases Non-Linear Transient analysis.

use std::collections::HashMap;

use resist::components::models::DiodeModel;
use resist::components::transient_voltage_source::Waveform;
use resist::{Circuit, NodeId};

use eframe::egui;
use resist_gui::app::ResistApp;
use resist_gui::sim_state::*;

fn main() -> eframe::Result {
    // ── Build Circuit ─────────────────────────────────────────
    let mut ckt = Circuit::new();

    let ac_p = ckt.add_node(); // 1
    let ac_n = ckt.add_node(); // 2
    let dc_p = ckt.add_node(); // 3
    let vac_p = ckt.add_node(); // 4

    // AC Source: 10V amplitude, 50 Hz
    ckt.add_transient_voltage_source(
        "Vac",
        vac_p,
        ac_n, // Floating AC source relative to the bridge!
        Waveform::Sine {
            offset: 0.0,
            amplitude: 10.0,
            freq: 50.0,
            phase_deg: 0.0,
        },
    );

    let diode_model = DiodeModel {
        is: 2.52e-9,
        n: 1.752,
        rs: 0.05,
        ..Default::default()
    };

    // D1: ac_p to dc_p
    ckt.add_diode("D1", ac_p, dc_p, diode_model.clone());
    // D2: GND to ac_p
    ckt.add_diode("D2", NodeId::GROUND, ac_p, diode_model.clone());
    // D3: ac_n to dc_p
    ckt.add_diode("D3", ac_n, dc_p, diode_model.clone());
    // D4: GND to ac_n
    ckt.add_diode("D4", NodeId::GROUND, ac_n, diode_model);

    // Smoothing Capacitor (100 uF) and Load (1k)
    ckt.add_capacitor("C1", dc_p, NodeId::GROUND, 100e-6);
    ckt.add_resistor("Rload", dc_p, NodeId::GROUND, 1000.0);

    // ── Run Simulations ───────────────────────────────────────
    let dc = ckt.build_nonlinear().solve().expect("DC failed");

    // Transient (Calculate 100ms with 50us steps -> 5 full 50Hz cycles)
    let transient = ckt.build_transient(0.1, 50e-6).solve().map_err(|e| e.to_string());

    // ── Build Schematic Layout ─────────────────────────────────
    let mut layout = CircuitLayout::default();

    // Node tooltips
    layout.node_positions.insert(ac_p, Position::new(100.0, 200.0));
    layout.node_positions.insert(vac_p, Position::new(160.0, 200.0));
    layout.node_positions.insert(ac_n, Position::new(300.0, 200.0));
    layout.node_positions.insert(dc_p, Position::new(200.0, 60.0));
    layout.node_positions.insert(NodeId::GROUND, Position::new(200.0, 340.0));

    // Wires (Explicit Manhattan)
    // ac_p net
    layout.wires.push(WireSegment::new(Position::new(140.0, 100.0), Position::new(100.0, 100.0)));
    layout.wires.push(WireSegment::new(Position::new(100.0, 100.0), Position::new(100.0, 300.0)));
    layout.wires.push(WireSegment::new(Position::new(140.0, 300.0), Position::new(100.0, 300.0)));
    layout.wires.push(WireSegment::new(Position::new(100.0, 200.0), Position::new(110.0, 200.0))); // ac_p to Rs

    // vac_p & vac_n internal cross net
    layout.wires.push(WireSegment::new(Position::new(150.0, 200.0), Position::new(210.0, 200.0))); // Rs to Vac
    layout.wires.push(WireSegment::new(Position::new(250.0, 200.0), Position::new(300.0, 200.0))); // Vac to ac_n

    // ac_n net
    layout.wires.push(WireSegment::new(Position::new(260.0, 100.0), Position::new(300.0, 100.0)));
    layout.wires.push(WireSegment::new(Position::new(300.0, 100.0), Position::new(300.0, 300.0)));
    layout.wires.push(WireSegment::new(Position::new(260.0, 300.0), Position::new(300.0, 300.0)));
    layout.wires.push(WireSegment::new(Position::new(220.0, 200.0), Position::new(300.0, 200.0)));

    // dc_p net
    layout.wires.push(WireSegment::new(Position::new(180.0, 100.0), Position::new(220.0, 100.0))); // D1.B to D3.B
    layout.wires.push(WireSegment::new(Position::new(200.0, 100.0), Position::new(200.0, 60.0))); // bridge top to DC line
    layout.wires.push(WireSegment::new(Position::new(200.0, 60.0), Position::new(440.0, 60.0))); // DC line
    layout.wires.push(WireSegment::new(Position::new(360.0, 60.0), Position::new(360.0, 180.0))); // C1 top
    layout.wires.push(WireSegment::new(Position::new(440.0, 60.0), Position::new(440.0, 180.0))); // RL top

    // GND net
    layout.wires.push(WireSegment::new(Position::new(180.0, 300.0), Position::new(220.0, 300.0))); // D2.A to D4.A
    layout.wires.push(WireSegment::new(Position::new(200.0, 300.0), Position::new(200.0, 340.0))); // bridge bot to GND line
    layout.wires.push(WireSegment::new(Position::new(200.0, 340.0), Position::new(440.0, 340.0))); // GND line
    layout.wires.push(WireSegment::new(Position::new(360.0, 340.0), Position::new(360.0, 220.0))); // C1 bot
    layout.wires.push(WireSegment::new(Position::new(440.0, 340.0), Position::new(440.0, 220.0))); // RL bot

    // Junctions
    layout.junctions.push(Position::new(100.0, 200.0)); // ac_p
    layout.junctions.push(Position::new(300.0, 200.0)); // ac_n
    layout.junctions.push(Position::new(200.0, 100.0)); // bridge top
    layout.junctions.push(Position::new(200.0, 300.0)); // bridge bot
    layout.junctions.push(Position::new(360.0, 60.0));
    layout.junctions.push(Position::new(360.0, 340.0));
    layout.junctions.push(Position::new(200.0, 60.0));
    layout.junctions.push(Position::new(200.0, 340.0));

    // Components
    layout.components.push(ComponentInfo {
        id: "Vac".into(),
        name: "Vac 10V 50Hz".into(),
        kind: ComponentKind::TransientSource,
        node_a: vac_p,
        node_b: ac_n,
        pos: Position::new(230.0, 200.0),
        rotation: Rotation::Deg0,
    });

    layout.components.push(ComponentInfo {
        id: "Rs".into(),
        name: "Rs 1 Ohm".into(),
        kind: ComponentKind::Resistor(1.0),
        node_a: vac_p,
        node_b: ac_p,
        pos: Position::new(130.0, 200.0),
        rotation: Rotation::Deg0,
    });

    layout.components.push(ComponentInfo {
        id: "D1".into(),
        name: "D1".into(),
        kind: ComponentKind::Diode,
        node_a: ac_p,
        node_b: dc_p,
        pos: Position::new(160.0, 100.0),
        rotation: Rotation::Deg0,
    });

    layout.components.push(ComponentInfo {
        id: "D2".into(),
        name: "D2".into(),
        kind: ComponentKind::Diode,
        node_a: NodeId::GROUND,
        node_b: ac_p,
        pos: Position::new(160.0, 300.0),
        rotation: Rotation::Deg180,
    });

    layout.components.push(ComponentInfo {
        id: "D3".into(),
        name: "D3".into(),
        kind: ComponentKind::Diode,
        node_a: ac_n,
        node_b: dc_p,
        pos: Position::new(240.0, 100.0),
        rotation: Rotation::Deg180,
    });

    layout.components.push(ComponentInfo {
        id: "D4".into(),
        name: "D4".into(),
        kind: ComponentKind::Diode,
        node_a: NodeId::GROUND,
        node_b: ac_n,
        pos: Position::new(240.0, 300.0),
        rotation: Rotation::Deg0,
    });

    layout.components.push(ComponentInfo {
        id: "C1".into(),
        name: "C1 100uF".into(),
        kind: ComponentKind::Capacitor(100e-6),
        node_a: dc_p,
        node_b: NodeId::GROUND,
        pos: Position::new(360.0, 200.0),
        rotation: Rotation::Deg90,
    });

    layout.components.push(ComponentInfo {
        id: "Rload".into(),
        name: "Rload 1k".into(),
        kind: ComponentKind::Resistor(1000.0),
        node_a: dc_p,
        node_b: NodeId::GROUND,
        pos: Position::new(440.0, 200.0),
        rotation: Rotation::Deg90,
    });

    let sim = SimState {
        dc: Some(dc),
        transient: Some(transient),
        bode: Vec::new(),
        iv_sweeps: HashMap::new(),
        layout,
        selected_nodes: vec![],
        active_tab: PlotTab::Transient,
        selection: SelectedEntity::None,
    };

    // ── Launch GUI ─────────────────────────────────────────────
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 700.0])
            .with_title("Resist: Diode Bridge Rectifier"),
        ..Default::default()
    };

    eframe::run_native(
        "resist_gui",
        options,
        Box::new(|cc| {
            let mut app = ResistApp::new(cc);
            app.sim = sim;
            Ok(Box::new(app))
        }),
    )
}
