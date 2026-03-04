use crate::core::ComplexMnaMatrix;
use crate::error::ResistError;

/// A component that can stamp values into the **complex-valued** MNA matrix
/// used for AC (frequency-domain) analysis.
///
/// The angular frequency `omega = 2πf` is provided so that reactive
/// components (capacitors, inductors) can compute their admittances.
pub trait AcComponent: Send + Sync {
    fn stamp_ac(&self, mna: &mut ComplexMnaMatrix, omega: f64) -> Result<(), ResistError>;
}
