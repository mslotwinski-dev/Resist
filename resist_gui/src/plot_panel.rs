use eframe::egui;
use egui_plot::{Legend, Line, Plot, PlotPoints};

use crate::sim_state::{PlotTab, SimState};

/// Palette of vivid waveform colors.
const COLORS: &[(u8, u8, u8)] = &[
    (0, 200, 255),   // cyan
    (255, 100, 100),  // red
    (100, 255, 130),  // green
    (255, 200, 60),   // amber
    (180, 120, 255),  // purple
    (255, 140, 200),  // pink
    (60, 220, 180),   // teal
    (255, 180, 80),   // orange
];

fn color(i: usize) -> egui::Color32 {
    let (r, g, b) = COLORS[i % COLORS.len()];
    egui::Color32::from_rgb(r, g, b)
}

pub fn draw_plot(ui: &mut egui::Ui, sim: &SimState) {
    match sim.active_tab {
        PlotTab::Transient => draw_transient(ui, sim),
        PlotTab::Bode => draw_bode(ui, sim),
        PlotTab::IvCurve => draw_iv(ui, sim),
    }
}

fn draw_transient(ui: &mut egui::Ui, sim: &SimState) {
    let tr = match &sim.transient {
        Some(Ok(res)) => res,
        Some(Err(err_msg)) => {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new(format!("🚨 Simulation Failed: {}", err_msg))
                        .color(egui::Color32::RED)
                        .strong()
                        .size(18.0),
                );
            });
            return;
        }
        None => {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("No transient data loaded.")
                        .color(egui::Color32::from_rgb(120, 120, 140))
                        .size(16.0),
                );
            });
            return;
        }
    };

    ui.heading(
        egui::RichText::new("⏱ Transient Waveforms")
            .color(egui::Color32::from_rgb(100, 200, 255)),
    );

    Plot::new("transient_plot")
        .legend(Legend::default())
        .x_axis_label("Time (s)")
        .y_axis_label("Voltage (V)")
        .show(ui, |plot_ui| {
            use crate::sim_state::SelectedEntity;
            
            let pts: PlotPoints = match &sim.selection {
                SelectedEntity::None => return,
                SelectedEntity::Node(n) => {
                    tr.time_points.iter().map(|tp| {
                        let v = tp.node_voltages.get(n).copied().unwrap_or(0.0);
                        [tp.time, v]
                    }).collect()
                }
                SelectedEntity::NodePair(n1, n2) => {
                    tr.time_points.iter().map(|tp| {
                        let v1 = tp.node_voltages.get(n1).copied().unwrap_or(0.0);
                        let v2 = tp.node_voltages.get(n2).copied().unwrap_or(0.0);
                        [tp.time, v1 - v2]
                    }).collect()
                }
                SelectedEntity::Component(id) => {
                    if let Some(comp) = sim.layout.components.iter().find(|c| &c.id == id) {
                        tr.time_points.iter().map(|tp| {
                            let v1 = tp.node_voltages.get(&comp.node_a).copied().unwrap_or(0.0);
                            let v2 = tp.node_voltages.get(&comp.node_b).copied().unwrap_or(0.0);
                            [tp.time, v1 - v2] // Plotting voltage drop
                        }).collect()
                    } else {
                        return;
                    }
                }
            };

            let label = match &sim.selection {
                SelectedEntity::Node(n) => format!("V(Node {:?})", n),
                SelectedEntity::NodePair(n1, n2) => format!("V({:?}) - V({:?})", n1, n2),
                SelectedEntity::Component(id) => format!("ΔV({})", id),
                _ => String::new(),
            };

            plot_ui.line(
                Line::new(pts)
                    .name(&label)
                    .color(color(0))
                    .width(2.0),
            );
        });
}

