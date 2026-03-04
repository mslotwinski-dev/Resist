pub mod analysis;
pub mod components;
pub mod core;
pub mod error;

pub use crate::core::{Circuit, NodeId};
pub use analysis::ac::{AcAnalysisResult, AcAnalyzer};
pub use analysis::dc::{DcAnalysisResult, DcAnalyzer};
pub use analysis::nonlinear::{NonLinearAnalyzer, NonLinearDcResult};
pub use analysis::transient::{TimePoint, TransientAnalyzer, TransientResult};
pub use error::ResistError;

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // DC tests (must not regress)
    // -----------------------------------------------------------------------

    #[test]
    fn test_resistor_divider() {
        let mut ckt = Circuit::new();
        let n1 = ckt.add_node();
        let n2 = ckt.add_node();

        ckt.add_voltage_source("V1", n1, NodeId::GROUND, 10.0);
        ckt.add_resistor("R1", n1, n2, 10.0);
        ckt.add_resistor("R2", n2, NodeId::GROUND, 10.0);

        let result = ckt.build().solve().unwrap();

        assert!((result.node_voltages[&n1] - 10.0).abs() < 1e-6);
        assert!((result.node_voltages[&n2] - 5.0).abs() < 1e-6);
        assert!((result.voltage_source_currents[0] - (-0.5)).abs() < 1e-6);
    }

    #[test]
    fn test_current_source_and_resistor() {
        let mut ckt = Circuit::new();
        let n1 = ckt.add_node();

        ckt.add_current_source("I1", NodeId::GROUND, n1, 2.0);
        ckt.add_resistor("R1", n1, NodeId::GROUND, 5.0);

        let result = ckt.build().solve().unwrap();

        assert!((result.node_voltages[&n1] - 10.0).abs() < 1e-6);
    }

    // -----------------------------------------------------------------------
    // AC tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_rc_lowpass_at_cutoff() {
        let mut ckt = Circuit::new();
        let n_in = ckt.add_node();
        let n_out = ckt.add_node();

        ckt.add_ac_voltage_source("V1", n_in, NodeId::GROUND, 1.0, 0.0);
        ckt.add_resistor("R1", n_in, n_out, 1_000.0);
        ckt.add_capacitor("C1", n_out, NodeId::GROUND, 1e-6);

        let r = 1_000.0_f64;
        let c = 1e-6_f64;
        let fc = 1.0 / (2.0 * std::f64::consts::PI * r * c);

        let result = ckt.build_ac(fc).solve().unwrap();
        let mag = result.magnitude(n_out);

        assert!(
            (mag - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-4,
            "Expected ~0.7071, got {mag}"
        );

        let db = result.magnitude_db(n_out);
        assert!(
            (db - (-3.0103)).abs() < 0.05,
            "Expected ~−3 dB, got {db}"
        );
    }

    #[test]
    fn test_vcvs_inverting_amplifier() {
        let mut ckt = Circuit::new();
        let n_in = ckt.add_node();
        let n_inv = ckt.add_node();
        let n_out = ckt.add_node();

        ckt.add_ac_voltage_source("V1", n_in, NodeId::GROUND, 1.0, 0.0);
        ckt.add_resistor("Rin", n_in, n_inv, 1_000.0);
        ckt.add_resistor("Rf", n_inv, n_out, 10_000.0);
        ckt.add_vcvs("E1", n_out, NodeId::GROUND, NodeId::GROUND, n_inv, 1e6);

        let result = ckt.build_ac(1000.0).solve().unwrap();
        let v_out = result.node_voltages[&n_out];

        assert!(
            (v_out.re - (-10.0)).abs() < 0.01,
            "Expected Vout ≈ −10, got {v_out}"
        );
        assert!(
            v_out.im.abs() < 0.01,
            "Expected purely real output, got {v_out}"
        );
    }

    // -----------------------------------------------------------------------
    // Non-linear DC tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_diode_forward_bias() {
        let mut ckt = Circuit::new();
        let n1 = ckt.add_node();
        let n2 = ckt.add_node();

        ckt.add_voltage_source("V1", n1, NodeId::GROUND, 5.0);
        ckt.add_resistor("R1", n1, n2, 1_000.0);
        
        let mut model = crate::components::models::DiodeModel::default();
        model.is = 1e-12;
        model.n = 1.0;
        ckt.add_diode("D1", n2, NodeId::GROUND, model);

        let result = ckt.build_nonlinear().solve().unwrap();
        let vd = result.node_voltages[&n2];

        assert!(
            vd > 0.55 && vd < 0.75,
            "Expected diode Vf ≈ 0.6–0.7 V, got {vd:.4} V"
        );
    }

    // -----------------------------------------------------------------------
    // Transient tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_rc_transient_step_response() {
        // RC circuit with step input: V(out) = V_in × (1 − e^{−t/(RC)})
        // R = 1 kΩ, C = 1 µF → τ = 1 ms
        use crate::components::transient_voltage_source::Waveform;

        let mut ckt = Circuit::new();
        let n_in = ckt.add_node();
        let n_out = ckt.add_node();

        ckt.add_transient_voltage_source(
            "V1",
            n_in,
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
        ckt.add_resistor("R1", n_in, n_out, 1_000.0);
        ckt.add_capacitor("C1", n_out, NodeId::GROUND, 1e-6);

        let tau = 1_000.0 * 1e-6; // 1 ms
        let dt = 1e-5; // 10 µs
        let t_stop = 5e-3; // 5 ms (5τ)

        let result = ckt.build_transient(t_stop, dt).solve().unwrap();

        // Check at t ≈ τ: V should be ≈ 5 × (1 − e^{−1}) ≈ 3.16 V
        let at_tau = result
            .time_points
            .iter()
            .min_by(|a, b| (a.time - tau).abs().partial_cmp(&(b.time - tau).abs()).unwrap())
            .expect("Should have points");

        let v_out = at_tau.node_voltages[&n_out];
        let expected = 5.0 * (1.0 - (-1.0_f64).exp()); // 3.1606

        assert!(
            (v_out - expected).abs() < 0.15,
            "At t ≈ τ, expected ~{expected:.2} V, got {v_out:.4} V"
        );
    }
}
