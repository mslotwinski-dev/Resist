//! # Interactive Active Low-Pass Filter
//!
//! Visualizes an active low-pass filter (RC + Op-Amp Buffer) using the
//! `resist_gui` interface. Showcases Bode plotting and Transient analysis.

use std::collections::HashMap;

use resist::components::transient_voltage_source::Waveform;
use resist::{Circuit, NodeId};

use eframe::egui;
use resist_gui::app::ResistApp;
use resist_gui::sim_state::*;

fn main() -> eframe::Result {
    // ── Build Circuit ─────────────────────────────────────────
    let mut ckt = Circuit::new();

    let in_node = ckt.add_node(); // 1
    let filter_node = ckt.add_node(); // 2
    let out_node = ckt.add_node(); // 3

    // AC & Transient Source
    ckt.add_transient_voltage_source(
        "Vin",
        in_node,
        NodeId::GROUND,
        Waveform::Sine {
            offset: 0.0,
            amplitude: 1.0,
            freq: 1_000.0,
            phase_deg: 0.0,
        },
    );

    // Filter RC: R=1.59k, C=100nF -> fc ~ 1kHz
    ckt.add_resistor("R1", in_node, filter_node, 1590.0);
    ckt.add_capacitor("C1", filter_node, NodeId::GROUND, 100e-9);

    // Op-Amp Buffer (VCVS with Gain = 100k, IN+ = filter_node, IN- = out_node)
    ckt.add_vcvs("U1 (Op-Amp)", out_node, NodeId::GROUND, filter_node, out_node, 100_000.0);

    // Load Resistor
    ckt.add_resistor("Rload", out_node, NodeId::GROUND, 10_000.0);

    // ── Run Simulations ───────────────────────────────────────
    let dc = ckt.build_nonlinear().solve().expect("DC failed");

    // AC Sweep for Bode (10 Hz to 100 kHz)
    let mut bode = Vec::new();
    let mut ac_ckt = Circuit::new();
    let ac_in = ac_ckt.add_node();
    let ac_filt = ac_ckt.add_node();
    let ac_out = ac_ckt.add_node();
    ac_ckt.add_ac_voltage_source("Vin", ac_in, NodeId::GROUND, 1.0, 0.0);
    ac_ckt.add_resistor("R1", ac_in, ac_filt, 1590.0);
    ac_ckt.add_capacitor("C1", ac_filt, NodeId::GROUND, 100e-9);
    ac_ckt.add_vcvs("U1", ac_out, NodeId::GROUND, ac_filt, ac_out, 100_000.0);
    ac_ckt.add_resistor("Rload", ac_out, NodeId::GROUND, 10_000.0);

    let mut f = 10.0_f64;
    while f <= 1e5 {
        if let Ok(result) = ac_ckt.build_ac(f).solve() {
            bode.push((f, result));
        }
        f *= 10.0_f64.powf(0.1); // 10 pts per decade
    }

    // Transient (Calculate 5ms with 10us steps)
    let transient = ckt.build_transient(5e-3, 10e-6).solve().map_err(|e| e.to_string());

    // ── Build Schematic Layout ─────────────────────────────────
    let mut layout = CircuitLayout::default();

    // Node positions
    layout.node_positions.insert(in_node, Position::new(100.0, 200.0));
    layout.node_positions.insert(filter_node, Position::new(220.0, 200.0));
    layout.node_positions.insert(out_node, Position::new(350.0, 200.0));
    layout.node_positions.insert(NodeId::GROUND, Position::new(220.0, 320.0));

    // Wires
    // In Net
    layout.wires.push(WireSegment::new(Position::new(100.0, 240.0), Position::new(100.0, 200.0))); // Vin top
    layout.wires.push(WireSegment::new(Position::new(100.0, 200.0), Position::new(140.0, 200.0))); // R1 left
    // Filter Net
    layout.wires.push(WireSegment::new(Position::new(180.0, 200.0), Position::new(220.0, 200.0))); // R1 right
    layout.wires.push(WireSegment::new(Position::new(220.0, 200.0), Position::new(256.0, 200.0))); // IN+
    layout.wires.push(WireSegment::new(Position::new(220.0, 200.0), Position::new(220.0, 240.0))); // C1 top
    // Out Net
    layout.wires.push(WireSegment::new(Position::new(308.0, 190.0), Position::new(350.0, 190.0))); // OUT
    layout.wires.push(WireSegment::new(Position::new(350.0, 190.0), Position::new(350.0, 200.0))); // OUT down to out_node
    layout.wires.push(WireSegment::new(Position::new(350.0, 200.0), Position::new(350.0, 240.0))); // Rload top
    // Feedback loop
    layout.wires.push(WireSegment::new(Position::new(320.0, 190.0), Position::new(320.0, 150.0)));
    layout.wires.push(WireSegment::new(Position::new(320.0, 150.0), Position::new(256.0, 150.0)));
    layout.wires.push(WireSegment::new(Position::new(256.0, 150.0), Position::new(256.0, 180.0)));
    // GND Net
    layout.wires.push(WireSegment::new(Position::new(100.0, 280.0), Position::new(100.0, 320.0))); // Vin bot
    layout.wires.push(WireSegment::new(Position::new(220.0, 280.0), Position::new(220.0, 320.0))); // C1 bot
    layout.wires.push(WireSegment::new(Position::new(350.0, 280.0), Position::new(350.0, 320.0))); // Rload bot
    layout.wires.push(WireSegment::new(Position::new(100.0, 320.0), Position::new(350.0, 320.0))); // GND spine

    // Junctions
    layout.junctions.push(Position::new(100.0, 200.0));
    layout.junctions.push(Position::new(220.0, 200.0));
    layout.junctions.push(Position::new(350.0, 200.0));
    layout.junctions.push(Position::new(320.0, 190.0));
    layout.junctions.push(Position::new(100.0, 320.0));
    layout.junctions.push(Position::new(220.0, 320.0));
    layout.junctions.push(Position::new(350.0, 320.0));

    // Components
    layout.components.push(ComponentInfo {
        id: "Vin".into(),
        name: "Vin".into(),
        kind: ComponentKind::TransientSource,
        node_a: in_node,
        node_b: NodeId::GROUND,
        pos: Position::new(100.0, 260.0),
        rotation: Rotation::Deg90,
    });
    layout.components.push(ComponentInfo {
        id: "R1".into(),
        name: "R1".into(),
        kind: ComponentKind::Resistor(1590.0),
        node_a: in_node,
        node_b: filter_node,
        pos: Position::new(160.0, 200.0),
        rotation: Rotation::Deg0,
    });
    layout.components.push(ComponentInfo {
        id: "C1".into(),
        name: "C1".into(),
        kind: ComponentKind::Capacitor(100e-9),
        node_a: filter_node,
        node_b: NodeId::GROUND,
        pos: Position::new(220.0, 260.0),
        rotation: Rotation::Deg90,
    });
    layout.components.push(ComponentInfo {
        id: "U1".into(),
        name: "OpAmp Buffer (VCVS)".into(),
        kind: ComponentKind::OpAmp,
        node_a: out_node,
        node_b: NodeId::GROUND,
        pos: Position::new(280.0, 190.0), 
        rotation: Rotation::Deg0,
    });
    layout.components.push(ComponentInfo {
        id: "Rload".into(),
        name: "Rload".into(),
        kind: ComponentKind::Resistor(10_000.0),
        node_a: out_node,
        node_b: NodeId::GROUND,
        pos: Position::new(350.0, 260.0),
        rotation: Rotation::Deg90,
    });

    let sim = SimState {
        dc: Some(dc),
        transient: Some(transient),
        bode,
        iv_sweeps: HashMap::new(),
        layout,
        selected_nodes: vec![],
        active_tab: PlotTab::Bode, // Start on Bode tab
        selection: SelectedEntity::None,
    };

    // ── Launch GUI ─────────────────────────────────────────────
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 700.0])
            .with_title("Resist: Active Low-Pass Filter"),
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
