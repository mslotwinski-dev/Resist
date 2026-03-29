use std::collections::{HashMap, HashSet};

use eframe::egui;

use crate::expression_parser;
use crate::plot_panel;
use crate::schematic;
use crate::sim_state::{
    ComponentInfo, ComponentKind, ConsoleLine, EditorMode, PinRef, PlotTab, SelectedEntity, SimState,
};
use resist::NodeId;

pub struct ResistApp {
    pub sim: SimState,
}

impl ResistApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::from_rgb(20, 23, 30);
        visuals.window_fill = egui::Color32::from_rgb(28, 32, 42);
        visuals.extreme_bg_color = egui::Color32::from_rgb(14, 16, 22);
        cc.egui_ctx.set_visuals(visuals);

        Self {
            sim: SimState::default(),
        }
    }

    fn log(&mut self, text: impl Into<String>, is_error: bool) {
        self.sim.console_output.push(ConsoleLine {
            text: text.into(),
            is_error,
        });
    }

    fn add_component(&mut self, kind: ComponentKind) {
        let prefix = match kind {
            ComponentKind::Resistor => "R",
            ComponentKind::Capacitor => "C",
            ComponentKind::Inductor => "L",
            ComponentKind::VoltageSource => "V",
            ComponentKind::CurrentSource => "I",
            ComponentKind::FunctionalVoltageSource => "VF",
            ComponentKind::FunctionalCurrentSource => "IF",
            ComponentKind::Ground => "GND",
        };

        let next = self
            .sim
            .layout
            .components
            .iter()
            .filter_map(|c| c.id.strip_prefix(prefix))
            .filter_map(|suffix| suffix.parse::<usize>().ok())
            .max()
            .unwrap_or(0)
            + 1;

        let id = if kind == ComponentKind::Ground {
            format!("{}{}", prefix, next)
        } else {
            format!("{}{}", prefix, next)
        };

        let default_expr = match kind {
            ComponentKind::FunctionalVoltageSource => Some("if t > 1m then 5 else 0".to_string()),
            ComponentKind::FunctionalCurrentSource => Some("if t > 1m then 1e-3 else 0".to_string()),
            _ => None,
        };

        self.sim.layout.components.push(ComponentInfo {
            id: id.clone(),
            name: id,
            kind,
            value: kind.default_value(),
            pos: crate::sim_state::Position::new(12, 8),
            rotation: crate::sim_state::Rotation::Deg0,
            expression: default_expr,
        });
    }

    fn remove_selected_component(&mut self) {
        let id = match &self.sim.selection {
            SelectedEntity::Component(id) => id.clone(),
            _ => return,
        };

        self.sim.layout.components.retain(|c| c.id != id);
        self.sim
            .layout
            .wires
            .retain(|w| w.from.component_id != id && w.to.component_id != id);
        self.sim.selection = SelectedEntity::None;
    }

    fn build_circuit_from_layout(
        &self,
    ) -> Result<(resist::Circuit, HashMap<String, Vec<NodeId>>), String> {
        let mut circuit = resist::Circuit::new();

        let mut all_pins: Vec<PinRef> = Vec::new();
        for comp in &self.sim.layout.components {
            for pin_index in 0..comp.kind.pin_count() {
                all_pins.push(PinRef {
                    component_id: comp.id.clone(),
                    pin_index,
                });
            }
        }

        if all_pins.is_empty() {
            return Err("Brak komponentow na schemacie.".to_string());
        }

        let mut pin_idx: HashMap<PinRef, usize> = HashMap::new();
        for (i, pin) in all_pins.iter().enumerate() {
            pin_idx.insert(pin.clone(), i);
        }

        let mut dsu = Dsu::new(all_pins.len());
        for wire in &self.sim.layout.wires {
            let a = pin_idx.get(&wire.from).copied().ok_or_else(|| {
                format!("Nieprawidlowy kabel: {:?}:{:?}", wire.from.component_id, wire.from.pin_index)
            })?;
            let b = pin_idx.get(&wire.to).copied().ok_or_else(|| {
                format!("Nieprawidlowy kabel: {:?}:{:?}", wire.to.component_id, wire.to.pin_index)
            })?;
            dsu.union(a, b);
        }

        let mut ground_roots: HashSet<usize> = HashSet::new();
        for comp in &self.sim.layout.components {
            if comp.kind == ComponentKind::Ground {
                for pin_index in 0..comp.kind.pin_count() {
                    let key = PinRef {
                        component_id: comp.id.clone(),
                        pin_index,
                    };
                    if let Some(idx) = pin_idx.get(&key) {
                        ground_roots.insert(dsu.find(*idx));
                    }
                }
            }
        }

        let mut root_to_node: HashMap<usize, NodeId> = HashMap::new();
        let mut node_for_pin: Vec<NodeId> = vec![NodeId::GROUND; all_pins.len()];

        for i in 0..all_pins.len() {
            let root = dsu.find(i);
            let node = if ground_roots.contains(&root) {
                NodeId::GROUND
            } else {
                *root_to_node.entry(root).or_insert_with(|| circuit.add_node())
            };
            node_for_pin[i] = node;
        }

        let mut component_nodes: HashMap<String, Vec<NodeId>> = HashMap::new();
        for comp in &self.sim.layout.components {
            let mut pins = Vec::new();
            for pin_index in 0..comp.kind.pin_count() {
                let key = PinRef {
                    component_id: comp.id.clone(),
                    pin_index,
                };
                let idx = pin_idx
                    .get(&key)
                    .copied()
                    .ok_or_else(|| format!("Brak pinu {}:{}", comp.id, pin_index))?;
                pins.push(node_for_pin[idx]);
            }
            component_nodes.insert(comp.id.clone(), pins);
        }

        for comp in &self.sim.layout.components {
            let nodes = component_nodes
                .get(&comp.id)
                .ok_or_else(|| format!("Brak mapowania wezlow dla {}", comp.id))?;

            match comp.kind {
                ComponentKind::Resistor => {
                    circuit.add_resistor(&comp.name, nodes[0], nodes[1], comp.value.max(1e-12));
                }
                ComponentKind::Capacitor => {
                    circuit.add_capacitor(&comp.name, nodes[0], nodes[1], comp.value.max(1e-18));
                }
                ComponentKind::Inductor => {
                    circuit.add_inductor(&comp.name, nodes[0], nodes[1], comp.value.max(1e-18));
                }
                ComponentKind::VoltageSource => {
                    circuit.add_voltage_source(&comp.name, nodes[0], nodes[1], comp.value);
                }
                ComponentKind::CurrentSource => {
                    circuit.add_current_source(&comp.name, nodes[0], nodes[1], comp.value);
                }
                ComponentKind::FunctionalVoltageSource => {
                    if let Some(expr_str) = &comp.expression {
                        match expression_parser::parse_expression(expr_str) {
                            Ok(closure) => {
                                use resist::components::transient_voltage_source::Waveform;
                                let waveform = Waveform::Custom(std::sync::Arc::new(closure));
                                circuit.add_transient_voltage_source(&comp.name, nodes[0], nodes[1], waveform);
                            }
                            Err(e) => {
                                return Err(format!("Blad parsowania {} ({}): {}", comp.id, expr_str, e));
                            }
                        }
                    } else {
                        return Err(format!("Brak wyrażenia dla {}", comp.id));
                    }
                }
                ComponentKind::FunctionalCurrentSource => {
                    if let Some(expr_str) = &comp.expression {
                        // For functional current sources, evaluate at t=0 for DC analysis
                        match expression_parser::parse_expression(expr_str) {
                            Ok(closure) => {
                                let value_at_zero = closure(0.0);
                                circuit.add_current_source(&comp.name, nodes[0], nodes[1], value_at_zero);
                            }
                            Err(e) => {
                                return Err(format!("Blad parsowania {} ({}): {}", comp.id, expr_str, e));
                            }
                        }
                    } else {
                        return Err(format!("Brak wyrażenia dla {}", comp.id));
                    }
                }
                ComponentKind::Ground => {}
            }
        }

        Ok((circuit, component_nodes))
    }

    fn run_dc(&mut self) {
        self.log("[DC] start", false);
        match self.build_circuit_from_layout() {
            Ok((circuit, component_nodes)) => match circuit.build_nonlinear().solve() {
                Ok(result) => {
                    self.sim.last_component_nodes = component_nodes;
                    self.sim.dc = Some(result);
                    self.log("[DC] done", false);
                }
                Err(e) => self.log(format!("[DC] error: {}", e), true),
            },
            Err(e) => self.log(format!("[DC] build error: {}", e), true),
        }
    }

    fn run_transient(&mut self) {
        self.log("[Transient] start", false);
        match self.build_circuit_from_layout() {
            Ok((circuit, component_nodes)) => {
                let analyzer = circuit
                    .build_transient(self.sim.transient_stop, self.sim.transient_step)
                    .with_uic(false);
                match analyzer.solve() {
                    Ok(result) => {
                        self.sim.last_component_nodes = component_nodes;
                        self.sim.transient = Some(Ok(result));
                        self.log("[Transient] done", false);
                    }
                    Err(e) => {
                        let msg = e.to_string();
                        self.sim.transient = Some(Err(msg.clone()));
                        self.log(format!("[Transient] error: {}", msg), true);
                    }
                }
            }
            Err(e) => self.log(format!("[Transient] build error: {}", e), true),
        }
    }

    fn run_ac(&mut self) {
        self.log("[AC] start", false);
        match self.build_circuit_from_layout() {
            Ok((circuit, component_nodes)) => {
                self.sim.last_component_nodes = component_nodes;
                self.sim.bode.clear();

                let points = self.sim.ac_points.max(2);
                let start_f = self.sim.ac_start.max(1e-9);
                let stop_f = self.sim.ac_stop.max(start_f * 1.01);
                let ratio = (stop_f / start_f).powf(1.0 / (points as f64 - 1.0));
                let mut f = start_f;
                for _ in 0..points {
                    if let Ok(res) = circuit.build_ac(f).solve() {
                        self.sim.bode.push((f, res));
                    }
                    f *= ratio;
                }
                self.log(format!("[AC] done, points={}", self.sim.bode.len()), false);
            }
            Err(e) => self.log(format!("[AC] build error: {}", e), true),
        }
    }
}

