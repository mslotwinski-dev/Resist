use crate::analysis::nonlinear::{NonLinearAnalyzer, NonLinearDcResult};
use crate::core::Circuit;
use crate::error::ResistError;

pub struct DcSweepAnalyzer<'a> {
    circuit: &'a mut Circuit,
    source_name: String,
    start: f64,
    stop: f64,
    step: f64,
}

pub struct DcSweepResult {
    /// Tuple of (Source Value, Result)
    pub steps: Vec<(f64, NonLinearDcResult)>,
}

impl<'a> DcSweepAnalyzer<'a> {
    pub(crate) fn new(circuit: &'a mut Circuit, source_name: &str, start: f64, stop: f64, step: f64) -> Self {
        Self {
            circuit,
            source_name: source_name.to_string(),
            start,
            stop,
            step,
        }
    }

    pub fn solve(&mut self) -> Result<DcSweepResult, ResistError> {
        let mut steps = Vec::new();

        // 1. Check valid parameters
        if self.step == 0.0 || (self.stop - self.start).signum() != self.step.signum() {
            return Err(ResistError::InvalidParameters(
                "DC sweep step must be non-zero and in the right direction".into(),
            ));
        }

        let mut current_val = self.start;
        let mut original_val = None;

        // Loop through values
        loop {
            // Check loop bounds considering floating point inaccuracies
            if self.step > 0.0 && current_val > self.stop + 1e-9 {
                break;
            } else if self.step < 0.0 && current_val < self.stop - 1e-9 {
                break;
            }

            // 2. Mutate the source parameter
            {
                let comp = self.circuit.get_component_mut(&self.source_name);
                if let Some(c) = comp {
                    // Cache the original voltage if not set
                    if original_val.is_none() {
                        // We must assume 0.0 original since we don't have a get() easily.
                        original_val = Some(0.0);
                    }
                    c.set_dc_voltage(current_val);
                } else {
                    return Err(ResistError::ComponentNotFound(self.source_name.clone()));
                }
            }

            // 3. Solve the circuit at this operating point
            let analyzer = NonLinearAnalyzer::new(&self.circuit);
            let result = analyzer.solve()?;
            steps.push((current_val, result));

            current_val += self.step;
        }

        Ok(DcSweepResult { steps })
    }
}
