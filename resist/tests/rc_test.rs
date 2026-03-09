#[cfg(test)]
mod tests {
    use resist::core::{Circuit, NodeId};
    use resist::components::transient_voltage_source::{TransientVoltageSource, Waveform};

    #[test]
    fn test_rc_transient_step() {
        // 1-stage RC filter
        // R = 1k, C = 1uF, V = 5V step
        // At t = 1ms, V_c should be 5 * (1 - e^-1) = 3.16V
        let mut ckt = Circuit::new();
        let in_node = ckt.add_node();
        let out_node = ckt.add_node();

        // 5V DC step starting at t=0
        let eq = 1; // Assuming it's the 1st and only voltage source
        ckt.add_transient_voltage_source(
            "V1",
            in_node,
            NodeId::GROUND,
            Waveform::Pulse {
                v1: 0.0,
                v2: 5.0,
                delay: 0.0,
                rise: 1e-9,
                fall: 1e-9,
                width: 10.0,
                period: 20.0,
            },
        );

        ckt.add_resistor("R1", in_node, out_node, 1000.0);
        ckt.add_capacitor("C1", out_node, NodeId::GROUND, 1e-6);

        // Solve transient up to 2ms
        let analyzer = ckt.build_transient(2e-3, 10e-6);
        let result = analyzer.solve().expect("Transient solve failed");

        // Find the voltage at t = 1ms (approx)
        let exact_time = 1e-3;
        let mut closest_val = 0.0;
        let mut min_diff = f64::MAX;

        for pt in &result.time_points {
            let diff = (pt.time - exact_time).abs();
            if diff < min_diff {
                min_diff = diff;
                closest_val = *pt.node_voltages.get(&out_node).unwrap();
            }
        }

        let expected = 5.0 * (1.0 - (-1.0_f64).exp());
        assert!(
            (closest_val - expected).abs() < 0.05,
            "Expected {}, got {}",
            expected,
            closest_val
        );
    }
}
