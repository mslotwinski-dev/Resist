//! # Interactive BJT Common-Emitter Amplifier
//!
//! Builds a BJT amplifier, runs DC + AC + Transient analysis,
//! then launches the `resist_gui` visualizer with the schematic
//! and waveform data pre-loaded.

use std::collections::HashMap;

use resist::components::models::BjtModel;
use resist::components::transient_voltage_source::Waveform;
use resist::{Circuit, NodeId};

use eframe::egui;
use resist_gui::app::ResistApp;
use resist_gui::sim_state::*;

fn main() -> eframe::Result {
    // ── Build Circuit ─────────────────────────────────────────
    let mut ckt = Circuit::new();

    let vcc = ckt.add_node(); // 1
    let base = ckt.add_node(); // 2
    let coll = ckt.add_node(); // 3
    let emit = ckt.add_node(); // 4
    let in_node = ckt.add_node(); // 5

    ckt.add_voltage_source("VCC", vcc, NodeId::GROUND, 12.0);

    ckt.add_transient_voltage_source(
        "Vin",
        in_node,
        NodeId::GROUND,
        Waveform::Sine {
            offset: 0.0,
            amplitude: 0.01,
            freq: 1_000.0,
            phase_deg: 0.0,
        },
    );

    ckt.add_resistor("R1", vcc, base, 47_000.0);
    ckt.add_resistor("R2", base, NodeId::GROUND, 10_000.0);
    ckt.add_capacitor("Cin", in_node, base, 10e-6);
    ckt.add_resistor("Rc", vcc, coll, 4_700.0);
    ckt.add_resistor("Re", emit, NodeId::GROUND, 1_000.0);
    ckt.add_capacitor("Ce", emit, NodeId::GROUND, 100e-6);

    let mut bjt_model = BjtModel::default();
    bjt_model.bf = 100.0;
    bjt_model.is = 1e-14;
    bjt_model.cje = 5e-12;
    bjt_model.cjc = 2e-12;
    ckt.add_bjt("Q1", coll, base, emit, bjt_model);

    // ── Run Simulations ───────────────────────────────────────
    let dc = ckt.build_nonlinear().solve().expect("DC failed");

    // AC sweep (Bode)
    let mut bode = Vec::new();
    {
        // Need a separate circuit with an AC source for AC sweep
        let mut ac_ckt = Circuit::new();
        let ac_vcc = ac_ckt.add_node();
        let ac_base = ac_ckt.add_node();
        let ac_coll = ac_ckt.add_node();
        let ac_emit = ac_ckt.add_node();
        let ac_in = ac_ckt.add_node();

        ac_ckt.add_voltage_source("VCC", ac_vcc, NodeId::GROUND, 12.0);
        ac_ckt.add_ac_voltage_source("Vin", ac_in, NodeId::GROUND, 0.01, 0.0);
        ac_ckt.add_resistor("R1", ac_vcc, ac_base, 47_000.0);
        ac_ckt.add_resistor("R2", ac_base, NodeId::GROUND, 10_000.0);
        ac_ckt.add_capacitor("Cin", ac_in, ac_base, 10e-6);
        ac_ckt.add_resistor("Rc", ac_vcc, ac_coll, 4_700.0);
        ac_ckt.add_resistor("Re", ac_emit, NodeId::GROUND, 1_000.0);
        ac_ckt.add_capacitor("Ce", ac_emit, NodeId::GROUND, 100e-6);

        let mut f = 10.0_f64;
        while f <= 1e6 {
            if let Ok(result) = ac_ckt.build_ac(f).solve() {
                bode.push((f, result));
            }
            f *= 10.0_f64.powf(0.1);
        }
    }

    // Transient
    let transient = ckt
        .build_transient(3e-3, 1e-6)
        .solve()
        .map_err(|e| e.to_string());

    // DC Sweep (I-V curve of the input)
    let mut iv_sweeps = HashMap::new();
    let sweep_result = ckt
        .build_dc_sweep("VCC", 0.0, 15.0, 0.5)
        .solve()
        .expect("DC Sweep failed");

    let mut iv_pts = Vec::new();
    for (v_src, result) in sweep_result.steps {
        // Collect current assuming Vin is node 5 to GND
        let v_coll = result.node_voltages.get(&coll).copied().unwrap_or(0.0);
        let v_emit = result.node_voltages.get(&emit).copied().unwrap_or(0.0);

        // I_c = (VCC - V_coll) / Rc
        let i_c = (v_src - v_coll) / 4700.0;

        iv_pts.push(IvPoint {
            v: v_coll - v_emit, // V_CE
            i: i_c,             // I_C
        });
    }
    iv_sweeps.insert("Q1 I_C vs V_CE (VCC sweep)".to_string(), iv_pts);

    // ── Build Schematic Layout ─────────────────────────────────
    let mut layout = CircuitLayout::default();

    // Node tooltips
    layout.node_positions.insert(in_node, Position::new(80.0, 200.0));
    layout.node_positions.insert(base, Position::new(260.0, 200.0));
    layout.node_positions.insert(coll, Position::new(360.0, 140.0));
    layout.node_positions.insert(emit, Position::new(360.0, 260.0));
    layout.node_positions.insert(vcc, Position::new(480.0, 40.0));
    layout.node_positions.insert(NodeId::GROUND, Position::new(480.0, 400.0));

    // Wires (Explicit Manhattan)
    // VCC Net (Y = 40.0)
    layout.wires.push(WireSegment::new(Position::new(260.0, 40.0), Position::new(480.0, 40.0)));
    layout.wires.push(WireSegment::new(Position::new(260.0, 80.0), Position::new(260.0, 40.0))); // R1 top
    layout.wires.push(WireSegment::new(Position::new(360.0, 80.0), Position::new(360.0, 40.0))); // Rc top
    layout.wires.push(WireSegment::new(Position::new(480.0, 160.0), Position::new(480.0, 40.0))); // V1 top

    // GND Net (Y = 400.0)
    layout.wires.push(WireSegment::new(Position::new(80.0, 400.0), Position::new(480.0, 400.0)));
    layout.wires.push(WireSegment::new(Position::new(80.0, 320.0), Position::new(80.0, 400.0))); // Vin bot
    layout.wires.push(WireSegment::new(Position::new(260.0, 320.0), Position::new(260.0, 400.0))); // R2 bot
    layout.wires.push(WireSegment::new(Position::new(360.0, 320.0), Position::new(360.0, 400.0))); // Re bot
    layout.wires.push(WireSegment::new(Position::new(420.0, 320.0), Position::new(420.0, 400.0))); // Ce bot
    layout.wires.push(WireSegment::new(Position::new(480.0, 200.0), Position::new(480.0, 400.0))); // V1 bot

    // In Net
    layout.wires.push(WireSegment::new(Position::new(80.0, 280.0), Position::new(80.0, 200.0))); // Vin top
    layout.wires.push(WireSegment::new(Position::new(80.0, 200.0), Position::new(140.0, 200.0))); // Cin left

    // Base Net
    layout.wires.push(WireSegment::new(Position::new(180.0, 200.0), Position::new(320.0, 200.0))); // Cin right to Q1 base
    layout.wires.push(WireSegment::new(Position::new(260.0, 200.0), Position::new(260.0, 120.0))); // R1 bot
    layout.wires.push(WireSegment::new(Position::new(260.0, 200.0), Position::new(260.0, 280.0))); // R2 top

    // Coll Net
    layout.wires.push(WireSegment::new(Position::new(360.0, 120.0), Position::new(360.0, 180.0))); // Rc bot to Q1 coll

    // Emit Net
    layout.wires.push(WireSegment::new(Position::new(360.0, 220.0), Position::new(360.0, 280.0))); // Q1 emit to Re top
    layout.wires.push(WireSegment::new(Position::new(360.0, 260.0), Position::new(420.0, 260.0))); // Emit node to Ce block
    layout.wires.push(WireSegment::new(Position::new(420.0, 260.0), Position::new(420.0, 280.0))); // Ce top

    // Junctions
    layout.junctions.push(Position::new(260.0, 40.0));
    layout.junctions.push(Position::new(360.0, 40.0));
    layout.junctions.push(Position::new(80.0, 400.0));
    layout.junctions.push(Position::new(260.0, 400.0));
    layout.junctions.push(Position::new(360.0, 400.0));
    layout.junctions.push(Position::new(420.0, 400.0));
    layout.junctions.push(Position::new(80.0, 200.0));
    layout.junctions.push(Position::new(260.0, 200.0));
    layout.junctions.push(Position::new(360.0, 260.0));

    // Components
    layout.components.push(ComponentInfo {
        id: "Vin".into(),
        name: "Vin".into(),
        kind: ComponentKind::TransientSource,
        node_a: in_node,
        node_b: NodeId::GROUND,
        pos: Position::new(80.0, 300.0),
        rotation: Rotation::Deg90,
    });
    layout.components.push(ComponentInfo {
        id: "Cin".into(),
        name: "Cin".into(),
        kind: ComponentKind::Capacitor(10e-6),
        node_a: in_node,
        node_b: base,
        pos: Position::new(160.0, 200.0),
        rotation: Rotation::Deg0,
    });
    layout.components.push(ComponentInfo {
        id: "R1".into(),
        name: "R1 47k".into(),
        kind: ComponentKind::Resistor(47_000.0),
        node_a: vcc,
        node_b: base,
        pos: Position::new(260.0, 100.0),
        rotation: Rotation::Deg90,
    });
    layout.components.push(ComponentInfo {
        id: "R2".into(),
        name: "R2 10k".into(),
        kind: ComponentKind::Resistor(10_000.0),
        node_a: base,
        node_b: NodeId::GROUND,
        pos: Position::new(260.0, 300.0),
        rotation: Rotation::Deg90,
    });
    layout.components.push(ComponentInfo {
        id: "Q1".into(),
        name: "Q1".into(),
        kind: ComponentKind::Bjt { is_npn: true },
        node_a: coll,
        node_b: emit,
        pos: Position::new(340.0, 200.0),
        rotation: Rotation::Deg0,
    });
    layout.components.push(ComponentInfo {
        id: "Rc".into(),
        name: "Rc 4.7k".into(),
        kind: ComponentKind::Resistor(4_700.0),
        node_a: vcc,
        node_b: coll,
        pos: Position::new(360.0, 100.0),
        rotation: Rotation::Deg90,
    });
    layout.components.push(ComponentInfo {
        id: "Re".into(),
        name: "Re 1k".into(),
        kind: ComponentKind::Resistor(1_000.0),
        node_a: emit,
        node_b: NodeId::GROUND,
        pos: Position::new(360.0, 300.0),
        rotation: Rotation::Deg90,
    });
    layout.components.push(ComponentInfo {
        id: "Ce".into(),
        name: "Ce 100uF".into(),
        kind: ComponentKind::Capacitor(100e-6),
        node_a: emit,
        node_b: NodeId::GROUND,
        pos: Position::new(420.0, 300.0),
        rotation: Rotation::Deg90,
    });
    layout.components.push(ComponentInfo {
        id: "V1".into(),
        name: "V1 12V".into(),
        kind: ComponentKind::VoltageSource(12.0),
        node_a: vcc,
        node_b: NodeId::GROUND,
        pos: Position::new(480.0, 180.0),
        rotation: Rotation::Deg90,
    });

    // ── Assemble GUI State ─────────────────────────────────────
    let sim = SimState {
        dc: Some(dc),
        transient: Some(transient),
        bode,
        iv_sweeps,
        layout,
        selected_nodes: vec![
            (in_node, "V_in".into()),
            (coll, "V_collector".into()),
            (base, "V_base".into()),
        ],
        active_tab: PlotTab::Transient,
        selection: Default::default(),
    };

    // ── Launch GUI ─────────────────────────────────────────────
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_title("Resist — BJT Common-Emitter Amplifier"),
        ..Default::default()
    };

    eframe::run_native(
        "Resist GUI",
        options,
        Box::new(move |cc| {
            let mut app = ResistApp::new(cc);
            app.sim = sim;
            Ok(Box::new(app))
        }),
    )
}
