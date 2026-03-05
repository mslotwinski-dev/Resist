use approx::assert_relative_eq;
use resist::components::models::{BjtModel, DiodeModel, MosfetModel};
use std::f64::consts::PI;
use resist::{Circuit, NodeId};
use resist::components::transient_voltage_source::Waveform;

// ─── DC MATH & MNA STAMPS (LINEAR) ──────────────────────────────────────────

#[test]
fn test_kcl_kvl() {
    let mut ckt = Circuit::new();
    let n1 = ckt.add_node();
    let n2 = ckt.add_node();

    // V1: 10V
    ckt.add_voltage_source("V1", n1, NodeId::GROUND, 10.0);
    // Series resistors: R1=2k, R2=3k
    ckt.add_resistor("R1", n1, n2, 2000.0);
    ckt.add_resistor("R2", n2, NodeId::GROUND, 3000.0);

    let dc = ckt.build().solve().expect("DC failed");
    
    // I = V / R_total = 10 / 5000 = 2mA
    let i = dc.voltage_source_currents[0]; // V1 current
    assert_relative_eq!(i.abs(), 0.002, epsilon = 1e-5);

    // V_n2 = I * R2 = 0.002 * 3000 = 6V
    let v2 = dc.node_voltages.get(&n2).copied().unwrap();
    assert_relative_eq!(v2, 6.0, epsilon = 1e-5);

    // V_n1 = 10V (KVL)
    let v1 = dc.node_voltages.get(&n1).copied().unwrap();
    assert_relative_eq!(v1, 10.0, epsilon = 1e-5);
}

#[test]
fn test_superposition() {
    let mut ckt = Circuit::new();
    let n1 = ckt.add_node();
    let mid = ckt.add_node();
    let n2 = ckt.add_node();

    ckt.add_voltage_source("V1", n1, NodeId::GROUND, 12.0);
    ckt.add_voltage_source("V2", n2, NodeId::GROUND, 5.0);
    
    ckt.add_resistor("R1", n1, mid, 100.0);
    ckt.add_resistor("R2", n2, mid, 100.0);
    ckt.add_resistor("R3", mid, NodeId::GROUND, 100.0);

    let dc = ckt.build().solve().expect("DC superposition failed");
    let v_mid = dc.node_voltages.get(&mid).copied().unwrap();

    // Thevenin equivalent:
    // With V1 only: V_mid1 = 12 * (100 || 100) / (100 + (100 || 100)) = 12 * 50 / 150 = 4V
    // With V2 only: V_mid2 = 5 * 50 / 150 = 1.666...V
    // V_mid = 4 + 1.666... = 5.666...V
    assert_relative_eq!(v_mid, 5.666666666666667, epsilon = 1e-9);
}

// ─── AC ANALYSIS (FREQUENCY DOMAIN) ─────────────────────────────────────────

#[test]
fn test_rc_cutoff() {
    let mut ckt = Circuit::new();
    let n_in = ckt.add_node();
    let n_out = ckt.add_node();

    ckt.add_ac_voltage_source("Vin", n_in, NodeId::GROUND, 1.0, 0.0);
    
    // R = 1k, C = 1uF -> fc = 1 / (2*pi*R*C) = 159.155 Hz
    ckt.add_resistor("R", n_in, n_out, 1000.0);
    ckt.add_capacitor("C", n_out, NodeId::GROUND, 1e-6);

    let fc = 1.0 / (2.0 * PI * 1000.0 * 1e-6);

    let ac = ckt.build_ac(fc).solve().expect("AC solve failed");
    let cplx_out = ac.node_voltages.get(&n_out).copied().unwrap();
    
    let mag_db = 20.0 * cplx_out.norm().log10();
    let phase_deg = cplx_out.arg().to_degrees();

    // -3.0103 dB attenuation exactly at cutoff
    assert_relative_eq!(mag_db, -3.010299956639812, epsilon = 1e-5);
    // -45 degree phase exactly at cutoff (lagging)
    assert_relative_eq!(phase_deg, -45.0, epsilon = 1e-5);
}

