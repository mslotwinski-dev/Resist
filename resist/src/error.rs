use thiserror::Error;

/// Errors that can occur during circuit construction or analysis.
#[derive(Error, Debug)]
pub enum ResistError {
    /// The MNA matrix is singular — typically caused by a floating node,
    /// a short-circuited voltage source, or an under-constrained topology.
    #[error("Matrix is singular, possible floating node or short circuit")]
    SingularMatrix,

    /// A referenced node does not exist in the circuit.
    #[error("Node {0} does not exist in the circuit")]
    NodeNotFound(usize),

    /// A general solver failure with a descriptive message.
    #[error("MNA solver failed: {0}")]
    SolverFailed(String),

    /// Newton-Raphson iteration did not converge within the allowed
    /// number of iterations.
    #[error("Convergence failed after {iterations} iterations (residual = {residual:.2e})")]
    ConvergenceError {
        iterations: usize,
        residual: f64,
    },

    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    #[error("Component not found: {0}")]
    ComponentNotFound(String),
}
