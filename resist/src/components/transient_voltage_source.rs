use crate::components::Component;
use crate::components::ac::AcComponent;
use crate::core::{ComplexMnaMatrix, MnaMatrix, NodeId};
use crate::error::ResistError;

/// Built-in waveforms for transient analysis.
#[derive(Clone)]
pub enum Waveform {
    /// Constant DC value.
    Dc(f64),

    /// Pure step waveform.
    Step {
        /// Initial voltage (V) before delay.
        v1: f64,
        /// Step voltage (V) after delay.
        v2: f64,
        /// Time delay before step (s).
        delay: f64,
    },

    /// Pulsed waveform.
    Pulse {
        /// Initial voltage (V).
        v1: f64,
        /// Pulsed voltage (V).
        v2: f64,
        /// Time delay before the first pulse (s).
        delay: f64,
        /// Rise time (s).
        rise: f64,
        /// Fall time (s).
        fall: f64,
        /// Pulse width at v2 (s).
        width: f64,
        /// Period of the pulse train (s).
        period: f64,
    },

    /// Sinusoidal waveform: `offset + amplitude × sin(2π·freq·t + phase)`.
    Sine {
        offset: f64,
        amplitude: f64,
        freq: f64,
        phase_deg: f64,
    },

    /// Custom mathematical or programmatic waveform.
    Custom(std::sync::Arc<dyn Fn(f64) -> f64 + Send + Sync>),
}

impl std::fmt::Debug for Waveform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dc(arg0) => f.debug_tuple("Dc").field(arg0).finish(),
            Self::Step { v1, v2, delay } => f.debug_struct("Step").field("v1", v1).field("v2", v2).field("delay", delay).finish(),
            Self::Pulse { v1, v2, delay, rise, fall, width, period } => f.debug_struct("Pulse").field("v1", v1).field("v2", v2).field("delay", delay).field("rise", rise).field("fall", fall).field("width", width).field("period", period).finish(),
            Self::Sine { offset, amplitude, freq, phase_deg } => f.debug_struct("Sine").field("offset", offset).field("amplitude", amplitude).field("freq", freq).field("phase_deg", phase_deg).finish(),
            Self::Custom(_) => f.debug_tuple("Custom").field(&"<closure>").finish(),
        }
    }
}

impl Waveform {
    /// Evaluate the waveform at time `t` (in seconds).
    pub fn evaluate(&self, t: f64) -> f64 {
        match self {
            Waveform::Dc(v) => *v,

            Waveform::Step { v1, v2, delay } => {
                if t < *delay {
                    *v1
                } else {
                    *v2
                }
            }

            Waveform::Pulse {
                v1,
                v2,
                delay,
                rise,
                fall,
                width,
                period,
            } => {
                if t < *delay {
                    return *v1;
                }
                let t_rel = (t - delay) % period;
                if t_rel < *rise {
                    // Rising edge
                    v1 + (v2 - v1) * t_rel / rise
                } else if t_rel < rise + width {
                    // High
                    *v2
                } else if t_rel < rise + width + fall {
                    // Falling edge
                    v2 + (v1 - v2) * (t_rel - rise - width) / fall
                } else {
                    // Low
                    *v1
                }
            }

            Waveform::Sine {
                offset,
                amplitude,
                freq,
                phase_deg,
            } => {
                let phase_rad = phase_deg.to_radians();
                offset + amplitude * (2.0 * std::f64::consts::PI * freq * t + phase_rad).sin()
            }
            
            Waveform::Custom(closure) => closure(t),
        }
    }
}

/// A time-varying voltage source for transient analysis.
///
/// This component stamps into the MNA matrix like a regular independent
/// voltage source but evaluates its voltage from a [`Waveform`] at the
/// current simulation time.
///
/// # Examples
///
/// ```
/// use resist::{Circuit, NodeId};
/// use resist::components::transient_voltage_source::Waveform;
///
/// let mut ckt = Circuit::new();
/// let n1 = ckt.add_node();
///
/// ckt.add_transient_voltage_source("V1", n1, NodeId::GROUND,
///     Waveform::Sine { offset: 0.0, amplitude: 5.0, freq: 1e3, phase_deg: 0.0 });
/// ```
#[derive(Clone)]
pub struct TransientVoltageSource {
    pub name: String,
    pub node_a: NodeId,
    pub node_b: NodeId,
    pub waveform: Waveform,
    pub equation_idx: usize,
}

impl TransientVoltageSource {
    pub fn new(
        name: &str,
        node_a: NodeId,
        node_b: NodeId,
        waveform: Waveform,
        equation_idx: usize,
    ) -> Self {
        Self {
            name: name.to_string(),
            node_a,
            node_b,
            waveform,
            equation_idx,
        }
    }

    /// Stamp the voltage source into the MNA matrix with its voltage at time `t`.
    pub fn stamp_at(&self, mna: &mut MnaMatrix, t: f64) -> Result<(), ResistError> {
        let eq = mna.num_nodes + self.equation_idx;
        let v = self.waveform.evaluate(t);

        if let Some(a) = self.node_a.matrix_idx() {
            mna.matrix[(a, eq)] += 1.0;
            mna.matrix[(eq, a)] += 1.0;
        }
        if let Some(b) = self.node_b.matrix_idx() {
            mna.matrix[(b, eq)] -= 1.0;
            mna.matrix[(eq, b)] -= 1.0;
        }
        mna.rhs[eq] += v;
        Ok(())
    }
}

/// At DC (build().solve()), a transient source evaluates at t = 0.
impl Component for TransientVoltageSource {
    fn stamp(&self, mna: &mut MnaMatrix) -> Result<(), ResistError> {
        self.stamp_at(mna, 0.0)
    }
}

impl AcComponent for TransientVoltageSource {
    fn stamp_ac(&self, mna: &mut ComplexMnaMatrix, _omega: f64) -> Result<(), ResistError> {
        let eq = mna.num_nodes + self.equation_idx;
        let one = num_complex::Complex64::new(1.0, 0.0);
        
        if let Some(a) = self.node_a.matrix_idx() {
            mna.matrix[(a, eq)] += one;
            mna.matrix[(eq, a)] += one;
        }
        if let Some(b) = self.node_b.matrix_idx() {
            mna.matrix[(b, eq)] -= one;
            mna.matrix[(eq, b)] -= one;
        }
        
        // 0V in AC
        Ok(())
    }
}
