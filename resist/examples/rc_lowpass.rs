//! # RC Low-Pass Filter — Frequency Response
//!
//! Builds a simple RC low-pass filter and sweeps 10 Hz → 100 kHz,
//! printing the magnitude (dB) and phase (degrees) at each decade.
//!
//! ```text
//!        n_in        R        n_out
//!   V1 ──┤──────/\/\/──────┤──── out
//!        │                  │
//!       GND                 C
//!                           │
//!                          GND
//! ```
//!
//! **Theoretical cutoff:** f_c = 1 / (2πRC) ≈ 159.15 Hz

use resist::{Circuit, NodeId};

fn main() {
    let r = 1_000.0;  // 1 kΩ
    let c = 1e-6;     // 1 µF

    // Build circuit once, reuse for every frequency point.
    let mut ckt = Circuit::new();
    let n_in = ckt.add_node();
    let n_out = ckt.add_node();

    // 1 V AC source at 0° phase
    ckt.add_ac_voltage_source("V1", n_in, NodeId::GROUND, 1.0, 0.0);
    ckt.add_resistor("R1", n_in, n_out, r);
    ckt.add_capacitor("C1", n_out, NodeId::GROUND, c);

    let fc = 1.0 / (2.0 * std::f64::consts::PI * r * c);

    println!("RC Low-Pass Filter  (R = {r} Ω, C = {c} F)");
    println!("Theoretical cutoff frequency: {fc:.2} Hz\n");
    println!("{:<12} {:>12} {:>12}", "Freq (Hz)", "Mag (dB)", "Phase (°)");
    println!("{:-<12} {:->12} {:->12}", "", "", "");

    // Logarithmic sweep: 10 Hz to 100 kHz, 10 points per decade
    let mut f = 10.0_f64;
    while f <= 100_001.0 {
        let result = ckt.build_ac(f).solve().unwrap();
        let db = result.magnitude_db(n_out);
        let phase = result.phase_deg(n_out);
        println!("{f:<12.1} {db:>12.4} {phase:>12.2}");

        f *= 10.0_f64.powf(0.1); // ~10 steps per decade
    }
}
