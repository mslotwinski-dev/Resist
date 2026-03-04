use std::collections::HashMap;

use crate::components::Component;
use crate::core::{Circuit, MnaMatrix, NodeId};
use crate::error::ResistError;

/// Performs a **DC operating-point** analysis on a circuit.
///
/// Constructs the real-valued MNA system `A x = z`, solves it via LU
/// decomposition, and returns node voltages and voltage-source currents.
pub struct DcAnalyzer<'a> {
    circuit: &'a Circuit,
}

/// Results of a DC operating-point analysis.
pub struct DcAnalysisResult {
    /// Voltage at every circuit node (ground = 0 V is implicit).
    pub node_voltages: HashMap<NodeId, f64>,
    /// Current through each voltage source, indexed by its equation order.
    pub voltage_source_currents: Vec<f64>,
}

impl<'a> DcAnalyzer<'a> {
    pub(crate) fn new(circuit: &'a Circuit) -> Self {
        Self { circuit }
    }

    /// Solve the DC operating point and return node voltages / branch currents.
    pub fn solve(&self) -> Result<DcAnalysisResult, ResistError> {
        let n = self.circuit.num_nodes();
        let m = self.circuit.num_voltage_sources();

        let mut mna = MnaMatrix::new(n, m);

        for comp in &self.circuit.components {
            comp.stamp(&mut mna)?;
        }

        // Transient voltage sources also stamp at t = 0 for DC
        for tvs in &self.circuit.transient_sources {
            tvs.stamp(&mut mna)?;
        }

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

        Ok(DcAnalysisResult {
            node_voltages,
            voltage_source_currents,
        })
    }
}
