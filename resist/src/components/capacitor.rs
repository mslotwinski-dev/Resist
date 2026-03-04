use nalgebra::DVector;
use num_complex::Complex64;

use crate::components::ac::AcComponent;
use crate::components::transient::TransientComponent;
use crate::core::{ComplexMnaMatrix, MnaMatrix, NodeId};
use crate::error::ResistError;

/// An ideal capacitor with capacitance `C` (in farads).
///
/// - **AC analysis:** stamps admittance `Y = jωC`.
/// - **DC analysis:** open circuit (no stamp).
/// - **Transient analysis (Backward Euler):** equivalent conductance
///   `G = C/Δt` and history current source `I_hist = G · V_prev`.
///
/// # Examples
///
/// ```
/// use resist::{Circuit, NodeId};
///
/// let mut ckt = Circuit::new();
/// let n1 = ckt.add_node();
/// ckt.add_capacitor("C1", n1, NodeId::GROUND, 1e-6); // 1 µF
/// ```
pub struct Capacitor {
    pub name: String,
    pub node_a: NodeId,
    pub node_b: NodeId,
    pub capacitance: f64,
}

impl Capacitor {
    pub fn new(name: &str, node_a: NodeId, node_b: NodeId, capacitance: f64) -> Self {
        Self {
            name: name.to_string(),
            node_a,
            node_b,
            capacitance,
        }
    }
}

impl AcComponent for Capacitor {
    fn stamp_ac(&self, mna: &mut ComplexMnaMatrix, omega: f64) -> Result<(), ResistError> {
        let y = Complex64::new(0.0, omega * self.capacitance);

        if let Some(a) = self.node_a.matrix_idx() {
            mna.matrix[(a, a)] += y;
        }
        if let Some(b) = self.node_b.matrix_idx() {
            mna.matrix[(b, b)] += y;
        }
        if let (Some(a), Some(b)) = (self.node_a.matrix_idx(), self.node_b.matrix_idx()) {
            mna.matrix[(a, b)] -= y;
            mna.matrix[(b, a)] -= y;
        }
        Ok(())
    }
}

/// Backward Euler companion model for a capacitor:
///
/// ```text
///   G_C = C / Δt
///   I_hist = G_C × V_C(t_prev)
/// ```
///
/// The capacitor is replaced by a conductance `G_C` in **parallel** with
/// a current source `I_hist` derived from the previous time step voltage.
impl TransientComponent for Capacitor {
    fn stamp_transient(
        &self,
        mna: &mut MnaMatrix,
        dt: f64,
        x_prev: &DVector<f64>,
    ) -> Result<(), ResistError> {
        let g = self.capacitance / dt;

        // Stamp conductance (same as resistor with G = C/dt)
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

        // History current source: I_hist = G_C × V_C(prev)
        let va_prev = self.node_a.matrix_idx().map(|i| x_prev[i]).unwrap_or(0.0);
        let vb_prev = self.node_b.matrix_idx().map(|i| x_prev[i]).unwrap_or(0.0);
        let i_hist = g * (va_prev - vb_prev);

        // Current flows from a to b, so stamp as a current source
        if let Some(a) = self.node_a.matrix_idx() {
            mna.rhs[a] += i_hist;
        }
        if let Some(b) = self.node_b.matrix_idx() {
            mna.rhs[b] -= i_hist;
        }

        Ok(())
    }
}
