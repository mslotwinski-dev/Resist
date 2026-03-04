use nalgebra::DVector;

use crate::core::MnaMatrix;
use crate::error::ResistError;

/// A component that has energy-storage behaviour and must be discretised
/// in time using a companion model (e.g. capacitor, inductor).
///
/// During transient analysis, at each time step the component stamps an
/// equivalent conductance and history current source computed from the
/// previous solution.
pub trait TransientComponent: Send + Sync {
    /// Stamp the companion model into `mna` for a time step of `dt` seconds.
    ///
    /// `x_prev` is the full solution vector from the **previous** time step
    /// (node voltages followed by branch currents).
    fn stamp_transient(
        &self,
        mna: &mut MnaMatrix,
        dt: f64,
        x_prev: &DVector<f64>,
    ) -> Result<(), ResistError>;
}
