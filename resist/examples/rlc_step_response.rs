//! # Series RLC Step Response — Transient Analysis
//!
//! A voltage pulse drives a series R-L-C circuit. Depending on the
//! component values the response is underdamped, critically damped,
//! or overdamped.
//!
//! ```text
//!        n1    R (100 Ω)   n2    L (10 mH)    n3
//!   V1 ──┤───/\/\/────┤───⏜⏜⏜────┤
//!        │                                      │
//!       GND                                     C (1 µF)
//!                                               │
//!                                              GND
//! ```
//!
//! With R = 100 Ω, L = 10 mH, C = 1 µF:
//!   ω₀ = 1/√(LC) ≈ 10 000 rad/s,  ζ = R/(2)·√(C/L) ≈ 0.158 → **underdamped**

use resist::components::transient_voltage_source::Waveform;
use resist::{Circuit, NodeId};

fn main() {
    let mut ckt = Circuit::new();
    let n1 = ckt.add_node();
    let n2 = ckt.add_node();
    let n3 = ckt.add_node();

    // 5 V step (stays high for the entire simulation)
    ckt.add_transient_voltage_source(
        "V1",
        n1,
        NodeId::GROUND,
        Waveform::Pulse {
            v1: 0.0,
            v2: 5.0,
            delay: 0.0,
            rise: 1e-9,
            fall: 1e-9,
            width: 1.0,
            period: 2.0,
        },
    );

    ckt.add_resistor("R1", n1, n2, 100.0);
    ckt.add_inductor("L1", n2, n3, 10e-3);     // 10 mH
    ckt.add_capacitor("C1", n3, NodeId::GROUND, 1e-6);  // 1 µF

    let dt = 1e-6;      // 1 µs step
    let t_stop = 5e-3;  // 5 ms

    let result = ckt.build_transient(t_stop, dt).solve().unwrap();

    let r = 100.0_f64;
    let l = 10e-3_f64;
    let c = 1e-6_f64;
    let omega0 = 1.0 / (l * c).sqrt();
    let zeta = (r / 2.0) * (c / l).sqrt();

    println!("Series RLC Step Response");
    println!("  R = {r} Ω,  L = {l} H,  C = {c} F");
    println!("  ω₀ = {omega0:.1} rad/s,  ζ = {zeta:.3}  ({})\n",
        if zeta < 1.0 { "underdamped" } else if zeta > 1.0 { "overdamped" } else { "critically damped" }
    );

    println!("{:<14} {:>12}", "Time (s)", "V_cap (V)");
    println!("{:-<14} {:->12}", "", "");

    // Print every 100th point
    for point in result.time_points.iter().step_by(100) {
        let v_cap = point.node_voltages[&n3];
        println!("{:<14.6e} {:>12.4}", point.time, v_cap);
    }
}
