//! # Inverting Operational Amplifier
//!
//! Models an ideal inverting op-amp using a Voltage-Controlled Voltage
//! Source (VCVS) with a very large open-loop gain (~1 × 10⁶).
//!
//! ```text
//!             Rin (1 kΩ)      Rf (10 kΩ)
//!   Vin ────/\/\/────┬────/\/\/────┬──── Vout
//!                    │              │
//!                   (−) op-amp     (out)
//!                   (+) → GND
//! ```
//!
//! **Expected closed-loop gain:** −R_f / R_in = −10 kΩ / 1 kΩ = **−10**

use resist::{Circuit, NodeId};

fn main() {
    let mut ckt = Circuit::new();
    let n_in = ckt.add_node();   // signal input
    let n_inv = ckt.add_node();  // inverting input (virtual ground)
    let n_out = ckt.add_node();  // amplifier output

    // 1 V AC source, 0° phase
    ckt.add_ac_voltage_source("Vin", n_in, NodeId::GROUND, 1.0, 0.0);

    // Input resistor
    ckt.add_resistor("Rin", n_in, n_inv, 1_000.0);

    // Feedback resistor
    ckt.add_resistor("Rf", n_inv, n_out, 10_000.0);

    // Ideal op-amp modelled as VCVS:
    //   V(out) − V(GND) = A × (V(+) − V(−))
    //   Non-inverting input (+) is tied to GND.
    //   So: V(out) = A × (0 − V(inv)) = −A × V(inv)
    ckt.add_vcvs("OpAmp", n_out, NodeId::GROUND, NodeId::GROUND, n_inv, 1e6);

    let freq = 1_000.0; // 1 kHz
    let result = ckt.build_ac(freq).solve().unwrap();

    let v_out = result.node_voltages[&n_out];
    let gain = v_out.re; // should be ≈ −10

    println!("Inverting Op-Amp Amplifier");
    println!("  Rin = 1 kΩ,  Rf = 10 kΩ,  A_OL = 1e6\n");
    println!("  Frequency:          {freq} Hz");
    println!("  V_out (complex):    {v_out}");
    println!("  Closed-loop gain:   {gain:.6}");
    println!("  Expected gain:      -10.0");
    println!("  Magnitude (dB):     {:.4} dB", result.magnitude_db(n_out));
    println!("  Phase:              {:.2}°", result.phase_deg(n_out));
}
