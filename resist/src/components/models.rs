/// SPICE-like parameter model card for a Diode.
#[derive(Debug, Clone, Copy)]
pub struct DiodeModel {
    /// Saturation current (A). Default: 1e-14
    pub is: f64,
    /// Emission (ideality) coefficient. Default: 1.0
    pub n: f64,
    /// Ohmic series resistance (Ω). Default: 0.0
    pub rs: f64,
    /// Zero-bias junction capacitance (F). Default: 0.0
    pub cj0: f64,
    /// Junction potential (V). Default: 1.0
    pub vj: f64,
    /// Grading coefficient. Default: 0.5
    pub m: f64,
    /// Transit time (s). Default: 0.0
    pub tt: f64,
}

impl Default for DiodeModel {
    fn default() -> Self {
        Self {
            is: 1e-14,
            n: 1.0,
            rs: 0.0,
            cj0: 0.0,
            vj: 1.0,
            m: 0.5,
            tt: 0.0,
        }
    }
}

/// SPICE-like parameter model card for a Bipolar Junction Transistor (BJT).
#[derive(Debug, Clone, Copy)]
pub struct BjtModel {
    /// True for NPN, false for PNP.
    pub is_npn: bool,
    /// Transport saturation current (A). Default: 1e-16
    pub is: f64,
    /// Ideal maximum forward beta. Default: 100.0
    pub bf: f64,
    /// Ideal maximum reverse beta. Default: 1.0
    pub br: f64,
    /// Forward Early voltage (V). Default: 0.0 (infinite)
    pub va: f64,
    /// Base resistance (Ω). Default: 0.0
    pub rb: f64,
    /// Emitter resistance (Ω). Default: 0.0
    pub re: f64,
    /// Collector resistance (Ω). Default: 0.0
    pub rc: f64,
    /// B-E zero-bias depletion capacitance (F). Default: 0.0
    pub cje: f64,
    /// B-C zero-bias depletion capacitance (F). Default: 0.0
    pub cjc: f64,
}

impl Default for BjtModel {
    fn default() -> Self {
        Self {
            is_npn: true,
            is: 1e-16,
            bf: 100.0,
            br: 1.0,
            va: 0.0, // 0 means infinity (no early effect) by convention in our solver
            rb: 0.0,
            re: 0.0,
            rc: 0.0,
            cje: 0.0,
            cjc: 0.0,
        }
    }
}

/// SPICE-like parameter model card for a MOSFET (Level 1 Shichman-Hodges).
#[derive(Debug, Clone, Copy)]
pub struct MosfetModel {
    /// True for NMOS, false for PMOS.
    pub is_nmos: bool,
    /// Transconductance parameter (A/V²). Default: 2e-5
    pub kp: f64,
    /// Zero-bias threshold voltage (V). Default: 1.0 (NMOS) / -1.0 (PMOS)
    pub vto: f64,
    /// Channel-length modulation (1/V). Default: 0.0
    pub lambda: f64,
    /// Bulk threshold parameter (V^0.5). Default: 0.0
    pub gamma: f64,
    /// Surface potential (V). Default: 0.6
    pub phi: f64,
    /// Gate-Source overlap capacitance per channel length (F). Default: 0.0
    pub cgs: f64,
    /// Gate-Drain overlap capacitance per channel length (F). Default: 0.0
    pub cgd: f64,
}

impl Default for MosfetModel {
    fn default() -> Self {
        Self {
            is_nmos: true,
            kp: 2e-5,
            vto: 1.0,
            lambda: 0.0,
            gamma: 0.0,
            phi: 0.6,
            cgs: 0.0,
            cgd: 0.0,
        }
    }
}
