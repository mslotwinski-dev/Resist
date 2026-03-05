use std::collections::HashMap;

use eframe::egui;
use eframe::egui::{Color32, Pos2, Stroke, Vec2};

use crate::sim_state::{ComponentKind, Position, Rotation, SelectedEntity, SimState};

const COMP_COLOR: Color32 = Color32::from_rgb(180, 210, 240);
const WIRE_COLOR: Color32 = Color32::from_rgb(80, 180, 80);
const TEXT_COLOR: Color32 = Color32::from_rgb(200, 200, 220);
const SELECT_COLOR: Color32 = Color32::from_rgb(255, 80, 200);

pub fn world_to_screen(origin: Pos2, world: Position) -> Pos2 {
    Pos2::new(origin.x + world.x, origin.y + world.y)
}

pub fn screen_to_world(origin: Pos2, screen: Pos2) -> Position {
    Position::new(screen.x - origin.x, screen.y - origin.y)
}

fn rot(v: Vec2, r: Rotation) -> Vec2 {
    match r {
        Rotation::Deg0 => v,
        Rotation::Deg90 => Vec2::new(-v.y, v.x),
        Rotation::Deg180 => Vec2::new(-v.x, -v.y),
        Rotation::Deg270 => Vec2::new(v.y, -v.x),
    }
}

