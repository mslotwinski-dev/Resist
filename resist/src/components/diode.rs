use crate::components::models::DiodeModel;
use crate::components::nonlinear::{NonLinearComponent, NonLinearState};
use crate::core::{MnaMatrix, NodeId};
use crate::error::ResistError;

/// An ideal or realistic diode modelled by the Shockley equation with
/// optional parasitic elements (series resistance, junction capacitance).
///
/// Implements `NonLinearComponent` to stamp its linearized equivalent
/// conductance `G_eq = dI/dV_D` and current source `I_eq` into the MNA matrix
/// during Newton-Raphson iterations.
///
/// Convention: anode = `node_a`, cathode = `node_b`. Current flows
/// from anode to cathode when forward-biased.
pub struct Diode {
    pub name: String,
    pub anode: NodeId,
    pub cathode: NodeId,
    pub model: DiodeModel,
}

/// Thermal voltage at ~300 K (room temperature).
const VT: f64 = 0.02585;

/// Maximum exponent to avoid overflow in f64 (e^709 ≈ 8.2e307).
const MAX_EXP_ARG: f64 = 500.0;

impl Diode {
    pub fn new(name: &str, anode: NodeId, cathode: NodeId, model: DiodeModel) -> Self {
        Self {
            name: name.to_string(),
            anode,
            cathode,
            model,
        }
    }

    /// Compute diode DC current and DC conductance at a given junction voltage `v_j`.
    fn evaluate_dc(&self, v_j: f64) -> (f64, f64) {
        let n_vt = self.model.n * VT;
        let vmax = 0.8; // Voltage to start linear extrapolation

        if v_j > vmax {
            let exp_val = (vmax / n_vt).exp();
            let i_d_max = self.model.is * (exp_val - 1.0);
            let g_eq = self.model.is / n_vt * exp_val;

            let i_d = i_d_max + g_eq * (v_j - vmax);
            (i_d, g_eq)
        } else {
            let exp_val = (v_j / n_vt).max(-200.0).exp();
            let i_d = self.model.is * (exp_val - 1.0);
            let g_eq = self.model.is / n_vt * exp_val;
            (i_d, g_eq)
        }
    }

    /// Compute junction charge and its derivative (capacitance) at `v_j`.
    /// Simplified model: depletion capacitance `Cj(v) = Cj0 / (1 - v/Vj)^m`
    /// and diffusion capacitance `Cd = Tt * g_eq`.
    fn evaluate_charge(&self, v_j: f64, g_eq: f64) -> (f64, f64) {
        // Diffusion capacitance part
        let q_diff = self.model.tt * self.model.is * ((v_j / (self.model.n * VT)).min(MAX_EXP_ARG).exp() - 1.0);
        let c_diff = self.model.tt * g_eq;

        // Depletion capacitance part
        let mut c_dep = self.model.cj0;
        let mut q_dep = self.model.cj0 * v_j;

        let fc = 0.5; // forward-bias depletion capacitance limit
        let fcvj = fc * self.model.vj;

        if self.model.cj0 > 0.0 {
            if v_j < fcvj {
                let factor = (1.0 - v_j / self.model.vj).powf(-self.model.m);
                c_dep = self.model.cj0 * factor;
                q_dep = self.model.cj0 * self.model.vj / (1.0 - self.model.m) * (1.0 - (1.0 - v_j / self.model.vj).powf(1.0 - self.model.m));
            } else {
                let f1 = (1.0 - fc).powf(-(1.0 + self.model.m));
                let f2 = (1.0 - fc).powf(-self.model.m);
                let f3 = 1.0 - fc * (1.0 + self.model.m);
                c_dep = self.model.cj0 * f2 * (1.0 + self.model.m * (v_j - fcvj) / self.model.vj);
                q_dep = self.model.cj0 * self.model.vj * (1.0 - f2) / (1.0 - self.model.m) +
                        self.model.cj0 * f1 * (f3 * (v_j - fcvj) + 0.5 * self.model.m / self.model.vj * (v_j.powi(2) - fcvj.powi(2)));
            }
        }

        (q_diff + q_dep, c_diff + c_dep)
    }
}

impl NonLinearComponent for Diode {
    fn stamp_nonlinear(
        &self,
        mna: &mut MnaMatrix,
        state: &NonLinearState,
    ) -> Result<(), ResistError> {
        let va = self.anode.matrix_idx().map(|i| state.x[i]).unwrap_or(0.0);
        let vc = self.cathode.matrix_idx().map(|i| state.x[i]).unwrap_or(0.0);
        let v_j = va - vc;

        let (mut i_eq_total, mut g_eq_total) = self.evaluate_dc(v_j);

        // Transient evaluation (Backward Euler companion model for charge)
        if let (Some(dt), Some(x_prev)) = (state.dt, state.x_prev) {
            let va_prev = self.anode.matrix_idx().map(|i| x_prev[i]).unwrap_or(0.0);
            let vc_prev = self.cathode.matrix_idx().map(|i| x_prev[i]).unwrap_or(0.0);
            let v_j_prev = va_prev - vc_prev;

            // DC g_eq is needed for diffusion capacitance calculation
            let (_, g_eq_prev) = self.evaluate_dc(v_j_prev);
            
            let (q_now, c_eq) = self.evaluate_charge(v_j, g_eq_total);
            let (q_prev, _) = self.evaluate_charge(v_j_prev, g_eq_prev);

            // Backward Euler: I_cap = C_eq*V_j + (Q_now - Q_prev - C_eq*V_j) / dt
            let g_cap = c_eq / dt;
            let i_cap_hist = (q_now - q_prev) / dt - g_cap * v_j;

            // Add transient contributions
            g_eq_total += g_cap;
            
            // Note: i_eq_total is the equivalent current source. For DC: I_eq = I_D - G_eq_DC * V_j
            // For transient, we add the history current.
            // Full current = I_DC + I_cap = (G_dc*Vj + I_eq_dc) + (G_cap*Vj + I_cap_hist)
            // Full I_eq = I_eq_dc + I_cap_hist
            let i_eq_dc = i_eq_total - (self.evaluate_dc(v_j).1) * v_j;
            i_eq_total = i_eq_dc + i_cap_hist;
        } else {
            // DC only: I_eq = I_D - G_eq * V_j
            i_eq_total = i_eq_total - g_eq_total * v_j;
        }

        // Add Gmin for convergence
        g_eq_total += state.gmin;

        // Stamp conductance G_eq
        if let Some(a) = self.anode.matrix_idx() {
            mna.matrix[(a, a)] += g_eq_total;
        }
        if let Some(c) = self.cathode.matrix_idx() {
            mna.matrix[(c, c)] += g_eq_total;
        }
        if let (Some(a), Some(c)) = (self.anode.matrix_idx(), self.cathode.matrix_idx()) {
            mna.matrix[(a, c)] -= g_eq_total;
            mna.matrix[(c, a)] -= g_eq_total;
        }

        // Stamp equivalent current source I_eq (current leaves a, enters c)
        if let Some(a) = self.anode.matrix_idx() {
            mna.rhs[a] -= i_eq_total;
        }
        if let Some(c) = self.cathode.matrix_idx() {
            mna.rhs[c] += i_eq_total;
        }

        Ok(())
    }
}
