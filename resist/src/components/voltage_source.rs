use num_complex::Complex64;

use crate::components::ac::AcComponent;
use crate::components::Component;
use crate::core::{ComplexMnaMatrix, MnaMatrix, NodeId};
use crate::error::ResistError;

/// An independent voltage source that enforces `V(node_a) − V(node_b) = voltage`.
///
/// In the MNA formulation this adds an extra equation row/column, stamping
/// ±1 into the B and C sub-matrices and the voltage value into the RHS.
///
/// For AC analysis the same stamp is applied with the voltage promoted to
/// a real-valued complex number.
#[derive(Clone)]
pub struct VoltageSource {
    pub name: String,
    pub node_a: NodeId,
    pub node_b: NodeId,
    pub voltage: f64,
    pub equation_idx: usize,
}

impl VoltageSource {
    pub fn new(name: &str, node_a: NodeId, node_b: NodeId, voltage: f64, equation_idx: usize) -> Self {
        Self {
            name: name.to_string(),
            node_a,
            node_b,
            voltage,
            equation_idx,
        }
    }
}

impl Component for VoltageSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn set_dc_voltage(&mut self, v: f64) {
        self.voltage = v;
    }

    fn stamp(&self, mna: &mut MnaMatrix) -> Result<(), ResistError> {
        let eq = mna.num_nodes + self.equation_idx;

        if let Some(a) = self.node_a.matrix_idx() {
            mna.matrix[(a, eq)] += 1.0;
            mna.matrix[(eq, a)] += 1.0;
        }
        if let Some(b) = self.node_b.matrix_idx() {
            mna.matrix[(b, eq)] -= 1.0;
            mna.matrix[(eq, b)] -= 1.0;
        }
        mna.rhs[eq] += self.voltage;
        Ok(())
    }
}

impl AcComponent for VoltageSource {
    fn stamp_ac(&self, mna: &mut ComplexMnaMatrix, _omega: f64) -> Result<(), ResistError> {
        let eq = mna.num_nodes + self.equation_idx;
        let one = Complex64::new(1.0, 0.0);

        if let Some(a) = self.node_a.matrix_idx() {
            mna.matrix[(a, eq)] += one;
            mna.matrix[(eq, a)] += one;
        }
        if let Some(b) = self.node_b.matrix_idx() {
            mna.matrix[(b, eq)] -= one;
            mna.matrix[(eq, b)] -= one;
        }
        mna.rhs[eq] += Complex64::new(self.voltage, 0.0);
        Ok(())
    }
}
