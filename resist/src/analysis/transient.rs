use std::collections::HashMap;

use nalgebra::DVector;

use crate::components::nonlinear::NonLinearState;
use crate::core::{Circuit, MnaMatrix, NodeId};
use crate::error::ResistError;

/// Performs **transient** (time-domain) analysis using Backward Euler
/// numerical integration and Adaptive Time Stepping.
pub struct TransientAnalyzer<'a> {
    circuit: &'a Circuit,
    t_stop: f64,
    dt_initial: f64,
}

#[derive(Clone)]
pub struct TimePoint {
    pub time: f64,
    pub node_voltages: HashMap<NodeId, f64>,
    pub voltage_source_currents: Vec<f64>,
}

pub struct TransientResult {
    pub time_points: Vec<TimePoint>,
}

impl<'a> TransientAnalyzer<'a> {
    pub(crate) fn new(circuit: &'a Circuit, t_stop: f64, dt_initial: f64) -> Self {
        Self {
            circuit,
            t_stop,
            dt_initial,
        }
    }

    pub fn solve(&self) -> Result<TransientResult, ResistError> {
        let n = self.circuit.num_nodes();
        let m = self.circuit.num_voltage_sources();
        let size = n + m;

        let mut x_prev = DVector::zeros(size);
        let mut time_points = Vec::new();

        // Solve DC operating point at t=0
        let dc = self.circuit.build_nonlinear().solve()?;
        for (node, &vol) in &dc.node_voltages {
            if let Some(i) = node.matrix_idx() {
                x_prev[i] = vol;
            }
        }
        for (i, &curr) in dc.voltage_source_currents.iter().enumerate() {
            x_prev[n + i] = curr;
        }

        time_points.push(self.extract_time_point(0.0, &x_prev));

        let mut t = 0.0;
        let mut dt = self.dt_initial;
        let dt_min = self.dt_initial * 1e-6; // Don't get stuck forever
        let dt_max = self.dt_initial * 100.0;

        while t < self.t_stop {
            if t + dt > self.t_stop {
                dt = self.t_stop - t;
            }

            let mut iter_count = 0;
            let mut converged = false;
            let mut x_guess = x_prev.clone();

            for iter in 1..=100 {
                iter_count = iter;
                let mut mna = MnaMatrix::new(n, m);

                // Linear components
                for comp in &self.circuit.components {
                    comp.stamp(&mut mna)?;
                }

                // Transient sources
                for tvs in &self.circuit.transient_sources {
                    tvs.stamp_at(&mut mna, t + dt)?;
                }

                // Companion models (Capacitor/Inductor)
                for tc in &self.circuit.transient_components {
                    tc.stamp_transient(&mut mna, dt, &x_prev)?;
                }

                // Non-linear / semiconductor devices (Diodes, BJTs, MOSFETs)
                let state = NonLinearState {
                    x: &x_guess,
                    gmin: 1e-12, // Always use standard Gmin for transient to preserve dynamics, relying on small dt to converge
                    dt: Some(dt),
                    x_prev: Some(&x_prev),
                    time: t + dt,
                };

                for nlc in &self.circuit.nonlinear_components {
                    nlc.stamp_nonlinear(&mut mna, &state)?;
                }

                let lu = mna.matrix.clone().lu();
                let x_new = match lu.solve(&mna.rhs) {
                    Some(x) => x,
                    None => {
                        // Singular matrix, reject step
                        break;
                    }
                };

                let delta = (&x_new - &x_guess).amax();
                x_guess = x_new;

                if delta < 1e-6 {
                    converged = true;
                    break;
                }
            }

            if converged {
                // Step accepted
                t += dt;
                x_prev = x_guess.clone();
                time_points.push(self.extract_time_point(t, &x_prev));

                // Adaptive step sizing
                if iter_count <= 4 {
                    dt = (dt * 1.5).min(dt_max); // Too easy, increase step
                } else if iter_count >= 10 {
                    dt = (dt * 0.5).max(dt_min); // Just scraping by, reduce next step
                }
            } else {
                // Step rejected
                dt *= 0.1; // Cut drastically
                if dt < dt_min {
                    return Err(ResistError::ConvergenceError {
                        iterations: 100,
                        residual: 0.0,
                    });
                }
            }
        }

        Ok(TransientResult { time_points })
    }

    fn extract_time_point(&self, time: f64, x: &DVector<f64>) -> TimePoint {
        let n = self.circuit.num_nodes();
        let m = self.circuit.num_voltage_sources();

        let mut node_voltages = HashMap::new();
        for i in 1..=n {
            node_voltages.insert(NodeId(i), x[i - 1]);
        }

        let mut voltage_source_currents = Vec::with_capacity(m);
        for i in 0..m {
            voltage_source_currents.push(x[n + i]);
        }

        TimePoint {
            time,
            node_voltages,
            voltage_source_currents,
        }
    }
}
