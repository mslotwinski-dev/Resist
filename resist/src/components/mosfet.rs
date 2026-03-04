use crate::components::models::MosfetModel;
use crate::components::nonlinear::{NonLinearComponent, NonLinearState};
use crate::core::{MnaMatrix, NodeId};
use crate::error::ResistError;

/// Shichman-Hodges (Level 1) MOSFET.
pub struct Mosfet {
    pub name: String,
    pub drain: NodeId,
    pub gate: NodeId,
    pub source: NodeId,
    pub bulk: NodeId,
    pub model: MosfetModel,
}

impl Mosfet {
    pub fn new(
        name: &str,
        drain: NodeId,
        gate: NodeId,
        source: NodeId,
        bulk: NodeId,
        model: MosfetModel,
    ) -> Self {
        Self {
            name: name.to_string(),
            drain,
            gate,
            source,
            bulk,
            model,
        }
    }

    /// Evaluates MOSFET Level 1 currents and conductances.
    fn evaluate_dc(&self, v_d: f64, v_g: f64, v_s: f64, v_b: f64) -> (f64, f64, f64) {
        let polarity = if self.model.is_nmos { 1.0 } else { -1.0 };
        let mut v_ds = polarity * (v_d - v_s);
        let mut v_gs = polarity * (v_g - v_s);
        let _v_bs = polarity * (v_b - v_s);

        // Very basic threshold voltage (ignoring body effect for simplicity here,
        // standard Level 1 includes it if gamma > 0).
        let v_th = self.model.vto; // + self.model.gamma * ((self.model.phi - v_bs).sqrt() - self.model.phi.sqrt());

        let mut is_reverse = false;
        if v_ds < 0.0 {
            // Source and Drain are swapped
            is_reverse = true;
            v_ds = -v_ds;
            v_gs = polarity * (v_g - v_d);
        }

        let mut id: f64;
        let gm: f64; // dId / dVgs
        let gds: f64; // dId / dVds

        if v_gs <= v_th {
            // Cutoff region
            id = 0.0;
            gm = 0.0;
            gds = 0.0;
        } else if v_ds <= v_gs - v_th {
            // Linear (Triode) region
            let v_eff = v_gs - v_th;
            let lambda_factor = 1.0 + self.model.lambda * v_ds;
            id = self.model.kp * (v_eff - v_ds / 2.0) * v_ds * lambda_factor;
            
            gm = self.model.kp * v_ds * lambda_factor;
            gds = self.model.kp * ((v_eff - v_ds) * lambda_factor + (v_eff - v_ds / 2.0) * v_ds * self.model.lambda);
        } else {
            // Saturation region
            let v_eff = v_gs - v_th;
            let lambda_factor = 1.0 + self.model.lambda * v_ds;
            id = self.model.kp / 2.0 * v_eff.powi(2) * lambda_factor;

            gm = self.model.kp * v_eff * lambda_factor;
            gds = self.model.kp / 2.0 * v_eff.powi(2) * self.model.lambda;
        }

        id *= polarity;

        if is_reverse {
            id = -id;
            // Conductances remain positive, but effectively refer to the swapped terminals
        }

        // Return: Id, Gm (dId/dVgs), Gds (dId/dVds)
        (id, gm, gds)
    }
}

impl NonLinearComponent for Mosfet {
    fn stamp_nonlinear(
        &self,
        mna: &mut MnaMatrix,
        state: &NonLinearState,
    ) -> Result<(), ResistError> {
        let d = self.drain.matrix_idx();
        let g = self.gate.matrix_idx();
        let s = self.source.matrix_idx();
        let b = self.bulk.matrix_idx();

        let v_d = d.map(|i| state.x[i]).unwrap_or(0.0);
        let v_g = g.map(|i| state.x[i]).unwrap_or(0.0);
        let v_s = s.map(|i| state.x[i]).unwrap_or(0.0);
        let v_b = b.map(|i| state.x[i]).unwrap_or(0.0);

        let (id, gm, gds_val) = self.evaluate_dc(v_d, v_g, v_s, v_b);

        let gds = gds_val + state.gmin;
        let i_eq = id - gm * (v_g - v_s) - gds * (v_d - v_s);

        // Capacitances
        if let (Some(dt), Some(x_prev)) = (state.dt, state.x_prev) {
            let vd_prev = d.map(|i| x_prev[i]).unwrap_or(0.0);
            let vg_prev = g.map(|i| x_prev[i]).unwrap_or(0.0);
            let vs_prev = s.map(|i| x_prev[i]).unwrap_or(0.0);

            let vgs_prev = vg_prev - vs_prev;
            let vgd_prev = vg_prev - vd_prev;

            // Gate-Source Capacitance
            let g_cgs = self.model.cgs / dt;
            let i_cgs_hist = -g_cgs * vgs_prev;
            
            // Gate-Drain Capacitance
            let g_cgd = self.model.cgd / dt;
            let i_cgd_hist = -g_cgd * vgd_prev;

            // Gate is completely insulated at DC, only AC/transient coupling
            if let Some(gi) = g { mna.matrix[(gi, gi)] += g_cgs + g_cgd; }
            if let Some(si) = s { mna.matrix[(si, si)] += g_cgs; }
            if let Some(di) = d { mna.matrix[(di, di)] += g_cgd; }

            if let (Some(gi), Some(si)) = (g, s) {
                mna.matrix[(gi, si)] -= g_cgs;
                mna.matrix[(si, gi)] -= g_cgs;
            }
            if let (Some(gi), Some(di)) = (g, d) {
                mna.matrix[(gi, di)] -= g_cgd;
                mna.matrix[(di, gi)] -= g_cgd;
            }

            // Gate currents
            let i_gs = g_cgs * (v_g - v_s) + i_cgs_hist;
            let i_gd = g_cgd * (v_g - v_d) + i_cgd_hist;

            if let Some(gi) = g { mna.rhs[gi] -= i_gs + i_gd; }
            if let Some(si) = s { mna.rhs[si] += i_gs; }
            if let Some(di) = d { mna.rhs[di] += i_gd; }
        }

        // Drain-Source Conductance (gds)
        if let Some(di) = d { mna.matrix[(di, di)] += gds; }
        if let Some(si) = s { mna.matrix[(si, si)] += gds; }
        if let (Some(di), Some(si)) = (d, s) {
            mna.matrix[(di, si)] -= gds;
            mna.matrix[(si, di)] -= gds;
        }

        // VCCS Transconductance (gm * Vgs)
        // Drain leaves gm*Vgs, Source enters gm*Vgs
        if let Some(di) = d {
            if let Some(gi) = g { mna.matrix[(di, gi)] += gm; }
            if let Some(si) = s { mna.matrix[(di, si)] -= gm; }
        }
        if let Some(si) = s {
            if let Some(gi) = g { mna.matrix[(si, gi)] -= gm; }
            if let Some(si2) = s { mna.matrix[(si, si2)] += gm; } // (si, si)
        }

        // Equivalent Current source
        if let Some(di) = d { mna.rhs[di] -= i_eq; }
        if let Some(si) = s { mna.rhs[si] += i_eq; }

        Ok(())
    }
}
