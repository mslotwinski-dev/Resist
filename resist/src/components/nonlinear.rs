use nalgebra::DVector;

use crate::core::MnaMatrix;
use crate::error::ResistError;

/// The state of the simulator during a non-linear (Newton-Raphson) iteration.
///
/// This provides the current voltage guess (`x`), and if the simulation is
/// transient, it also provides the time step (`dt`) and previous state (`x_prev`)
/// so that charge-based devices (e.g. junction capacitances) can evaluate their
/// dynamic companion models inside the non-linear loop.
pub struct NonLinearState<'a> {
    /// Current guess for node voltages and branch currents.
    pub x: &'a DVector<f64>,
    /// Minimum conductance (Gmin) to add across non-linear junctions to aid convergence.
    pub gmin: f64,
    /// Time step in seconds (if in transient analysis), otherwise `None`.
    pub dt: Option<f64>,
    /// Solution vector from the previous time step (if in transient analysis).
    pub x_prev: Option<&'a DVector<f64>>,
    /// Current simulation time (if in transient analysis).
    pub time: f64,
}

/// A component with non-linear V-I characteristics (e.g. diode, BJT, MOSFET).
///
/// During Newton-Raphson iteration the component linearises itself around
/// the current operating point and stamps an equivalent conductance `G_eq`
/// and current source `I_eq` into the MNA matrix.
pub trait NonLinearComponent: Send + Sync {
    /// Stamp the linearised companion model into `mna` based on the
    /// current simulator state.
    fn stamp_nonlinear(
        &self,
        mna: &mut MnaMatrix,
        state: &NonLinearState,
    ) -> Result<(), ResistError>;
}
