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
        let eq = mna.num_nodes + self.equation_idx;
        let one = Complex64::new(1.0, 0.0);

        // V_A - V_B - I_L * jwL = 0
        if let Some(a) = self.node_a.matrix_idx() {
            mna.matrix[(a, eq)] += one;
            mna.matrix[(eq, a)] += one;
        }
        if let Some(b) = self.node_b.matrix_idx() {
            mna.matrix[(b, eq)] -= one;
            mna.matrix[(eq, b)] -= one;
        }

        if omega > 0.0 {
            mna.matrix[(eq, eq)] -= Complex64::new(0.0, omega * self.inductance);
        }
        Ok(())
    }
}

/// Backward Euler companion model for an inductor (Equation Formulation):
///
/// V_a - V_b = L * dI/dt
/// V_a(t) - V_b(t) - (L / dt) * I_L(t) = - (L / dt) * I_L(t - dt)
///
/// The `Component::stamp` method already adds the cross terms representing
/// the voltage drop and branch current `I_L`. We just need to add the
/// `-L/dt` term to the diagonal of the equation row and the history RHS.
impl TransientComponent for Inductor {
    fn stamp_transient(
        &self,
        mna: &mut MnaMatrix,
        dt: f64,
        x_prev: &DVector<f64>,
    ) -> Result<(), ResistError> {
        let eq = mna.num_nodes + self.equation_idx;
        let l_dt = self.inductance / dt;

        mna.matrix[(eq, eq)] -= l_dt;

        let i_prev = x_prev[eq];
        mna.rhs[eq] -= l_dt * i_prev;

        Ok(())
    }
}