fn draw_bode(ui: &mut egui::Ui, sim: &SimState) {
    if sim.bode.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label(
                egui::RichText::new("No Bode data loaded.")
                    .color(egui::Color32::from_rgb(120, 120, 140))
                    .size(16.0),
            );
        });
        return;
    }

    ui.heading(
        egui::RichText::new("📊 Bode Plot")
            .color(egui::Color32::from_rgb(255, 200, 60)),
    );

    // Magnitude plot
    ui.label(egui::RichText::new("Magnitude").size(13.0));
    Plot::new("bode_mag")
        .legend(Legend::default())
        .x_axis_label("Frequency (Hz) [log]")
        .y_axis_label("Magnitude (dB)")
        .height(ui.available_height() / 2.0 - 30.0)
        .show(ui, |plot_ui| {
            use crate::sim_state::SelectedEntity;
            let pts: PlotPoints = match &sim.selection {
                SelectedEntity::Node(n) => sim.bode.iter().map(|(f, res)| {
                    [f.log10(), res.magnitude_db(*n)]
                }).collect(),
                SelectedEntity::NodePair(n1, n2) => sim.bode.iter().map(|(f, res)| {
                    let c1 = res.node_voltages.get(n1).copied().unwrap_or_default();
                    let c2 = res.node_voltages.get(n2).copied().unwrap_or_default();
                    let diff = c1 - c2;
                    let mag = diff.norm();
                    let mag_db = if mag > 1e-15 { 20.0 * mag.log10() } else { -300.0 };
                    [f.log10(), mag_db]
                }).collect(),
                SelectedEntity::Component(id) => {
                    if let Some(comp) = sim.layout.components.iter().find(|c| &c.id == id) {
                        sim.bode.iter().map(|(f, res)| {
                            let c1 = res.node_voltages.get(&comp.node_a).copied().unwrap_or_default();
                            let c2 = res.node_voltages.get(&comp.node_b).copied().unwrap_or_default();
                            let diff = c1 - c2;
                            let mag = diff.norm();
                            let mag_db = if mag > 1e-15 { 20.0 * mag.log10() } else { -300.0 };
                            [f.log10(), mag_db]
                        }).collect()
                    } else { return; }
                }
                _ => return,
            };

            let label = match &sim.selection {
                SelectedEntity::Node(n) => format!("|V(Node {:?})| dB", n),
                SelectedEntity::NodePair(n1, n2) => format!("|V({:?}) - V({:?})| dB", n1, n2),
                SelectedEntity::Component(id) => format!("|ΔV({})| dB", id),
                _ => String::new()
            };

            plot_ui.line(
                Line::new(pts)
                    .name(&label)
                    .color(color(0))
                    .width(2.0),
            );
        });

    // Phase plot
    ui.label(egui::RichText::new("Phase").size(13.0));
    Plot::new("bode_phase")
        .legend(Legend::default())
        .x_axis_label("Frequency (Hz) [log]")
        .y_axis_label("Phase (°)")
        .show(ui, |plot_ui| {
            use crate::sim_state::SelectedEntity;
            let pts: PlotPoints = match &sim.selection {
                SelectedEntity::Node(n) => sim.bode.iter().map(|(f, res)| {
                    [f.log10(), res.phase_deg(*n)]
                }).collect(),
                SelectedEntity::NodePair(n1, n2) => sim.bode.iter().map(|(f, res)| {
                    let c1 = res.node_voltages.get(n1).copied().unwrap_or_default();
                    let c2 = res.node_voltages.get(n2).copied().unwrap_or_default();
                    let diff = c1 - c2;
                    let phase = diff.arg().to_degrees();
                    [f.log10(), phase]
                }).collect(),
                SelectedEntity::Component(id) => {
                    if let Some(comp) = sim.layout.components.iter().find(|c| &c.id == id) {
                        sim.bode.iter().map(|(f, res)| {
                            let c1 = res.node_voltages.get(&comp.node_a).copied().unwrap_or_default();
                            let c2 = res.node_voltages.get(&comp.node_b).copied().unwrap_or_default();
                            let diff = c1 - c2;
                            let phase = diff.arg().to_degrees();
                            [f.log10(), phase]
                        }).collect()
                    } else { return; }
                }
                _ => return,
            };

            let label = match &sim.selection {
                SelectedEntity::Node(n) => format!("∠V(Node {:?}) °", n),
                SelectedEntity::NodePair(n1, n2) => format!("∠(V({:?}) - V({:?})) °", n1, n2),
                SelectedEntity::Component(id) => format!("∠ΔV({}) °", id),
                _ => String::new()
            };

            plot_ui.line(
                Line::new(pts)
                    .name(&label)
                    .color(color(3))
                    .width(2.0),
            );
        });
}

fn draw_iv(ui: &mut egui::Ui, sim: &SimState) {
    if sim.iv_sweeps.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label(
                egui::RichText::new("No I-V sweep data loaded.")
                    .color(egui::Color32::from_rgb(120, 120, 140))
                    .size(16.0),
            );
        });
        return;
    }

    ui.heading(
        egui::RichText::new("📐 I-V Characteristic Curves")
            .color(egui::Color32::from_rgb(100, 255, 130)),
    );

    Plot::new("iv_plot")
        .legend(Legend::default())
        .x_axis_label("Voltage (V)")
        .y_axis_label("Current (A)")
        .show(ui, |plot_ui| {
            for (i, (label, pts_data)) in sim.iv_sweeps.iter().enumerate() {
                let pts: PlotPoints = pts_data.iter().map(|p| [p.v, p.i]).collect();
                plot_ui.line(
                    Line::new(pts)
                        .name(label)
                        .color(color(i))
                        .width(2.0),
                );
            }
        });
}