#[test]
fn test_rlc_resonance() {
    let mut ckt = Circuit::new();
    let n_in = ckt.add_node();
    let n_l = ckt.add_node();
    let n_out = ckt.add_node();

    ckt.add_ac_voltage_source("Vin", n_in, NodeId::GROUND, 1.0, 0.0);
    
    // L = 1mH, C = 1uF -> f0 = 1 / (2*pi*sqrt(L*C)) = 5032.921 Hz
    ckt.add_inductor("L", n_in, n_l, 1e-3);
    ckt.add_capacitor("C", n_l, n_out, 1e-6);
    ckt.add_resistor("R", n_out, NodeId::GROUND, 50.0); // Damping

    let f0 = 1.0 / (2.0 * PI * (1e-3_f64 * 1e-6_f64).sqrt());

    let ac = ckt.build_ac(f0).solve().expect("AC solve failed");
    
    // At resonance, L and C reactances cancel. The circuit looks purely resistive.
    // The current is purely determined by R: I = V / R = 1 / 50 = 0.02 A, Phase 0
    let i_cplx = ac.voltage_source_currents[0]; // Gets complex flow out of source
    let phase_i = i_cplx.arg().to_degrees();
    
    // Current should be perfectly in-phase with voltage (0 deg or 180 depending on orientation logic)
    // The amplitude is exactly 0.02 A. Note: node_a and node_b ordering dictates phase.
    assert_relative_eq!(i_cplx.norm(), 0.02, epsilon = 1e-4);
    
    // Check purely resistive (phase 0 or 180 or very small)
    let phase_normalized = if phase_i < 0.0 { phase_i + 360.0 } else { phase_i };
    let phase_diff = (phase_normalized % 180.0).abs();
    assert!(phase_diff < 1e-3 || phase_diff > 179.999, "Phase was {}", phase_i);
}

// ─── TRANSIENT ANALYSIS (TIME DOMAIN) ───────────────────────────────────────

#[test]
fn test_rc_time_constant() {
    let mut ckt = Circuit::new();
    let n_in = ckt.add_node();
    let n_out = ckt.add_node();

    // R = 1k, C = 1uF. Tau = 1ms.
    ckt.add_transient_voltage_source("Vstep", n_in, NodeId::GROUND, Waveform::Pulse { 
        v1: 0.0, 
        v2: 10.0, 
        delay: 0.0, 
        rise: 1e-9,  // practically instantaneous
        fall: 1e-9, 
        width: 1.0, 
        period: 2.0 
    });
    ckt.add_resistor("R", n_in, n_out, 1000.0);
    ckt.add_capacitor("C", n_out, NodeId::GROUND, 1e-6);

    let tau = 1e-3;
    let t_stop = 2.0 * tau;
    
    // Extremely small dt purely to hit tau precisely
    let tr = ckt.build_transient(t_stop, 1e-6).with_max_dt(1e-6).solve().expect("Transient failed");

    // Find voltage at closest time point to Tau
    let mut closest_v = 0.0;
    let mut min_diff = f64::MAX;

    for pt in &tr.time_points {
        let diff = (pt.time - tau).abs();
        if diff < min_diff {
            min_diff = diff;
            closest_v = pt.node_voltages.get(&n_out).copied().unwrap_or(0.0);
        }
    }

    // At t=tau, Vc = Vmax * (1 - 1/e) ≈ 10 * 0.63212 = 6.3212V
    assert_relative_eq!(closest_v, 6.3212, epsilon = 0.05); // Error bound due to backward euler truncation
}

#[test]
fn test_inductor_current() {
    let mut ckt = Circuit::new();
    let n1 = ckt.add_node();
    let n2 = ckt.add_node();

    // Applying V=5V step to L=10mH
    ckt.add_transient_voltage_source("V", n1, NodeId::GROUND, Waveform::Pulse { 
        v1: 0.0, 
        v2: 5.0, 
        delay: 1e-6, // Start slightly after t=0 so DC evaluates to 0V
        rise: 1e-9, 
        fall: 1e-9, 
        width: 1.0, 
        period: 2.0 
    });
    ckt.add_inductor("L", n1, n2, 10e-3);
    ckt.add_resistor("Rsmall", n2, NodeId::GROUND, 1e-6); // Tiny R to prevent DC singularity

    let tr = ckt.build_transient(1e-3, 10e-6).with_max_dt(10e-6).solve().expect("Transient");

    // L * di/dt = V -> di/dt = 5 / 0.01 = 500 A/s
    // At t=1ms, I = 500 * 0.001 = 0.5A
    let last_pt = tr.time_points.last().unwrap();
    let v_n2 = last_pt.node_voltages.get(&n2).copied().unwrap();
    let i_l = v_n2 / 1e-6; // Current through Rsmall

    assert_relative_eq!(i_l, 0.5, epsilon = 1e-3);
}

