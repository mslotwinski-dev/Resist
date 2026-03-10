use eframe::egui;

use crate::plot_panel;
use crate::schematic;
use crate::sim_state::{
    PlotTab, SimState, ConsoleLine, ComponentInfo, ComponentKind, Position, Rotation,
    CircuitLayout,
};

use resist_lang::eval_api::{eval_script, AnalysisConfig};
use resist_lang::ast::{CompCtorType, AnalysisKind};
use resist::NodeId;

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

    /// Compile & run the source code through the ResistScript interpreter.
    fn compile_and_run(&mut self) {
        self.sim.console_output.clear();
        self.sim.console_output.push(ConsoleLine {
            text: "⚡ Compiling...".to_string(),
            is_error: false,
        });

        match eval_script(&self.sim.source_code) {
            Ok(result) => {
                // Log
                for line in &result.log {
                    self.sim.console_output.push(ConsoleLine {
                        text: line.clone(),
                        is_error: false,
                    });
                }

                // Convert layout entries to ComponentInfo for the schematic
                let mut layout = CircuitLayout::default();
                for entry in &result.layout {
                    let kind = match entry.comp_type {
                        CompCtorType::Resistor => {
                            ComponentKind::Resistor(0.0) // value not stored in layout
                        }
                        CompCtorType::Capacitor => ComponentKind::Capacitor(0.0),
                        CompCtorType::Inductor => ComponentKind::Inductor(0.0),
                        CompCtorType::VSource => ComponentKind::VoltageSource(0.0),
                        CompCtorType::ISource => ComponentKind::CurrentSource(0.0),
                        CompCtorType::Diode => ComponentKind::Diode,
                        CompCtorType::StepSource | CompCtorType::SineSource | CompCtorType::FuncVSource => ComponentKind::TransientSource,
                        CompCtorType::VCVS => ComponentKind::OpAmp,
                        CompCtorType::BJT => ComponentKind::Bjt { is_npn: true },
                        CompCtorType::MOSFET => ComponentKind::Mosfet { is_nmos: true },
                    };
                    let rotation = match entry.rotation {
                        90 => Rotation::Deg90,
                        180 => Rotation::Deg180,
                        270 => Rotation::Deg270,
                        _ => Rotation::Deg0,
                    };
                    layout.components.push(ComponentInfo {
                        id: entry.name.clone(),
                        name: entry.name.clone(),
                        kind,
                        pins: entry.nodes.clone(),
                        pos: Position::new(entry.x, entry.y),
                        rotation,
                    });
                }

                self.sim.layout = layout;

                // Run queued analyses
                let mut circuit = result.circuit;
                for config in &result.analyses {
                    self.run_analysis(&mut circuit, config, &result.nodes);
                }

                self.sim.console_output.push(ConsoleLine {
                    text: format!(
                        "\n✓ Done — {} components, {} nodes",
                        result.layout.len(),
                        result.nodes.len()
                    ),
                    is_error: false,
                });
            }
            Err(e) => {
                self.sim.console_output.push(ConsoleLine {
                    text: format!("✗ {}", e),
                    is_error: true,
                });
            }
        }
    }

    fn run_analysis(
        &mut self,
        circuit: &mut resist::Circuit,
        config: &AnalysisConfig,
        nodes: &std::collections::HashMap<String, NodeId>,
    ) {
        match config.kind {
            AnalysisKind::Dc => {
                self.sim.console_output.push(ConsoleLine {
                    text: "▶ Running DC Operating Point...".to_string(),
                    is_error: false,
                });
                match circuit.build_nonlinear().solve() {
                    Ok(result) => {
                        for (name, id) in nodes {
                            if *id != NodeId::GROUND {
                                if let Some(&v) = result.node_voltages.get(id) {
                                    self.sim.console_output.push(ConsoleLine {
                                        text: format!("  V({}) = {:.4} V", name, v),
                                        is_error: false,
                                    });
                                }
                            }
                        }
                        self.sim.dc = Some(result);
                    }
                    Err(e) => {
                        self.sim.console_output.push(ConsoleLine {
                            text: format!("  ✗ DC FAILED: {}", e),
                            is_error: true,
                        });
                    }
                }
            }
            AnalysisKind::Transient => {
                let stop = config.params.get("stop").copied().unwrap_or(1e-3);
                let step = config.params.get("step").copied().unwrap_or(1e-6);
                let uic = config.params.get("uic").copied().unwrap_or(0.0) > 0.5;
                self.sim.console_output.push(ConsoleLine {
                    text: format!("▶ Transient ({:.1e}s, {:.1e}s step, uic={})...", stop, step, uic),
                    is_error: false,
                });
                match circuit.build_transient(stop, step).with_uic(uic).solve() {
                    Ok(result) => {
                        // Collect selected nodes for plotting
                        self.sim.selected_nodes = nodes.iter()
                            .filter(|(_, id)| **id != NodeId::GROUND)
                            .map(|(name, id)| (*id, name.clone()))
                            .collect();
                        self.sim.console_output.push(ConsoleLine {
                            text: format!("  ✓ {} time points", result.time_points.len()),
                            is_error: false,
                        });
                        self.sim.transient = Some(Ok(result));
                    }
                    Err(e) => {
                        self.sim.console_output.push(ConsoleLine {
                            text: format!("  ✗ Transient FAILED: {}", e),
                            is_error: true,
                        });
                    }
                }
            }
            AnalysisKind::Ac => {
                let start_f = config.params.get("start").copied().unwrap_or(10.0);
                let stop_f = config.params.get("stop").copied().unwrap_or(1e6);
                let points = config.params.get("points").copied().unwrap_or(50.0) as usize;
                self.sim.console_output.push(ConsoleLine {
                    text: format!("▶ AC Sweep ({:.0} → {:.0} Hz)...", start_f, stop_f),
                    is_error: false,
                });
                let mut bode = Vec::new();
                let mut f = start_f;
                let ratio = (stop_f / start_f).powf(1.0 / (points.max(2) as f64 - 1.0));
                for _ in 0..points {
                    if let Ok(res) = circuit.build_ac(f).solve() {
                        bode.push((f, res));
                    }
                    f *= ratio;
                }
                self.sim.console_output.push(ConsoleLine {
                    text: format!("  ✓ {} frequency points", bode.len()),
                    is_error: false,
                });
                self.sim.bode = bode;
            }
        }
    }
}

