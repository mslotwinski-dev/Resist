use std::collections::HashMap;

use nalgebra::DVector;

use crate::components::Component;
use crate::components::nonlinear::NonLinearState;
use crate::core::{Circuit, MnaMatrix, NodeId};
use crate::error::ResistError;

/// Performs **non-linear DC** analysis using the Newton-Raphson method.
///
/// Iteratively linearises non-linear components (e.g. diodes) around the
/// current operating point until the solution converges within `tol`.
/// Implements SPICE Gmin stepping to guarantee convergence for 
/// highly non-linear circuits.
pub struct NonLinearAnalyzer<'a> {
    circuit: &'a Circuit,
    max_iter: usize,
    tol: f64,
}

/// Results of a non-linear DC analysis.
pub struct NonLinearDcResult {
    pub node_voltages: HashMap<NodeId, f64>,
    pub voltage_source_currents: Vec<f64>,
    pub iterations: usize,
}

impl<'a> NonLinearAnalyzer<'a> {
    pub(crate) fn new(circuit: &'a Circuit) -> Self {
        Self {
            circuit,
            max_iter: 200,
            tol: 1e-6,
        }
    }

    pub fn tolerance(mut self, tol: f64) -> Self {
        self.tol = tol;
        self
    }

    pub fn max_iterations(mut self, n: usize) -> Self {
        self.max_iter = n;
        self
    }

    pub fn solve(&self) -> Result<NonLinearDcResult, ResistError> {
        let n = self.circuit.num_nodes();
        let m = self.circuit.num_voltage_sources();
        let size = n + m;

        let mut x = DVector::zeros(size);
        let mut total_iter = 0;

        // Gmin stepping (1e-2 down to 1e-12)
        let gmin_steps = [1e-2, 1e-3, 1e-4, 1e-5, 1e-6, 1e-7, 1e-8, 1e-9, 1e-10, 1e-11, 1e-12];

        // Try direct NR first with Gmin = 1e-12
        match self.try_nr_solve(&mut x, 1e-12, &mut total_iter, n, m) {
            Ok(_) => return self.build_result(x, n, m, total_iter),
            Err(_) => {
                // If direct fails, start Gmin stepping
                x = DVector::zeros(size); // Reset guess
                for &gmin in &gmin_steps {
                    match self.try_nr_solve(&mut x, gmin, &mut total_iter, n, m) {
                        Ok(_) => {
                            // Converged for this Gmin. Keep 'x' as starting point for next Gmin
                            continue;
                        }
                        Err(e) => {
                            return Err(e); // Failed even with Gmin stepping
                        }
                    }
                }
                
                return self.build_result(x, n, m, total_iter);
            }
        }
    }

    fn try_nr_solve(&self, x: &mut DVector<f64>, gmin: f64, total_iter: &mut usize, n: usize, m: usize) -> Result<(), ResistError> {
        for _ in 0..self.max_iter {
            *total_iter += 1;
            
            let mut mna = MnaMatrix::new(n, m);

            for comp in &self.circuit.components {
                comp.stamp(&mut mna)?;
            }
            for tvs in &self.circuit.transient_sources {
                tvs.stamp(&mut mna)?;
            }
            
            let state = NonLinearState {
                x,
                gmin,
                dt: None,
                x_prev: None,
                time: 0.0,
            };

            for nlc in &self.circuit.nonlinear_components {
                nlc.stamp_nonlinear(&mut mna, &state)?;
            }

            let lu = mna.matrix.clone().lu();
            let x_new = lu.solve(&mna.rhs).ok_or(ResistError::SingularMatrix)?;
            let delta = (&x_new - &*x).amax();
            
            *x = x_new;

            if delta < self.tol {
                return Ok(());
            }
        }

        Err(ResistError::ConvergenceError {
            iterations: self.max_iter,
            residual: 0.0,
        })
    }

    fn build_result(&self, x: DVector<f64>, n: usize, m: usize, total_iter: usize) -> Result<NonLinearDcResult, ResistError> {
        let mut node_voltages = HashMap::new();
        for i in 1..=n {
            node_voltages.insert(NodeId(i), x[i - 1]);
        }

        let mut voltage_source_currents = Vec::with_capacity(m);
        for i in 0..m {
            voltage_source_currents.push(x[n + i]);
        }

        Ok(NonLinearDcResult {
            node_voltages,
            voltage_source_currents,
            iterations: total_iter,
        })
    }
}
