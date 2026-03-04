use nalgebra::{DMatrix, DVector};
use num_complex::Complex64;

use crate::components::Component;
use crate::components::ac::AcComponent;
use crate::components::models::{BjtModel, DiodeModel, MosfetModel};
use crate::components::nonlinear::NonLinearComponent;
use crate::components::transient::TransientComponent;
use crate::components::transient_voltage_source::TransientVoltageSource;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub usize);

impl NodeId {
    pub const GROUND: NodeId = NodeId(0);

    pub fn matrix_idx(&self) -> Option<usize> {
        if self.0 == 0 {
            None
        } else {
            Some(self.0 - 1)
        }
    }
}

pub struct MnaMatrix {
    pub matrix: DMatrix<f64>,
    pub rhs: DVector<f64>,
    pub num_nodes: usize,
}

impl MnaMatrix {
    pub fn new(num_nodes: usize, num_voltage_sources: usize) -> Self {
        let size = num_nodes + num_voltage_sources;
        Self {
            matrix: DMatrix::zeros(size, size),
            rhs: DVector::zeros(size),
            num_nodes,
        }
    }
}

pub struct ComplexMnaMatrix {
    pub matrix: DMatrix<Complex64>,
    pub rhs: DVector<Complex64>,
    pub num_nodes: usize,
}

impl ComplexMnaMatrix {
    pub fn new(num_nodes: usize, num_voltage_sources: usize) -> Self {
        let size = num_nodes + num_voltage_sources;
        Self {
            matrix: DMatrix::from_element(size, size, Complex64::new(0.0, 0.0)),
            rhs: DVector::from_element(size, Complex64::new(0.0, 0.0)),
            num_nodes,
        }
    }
}

pub struct Circuit {
    pub(crate) nodes: usize,
    pub(crate) voltage_sources: usize,
    pub(crate) components: Vec<Box<dyn Component>>,
    pub(crate) ac_components: Vec<Box<dyn AcComponent>>,
    pub(crate) nonlinear_components: Vec<Box<dyn NonLinearComponent>>,
    pub(crate) transient_components: Vec<Box<dyn TransientComponent>>,
    pub(crate) transient_sources: Vec<TransientVoltageSource>,
}

impl Circuit {
    pub fn new() -> Self {
        Self {
            nodes: 0,
            voltage_sources: 0,
            components: Vec::new(),
            ac_components: Vec::new(),
            nonlinear_components: Vec::new(),
            transient_components: Vec::new(),
            transient_sources: Vec::new(),
        }
    }

    pub fn add_node(&mut self) -> NodeId {
        self.nodes += 1;
        NodeId(self.nodes)
    }

    pub fn num_nodes(&self) -> usize {
        self.nodes
    }

    pub fn add_voltage_source_equation(&mut self) -> usize {
        let eq_idx = self.voltage_sources;
        self.voltage_sources += 1;
        eq_idx
    }

    pub fn num_voltage_sources(&self) -> usize {
        self.voltage_sources
    }

