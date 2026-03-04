use num_complex::Complex64;

use crate::components::ac::AcComponent;
use crate::components::Component;
use crate::core::{ComplexMnaMatrix, MnaMatrix, NodeId};
use crate::error::ResistError;

/// An independent current source: current flows **from** `node_a` **to** `node_b`.
///
/// Convention: a positive current *leaves* `node_a` and *enters* `node_b`.
#[derive(Clone)]
pub struct CurrentSource {
    pub name: String,
    pub node_a: NodeId,
    pub node_b: NodeId,
    pub current: f64,
}

impl CurrentSource {
    pub fn new(name: &str, node_a: NodeId, node_b: NodeId, current: f64) -> Self {
        Self {
            name: name.to_string(),
            node_a,
            node_b,
            current,
        }
    }
}

impl Component for CurrentSource {
    fn stamp(&self, mna: &mut MnaMatrix) -> Result<(), ResistError> {
        if let Some(a) = self.node_a.matrix_idx() {
            mna.rhs[a] -= self.current;
        }
        if let Some(b) = self.node_b.matrix_idx() {
            mna.rhs[b] += self.current;
        }
        Ok(())
    }
}

impl AcComponent for CurrentSource {
    fn stamp_ac(&self, mna: &mut ComplexMnaMatrix, _omega: f64) -> Result<(), ResistError> {
        let i = Complex64::new(self.current, 0.0);
        if let Some(a) = self.node_a.matrix_idx() {
            mna.rhs[a] -= i;
        }
        if let Some(b) = self.node_b.matrix_idx() {
            mna.rhs[b] += i;
        }
        Ok(())
    }
}