pub fn draw_schematic(ui: &mut egui::Ui, sim: &mut SimState) {
    let layout = &sim.layout;
    if layout.components.is_empty() && layout.wires.is_empty() {
        ui.label(
            egui::RichText::new("No schematic layout defined.")
                .color(Color32::from_rgb(120, 120, 140)),
        );
        return;
    }

    let (response, painter) =
        ui.allocate_painter(ui.available_size(), egui::Sense::click());
    let origin = response.rect.left_top() + Vec2::new(100.0, 100.0);

    // 1. Draw grid dots
    let cols = 1000;
    let rows = 600;
    for r in (-100..rows).step_by(10) {
        for c in (-100..cols).step_by(10) {
            let p = world_to_screen(origin, Position::new(c as f32, r as f32));
            painter.circle_filled(p, 1.0, Color32::from_rgb(40, 40, 50));
        }
    }

    // 2. Draw wires
    for wire in &layout.wires {
        let pa = world_to_screen(origin, wire.start);
        let pb = world_to_screen(origin, wire.end);
        painter.line_segment([pa, pb], Stroke::new(2.0, WIRE_COLOR));
    }

    // 3. Draw junction dots (explicitly defined)
    for &pt in &layout.junctions {
        let px = world_to_screen(origin, pt);
        painter.circle_filled(px, 3.5, WIRE_COLOR);
    }

    // 4. Draw components
    for comp in &layout.components {
        let center = world_to_screen(origin, comp.pos);
        
        let is_selected = matches!(&sim.selection, SelectedEntity::Component(id) if id == &comp.id);
        
        if is_selected {
            painter.rect_stroke(
                egui::Rect::from_center_size(center, Vec2::new(50.0, 50.0)),
                4.0,
                Stroke::new(2.0, SELECT_COLOR),
                egui::StrokeKind::Middle,
            );
        }

        match &comp.kind {
            ComponentKind::Resistor(_) => draw_resistor(&painter, center, comp.rotation),
            ComponentKind::Capacitor(_) => draw_capacitor(&painter, center, comp.rotation),
            ComponentKind::Inductor(_) => draw_inductor(&painter, center, comp.rotation),
            ComponentKind::VoltageSource(_) | ComponentKind::TransientSource => {
                if comp.name.starts_with("VCC") || comp.name.starts_with("VDD") {
                    draw_vcc(&painter, center, comp.rotation, &comp.name);
                } else {
                    draw_voltage_source(&painter, center, comp.rotation);
                }
            }
            ComponentKind::CurrentSource(_) => draw_current_source(&painter, center, comp.rotation),
            ComponentKind::Diode => draw_diode(&painter, center, comp.rotation),
            ComponentKind::Bjt { is_npn } => draw_bjt(&painter, center, comp.rotation, *is_npn),
            ComponentKind::Mosfet { is_nmos } => draw_mosfet(&painter, center, comp.rotation, *is_nmos),
            ComponentKind::OpAmp => draw_opamp(&painter, center, comp.rotation),
        }

        // Component label
        let label_offset = rot(Vec2::new(20.0, -15.0), comp.rotation);
        painter.text(
            center + label_offset,
            egui::Align2::LEFT_CENTER,
            &comp.name,
            egui::FontId::monospace(12.0),
            TEXT_COLOR,
        );
    }

    // Explicit GND (Node 0)
    for (&node_id, &pos) in &layout.node_positions {
        let is_selected = match &sim.selection {
            SelectedEntity::Node(n) => *n == node_id,
            SelectedEntity::NodePair(n1, n2) => *n1 == node_id || *n2 == node_id,
            _ => false,
        };
        
        let px = world_to_screen(origin, pos);

        if is_selected {
            painter.circle_stroke(px, 12.0, Stroke::new(2.5, SELECT_COLOR));
        }

        if node_id == resist::NodeId::GROUND {
            painter.line_segment([px + Vec2::new(-12.0, 0.0), px + Vec2::new(12.0, 0.0)], Stroke::new(2.0, WIRE_COLOR));
            painter.line_segment([px + Vec2::new(-8.0, 4.0), px + Vec2::new(8.0, 4.0)], Stroke::new(2.0, WIRE_COLOR));
            painter.line_segment([px + Vec2::new(-4.0, 8.0), px + Vec2::new(4.0, 8.0)], Stroke::new(2.0, WIRE_COLOR));
        }
    }

    // 5. Interaction (Hit testing)
    let pointer_pos = response.hover_pos();
    let clicked = response.clicked();
    let shift_down = ui.input(|i| i.modifiers.shift);

    if let Some(pos) = pointer_pos {
        let mut hovered_entity = None;

        // Check nodes first (tighter hit radius)
        for (&node_id, &node_pos) in &layout.node_positions {
            let px = world_to_screen(origin, node_pos);
            if (pos - px).length() < 15.0 {
                hovered_entity = Some(SelectedEntity::Node(node_id));
                break; // Only test closest
            }
        }

        // Check components (larger bounding box)
        if hovered_entity.is_none() {
            for comp in &layout.components {
                let center = world_to_screen(origin, comp.pos);
                if (pos - center).length() < 25.0 {
                    hovered_entity = Some(SelectedEntity::Component(comp.id.clone()));
                    break;
                }
            }
        }

        if hovered_entity.is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        if clicked {
            if let Some(entity) = hovered_entity.clone() {
                match (&sim.selection, entity) {
                    (SelectedEntity::Node(n1), SelectedEntity::Node(n2)) if shift_down && n1 != &n2 => {
                        sim.selection = SelectedEntity::NodePair(*n1, n2);
                    }
                    (_, new_entity) => {
                        sim.selection = new_entity;
                    }
                }
            } else if !shift_down {
                sim.selection = SelectedEntity::None;
            }
        }

        // Hover tooltips for Nodes
        if let Some(SelectedEntity::Node(hovered_node)) = hovered_entity {
            if let Some(dc) = &sim.dc {
                if let Some(&v) = dc.node_voltages.get(&hovered_node) {
                    let node_pos = layout.node_positions.get(&hovered_node).unwrap();
                    let px = world_to_screen(origin, *node_pos);
                    
                    let label = format!("Node {:?}: {:.4} V", hovered_node, v);
                    painter.rect_filled(
                        egui::Rect::from_min_size(px + Vec2::new(10.0, -25.0), Vec2::new(120.0, 20.0)),
                        2.0,
                        Color32::from_black_alpha(200),
                    );
                    painter.text(
                        px + Vec2::new(15.0, -15.0),
                        egui::Align2::LEFT_CENTER,
                        label,
                        egui::FontId::monospace(12.0),
                        Color32::WHITE,
                    );
                }
            }
        }
    }
}

// All components are assumed to be 4 grid units long (-2 to +2) in the local X direction for 2-pin devices.
// The anchor is exactly at (0,0).

fn draw_resistor(painter: &egui::Painter, c: Pos2, r: Rotation) {
    let p1 = c + rot(Vec2::new(-20.0, 0.0), r);
    let p2 = c + rot(Vec2::new(20.0, 0.0), r);
    
    let dir = (p2 - p1).normalized();
    let perp = Vec2::new(-dir.y, dir.x) * 6.0;
    
    let step = (p2 - p1) / 6.0;
    let mut pts = vec![p1];
    
    for i in 1..6 {
        let base = p1 + step * (i as f32);
        let offset = if i % 2 == 1 { perp } else { -perp };
        pts.push(base + offset);
    }
    pts.push(p2);

    for w in pts.windows(2) {
        painter.line_segment([w[0], w[1]], Stroke::new(2.0, COMP_COLOR));
    }
}