    pub fn add_component<C: Component + 'static>(&mut self, comp: C) {
        self.components.push(Box::new(comp));
    }

    pub fn add_ac_only_component<C: AcComponent + 'static>(&mut self, comp: C) {
        self.ac_components.push(Box::new(comp));
    }

    pub fn add_dual_component<C: Component + AcComponent + Clone + 'static>(&mut self, comp: C) {
        self.ac_components.push(Box::new(comp.clone()));
        self.components.push(Box::new(comp));
    }

    pub fn add_resistor(&mut self, name: &str, node_a: NodeId, node_b: NodeId, resistance: f64) {
        let r = crate::components::resistor::Resistor::new(name, node_a, node_b, resistance);
        self.add_dual_component(r);
    }

    pub fn add_voltage_source(&mut self, name: &str, node_a: NodeId, node_b: NodeId, voltage: f64) {
        let eq_idx = self.add_voltage_source_equation();
        let vs = crate::components::voltage_source::VoltageSource::new(name, node_a, node_b, voltage, eq_idx);
        self.add_dual_component(vs);
    }

    pub fn add_current_source(&mut self, name: &str, node_a: NodeId, node_b: NodeId, current: f64) {
        let cs = crate::components::current_source::CurrentSource::new(name, node_a, node_b, current);
        self.add_dual_component(cs);
    }

    pub fn add_capacitor(&mut self, name: &str, node_a: NodeId, node_b: NodeId, capacitance: f64) {
        let cap = crate::components::capacitor::Capacitor::new(name, node_a, node_b, capacitance);
        self.ac_components.push(Box::new(
            crate::components::capacitor::Capacitor::new(name, node_a, node_b, capacitance),
        ));
        self.transient_components.push(Box::new(cap));
    }

    pub fn add_inductor(&mut self, name: &str, node_a: NodeId, node_b: NodeId, inductance: f64) {
        let eq_idx = self.add_voltage_source_equation();
        let ind = crate::components::inductor::Inductor::new(name, node_a, node_b, inductance, eq_idx);
        self.ac_components.push(Box::new(ind.clone()));
        self.components.push(Box::new(ind.clone()));
        self.transient_components.push(Box::new(ind));
    }

    pub fn add_ac_voltage_source(
        &mut self,
        name: &str,
        node_a: NodeId,
        node_b: NodeId,
        amplitude: f64,
        phase_deg: f64,
    ) {
        let eq_idx = self.add_voltage_source_equation();
        let avs = crate::components::ac_voltage_source::AcVoltageSource::new(
            name, node_a, node_b, amplitude, phase_deg, eq_idx,
        );
        self.add_dual_component(avs);
    }

    pub fn add_ac_current_source(
        &mut self,
        name: &str,
        node_a: NodeId,
        node_b: NodeId,
        amplitude: f64,
        phase_deg: f64,
    ) {
        let acs = crate::components::ac_current_source::AcCurrentSource::new(
            name, node_a, node_b, amplitude, phase_deg,
        );
        self.add_ac_only_component(acs);
    }

    pub fn add_vcvs(
        &mut self,
        name: &str,
        out_p: NodeId,
        out_n: NodeId,
        in_p: NodeId,
        in_n: NodeId,
        gain: f64,
    ) {
        let eq_idx = self.add_voltage_source_equation();
        let vcvs = crate::components::vcvs::Vcvs::new(name, out_p, out_n, in_p, in_n, gain, eq_idx);
        self.add_dual_component(vcvs);
    }

    pub fn add_diode(&mut self, name: &str, mut anode: NodeId, cathode: NodeId, model: DiodeModel) {
        if model.rs > 0.0 {
            let internal_anode = self.add_node();
            self.add_resistor(&format!("{}_rs", name), anode, internal_anode, model.rs);
            anode = internal_anode;
        }
        let d = crate::components::diode::Diode::new(name, anode, cathode, model);
        self.nonlinear_components.push(Box::new(d));
    }

    pub fn add_bjt(&mut self, name: &str, collector: NodeId, base: NodeId, emitter: NodeId, model: BjtModel) {
        let b = crate::components::bjt::Bjt::new(name, collector, base, emitter, model);
        self.nonlinear_components.push(Box::new(b));
    }

    pub fn add_mosfet(&mut self, name: &str, drain: NodeId, gate: NodeId, source: NodeId, bulk: NodeId, model: MosfetModel) {
        let m = crate::components::mosfet::Mosfet::new(name, drain, gate, source, bulk, model);
        self.nonlinear_components.push(Box::new(m));
    }

    pub fn add_transient_voltage_source(
        &mut self,
        name: &str,
        node_a: NodeId,
        node_b: NodeId,
        waveform: crate::components::transient_voltage_source::Waveform,
    ) {
        let eq_idx = self.add_voltage_source_equation();
        let tvs = TransientVoltageSource::new(name, node_a, node_b, waveform, eq_idx);
        self.ac_components.push(Box::new(tvs.clone()));
        self.transient_sources.push(tvs);
    }

    pub fn get_component_mut(&mut self, name: &str) -> Option<&mut Box<dyn Component>> {
        self.components
            .iter_mut()
            .find(|c| c.name() == name)
    }

    // --- Analyzers ---

    pub fn build(&self) -> crate::analysis::dc::DcAnalyzer<'_> {
        crate::analysis::dc::DcAnalyzer::new(self)
    }

    pub fn build_ac(&self, freq_hz: f64) -> crate::analysis::ac::AcAnalyzer<'_> {
        crate::analysis::ac::AcAnalyzer::new(self, freq_hz)
    }

    pub fn build_nonlinear(&self) -> crate::analysis::nonlinear::NonLinearAnalyzer<'_> {
        crate::analysis::nonlinear::NonLinearAnalyzer::new(self)
    }

    pub fn build_transient(&self, t_stop: f64, dt_initial: f64) -> crate::analysis::transient::TransientAnalyzer<'_> {
        crate::analysis::transient::TransientAnalyzer::new(self, t_stop, dt_initial)
    }

    pub fn build_dc_sweep<'a>(
        &'a mut self,
        source_name: &str,
        start: f64,
        stop: f64,
        step: f64,
    ) -> crate::analysis::sweep::DcSweepAnalyzer<'a> {
        crate::analysis::sweep::DcSweepAnalyzer::new(self, source_name, start, stop, step)
    }
}

impl Default for Circuit {
    fn default() -> Self {
        Self::new()
    }
}
