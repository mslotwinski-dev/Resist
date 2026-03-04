//! # CMOS Inverter — Transient Switching
//!
//! A basic CMOS digital inverter simulated entirely at the transistor level.
//! Demonstrates Shichman-Hodges MOSFET models (`Mosfet`), Gmin stepping
//! for convergence, and Adaptive Time Stepping.

use resist::components::models::MosfetModel;
use resist::components::transient_voltage_source::Waveform;
use resist::{Circuit, NodeId};

fn main() {
    let mut ckt = Circuit::new();

    let vdd = ckt.add_node();
    let gate = ckt.add_node();
    let out = ckt.add_node();

    // VDD = 5 V
    ckt.add_voltage_source("VDD", vdd, NodeId::GROUND, 5.0);

    // Input pulse: 0 to 5V, 10ns rise/fall, 1us width
    ckt.add_transient_voltage_source(
        "Vin",
        gate,
        NodeId::GROUND,
        Waveform::Pulse { 
            v1: 0.0, 
            v2: 5.0, 
            delay: 0.5e-6, 
            rise: 10e-9, 
            fall: 10e-9, 
            width: 1e-6, 
            period: 2e-6 
        }
    );

    // PMOS (Pull-up)
    // Source = VDD, Bulk = VDD
    let mut pmos_mod = MosfetModel::default();
    pmos_mod.is_nmos = false;
    pmos_mod.vto = -1.0;
    pmos_mod.kp = 20e-6; // PMOS typically has lower mobility
    pmos_mod.cgs = 2e-12; // Parasitic capacitance for realistic switching delay
    pmos_mod.cgd = 0.5e-12;
    ckt.add_mosfet("M1_PMOS", out, gate, vdd, vdd, pmos_mod);

    // NMOS (Pull-down)
    // Source = GND, Bulk = GND
    let mut nmos_mod = MosfetModel::default();
    nmos_mod.is_nmos = true;
    nmos_mod.vto = 1.0;
    nmos_mod.kp = 50e-6; // NMOS
    nmos_mod.cgs = 2e-12;
    nmos_mod.cgd = 0.5e-12;
    ckt.add_mosfet("M2_NMOS", out, gate, NodeId::GROUND, NodeId::GROUND, nmos_mod);

    // Load capacitor to observe R-C charging curve during switching
    ckt.add_capacitor("CLoad", out, NodeId::GROUND, 10e-12); // 10 pF

    let t_stop = 3e-6; // 3 us
    // Start with 1ns dt. Adaptive stepper will expand it when flat, shrink on edges.
    let transient = ckt.build_transient(t_stop, 1e-9).solve().unwrap();

    println!("CMOS Inverter Switching (Adaptive Time Stepping)\n");
    println!("Total adaptive time points computed: {}", transient.time_points.len());
    println!("{:<14} {:>10} {:>10}", "Time (s)", "V_in (V)", "V_out (V)");
    println!("{:-<14} {:->10} {:->10}", "", "", "");

    // Print subset of points
    for (i, p) in transient.time_points.iter().enumerate() {
        if i % (transient.time_points.len() / 30).max(1) == 0 {
            println!("{:<14.6e} {:>10.3} {:>10.3}", p.time, p.node_voltages[&gate], p.node_voltages[&out]);
        }
    }
}
