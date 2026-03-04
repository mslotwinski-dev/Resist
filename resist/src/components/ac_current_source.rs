use num_complex::Complex64;

use crate::components::ac::AcComponent;
use crate::core::{ComplexMnaMatrix, NodeId};
use crate::error::ResistError;

/// An AC current source defined by a phasor (amplitude and phase).
///
/// Current flows **from** `node_a` **to** `node_b`. The complex current
/// is `I = amplitude · e^{j·phase}`.
pub struct AcCurrentSource {
    pub name: String,
    pub node_a: NodeId,
    pub node_b: NodeId,
    pub amplitude: f64,
    pub phase_deg: f64,
}

impl AcCurrentSource {
    pub fn new(name: &str, node_a: NodeId, node_b: NodeId, amplitude: f64, phase_deg: f64) -> Self {
        Self {
            name: name.to_string(),
            node_a,
            node_b,
            amplitude,
            phase_deg,
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

impl AcComponent for AcCurrentSource {
    fn stamp_ac(&self, mna: &mut ComplexMnaMatrix, _omega: f64) -> Result<(), ResistError> {
        let i = self.phasor();
        if let Some(a) = self.node_a.matrix_idx() {
            mna.rhs[a] -= i;
        }
        if let Some(b) = self.node_b.matrix_idx() {
            mna.rhs[b] += i;
        }
        Ok(())
    }
}
