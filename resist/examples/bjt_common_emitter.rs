//! # BJT Common-Emitter Amplifier — AC & Transient Analysis
//!
//! A classic NPN BJT amplifier with voltage divider bias, emitter 
//! degeneration (bypassed by a capacitor), and a collector resistor.
//!
//! Demonstrates the Ebers-Moll BJT model, DC operating point solving 
//! with Gmin stepping, AC small-signal gain, and Transient amplification.

use resist::components::models::BjtModel;
use resist::components::transient_voltage_source::Waveform;
use resist::{Circuit, NodeId};

fn main() {
    let mut ckt = Circuit::new();

    let vcc = ckt.add_node();
    let base = ckt.add_node();
    let coll = ckt.add_node();
    let emit = ckt.add_node();
    let in_ac = ckt.add_node();

    // Power supply: 12 V
    ckt.add_voltage_source("VCC", vcc, NodeId::GROUND, 12.0);

    let in_ac_source = ckt.add_node();
    // AC/Transient Input: 10 mV peak sine at 1 kHz
    // Note: Transient source evaluates to 0.0 at DC (t=0)
    ckt.add_transient_voltage_source(
        "Vin_t",
        in_ac_source,
        NodeId::GROUND,
        Waveform::Sine { offset: 0.0, amplitude: 0.01, freq: 1_000.0, phase_deg: 0.0 }
    );
    // Explicit AC source for frequency domain analysis (in series to avoid parallel voltage sources)
    ckt.add_ac_voltage_source("Vin_ac", in_ac, in_ac_source, 0.01, 0.0);

    // Base bias divider: R1=47k, R2=10k
    ckt.add_resistor("R1", vcc, base, 47_000.0);
    ckt.add_resistor("R2", base, NodeId::GROUND, 10_000.0);

    // Input coupling capacitor
    ckt.add_capacitor("Cin", in_ac, base, 10e-6); // 10 µF

    // Collector resistor: 4.7k
    ckt.add_resistor("Rc", vcc, coll, 4_700.0);

    // Emitter resistor: 1k, bypassed by 100uF
    ckt.add_resistor("Re", emit, NodeId::GROUND, 1_000.0);
    ckt.add_capacitor("Ce", emit, NodeId::GROUND, 100e-6);

    // NPN Transistor
    let mut bjt_model = BjtModel::default();
    bjt_model.is_npn = true;
    bjt_model.bf = 100.0;
    bjt_model.is = 1e-14;
    // Parasitic capacitances to make transient interesting
    bjt_model.cje = 5e-12; // 5 pF
    bjt_model.cjc = 2e-12; // 2 pF

    ckt.add_bjt("Q1", coll, base, emit, bjt_model);

    // 1. DC Operating Point (Non-Linear Newton-Raphson)
    println!("--- DC Operating Point ---");
    let dc = ckt.build_nonlinear().solve().expect("DC failed to converge");
    println!("V_base      = {:.3} V", dc.node_voltages[&base]);
    println!("V_collector = {:.3} V", dc.node_voltages[&coll]);
    println!("V_emitter   = {:.3} V", dc.node_voltages[&emit]);
    println!("NR Iters    = {}\n", dc.iterations);

    // 2. AC Analysis (1 kHz)
    println!("--- AC Analysis (1 kHz) ---");
    let ac = ckt.build_ac(1000.0).solve().unwrap();
    let v_out_ac = ac.magnitude(coll);
    let v_in_ac = ac.magnitude(in_ac);
    let gain = v_out_ac / v_in_ac;
    println!("AC Gain |V_coll / V_in| = {:.1} V/V\n", gain);

    // 3. Transient Analysis
    println!("--- Transient Analysis ---");
    let t_stop = 3e-3; // 3 ms (3 cycles)
    // Initial guess dt=1us. The adaptive stepper will adjust it up/down.
    let transient = ckt.build_transient(t_stop, 1e-6).solve().unwrap();
    
    println!("Total time points: {}", transient.time_points.len());
    println!("{:<14} {:>12} {:>12}", "Time (s)", "V_in (V)", "V_coll (V)");
    println!("{:-<14} {:->12} {:->12}", "", "", "");

    // Print a subset
    for (i, p) in transient.time_points.iter().enumerate() {
        if i % (transient.time_points.len() / 20).max(1) == 0 {
            println!("{:<14.6e} {:>12.4} {:>12.4}", p.time, p.node_voltages[&in_ac], p.node_voltages[&coll]);
        }
    }
}
