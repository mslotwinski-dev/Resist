use num_complex::Complex64;

use crate::components::ac::AcComponent;
use crate::components::Component;
use crate::core::{ComplexMnaMatrix, MnaMatrix, NodeId};
use crate::error::ResistError;

/// A Voltage-Controlled Voltage Source (VCVS).
///
/// Enforces the equation:
///
/// ```text
///   V(out+) − V(out−) = gain × (V(in+) − V(in−))
/// ```
///
/// This is the core building block for modelling **ideal operational
/// amplifiers**: set `gain` to a very large value (e.g. 1 × 10⁶).
///
/// Internally the VCVS adds one extra equation row/column to the MNA
/// matrix (like an independent voltage source) with modified stamps that
/// encode the gain relationship.
///
/// # Examples
///
/// ```
/// use resist::{Circuit, NodeId};
///
/// let mut ckt = Circuit::new();
/// let inp = ckt.add_node(); // non-inverting input
/// let inn = ckt.add_node(); // inverting input
/// let out = ckt.add_node(); // output
///
/// // Ideal op-amp: V(out) = 1e6 * (V(inp) - V(inn))
/// ckt.add_vcvs("E1", out, NodeId::GROUND, inp, inn, 1e6);
/// ```
#[derive(Clone)]
pub struct Vcvs {
    pub name: String,
    pub out_p: NodeId,
    pub out_n: NodeId,
    pub in_p: NodeId,
    pub in_n: NodeId,
    pub gain: f64,
    pub equation_idx: usize,
}

impl Vcvs {
    pub fn new(
        name: &str,
        out_p: NodeId,
        out_n: NodeId,
        in_p: NodeId,
        in_n: NodeId,
        gain: f64,
        equation_idx: usize,
    ) -> Self {
        Self {
            name: name.to_string(),
            out_p,
            out_n,
            in_p,
            in_n,
            gain,
            equation_idx,
        }
    }

    /// Stamp the VCVS into a real-valued MNA system.
    ///
    /// Extra equation row `eq`:
    ///   `+1 · V(out+) − 1 · V(out−) − gain · V(in+) + gain · V(in−) = 0`
    ///
    /// KCL columns (current variable `I_eq` contributes to out+ and out−):
    ///   `out+ ← +I_eq`,  `out− ← −I_eq`
    fn stamp_real(&self, n: usize, m: &mut nalgebra::DMatrix<f64>) {
        let eq = n + self.equation_idx;

        // B-matrix (KCL): current through VCVS enters out+, leaves out−
        if let Some(op) = self.out_p.matrix_idx() {
            m[(op, eq)] += 1.0;
        }
        if let Some(on) = self.out_n.matrix_idx() {
            m[(on, eq)] -= 1.0;
        }

        // C-matrix (voltage equation): V(out+) − V(out−) − gain·V(in+) + gain·V(in−) = 0
        if let Some(op) = self.out_p.matrix_idx() {
            m[(eq, op)] += 1.0;
        }
        if let Some(on) = self.out_n.matrix_idx() {
            m[(eq, on)] -= 1.0;
        }
        if let Some(ip) = self.in_p.matrix_idx() {
            m[(eq, ip)] -= self.gain;
        }
        if let Some(in_) = self.in_n.matrix_idx() {
            m[(eq, in_)] += self.gain;
        }
    }
}

impl Component for Vcvs {
    fn stamp(&self, mna: &mut MnaMatrix) -> Result<(), ResistError> {
        self.stamp_real(mna.num_nodes, &mut mna.matrix);
        Ok(())
    }
}

impl AcComponent for Vcvs {
    fn stamp_ac(&self, mna: &mut ComplexMnaMatrix, _omega: f64) -> Result<(), ResistError> {
        let eq = mna.num_nodes + self.equation_idx;

        // B-matrix
        let one = Complex64::new(1.0, 0.0);
        if let Some(op) = self.out_p.matrix_idx() {
            mna.matrix[(op, eq)] += one;
        }
        if let Some(on) = self.out_n.matrix_idx() {
            mna.matrix[(on, eq)] -= one;
        }

        // C-matrix
        if let Some(op) = self.out_p.matrix_idx() {
            mna.matrix[(eq, op)] += one;
        }
        if let Some(on) = self.out_n.matrix_idx() {
            mna.matrix[(eq, on)] -= one;
        }
        let g = Complex64::new(self.gain, 0.0);
        if let Some(ip) = self.in_p.matrix_idx() {
            mna.matrix[(eq, ip)] -= g;
        }
        if let Some(in_) = self.in_n.matrix_idx() {
            mna.matrix[(eq, in_)] += g;
        }

        Ok(())
    }
}