impl eframe::App for ResistApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle DELETE key
        if ctx.input(|i| i.key_pressed(egui::Key::Delete)) {
            if matches!(self.sim.selection, SelectedEntity::Component(_)) {
                self.remove_selected_component();
            }
        }

        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading(
                    egui::RichText::new("Resist - Visual Circuit Builder")
                        .color(egui::Color32::from_rgb(120, 206, 255)),
                );
                ui.separator();

                if ui.button("Run DC").clicked() {
                    self.run_dc();
                }
                if ui.button("Run Transient").clicked() {
                    self.run_transient();
                }
                if ui.button("Run AC").clicked() {
                    self.run_ac();
                }
                if ui.button("Clear Results").clicked() {
                    self.sim.dc = None;
                    self.sim.transient = None;
                    self.sim.bode.clear();
                    self.sim.iv_sweeps.clear();
                    self.sim.last_component_nodes.clear();
                    self.log("Results cleared", false);
                }

                ui.separator();
                ui.selectable_value(&mut self.sim.active_tab, PlotTab::Transient, "Transient");
                ui.selectable_value(&mut self.sim.active_tab, PlotTab::Bode, "Bode");
                ui.selectable_value(&mut self.sim.active_tab, PlotTab::IvCurve, "I-V");
            });
        });

        egui::SidePanel::left("palette")
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.heading("Palette");
                ui.separator();

                ui.label("Editor mode");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.sim.editor_mode, EditorMode::Select, "Select/Drag");
                    ui.selectable_value(&mut self.sim.editor_mode, EditorMode::Wire, "Wire");
                });

                if ui.button("Cancel pending wire").clicked() {
                    self.sim.pending_wire = None;
                }

                ui.separator();
                ui.label("Add block");
                if ui.button("+ Resistor").clicked() {
                    self.add_component(ComponentKind::Resistor);
                }
                if ui.button("+ Capacitor").clicked() {
                    self.add_component(ComponentKind::Capacitor);
                }
                if ui.button("+ Inductor").clicked() {
                    self.add_component(ComponentKind::Inductor);
                }
                if ui.button("+ Voltage Source").clicked() {
                    self.add_component(ComponentKind::VoltageSource);
                }
                if ui.button("+ Current Source").clicked() {
                    self.add_component(ComponentKind::CurrentSource);
                }
                
                ui.label("Functional sources");
                if ui.button("+ V(t) Function").clicked() {
                    self.add_component(ComponentKind::FunctionalVoltageSource);
                }
                if ui.button("+ I(t) Function").clicked() {
                    self.add_component(ComponentKind::FunctionalCurrentSource);
                }
                
                if ui.button("+ Ground").clicked() {
                    self.add_component(ComponentKind::Ground);
                }

                ui.separator();
                if ui.button("Delete selected block").clicked() {
                    self.remove_selected_component();
                }
                if ui.button("Clear all wires").clicked() {
                    self.sim.layout.wires.clear();
                    self.sim.pending_wire = None;
                }

                ui.separator();
                ui.heading("Simulation settings");
                ui.label("Transient stop [s]");
                ui.add(egui::DragValue::new(&mut self.sim.transient_stop).speed(1e-4));
                ui.label("Transient step [s]");
                ui.add(egui::DragValue::new(&mut self.sim.transient_step).speed(1e-6));
                ui.label("AC start [Hz]");
                ui.add(egui::DragValue::new(&mut self.sim.ac_start).speed(10.0));
                ui.label("AC stop [Hz]");
                ui.add(egui::DragValue::new(&mut self.sim.ac_stop).speed(1000.0));
                ui.label("AC points");
                ui.add(egui::DragValue::new(&mut self.sim.ac_points).range(2..=500));
            });

        egui::SidePanel::right("details")
            .default_width(290.0)
            .show(ctx, |ui| {
                ui.heading("Inspector");
                ui.separator();

                match &self.sim.selection {
                    SelectedEntity::Component(id) => {
                        ui.label(format!("Selected block: {}", id));
                        if let Some(comp) = self.sim.layout.components.iter_mut().find(|c| &c.id == id) {
                            ui.label(format!("Type: {}", comp.kind.label()));
                            
                            match comp.kind {
                                ComponentKind::FunctionalVoltageSource | ComponentKind::FunctionalCurrentSource => {
                                    ui.label("Expression (use 't' for time)");
                                    if let Some(expr) = &mut comp.expression {
                                        ui.text_edit_singleline(expr);
                                        ui.label("Examples:\n  if t > 1m then 5 else 0\n  5 * sin(2*pi*1k*t)\n  t < 100u ? 10 : 0");
                                    }
                                }
                                _ if comp.kind != ComponentKind::Ground => {
                                    ui.label("Value");
                                    ui.add(egui::DragValue::new(&mut comp.value).speed(0.1));
                                }
                                _ => {}
                            }
                            
                            if ui.button("Rotate 90 deg").clicked() {
                                comp.rotation = comp.rotation.next();
                            }
                        }
                    }
                    SelectedEntity::Node(n) => {
                        ui.label(format!("Selected node: {:?}", n));
                    }
                    SelectedEntity::NodePair(a, b) => {
                        ui.label(format!("Selected node pair: {:?} -> {:?}", a, b));
                    }
                    SelectedEntity::None => {
                        ui.label("Nothing selected");
                    }
                }

                ui.separator();
                draw_multimeter(ui, &self.sim);

                ui.separator();
                ui.heading("Console");
                egui::ScrollArea::vertical()
                    .id_salt("console_output")
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for line in &self.sim.console_output {
                            ui.colored_label(
                                if line.is_error {
                                    egui::Color32::from_rgb(255, 120, 120)
                                } else {
                                    egui::Color32::from_rgb(176, 226, 184)
                                },
                                &line.text,
                            );
                        }
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_height();

            ui.allocate_ui(egui::vec2(ui.available_width(), available * 0.58), |ui| {
                ui.heading("Schematic");
                ui.separator();
                schematic::draw_schematic(ui, &mut self.sim);
            });

            ui.separator();

            ui.allocate_ui(egui::vec2(ui.available_width(), ui.available_height()), |ui| {
                plot_panel::draw_plot(ui, &self.sim);
            });
        });
    }
}

