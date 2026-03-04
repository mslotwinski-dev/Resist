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
        .expect("Transient failed");

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

    // Node positions on grid for tooltips
    layout
        .node_positions
        .insert(in_node, GridPoint::new(10, 20));
    layout.node_positions.insert(base, GridPoint::new(24, 20));
    layout.node_positions.insert(coll, GridPoint::new(34, 16));
    layout.node_positions.insert(emit, GridPoint::new(34, 22));
    layout.node_positions.insert(vcc, GridPoint::new(50, 10));
    layout
        .node_positions
        .insert(NodeId::GROUND, GridPoint::new(30, 32));

    // Wires (Orthogonal / Manhattan Routing)
    // Input net
    layout.wires.push(WireSegment::new(
        GridPoint::new(10, 23),
        GridPoint::new(10, 20),
    ));
    layout.wires.push(WireSegment::new(
        GridPoint::new(10, 20),
        GridPoint::new(14, 20),
    ));
    // Base net
    layout.wires.push(WireSegment::new(
        GridPoint::new(18, 20),
        GridPoint::new(24, 20),
    ));
    layout.wires.push(WireSegment::new(
        GridPoint::new(24, 20),
        GridPoint::new(24, 17),
    ));
    layout.wires.push(WireSegment::new(
        GridPoint::new(24, 20),
        GridPoint::new(24, 23),
    ));
    layout.wires.push(WireSegment::new(
        GridPoint::new(24, 20),
        GridPoint::new(30, 20),
    ));
    // Collector net
    layout.wires.push(WireSegment::new(
        GridPoint::new(34, 18),
        GridPoint::new(34, 16),
    ));
    // Emitter net
    layout.wires.push(WireSegment::new(
        GridPoint::new(34, 22),
        GridPoint::new(34, 24),
    ));
    layout.wires.push(WireSegment::new(
        GridPoint::new(34, 22),
        GridPoint::new(40, 22),
    ));
    layout.wires.push(WireSegment::new(
        GridPoint::new(40, 22),
        GridPoint::new(40, 24),
    ));
    // VCC net
    layout.wires.push(WireSegment::new(
        GridPoint::new(24, 13),
        GridPoint::new(24, 10),
    ));
    layout.wires.push(WireSegment::new(
        GridPoint::new(34, 12),
        GridPoint::new(34, 10),
    ));
    layout.wires.push(WireSegment::new(
        GridPoint::new(24, 10),
        GridPoint::new(50, 10),
    ));
    layout.wires.push(WireSegment::new(
        GridPoint::new(50, 10),
        GridPoint::new(50, 18),
    ));
    // GND net
    layout.wires.push(WireSegment::new(
        GridPoint::new(10, 27),
        GridPoint::new(10, 32),
    ));
    layout.wires.push(WireSegment::new(
        GridPoint::new(24, 27),
        GridPoint::new(24, 32),
    ));
    layout.wires.push(WireSegment::new(
        GridPoint::new(34, 28),
        GridPoint::new(34, 32),
    ));
    layout.wires.push(WireSegment::new(
        GridPoint::new(40, 28),
        GridPoint::new(40, 32),
    ));
    layout.wires.push(WireSegment::new(
        GridPoint::new(50, 22),
        GridPoint::new(50, 32),
    ));
    layout.wires.push(WireSegment::new(
        GridPoint::new(10, 32),
        GridPoint::new(50, 32),
    ));

    // Components
    layout.components.push(ComponentInfo {
        id: "Vin".into(),
        name: "Vin".into(),
        kind: ComponentKind::TransientSource,
        node_a: in_node,
        node_b: NodeId::GROUND,
        pos: GridPoint::new(10, 25),
        rotation: Rotation::Deg90,
    });
    layout.components.push(ComponentInfo {
        id: "Cin".into(),
        name: "Cin".into(),
        kind: ComponentKind::Capacitor(10e-6),
        node_a: in_node,
        node_b: base,
        pos: GridPoint::new(16, 20),
        rotation: Rotation::Deg0,
    });
    layout.components.push(ComponentInfo {
        id: "R1".into(),
        name: "R1 47k".into(),
        kind: ComponentKind::Resistor(47_000.0),
        node_a: vcc,
        node_b: base,
        pos: GridPoint::new(24, 15),
        rotation: Rotation::Deg90,
    });
    layout.components.push(ComponentInfo {
        id: "R2".into(),
        name: "R2 10k".into(),
        kind: ComponentKind::Resistor(10_000.0),
        node_a: base,
        node_b: NodeId::GROUND,
        pos: GridPoint::new(24, 25),
        rotation: Rotation::Deg90,
    });
    layout.components.push(ComponentInfo {
        id: "Q1".into(),
        name: "Q1".into(),
        kind: ComponentKind::Bjt { is_npn: true },
        node_a: coll,
        node_b: emit,
        pos: GridPoint::new(32, 20),
        rotation: Rotation::Deg0,
    });
    layout.components.push(ComponentInfo {
        id: "Rc".into(),
        name: "Rc 4.7k".into(),
        kind: ComponentKind::Resistor(4_700.0),
        node_a: vcc,
        node_b: coll,
        pos: GridPoint::new(34, 14),
        rotation: Rotation::Deg90,
    });
    layout.components.push(ComponentInfo {
        id: "Re".into(),
        name: "Re 1k".into(),
        kind: ComponentKind::Resistor(1_000.0),
        node_a: emit,
        node_b: NodeId::GROUND,
        pos: GridPoint::new(34, 26),
        rotation: Rotation::Deg90,
    });
    layout.components.push(ComponentInfo {
        id: "Ce".into(),
        name: "Ce 100uF".into(),
        kind: ComponentKind::Capacitor(100e-6),
        node_a: emit,
        node_b: NodeId::GROUND,
        pos: GridPoint::new(40, 26),
        rotation: Rotation::Deg90,
    });
    layout.components.push(ComponentInfo {
        id: "V1".into(),
        name: "V1 12V".into(),
        kind: ComponentKind::VoltageSource(12.0),
        node_a: vcc,
        node_b: NodeId::GROUND,
        pos: GridPoint::new(50, 20),
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
