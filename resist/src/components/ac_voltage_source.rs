use num_complex::Complex64;

use crate::components::ac::AcComponent;
use crate::core::{ComplexMnaMatrix, NodeId};
use crate::error::ResistError;

/// An AC voltage source defined by a phasor (amplitude and phase).
///
/// The complex voltage is `V = amplitude · e^{j·phase}`.
/// Phase is specified in **degrees** at construction time.
///
/// This component adds an extra equation row/column to the MNA matrix
/// (like an independent voltage source) but stamps a complex RHS value.
#[derive(Clone)]
pub struct AcVoltageSource {
    pub name: String,
    pub node_a: NodeId,
    pub node_b: NodeId,
    pub amplitude: f64,
    pub phase_deg: f64,
    pub equation_idx: usize,
}

impl AcVoltageSource {
    pub fn new(
        name: &str,
        node_a: NodeId,
        node_b: NodeId,
        amplitude: f64,
        phase_deg: f64,
        equation_idx: usize,
    ) -> Self {
        Self {
            name: name.to_string(),
            node_a,
            node_b,
            amplitude,
            phase_deg,
            equation_idx,
        }
    }

    fn phasor(&self) -> Complex64 {
        let phase_rad = self.phase_deg.to_radians();
        Complex64::new(
            self.amplitude * phase_rad.cos(),
            self.amplitude * phase_rad.sin(),
        )
    }
}

impl crate::components::Component for AcVoltageSource {
    fn stamp(&self, mna: &mut crate::core::MnaMatrix) -> Result<(), ResistError> {
        let eq = mna.num_nodes + self.equation_idx;
        
        if let Some(a) = self.node_a.matrix_idx() {
            mna.matrix[(a, eq)] += 1.0;
            mna.matrix[(eq, a)] += 1.0;
        }
        if let Some(b) = self.node_b.matrix_idx() {
            mna.matrix[(b, eq)] -= 1.0;
            mna.matrix[(eq, b)] -= 1.0;
        }
        
        // 0V in DC
        Ok(())
    }
}

impl AcComponent for AcVoltageSource {
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
        mna.rhs[eq] += self.phasor();
        Ok(())
    }
}