fn draw_multimeter(ui: &mut egui::Ui, sim: &SimState) {
    ui.heading("Multimeter");
    match &sim.selection {
        SelectedEntity::None => {
            ui.label("Select node or block to inspect voltage.");
        }
        SelectedEntity::Node(n) => {
            if let Some(dc) = &sim.dc {
                let v = dc.node_voltages.get(n).copied().unwrap_or(0.0);
                ui.label(format!("DC V(node) = {:.5} V", v));
            } else {
                ui.label("Run DC to inspect node voltage.");
            }
        }
        SelectedEntity::NodePair(a, b) => {
            if let Some(dc) = &sim.dc {
                let va = dc.node_voltages.get(a).copied().unwrap_or(0.0);
                let vb = dc.node_voltages.get(b).copied().unwrap_or(0.0);
                ui.label(format!("V(a) = {:.5} V", va));
                ui.label(format!("V(b) = {:.5} V", vb));
                ui.colored_label(egui::Color32::YELLOW, format!("V(a)-V(b) = {:.5} V", va - vb));
            } else {
                ui.label("Run DC to inspect delta voltage.");
            }
        }
        SelectedEntity::Component(id) => {
            if let (Some(dc), Some(nodes)) = (sim.dc.as_ref(), sim.last_component_nodes.get(id)) {
                let va = nodes
                    .first()
                    .and_then(|n| dc.node_voltages.get(n))
                    .copied()
                    .unwrap_or(0.0);
                let vb = nodes
                    .get(1)
                    .and_then(|n| dc.node_voltages.get(n))
                    .copied()
                    .unwrap_or(0.0);
                ui.label(format!("V(pin1) = {:.5} V", va));
                ui.label(format!("V(pin2) = {:.5} V", vb));
                ui.colored_label(egui::Color32::YELLOW, format!("dV = {:.5} V", va - vb));
            } else {
                ui.label("Run DC to inspect selected block.");
            }
        }
    }
}

struct Dsu {
    parent: Vec<usize>,
    rank: Vec<u8>,
}

impl Dsu {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            rank: vec![0; n],
        }
    }

    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            let p = self.parent[x];
            self.parent[x] = self.find(p);
        }
        self.parent[x]
    }

    fn union(&mut self, a: usize, b: usize) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra == rb {
            return;
        }

        if self.rank[ra] < self.rank[rb] {
            self.parent[ra] = rb;
        } else if self.rank[rb] < self.rank[ra] {
            self.parent[rb] = ra;
        } else {
            self.parent[rb] = ra;
            self.rank[ra] += 1;
        }
    }
}
