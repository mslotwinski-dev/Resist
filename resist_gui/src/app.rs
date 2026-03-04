use eframe::egui;

use crate::plot_panel;
use crate::schematic;
use crate::sim_state::{PlotTab, SimState};

/// Top-level application state.
pub struct ResistApp {
    pub sim: SimState,
}

impl ResistApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Configure dark, premium visuals
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::from_rgb(18, 18, 24);
        visuals.window_fill = egui::Color32::from_rgb(24, 24, 32);
        visuals.extreme_bg_color = egui::Color32::from_rgb(12, 12, 16);
        cc.egui_ctx.set_visuals(visuals);

        Self {
            sim: SimState::default(),
        }
    }
}

impl eframe::App for ResistApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top menu bar
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(
                    egui::RichText::new("⚡ Resist Simulator")
                        .color(egui::Color32::from_rgb(100, 200, 255))
                        .size(18.0),
                );
                ui.separator();

                // Plot tab selector
                ui.selectable_value(
                    &mut self.sim.active_tab,
                    PlotTab::Transient,
                    "📈 Transient",
                );
                ui.selectable_value(&mut self.sim.active_tab, PlotTab::Bode, "📊 Bode");
                ui.selectable_value(&mut self.sim.active_tab, PlotTab::IvCurve, "📐 I-V Curve");
            });
        });

        // Left panel — Schematic
        egui::SidePanel::left("schematic_panel")
            .default_width(500.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading(
                    egui::RichText::new("Schematic")
                        .color(egui::Color32::from_rgb(180, 180, 200)),
                );
                ui.separator();
                schematic::draw_schematic(ui, &mut self.sim);
            });

        // Right panel — Properties / Multimeter
        egui::SidePanel::right("properties_panel")
            .default_width(250.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading(
                    egui::RichText::new("🪛 Multimeter")
                        .color(egui::Color32::from_rgb(180, 255, 180)),
                );
                ui.separator();

                match &self.sim.selection {
                    crate::sim_state::SelectedEntity::None => {
                        ui.label(egui::RichText::new("Nothing selected.").color(egui::Color32::DARK_GRAY));
                    }
                    crate::sim_state::SelectedEntity::Node(n) => {
                        ui.label(egui::RichText::new(format!("Node {:?}", n)).strong());
                        ui.separator();
                        if let Some(dc) = &self.sim.dc {
                            if let Some(&v) = dc.node_voltages.get(n) {
                                ui.label(format!("DC Voltage: {:.4} V", v));
                            } else {
                                ui.label("DC Voltage: 0.0000 V (GND)");
                            }
                        }
                    }
                    crate::sim_state::SelectedEntity::NodePair(n1, n2) => {
                        ui.label(egui::RichText::new(format!("Nodes {:?} → {:?}", n1, n2)).strong());
                        ui.separator();
                        if let Some(dc) = &self.sim.dc {
                            let v1 = dc.node_voltages.get(n1).copied().unwrap_or(0.0);
                            let v2 = dc.node_voltages.get(n2).copied().unwrap_or(0.0);
                            ui.label(format!("DC V_A: {:.4} V", v1));
                            ui.label(format!("DC V_B: {:.4} V", v2));
                            ui.label(egui::RichText::new(format!("ΔV: {:.4} V", v1 - v2)).color(egui::Color32::YELLOW));
                        }
                    }
                    crate::sim_state::SelectedEntity::Component(id) => {
                        ui.label(egui::RichText::new(format!("Component: {}", id)).strong());
                        ui.separator();
                        
                        // Find the component
                        if let Some(comp) = self.sim.layout.components.iter().find(|c| &c.id == id) {
                            if let Some(dc) = &self.sim.dc {
                                let va = dc.node_voltages.get(&comp.node_a).copied().unwrap_or(0.0);
                                let vb = dc.node_voltages.get(&comp.node_b).copied().unwrap_or(0.0);
                                let v_drop = va - vb;
                                
                                ui.label(format!("Node A ({:?}): {:.4} V", comp.node_a, va));
                                ui.label(format!("Node B ({:?}): {:.4} V", comp.node_b, vb));
                                ui.label(egui::RichText::new(format!("ΔV (A→B): {:.4} V", v_drop)).color(egui::Color32::YELLOW));
                                
                                // Approximate Current for basic components
                                match &comp.kind {
                                    crate::sim_state::ComponentKind::Resistor(r) => {
                                        let i = v_drop / r;
                                        ui.label(format!("Current (I): {:.4} mA", i * 1000.0));
                                        ui.label(format!("Power (P): {:.4} mW", (v_drop * i).abs() * 1000.0));
                                    }
                                    crate::sim_state::ComponentKind::Capacitor(_) => {
                                        ui.label("Current (I): 0.0000 mA (DC Block)");
                                    }
                                    _ => {
                                        ui.label(egui::RichText::new("Detailed non-linear currents evaluating...").italics());
                                    }
                                }
                            }
                        }
                    }
                }
            });

        // Central panel — Plot
        egui::CentralPanel::default().show(ctx, |ui| {
            plot_panel::draw_plot(ui, &self.sim);
        });
    }
}
