//! # Diode Clipper — Transient Analysis
//!
//! A sinusoidal voltage source drives through a resistor into a diode
//! that clips the negative half-cycle. The output shows the classic
//! half-wave rectifier waveform.
//!
//! ```text
//!        n_in     R (1 kΩ)     n_out
//!   V1 ──┤────/\/\/──────┤──── out
//!        │                │
//!       GND              D1 (anode→GND)
//!                         │
//!                        GND
//! ```

use resist::components::transient_voltage_source::Waveform;
use resist::{Circuit, NodeId};

fn main() {
    let mut ckt = Circuit::new();
    let n_in = ckt.add_node();
    let n_out = ckt.add_node();

    // 5 V peak, 1 kHz sine input
    ckt.add_transient_voltage_source(
        "V1",
        n_in,
        NodeId::GROUND,
        Waveform::Sine {
            offset: 0.0,
            amplitude: 5.0,
            freq: 1_000.0,
            phase_deg: 0.0,
        },
    );

    // Series resistor
    ckt.add_resistor("R1", n_in, n_out, 1_000.0);

    // Diode: anode = n_out, cathode = GND (clips negative excursions)
    let mut model = resist::components::models::DiodeModel::default();
    model.is = 1e-12;
    model.n = 1.0;
    ckt.add_diode("D1", n_out, NodeId::GROUND, model);

    let dt = 1e-6;       // 1 µs step
    let t_stop = 2e-3;   // 2 ms  (2 full cycles at 1 kHz)

    let result = ckt.build_transient(t_stop, dt).solve().unwrap();

    println!("Diode Half-Wave Clipper  (V_peak = 5 V, f = 1 kHz, R = 1 kΩ)\n");
    println!("{:<14} {:>12} {:>12}", "Time (s)", "V_in (V)", "V_out (V)");
    println!("{:-<14} {:->12} {:->12}", "", "", "");

    // Print every 50th point for a readable table
    for point in result.time_points.iter().step_by(50) {
        let v_in = point.node_voltages[&n_in];
        let v_out = point.node_voltages[&n_out];
        println!("{:<14.6e} {:>12.4} {:>12.4}", point.time, v_in, v_out);
    }
}