fn draw_capacitor(painter: &egui::Painter, c: Pos2, r: Rotation) {
    let gap = rot(Vec2::new(4.0, 0.0), r);
    let plate = rot(Vec2::new(0.0, 12.0), r);
    
    let p1 = c - rot(Vec2::new(20.0, 0.0), r);
    let p2 = c + rot(Vec2::new(20.0, 0.0), r);

    painter.line_segment([p1, c - gap], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([p2, c + gap], Stroke::new(2.0, COMP_COLOR));

    painter.line_segment([c - gap - plate, c - gap + plate], Stroke::new(3.0, COMP_COLOR));
    painter.line_segment([c + gap - plate, c + gap + plate], Stroke::new(3.0, COMP_COLOR));
}

fn draw_inductor(painter: &egui::Painter, c: Pos2, r: Rotation) {
    let p1 = c - rot(Vec2::new(20.0, 0.0), r);
    let p2 = c + rot(Vec2::new(20.0, 0.0), r);
    painter.line_segment([p1, p2], Stroke::new(2.5, COMP_COLOR));
    
    // Draw 3 semicircles
    for i in 0..3 {
        let frac = (i as f32 + 0.5) / 3.0;
        let mid = p1 + (p2 - p1) * frac;
        painter.circle_stroke(mid, 5.0, Stroke::new(1.5, COMP_COLOR));
    }
}

fn draw_voltage_source(painter: &egui::Painter, c: Pos2, r: Rotation) {
    let p1 = c - rot(Vec2::new(20.0, 0.0), r);
    let p2 = c + rot(Vec2::new(20.0, 0.0), r);
    let edge1 = c - rot(Vec2::new(14.0, 0.0), r);
    let edge2 = c + rot(Vec2::new(14.0, 0.0), r);
    
    painter.line_segment([p1, edge1], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([p2, edge2], Stroke::new(2.0, COMP_COLOR));
    
    painter.circle_stroke(c, 14.0, Stroke::new(2.0, COMP_COLOR));
    
    // Assuming node_a is positive, which corresponds to the first pin (p1 usually, but let's just draw generic)
    painter.text(c - rot(Vec2::new(6.0, 0.0), r), egui::Align2::CENTER_CENTER, "+", egui::FontId::monospace(12.0), COMP_COLOR);
    painter.text(c + rot(Vec2::new(6.0, 0.0), r), egui::Align2::CENTER_CENTER, "−", egui::FontId::monospace(12.0), COMP_COLOR);
}

fn draw_vcc(painter: &egui::Painter, c: Pos2, r: Rotation, _name: &str) {
    // Upward arrow
    let up = rot(Vec2::new(0.0, -20.0), r);
    let left = rot(Vec2::new(-6.0, -10.0), r);
    let right = rot(Vec2::new(6.0, -10.0), r);
    
    painter.line_segment([c, c + up], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + up, c + left], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + up, c + right], Stroke::new(2.0, COMP_COLOR));
}

fn draw_current_source(painter: &egui::Painter, c: Pos2, r: Rotation) {
    let p1 = c - rot(Vec2::new(20.0, 0.0), r);
    let p2 = c + rot(Vec2::new(20.0, 0.0), r);
    let edge1 = c - rot(Vec2::new(14.0, 0.0), r);
    let edge2 = c + rot(Vec2::new(14.0, 0.0), r);
    
    painter.line_segment([p1, edge1], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([p2, edge2], Stroke::new(2.0, COMP_COLOR));
    
    painter.circle_stroke(c, 14.0, Stroke::new(2.0, COMP_COLOR));
    
    // Arrow pointing from positive to negative
    let arrow_dir = rot(Vec2::new(1.0, 0.0), r);
    painter.line_segment([c - arrow_dir * 6.0, c + arrow_dir * 6.0], Stroke::new(2.0, COMP_COLOR));
    let head1 = c + arrow_dir * 6.0 + rot(Vec2::new(-4.0, -4.0), r);
    let head2 = c + arrow_dir * 6.0 + rot(Vec2::new(-4.0, 4.0), r);
    painter.line_segment([c + arrow_dir * 6.0, head1], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + arrow_dir * 6.0, head2], Stroke::new(2.0, COMP_COLOR));
}

