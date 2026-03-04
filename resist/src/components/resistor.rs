use num_complex::Complex64;

use crate::components::ac::AcComponent;
use crate::components::Component;
use crate::core::{ComplexMnaMatrix, MnaMatrix, NodeId};
use crate::error::ResistError;

/// An ideal resistor with constant resistance `R` (in ohms).
///
/// In DC analysis the resistor stamps a real conductance `G = 1/R`.
/// In AC analysis the same conductance is stamped as a purely real
/// complex number.
///
/// # Examples
///
/// ```
/// use resist::{Circuit, NodeId};
///
/// let mut ckt = Circuit::new();
/// let n1 = ckt.add_node();
/// ckt.add_voltage_source("V1", n1, NodeId::GROUND, 5.0);
/// ckt.add_resistor("R1", n1, NodeId::GROUND, 100.0);
/// ```
#[derive(Clone)]
pub struct Resistor {
    pub name: String,
    pub node_a: NodeId,
    pub node_b: NodeId,
    pub resistance: f64,
}

impl Resistor {
    pub fn new(name: &str, node_a: NodeId, node_b: NodeId, resistance: f64) -> Self {
        Self {
            name: name.to_string(),
            node_a,
            node_b,
            resistance,
        }
    }
}

impl Component for Resistor {
    fn stamp(&self, mna: &mut MnaMatrix) -> Result<(), ResistError> {
        if self.resistance == 0.0 {
            return Err(ResistError::SolverFailed(format!(
                "Resistor {} has 0 resistance. Use a 0V voltage source instead.",
                self.name
            )));
        }

        let g = 1.0 / self.resistance;

        if let Some(a) = self.node_a.matrix_idx() {
            mna.matrix[(a, a)] += g;
        }
        if let Some(b) = self.node_b.matrix_idx() {
            mna.matrix[(b, b)] += g;
        }
        if let (Some(a), Some(b)) = (self.node_a.matrix_idx(), self.node_b.matrix_idx()) {
            mna.matrix[(a, b)] -= g;
            mna.matrix[(b, a)] -= g;
        }
        Ok(())
    }
}

impl AcComponent for Resistor {
    fn stamp_ac(&self, mna: &mut ComplexMnaMatrix, _omega: f64) -> Result<(), ResistError> {
        if self.resistance == 0.0 {
            return Err(ResistError::SolverFailed(format!(
                "Resistor {} has 0 resistance. Use a 0V voltage source instead.",
                self.name
            )));
        }

        let g = Complex64::new(1.0 / self.resistance, 0.0);

        if let Some(a) = self.node_a.matrix_idx() {
            mna.matrix[(a, a)] += g;
        }
        if let Some(b) = self.node_b.matrix_idx() {
            mna.matrix[(b, b)] += g;
        }
        if let (Some(a), Some(b)) = (self.node_a.matrix_idx(), self.node_b.matrix_idx()) {
            mna.matrix[(a, b)] -= g;
            mna.matrix[(b, a)] -= g;
        }
        Ok(())
    }
}
