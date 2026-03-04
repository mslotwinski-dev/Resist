use nalgebra::DVector;
use num_complex::Complex64;

use crate::components::ac::AcComponent;
use crate::components::Component;
use crate::components::transient::TransientComponent;
use crate::core::{ComplexMnaMatrix, MnaMatrix, NodeId};
use crate::error::ResistError;

/// An ideal inductor with inductance `L` (in henrys).
///
/// - **AC analysis:** stamps admittance `Y = 1/(jωL) = −j/(ωL)`.
/// - **DC analysis:** short circuit (0 V voltage source equation).
/// - **Transient analysis (Backward Euler):** equivalent conductance
///   `G = Δt/L` and history current source `I_hist = G · V_prev`.
///
/// # Examples
///
/// ```
/// use resist::{Circuit, NodeId};
///
/// let mut ckt = Circuit::new();
/// let n1 = ckt.add_node();
/// ckt.add_inductor("L1", n1, NodeId::GROUND, 10e-3); // 10 mH
/// ```
#[derive(Clone)]
pub struct Inductor {
    pub name: String,
    pub node_a: NodeId,
    pub node_b: NodeId,
    pub inductance: f64,
    pub equation_idx: usize,
}

impl Inductor {
    pub fn new(name: &str, node_a: NodeId, node_b: NodeId, inductance: f64, equation_idx: usize) -> Self {
        Self {
            name: name.to_string(),
            node_a,
            node_b,
            inductance,
            equation_idx,
        }
    }
}

/// At DC the inductor behaves as a short circuit (0 V voltage source).
impl Component for Inductor {
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
        Ok(())
    }
}

impl AcComponent for Inductor {
    fn stamp_ac(&self, mna: &mut ComplexMnaMatrix, omega: f64) -> Result<(), ResistError> {
        if omega == 0.0 {
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
            return Ok(());
        }

        let y = Complex64::new(0.0, -1.0 / (omega * self.inductance));

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

/// Backward Euler companion model for an inductor:
///
/// ```text
///   G_L = Δt / L
///   I_hist = G_L × V_L(t_prev)  +  I_L(t_prev)
/// ```
///
/// The inductor is replaced by a conductance `G_L` in **parallel** with
/// a current source `I_hist`.  The current through the inductor at the
/// previous time step is stored in the branch-current slot of `x_prev`.
impl TransientComponent for Inductor {
    fn stamp_transient(
        &self,
        mna: &mut MnaMatrix,
        dt: f64,
        x_prev: &DVector<f64>,
    ) -> Result<(), ResistError> {
        let g = dt / self.inductance;

        // Stamp conductance
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

        // Previous voltage across inductor
        let va_prev = self.node_a.matrix_idx().map(|i| x_prev[i]).unwrap_or(0.0);
        let vb_prev = self.node_b.matrix_idx().map(|i| x_prev[i]).unwrap_or(0.0);

        // Previous current through inductor (from the branch current slot)
        let i_prev = x_prev[mna.num_nodes + self.equation_idx];

        // History current: I_hist = G_L × V_L(prev) + I_L(prev)
        let i_hist = g * (va_prev - vb_prev) + i_prev;

        if let Some(a) = self.node_a.matrix_idx() {
            mna.rhs[a] += i_hist;
        }
        if let Some(b) = self.node_b.matrix_idx() {
            mna.rhs[b] -= i_hist;
        }

        Ok(())
    }
}