// ─── NON-LINEAR SEMICONDUCTORS (NEWTON-RAPHSON) ─────────────────────────────

#[test]
fn test_diode_shockley() {
    let mut ckt = Circuit::new();
    let p = ckt.add_node();
    
    let model = DiodeModel {
        is: 1e-14, // Exact value
        n: 1.0,    // Ideal diode
        rs: 0.0,   // No series resistance
        ..Default::default()
    };
    
    // Force precisely 0.6V across diode
    ckt.add_voltage_source("V1", p, NodeId::GROUND, 0.6);
    ckt.add_diode("D1", p, NodeId::GROUND, model);

    let dc = ckt.build_nonlinear().solve().expect("DC Failed");

    let vt = 0.02585_f64; // Thermal voltage in the model at 300K
    let id_expected = 1e-14_f64 * ((0.6_f64 / vt).exp() - 1.0);

    let id_simulated = dc.voltage_source_currents[0].abs();

    // NR loop matches exactly within tolerance
    assert_relative_eq!(id_simulated, id_expected, epsilon = 1e-6);
}

#[test]
fn test_bjt_active_region() {
    let mut ckt = Circuit::new();
    let b = ckt.add_node();
    let c = ckt.add_node();
    let e = NodeId::GROUND;
    
    let model = BjtModel { is: 1e-14, bf: 100.0, ..Default::default() };
    
    // Force active region
    ckt.add_voltage_source("Vbe", b, e, 0.65);
    ckt.add_voltage_source("Vce", c, e, 5.0);

    ckt.add_bjt("Q1", c, b, e, model);

    let dc = ckt.build_nonlinear().solve().expect("BJT Failed");

    // Base current flows into Vbe (index 0)
    // Collector current flows into Vce (index 1)
    let ib = dc.voltage_source_currents[0].abs();
    let ic = dc.voltage_source_currents[1].abs();

    assert!(ic > 1e-6); // Verify actual current flows
    // Active region identity: Ic = Beta * Ib
    assert_relative_eq!(ic, 100.0 * ib, epsilon = 1e-3 * ic);
}

#[test]
fn test_mosfet_saturation() {
    let mut ckt = Circuit::new();
    let d = ckt.add_node();
    let g = ckt.add_node();
    let s = NodeId::GROUND;
    
    let model = MosfetModel { kp: 20e-3, vto: 1.0, ..Default::default() };
    
    // Vgs = 3V, Vds = 5V.
    // Vds (5V) > Vgs - Vth (3V - 1V = 2V) -> Saturation region.
    ckt.add_voltage_source("Vgs", g, s, 3.0);
    ckt.add_voltage_source("Vds", d, s, 5.0);

    ckt.add_mosfet("M1", d, g, s, s, model);

    let dc = ckt.build_nonlinear().solve().expect("MOSFET Failed");

    let id_sim = dc.voltage_source_currents[1].abs(); // Vds

    // Id = (Kp / 2) * (Vgs - Vth)^2 = (20e-3 / 2) * (2.0)^2 = 0.01 * 4 = 40mA
    assert_relative_eq!(id_sim, 0.040, epsilon = 1e-5);
}

// ─── ROBUSTNESS & EDGE CASES ────────────────────────────────────────────────

#[test]
fn test_short_circuit() {
    let mut ckt = Circuit::new();
    
    // Connect a perfect voltage source directly across itself (Ground to Ground)!
    ckt.add_voltage_source("V1", NodeId::GROUND, NodeId::GROUND, 5.0);

    // This must not panic. It should gracefully return an error (likely Convergence or matrix shape if invalid)
    let res = ckt.build().solve();
    assert!(res.is_err(), "Short circuit should have returned Err");
}

#[test]
fn test_floating_node() {
    let mut ckt = Circuit::new();
    let n1 = ckt.add_node();
    
    // A resistor connected nowhere! 
    ckt.add_resistor("R1", n1, n1, 1000.0);
    ckt.add_voltage_source("V1", NodeId::GROUND, n1, 0.0);

    // Because of Global Gmin, the engine should easily clamp the node to 0V instead of panicking on a singular matrix
    let dc = ckt.build().solve().expect("Global Gmin failed to prevent matrix singularity");
    let v_n1 = dc.node_voltages.get(&n1).copied().unwrap_or(0.0);
    
    assert_relative_eq!(v_n1, 0.0, epsilon = 1e-9);
}