fn draw_diode(painter: &egui::Painter, c: Pos2, r: Rotation) {
    let p1 = c - rot(Vec2::new(20.0, 0.0), r); // Anode
    let p2 = c + rot(Vec2::new(20.0, 0.0), r); // Cathode
    
    let mid_a = c - rot(Vec2::new(6.0, 0.0), r);
    let mid_c = c + rot(Vec2::new(6.0, 0.0), r);
    
    painter.line_segment([p1, mid_a], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([p2, mid_c], Stroke::new(2.0, COMP_COLOR));
    
    let perp = rot(Vec2::new(0.0, 8.0), r);
    
    let tri = [mid_a + perp, mid_a - perp, mid_c];
    painter.add(egui::Shape::convex_polygon(
        tri.to_vec(),
        COMP_COLOR,
        Stroke::NONE,
    ));
    
    painter.line_segment([mid_c + perp, mid_c - perp], Stroke::new(2.5, COMP_COLOR));
}

fn draw_bjt(painter: &egui::Painter, c: Pos2, r: Rotation, is_npn: bool) {
    // Center is (0,0). 
    // Base pin: (-20, 0).
    // Coll pin: (+20, -20). (Top right relative to base)
    // Emit pin: (+20, +20).
    
    let base_pin = c + rot(Vec2::new(-20.0, 0.0), r);
    let coll_pin = c + rot(Vec2::new(20.0, -20.0), r);
    let emit_pin = c + rot(Vec2::new(20.0, 20.0), r);
    
    painter.circle_stroke(c, 16.0, Stroke::new(1.5, COMP_COLOR));
    
    // Base wire
    painter.line_segment([base_pin, c + rot(Vec2::new(-6.0, 0.0), r)], Stroke::new(2.0, COMP_COLOR));
    
    // Base plate
    painter.line_segment(
        [c + rot(Vec2::new(-6.0, -10.0), r), c + rot(Vec2::new(-6.0, 10.0), r)],
        Stroke::new(2.5, COMP_COLOR),
    );
    
    // Collector wire inside circle
    painter.line_segment(
        [c + rot(Vec2::new(-6.0, -6.0), r), c + rot(Vec2::new(10.0, -14.0), r)],
        Stroke::new(2.0, COMP_COLOR),
    );
    // Collector wire outside
    painter.line_segment(
        [c + rot(Vec2::new(10.0, -14.0), r), coll_pin],
        Stroke::new(2.0, COMP_COLOR),
    );
    
    // Emitter wire inside circle
    painter.line_segment(
        [c + rot(Vec2::new(-6.0, 6.0), r), c + rot(Vec2::new(10.0, 14.0), r)],
        Stroke::new(2.0, COMP_COLOR),
    );
    // Emitter wire outside
    painter.line_segment(
        [c + rot(Vec2::new(10.0, 14.0), r), emit_pin],
        Stroke::new(2.0, COMP_COLOR),
    );

    // Emitter Arrow
    if is_npn {
        let arrow_pt = c + rot(Vec2::new(10.0, 14.0), r);
        let a1 = arrow_pt + rot(Vec2::new(-5.0, 0.0), r);
        let a2 = arrow_pt + rot(Vec2::new(0.0, -5.0), r);
        painter.line_segment([arrow_pt, a1], Stroke::new(2.0, COMP_COLOR));
        painter.line_segment([arrow_pt, a2], Stroke::new(2.0, COMP_COLOR));
    } else {
        let arrow_pt = c + rot(Vec2::new(-6.0, 6.0), r);
        let a1 = arrow_pt + rot(Vec2::new(5.0, 0.0), r);
        let a2 = arrow_pt + rot(Vec2::new(0.0, 5.0), r);
        painter.line_segment([arrow_pt, a1], Stroke::new(2.0, COMP_COLOR));
        painter.line_segment([arrow_pt, a2], Stroke::new(2.0, COMP_COLOR));
    }
}

fn draw_mosfet(painter: &egui::Painter, c: Pos2, r: Rotation, is_nmos: bool) {
    let gate_pin = c + rot(Vec2::new(-20.0, 0.0), r);
    let drain_pin = c + rot(Vec2::new(20.0, -20.0), r);
    let source_pin = c + rot(Vec2::new(20.0, 20.0), r);
    
    painter.circle_stroke(c, 16.0, Stroke::new(1.5, COMP_COLOR));
    
    // Gate wire
    painter.line_segment([gate_pin, c + rot(Vec2::new(-8.0, 0.0), r)], Stroke::new(2.0, COMP_COLOR));
    // Gate plate
    painter.line_segment(
        [c + rot(Vec2::new(-8.0, -10.0), r), c + rot(Vec2::new(-8.0, 10.0), r)],
        Stroke::new(2.5, COMP_COLOR),
    );
    
    // Channel segments (drain, bulk, source)
    painter.line_segment([c + rot(Vec2::new(-4.0, -10.0), r), c + rot(Vec2::new(-4.0, -6.0), r)], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot(Vec2::new(-4.0, -2.0), r), c + rot(Vec2::new(-4.0, 2.0), r)], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot(Vec2::new(-4.0, 6.0), r), c + rot(Vec2::new(-4.0, 10.0), r)], Stroke::new(2.0, COMP_COLOR));
    
    // Drain
    painter.line_segment([c + rot(Vec2::new(-4.0, -8.0), r), c + rot(Vec2::new(12.0, -8.0), r)], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot(Vec2::new(12.0, -8.0), r), c + rot(Vec2::new(12.0, -20.0), r)], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot(Vec2::new(12.0, -20.0), r), drain_pin], Stroke::new(2.0, COMP_COLOR));
    
    // Source
    painter.line_segment([c + rot(Vec2::new(-4.0, 8.0), r), c + rot(Vec2::new(12.0, 8.0), r)], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot(Vec2::new(12.0, 8.0), r), c + rot(Vec2::new(12.0, 20.0), r)], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot(Vec2::new(12.0, 20.0), r), source_pin], Stroke::new(2.0, COMP_COLOR));

    // Bulk (tied to source for this simple 3-terminal draw)
    painter.line_segment([c + rot(Vec2::new(-4.0, 0.0), r), c + rot(Vec2::new(12.0, 0.0), r)], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot(Vec2::new(12.0, 0.0), r), c + rot(Vec2::new(12.0, 8.0), r)], Stroke::new(2.0, COMP_COLOR));
    
    // Arrow on bulk (PMOS arrow points out from channel, NMOS points in to channel)
    // Since NMOS bulk is P-type, arrow is P -> N (Bulk to Channel)
    let bulk_pt = c + rot(Vec2::new(-4.0, 0.0), r);
    if is_nmos {
        let a1 = bulk_pt + rot(Vec2::new(4.0, -3.0), r);
        let a2 = bulk_pt + rot(Vec2::new(4.0, 3.0), r);
        painter.line_segment([bulk_pt, a1], Stroke::new(2.0, COMP_COLOR));
        painter.line_segment([bulk_pt, a2], Stroke::new(2.0, COMP_COLOR));
    } else {
        let arrow_pt = c + rot(Vec2::new(4.0, 0.0), r);
        let a1 = arrow_pt + rot(Vec2::new(-4.0, -3.0), r);
        let a2 = arrow_pt + rot(Vec2::new(-4.0, 3.0), r);
        painter.line_segment([arrow_pt, a1], Stroke::new(2.0, COMP_COLOR));
        painter.line_segment([arrow_pt, a2], Stroke::new(2.0, COMP_COLOR));
    }
}

