use crate::components::models::BjtModel;
use crate::components::nonlinear::{NonLinearComponent, NonLinearState};
use crate::core::{MnaMatrix, NodeId};
use crate::error::ResistError;

/// Bipolar Junction Transistor (NPN or PNP) modelled using Ebers-Moll.
pub struct Bjt {
    pub name: String,
    pub collector: NodeId,
    pub base: NodeId,
    pub emitter: NodeId,
    pub model: BjtModel,
}

const VT: f64 = 0.02585;

impl Bjt {
    pub fn new(
        name: &str,
        collector: NodeId,
        base: NodeId,
        emitter: NodeId,
        model: BjtModel,
    ) -> Self {
        Self {
            name: name.to_string(),
            collector,
            base,
            emitter,
            model,
        }
    }

    /// Evaluates Ebers-Moll currents and conductances based on Vbe and Vbc.
    fn evaluate_dc(&self, v_be: f64, v_bc: f64) -> (f64, f64, f64, f64, f64) {
        let polarity = if self.model.is_npn { 1.0 } else { -1.0 };
        let v_be_eff = polarity * v_be;
        let v_bc_eff = polarity * v_bc;

        let vmax = 0.8;

        // Transport model and Base currents with linear extrapolation
        let (i_tbe, g_tbe) = Self::diode_eval(v_be_eff, self.model.is, VT, vmax);
        let (i_tbc, g_tbc) = Self::diode_eval(v_bc_eff, self.model.is, VT, vmax);

        // Base currents
        let i_be = i_tbe / self.model.bf;
        let i_bc = i_tbc / self.model.br;

        // Early effect factor (simplified)
        let qb = if self.model.va > 0.0 {
            1.0 + (polarity * (v_bc - v_be).max(0.0)) / self.model.va
        } else {
            1.0
        };

        let i_c_tot = polarity * ((i_tbe - i_tbc) / qb - i_bc);
        let i_b_tot = polarity * (i_be + i_bc);

        // Conductances
        let g_pi = polarity * g_tbe / self.model.bf;
        let g_mu = polarity * g_tbc / self.model.br;

        let g_m_f = polarity * g_tbe / qb;
        let g_m_r = polarity * g_tbc / qb;

        (i_c_tot, i_b_tot, g_pi, g_mu, g_m_f - g_m_r)
    }

    fn diode_eval(v: f64, is: f64, vt: f64, vmax: f64) -> (f64, f64) {
        if v > vmax {
            let exp_val = (vmax / vt).exp();
            let i_max = is * (exp_val - 1.0);
            let g_eq = is / vt * exp_val;
            let i = i_max + g_eq * (v - vmax);
            (i, g_eq)
        } else {
            let exp_val = (v / vt).max(-200.0).exp();
            let i = is * (exp_val - 1.0);
            let g_eq = is / vt * exp_val;
            (i, g_eq)
        }
    }
}

impl NonLinearComponent for Bjt {
    fn stamp_nonlinear(
        &self,
        mna: &mut MnaMatrix,
        state: &NonLinearState,
    ) -> Result<(), ResistError> {
        let c = self.collector.matrix_idx();
        let b = self.base.matrix_idx();
        let e = self.emitter.matrix_idx();

        let vc = c.map(|i| state.x[i]).unwrap_or(0.0);
        let vb = b.map(|i| state.x[i]).unwrap_or(0.0);
        let ve = e.map(|i| state.x[i]).unwrap_or(0.0);

        let v_be = vb - ve;
        let v_bc = vb - vc;

        let (mut i_c, mut i_b, mut g_pi, mut g_mu, gm) = self.evaluate_dc(v_be, v_bc);

        // Simplified transient capacitance (linear Cje/Cjc for stepping demonstration)
        if let (Some(dt), Some(x_prev)) = (state.dt, state.x_prev) {
            let vc_prev = c.map(|i| x_prev[i]).unwrap_or(0.0);
            let vb_prev = b.map(|i| x_prev[i]).unwrap_or(0.0);
            let ve_prev = e.map(|i| x_prev[i]).unwrap_or(0.0);
            
            let v_be_prev = vb_prev - ve_prev;
            let v_bc_prev = vb_prev - vc_prev;

            // B-E capacitance
            let g_cje = self.model.cje / dt;
            let i_cje_hist = -g_cje * v_be_prev;
            
            // B-C capacitance
            let g_cjc = self.model.cjc / dt;
            let i_cjc_hist = -g_cjc * v_bc_prev;

            g_pi += g_cje;
            g_mu += g_cjc;
            
            i_b += g_cje * v_be + i_cje_hist + g_cjc * v_bc + i_cjc_hist;
            // Charge leaves collector through Cjc
            i_c -= g_cjc * v_bc + i_cjc_hist; 
        }

        g_pi += state.gmin;
        g_mu += state.gmin;

        // Equivalent linearised currents
        let i_eq_b = i_b - g_pi * v_be - g_mu * v_bc;
        let i_eq_c = i_c - gm * v_be + g_mu * v_bc;

        // Stamp Base-Emitter conductance (g_pi)
        if let Some(bi) = b { mna.matrix[(bi, bi)] += g_pi; }
        if let Some(ei) = e { mna.matrix[(ei, ei)] += g_pi; }
        if let (Some(bi), Some(ei)) = (b, e) {
            mna.matrix[(bi, ei)] -= g_pi;
            mna.matrix[(ei, bi)] -= g_pi;
        }

        // Stamp Base-Collector conductance (g_mu)
        if let Some(bi) = b { mna.matrix[(bi, bi)] += g_mu; }
        if let Some(ci) = c { mna.matrix[(ci, ci)] += g_mu; }
        if let (Some(bi), Some(ci)) = (b, c) {
            mna.matrix[(bi, ci)] -= g_mu;
            mna.matrix[(ci, bi)] -= g_mu;
        }

        // Stamp VCCS Transconductance (gm * Vbe) into Collector-Emitter
        // C receives -gm*Vbe, E receives +gm*Vbe
        if let Some(ci) = c {
            if let Some(bi) = b { mna.matrix[(ci, bi)] += gm; }
            if let Some(ei) = e { mna.matrix[(ci, ei)] -= gm; }
        }
        if let Some(ei) = e {
            if let Some(bi) = b { mna.matrix[(ei, bi)] -= gm; }
            if let Some(ei2) = e { mna.matrix[(ei, ei2)] += gm; }
        }

        // Stamp equivalent currents
        if let Some(bi) = b { mna.rhs[bi] -= i_eq_b; }
        if let Some(ci) = c { mna.rhs[ci] -= i_eq_c; }
        if let Some(ei) = e { mna.rhs[ei] += i_eq_b + i_eq_c; }

        Ok(())
    }
}
