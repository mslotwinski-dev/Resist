use std::collections::HashMap;

use num_complex::Complex64;

use crate::core::{Circuit, ComplexMnaMatrix, NodeId};
use crate::error::ResistError;

/// Performs **AC (frequency-domain) analysis** on a circuit at a single
/// frequency.
///
/// Builds the complex-valued MNA system, solves it, and returns phasor
/// voltages and currents.
///
/// # Examples
///
/// ```
/// use resist::{Circuit, NodeId};
///
/// let mut ckt = Circuit::new();
/// let n1 = ckt.add_node();
/// let n2 = ckt.add_node();
///
/// ckt.add_ac_voltage_source("V1", n1, NodeId::GROUND, 1.0, 0.0);
/// ckt.add_resistor("R1", n1, n2, 1_000.0);
/// ckt.add_capacitor("C1", n2, NodeId::GROUND, 1e-6);
///
/// let result = ckt.build_ac(1_000.0).solve().unwrap();
/// let mag_db = result.magnitude_db(n2);
/// println!("Gain at 1 kHz: {:.2} dB", mag_db);
/// ```
pub struct AcAnalyzer<'a> {
    circuit: &'a Circuit,
    freq_hz: f64,
}

/// Results of an AC analysis at a single frequency.
///
/// Node voltages and voltage-source currents are stored as complex phasors.
/// Convenience methods convert to magnitude, dB, and phase representations.
pub struct AcAnalysisResult {
    /// Complex phasor voltage at every circuit node.
    pub node_voltages: HashMap<NodeId, Complex64>,
    /// Complex phasor current through each voltage source.
    pub voltage_source_currents: Vec<Complex64>,
}

impl AcAnalysisResult {
    /// Magnitude of the voltage at a node (in volts).
    pub fn magnitude(&self, node: NodeId) -> f64 {
        self.node_voltages
            .get(&node)
            .map_or(0.0, |v| v.norm())
    }

    /// Magnitude of the voltage at a node expressed in decibels
    /// (relative to 1 V: `20 log₁₀(|V|)`).
    pub fn magnitude_db(&self, node: NodeId) -> f64 {
        20.0 * self.magnitude(node).log10()
    }

    /// Phase of the voltage at a node in **degrees**.
    pub fn phase_deg(&self, node: NodeId) -> f64 {
        self.node_voltages
            .get(&node)
            .map_or(0.0, |v| v.arg().to_degrees())
    }

    /// Phase of the voltage at a node in **radians**.
    pub fn phase_rad(&self, node: NodeId) -> f64 {
        self.node_voltages
            .get(&node)
            .map_or(0.0, |v| v.arg())
    }
}

impl<'a> AcAnalyzer<'a> {
    pub(crate) fn new(circuit: &'a Circuit, freq_hz: f64) -> Self {
        Self { circuit, freq_hz }
    }

    /// Solve the complex MNA system at the configured frequency.
    pub fn solve(&self) -> Result<AcAnalysisResult, ResistError> {
        let n = self.circuit.num_nodes();
        let m = self.circuit.num_voltage_sources();
        let omega = 2.0 * std::f64::consts::PI * self.freq_hz;

        let mut mna = ComplexMnaMatrix::new(n, m);

        // Stamp all AC-capable components (dual-trait and AC-only).
        for comp in &self.circuit.ac_components {
            comp.stamp_ac(&mut mna, omega)?;
        }

        // Solve A x = z  (complex LU)
        let lu = mna.matrix.clone().lu();
        let x = lu.solve(&mna.rhs).ok_or(ResistError::SingularMatrix)?;

        let mut node_voltages = HashMap::new();
        for i in 1..=n {
            node_voltages.insert(NodeId(i), x[i - 1]);
        }

        let mut voltage_source_currents = Vec::with_capacity(m);
        for i in 0..m {
            voltage_source_currents.push(x[n + i]);
        }

        Ok(AcAnalysisResult {
            node_voltages,
            voltage_source_currents,
        })
    }
}