fn highlight_code(theme: &egui::Style, text: &str) -> egui::text::LayoutJob {
    use egui::text::{LayoutJob, TextFormat};
    use egui::{Color32, FontId};
    use regex::Regex;

    let mut job = LayoutJob::default();
    let font_id = FontId::monospace(14.0);

    let re = Regex::new(
        r"(?P<comment>//[^\n]*)|(?P<keyword>\b(?:let|for|in|analyze|dc|transient|ac|if|else|true|false)\b)|(?P<component>\b(?:Resistor|Capacitor|Inductor|VSource|StepSource|ISource|Diode)\b)|(?P<number>\b\d+(\.\d+)?([eE][+-]?\d+)?(k|K|M|Meg|G|T|m|u|n|p|f|V|A|Hz|s)?\b)"
    ).unwrap();

    let mut last_end = 0;
    for cap in re.captures_iter(text) {
        let mat = cap.get(0).unwrap();
        
        if mat.start() > last_end {
            job.append(
                &text[last_end..mat.start()],
                0.0,
                TextFormat::simple(font_id.clone(), theme.visuals.text_color()),
            );
        }

        let color = if cap.name("comment").is_some() {
            Color32::from_rgb(100, 100, 100)
        } else if cap.name("keyword").is_some() {
            Color32::from_rgb(255, 119, 255)
        } else if cap.name("component").is_some() {
            Color32::from_rgb(137, 221, 255)
        } else if cap.name("number").is_some() {
            Color32::from_rgb(255, 184, 108)
        } else {
            theme.visuals.text_color()
        };

        job.append(
            mat.as_str(),
            0.0,
            TextFormat::simple(font_id.clone(), color),
        );
        last_end = mat.end();
    }

    if last_end < text.len() {
        job.append(
            &text[last_end..],
            0.0,
            TextFormat::simple(font_id.clone(), theme.visuals.text_color()),
        );
    }
    job
}