fn draw_opamp(painter: &egui::Painter, c: Pos2, r: Rotation) {
    let p1 = c + rot(Vec2::new(-16.0, -16.0), r);
    let p2 = c + rot(Vec2::new(-16.0, 16.0), r);
    let p3 = c + rot(Vec2::new(20.0, 0.0), r);

    // Triangle
    painter.add(egui::Shape::convex_polygon(
        vec![p1, p2, p3],
        Color32::TRANSPARENT,
        Stroke::new(2.0, COMP_COLOR),
    ));

    // IN+ (bottom)
    painter.line_segment([c + rot(Vec2::new(-16.0, 10.0), r), c + rot(Vec2::new(-24.0, 10.0), r)], Stroke::new(2.0, COMP_COLOR));
    painter.text(c + rot(Vec2::new(-10.0, 10.0), r), egui::Align2::LEFT_CENTER, "+", egui::FontId::monospace(10.0), COMP_COLOR);

    // IN- (top)
    painter.line_segment([c + rot(Vec2::new(-16.0, -10.0), r), c + rot(Vec2::new(-24.0, -10.0), r)], Stroke::new(2.0, COMP_COLOR));
    painter.text(c + rot(Vec2::new(-10.0, -10.0), r), egui::Align2::LEFT_CENTER, "−", egui::FontId::monospace(10.0), COMP_COLOR);

    // OUT
    painter.line_segment([p3, c + rot(Vec2::new(28.0, 0.0), r)], Stroke::new(2.0, COMP_COLOR));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinate_transforms() {
        let origin = Pos2::new(100.0, 100.0);
        let world = Position::new(50.0, 20.0);
        
        // World -> Screen
        let screen = world_to_screen(origin, world);
        assert_eq!(screen.x, 150.0);
        assert_eq!(screen.y, 120.0);
        
        // Screen -> World
        let back_to_world = screen_to_world(origin, screen);
        assert_eq!(back_to_world.x, world.x);
        assert_eq!(back_to_world.y, world.y);
    }
}
