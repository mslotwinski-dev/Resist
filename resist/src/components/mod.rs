use crate::core::MnaMatrix;
use crate::error::ResistError;

pub mod ac;
pub mod ac_current_source;
pub mod ac_voltage_source;
pub mod bjt;
pub mod capacitor;
pub mod current_source;
pub mod diode;
pub mod inductor;
pub mod models;
pub mod mosfet;
pub mod nonlinear;
pub mod resistor;
pub mod transient;
pub mod transient_voltage_source;
pub mod vcvs;
pub mod voltage_source;

/// A component that can stamp values into the **real-valued** MNA matrix
/// (used for DC operating-point analysis).
pub trait Component: Send + Sync {
    /// The unique name of the component, if applicable.
    fn name(&self) -> &str { "" }

    /// Mutates the DC voltage of this component (used for DC sweeping).
    fn set_dc_voltage(&mut self, _v: f64) {}

    fn stamp(&self, mna: &mut MnaMatrix) -> Result<(), ResistError>;
}

/// Submodule re-exporting the AC trait so the rest of the crate can refer
/// to `crate::components::ac::AcComponent`.
pub mod ac_trait {
    pub use super::ac::AcComponent;
}