impl eframe::App for ResistApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── Top Bar ─────────────────────────────────────────────────
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(
                    egui::RichText::new("⚡ Resist IDE")
                        .color(egui::Color32::from_rgb(100, 200, 255))
                        .size(18.0),
                );
                ui.separator();

                // Compile & Run button
                let btn = ui.button(
                    egui::RichText::new("▶ Compile & Run")
                        .color(egui::Color32::from_rgb(80, 255, 120))
                        .strong(),
                );
                if btn.clicked() {
                    self.compile_and_run();
                }

                ui.separator();

                // Plot tab selector
                ui.selectable_value(&mut self.sim.active_tab, PlotTab::Transient, "📈 Transient");
                ui.selectable_value(&mut self.sim.active_tab, PlotTab::Bode, "📊 Bode");
                ui.selectable_value(&mut self.sim.active_tab, PlotTab::IvCurve, "📐 I-V Curve");
            });
        });

        let screen_width = ctx.screen_rect().width();

        // ── Left Panel: Code Editor + Console ───────────────────────
        egui::SidePanel::left("editor_panel")
            .default_width(screen_width * 0.35)
            .min_width(300.0)
            .resizable(true)
            .show(ctx, |ui| {
                // Code Editor
                ui.heading(
                    egui::RichText::new("📝 Editor")
                        .color(egui::Color32::from_rgb(180, 180, 220)),
                );
                ui.separator();

                egui::ScrollArea::vertical()
                    .id_salt("editor_scroll")
                    .max_height(ui.available_height() * 0.65)
                    .show(ui, |ui| {
                        let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
                            let mut layout_job = highlight_code(ui.style(), string);
                            layout_job.wrap.max_width = wrap_width;
                            ui.fonts(|f| f.layout_job(layout_job))
                        };

                        ui.add(
                            egui::TextEdit::multiline(&mut self.sim.source_code)
                                .font(egui::TextStyle::Monospace)
                                .code_editor()
                                .layouter(&mut layouter)
                                .desired_width(f32::INFINITY)
                                .desired_rows(20)
                                .lock_focus(true),
                        );
                    });

                ui.separator();

                // Console Output
                ui.heading(
                    egui::RichText::new("🖥 Console")
                        .color(egui::Color32::from_rgb(180, 220, 180))
                        .size(14.0),
                );

                egui::ScrollArea::vertical()
                    .id_salt("console_scroll")
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for line in &self.sim.console_output {
                            let color = if line.is_error {
                                egui::Color32::from_rgb(255, 80, 80)
                            } else {
                                egui::Color32::from_rgb(180, 200, 180)
                            };
                            ui.label(
                                egui::RichText::new(&line.text)
                                    .color(color)
                                    .font(egui::FontId::monospace(12.0)),
                            );
                        }
                        if self.sim.console_output.is_empty() {
                            ui.label(
                                egui::RichText::new("Press ▶ Compile & Run to execute the script.")
                                    .color(egui::Color32::DARK_GRAY)
                                    .italics(),
                            );
                        }
                    });
            });

        // ── Right Panel: Properties / Multimeter ────────────────────
        egui::SidePanel::right("properties_panel")
            .default_width(220.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading(
                    egui::RichText::new("🪛 Multimeter")
                        .color(egui::Color32::from_rgb(180, 255, 180)),
                );
                ui.separator();
                draw_multimeter(ui, &self.sim);
            });

        // ── Central Panel: Schematic (top) + Plot (bottom) ──────────
        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_height();

            // Top: Schematic
            ui.allocate_ui(egui::vec2(ui.available_width(), available * 0.55), |ui| {
                ui.heading(
                    egui::RichText::new("Schematic")
                        .color(egui::Color32::from_rgb(180, 180, 200))
                        .size(14.0),
                );
                ui.separator();
                schematic::draw_schematic(ui, &mut self.sim);
            });

            ui.separator();

            // Bottom: Plot
            ui.allocate_ui(egui::vec2(ui.available_width(), ui.available_height()), |ui| {
                plot_panel::draw_plot(ui, &self.sim);
            });
        });
    }
}

/// Draw the multimeter panel for the selected entity.
fn draw_multimeter(ui: &mut egui::Ui, sim: &SimState) {
    match &sim.selection {
        crate::sim_state::SelectedEntity::None => {
            ui.label(
                egui::RichText::new("Nothing selected.")
                    .color(egui::Color32::DARK_GRAY),
            );
        }
        crate::sim_state::SelectedEntity::Node(n) => {
            ui.label(egui::RichText::new(format!("Node {:?}", n)).strong());
            ui.separator();
            if let Some(dc) = &sim.dc {
                if let Some(&v) = dc.node_voltages.get(n) {
                    ui.label(format!("DC Voltage: {:.4} V", v));
                } else {
                    ui.label("DC Voltage: 0.0000 V (GND)");
                }
            }
        }
        crate::sim_state::SelectedEntity::NodePair(n1, n2) => {
            ui.label(
                egui::RichText::new(format!("{:?} → {:?}", n1, n2)).strong(),
            );
            ui.separator();
            if let Some(dc) = &sim.dc {
                let v1 = dc.node_voltages.get(n1).copied().unwrap_or(0.0);
                let v2 = dc.node_voltages.get(n2).copied().unwrap_or(0.0);
                ui.label(format!("V_A: {:.4} V", v1));
                ui.label(format!("V_B: {:.4} V", v2));
                ui.label(
                    egui::RichText::new(format!("ΔV: {:.4} V", v1 - v2))
                        .color(egui::Color32::YELLOW),
                );
            }
        }
        crate::sim_state::SelectedEntity::Component(id) => {
            ui.label(egui::RichText::new(format!("Component: {}", id)).strong());
            ui.separator();
            if let Some(comp) = sim.layout.components.iter().find(|c| &c.id == id) {
                if let Some(dc) = &sim.dc {
                    let va = comp.pins.get(0).and_then(|id| dc.node_voltages.get(id)).copied().unwrap_or(0.0);
                    let vb = comp.pins.get(1).and_then(|id| dc.node_voltages.get(id)).copied().unwrap_or(0.0);
                    let v_drop = va - vb;
                    ui.label(format!("Node A: {:.4} V", va));
                    ui.label(format!("Node B: {:.4} V", vb));
                    ui.label(
                        egui::RichText::new(format!("ΔV: {:.4} V", v_drop))
                            .color(egui::Color32::YELLOW),
                    );
                    if let ComponentKind::Resistor(r) = &comp.kind {
                        if *r > 0.0 {
                            let i = v_drop / r;
                            ui.label(format!("I: {:.4} mA", i * 1000.0));
                            ui.label(format!("P: {:.4} mW", (v_drop * i).abs() * 1000.0));
                        }
                    }
                }
            }
        }
    }
}
