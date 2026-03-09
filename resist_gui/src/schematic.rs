use std::collections::HashMap;

use eframe::egui;
use eframe::egui::{Color32, Pos2, Stroke, Vec2};

use resist::NodeId;
use crate::sim_state::{ComponentInfo, ComponentKind, Position, Rotation, SelectedEntity, SimState};

const COMP_COLOR: Color32 = Color32::from_rgb(180, 210, 240);
const WIRE_COLOR: Color32 = Color32::from_rgb(80, 180, 80);
const GND_COLOR: Color32 = Color32::from_rgb(100, 200, 100);
const TEXT_COLOR: Color32 = Color32::from_rgb(200, 200, 220);
const SELECT_COLOR: Color32 = Color32::from_rgb(255, 80, 200);
const JUNCTION_COLOR: Color32 = Color32::from_rgb(80, 220, 80);

/// Pin half-length: components extend ±20px from center along local X.
const PIN_OFFSET: f32 = 20.0;

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

/// Compute the two pin world-positions for a 2-terminal component.
fn pin_positions(comp: &ComponentInfo) -> (Position, Position) {
    let offset_a = rot(Vec2::new(-PIN_OFFSET, 0.0), comp.rotation);
    let offset_b = rot(Vec2::new(PIN_OFFSET, 0.0), comp.rotation);
    (
        Position::new(comp.pos.x + offset_a.x, comp.pos.y + offset_a.y),
        Position::new(comp.pos.x + offset_b.x, comp.pos.y + offset_b.y),
    )
}

pub fn draw_schematic(ui: &mut egui::Ui, sim: &mut SimState) {
    let layout = &sim.layout;
    if layout.components.is_empty() {
        ui.label(
            egui::RichText::new("No schematic. Click ▶ Compile & Run.")
                .color(Color32::from_rgb(120, 120, 140)),
        );
        return;
    }

    let (response, painter) =
        ui.allocate_painter(ui.available_size(), egui::Sense::click());
    let origin = response.rect.left_top() + Vec2::new(20.0, 20.0);

    // ── 1. Draw subtle grid ─────────────────────────────────────────
    {
        let rect = response.rect;
        let step = 20.0;
        let mut x = rect.left();
        while x < rect.right() {
            let mut y = rect.top();
            while y < rect.bottom() {
                painter.circle_filled(Pos2::new(x, y), 0.8, Color32::from_rgb(35, 35, 45));
                y += step;
            }
            x += step;
        }
    }

    // ── 2. Build net map: NodeId → Vec<screen Pos2> ─────────────────
    let mut net_map: HashMap<NodeId, Vec<Pos2>> = HashMap::new();

    for comp in &layout.components {
        let (pin_a, pin_b) = pin_positions(comp);
        let screen_a = world_to_screen(origin, pin_a);
        let screen_b = world_to_screen(origin, pin_b);

        net_map.entry(comp.node_a).or_default().push(screen_a);
        net_map.entry(comp.node_b).or_default().push(screen_b);
    }

    // ── 3. Auto-route wires: trunk-and-branch Manhattan ─────────────
    for (&node_id, pins) in &net_map {
        if pins.len() < 2 { continue; }
        if node_id == NodeId::GROUND {
            // GND pins get ground symbols instead of wires
            continue;
        }

        // Sort pins left-to-right, then top-to-bottom
        let mut sorted_pins = pins.clone();
        sorted_pins.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap().then(a.y.partial_cmp(&b.y).unwrap()));

        for i in 0..sorted_pins.len() - 1 {
            let p1 = sorted_pins[i];
            let p2 = sorted_pins[i + 1];

            // Horizontal segment first
            if (p1.x - p2.x).abs() > 0.5 {
                painter.line_segment(
                    [p1, Pos2::new(p2.x, p1.y)],
                    Stroke::new(2.0, WIRE_COLOR),
                );
            }
            // Vertical segment second
            if (p1.y - p2.y).abs() > 0.5 {
                painter.line_segment(
                    [Pos2::new(p2.x, p1.y), p2],
                    Stroke::new(2.0, WIRE_COLOR),
                );
            }

            // Draw a junction dot if there is a corner and it connects multiple branches
            if sorted_pins.len() > 2 {
                painter.circle_filled(p1, 3.0, JUNCTION_COLOR);
                painter.circle_filled(p2, 3.0, JUNCTION_COLOR);
            }
        }
    }

    // ── 4. Draw ground symbols at every GND pin ─────────────────────
    if let Some(gnd_pins) = net_map.get(&NodeId::GROUND) {
        for pin in gnd_pins {
            draw_ground_symbol(&painter, *pin);
        }
    }

    // ── 5. Draw components ──────────────────────────────────────────
    for comp in &layout.components {
        let center = world_to_screen(origin, comp.pos);

        let is_selected = matches!(
            &sim.selection,
            SelectedEntity::Component(id) if id == &comp.id
        );

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
                draw_voltage_source(&painter, center, comp.rotation);
            }
            ComponentKind::CurrentSource(_) => draw_current_source(&painter, center, comp.rotation),
            ComponentKind::Diode => draw_diode(&painter, center, comp.rotation),
            ComponentKind::Bjt { is_npn } => draw_bjt(&painter, center, comp.rotation, *is_npn),
            ComponentKind::Mosfet { is_nmos } => draw_mosfet(&painter, center, comp.rotation, *is_nmos),
            ComponentKind::OpAmp => draw_opamp(&painter, center, comp.rotation),
        }

        // Component label
        let label_offset = rot(Vec2::new(0.0, -25.0), comp.rotation);
        painter.text(
            center + label_offset,
            egui::Align2::CENTER_BOTTOM,
            &comp.name,
            egui::FontId::monospace(11.0),
            TEXT_COLOR,
        );
    }

    // ── 6. Interaction (Hit testing) ─────────────────────────────────
    let pointer_pos = response.hover_pos();
    let clicked = response.clicked();
    let shift_down = ui.input(|i| i.modifiers.shift);

    if let Some(pos) = pointer_pos {
        let mut hovered_entity: Option<SelectedEntity> = None;

        // Check components
        for comp in &layout.components {
            let center = world_to_screen(origin, comp.pos);
            if (pos - center).length() < 25.0 {
                hovered_entity = Some(SelectedEntity::Component(comp.id.clone()));
                break;
            }
        }

        if hovered_entity.is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        if clicked {
            if let Some(entity) = hovered_entity {
                match (&sim.selection, entity) {
                    (SelectedEntity::Node(n1), SelectedEntity::Node(n2))
                        if shift_down && n1 != &n2 =>
                    {
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
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Symbol Drawing Functions
// ═══════════════════════════════════════════════════════════════════════

/// Draw the standard 3-line ground symbol pointing downward at the given screen position.
fn draw_ground_symbol(painter: &egui::Painter, pos: Pos2) {
    // Vertical stub down
    painter.line_segment(
        [pos, pos + Vec2::new(0.0, 10.0)],
        Stroke::new(2.0, GND_COLOR),
    );
    // Three horizontal lines of decreasing width
    painter.line_segment(
        [pos + Vec2::new(-10.0, 10.0), pos + Vec2::new(10.0, 10.0)],
        Stroke::new(2.0, GND_COLOR),
    );
    painter.line_segment(
        [pos + Vec2::new(-6.0, 14.0), pos + Vec2::new(6.0, 14.0)],
        Stroke::new(2.0, GND_COLOR),
    );
    painter.line_segment(
        [pos + Vec2::new(-3.0, 18.0), pos + Vec2::new(3.0, 18.0)],
        Stroke::new(2.0, GND_COLOR),
    );
}

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
    painter.text(c - rot(Vec2::new(6.0, 0.0), r), egui::Align2::CENTER_CENTER, "+", egui::FontId::monospace(12.0), COMP_COLOR);
    painter.text(c + rot(Vec2::new(6.0, 0.0), r), egui::Align2::CENTER_CENTER, "−", egui::FontId::monospace(12.0), COMP_COLOR);
}

fn draw_current_source(painter: &egui::Painter, c: Pos2, r: Rotation) {
    let p1 = c - rot(Vec2::new(20.0, 0.0), r);
    let p2 = c + rot(Vec2::new(20.0, 0.0), r);
    let edge1 = c - rot(Vec2::new(14.0, 0.0), r);
    let edge2 = c + rot(Vec2::new(14.0, 0.0), r);
    painter.line_segment([p1, edge1], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([p2, edge2], Stroke::new(2.0, COMP_COLOR));
    painter.circle_stroke(c, 14.0, Stroke::new(2.0, COMP_COLOR));
    let arrow_dir = rot(Vec2::new(1.0, 0.0), r);
    painter.line_segment([c - arrow_dir * 6.0, c + arrow_dir * 6.0], Stroke::new(2.0, COMP_COLOR));
    let head1 = c + arrow_dir * 6.0 + rot(Vec2::new(-4.0, -4.0), r);
    let head2 = c + arrow_dir * 6.0 + rot(Vec2::new(-4.0, 4.0), r);
    painter.line_segment([c + arrow_dir * 6.0, head1], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + arrow_dir * 6.0, head2], Stroke::new(2.0, COMP_COLOR));
}

fn draw_diode(painter: &egui::Painter, c: Pos2, r: Rotation) {
    let p1 = c - rot(Vec2::new(20.0, 0.0), r);
    let p2 = c + rot(Vec2::new(20.0, 0.0), r);
    let mid_a = c - rot(Vec2::new(6.0, 0.0), r);
    let mid_c = c + rot(Vec2::new(6.0, 0.0), r);
    painter.line_segment([p1, mid_a], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([p2, mid_c], Stroke::new(2.0, COMP_COLOR));
    let perp = rot(Vec2::new(0.0, 8.0), r);
    let tri = [mid_a + perp, mid_a - perp, mid_c];
    painter.add(egui::Shape::convex_polygon(tri.to_vec(), COMP_COLOR, Stroke::NONE));
    painter.line_segment([mid_c + perp, mid_c - perp], Stroke::new(2.5, COMP_COLOR));
}

fn draw_bjt(painter: &egui::Painter, c: Pos2, r: Rotation, is_npn: bool) {
    let base_pin = c + rot(Vec2::new(-20.0, 0.0), r);
    let coll_pin = c + rot(Vec2::new(20.0, -20.0), r);
    let emit_pin = c + rot(Vec2::new(20.0, 20.0), r);
    painter.circle_stroke(c, 16.0, Stroke::new(1.5, COMP_COLOR));
    painter.line_segment([base_pin, c + rot(Vec2::new(-6.0, 0.0), r)], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot(Vec2::new(-6.0, -10.0), r), c + rot(Vec2::new(-6.0, 10.0), r)], Stroke::new(2.5, COMP_COLOR));
    painter.line_segment([c + rot(Vec2::new(-6.0, -6.0), r), c + rot(Vec2::new(10.0, -14.0), r)], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot(Vec2::new(10.0, -14.0), r), coll_pin], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot(Vec2::new(-6.0, 6.0), r), c + rot(Vec2::new(10.0, 14.0), r)], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot(Vec2::new(10.0, 14.0), r), emit_pin], Stroke::new(2.0, COMP_COLOR));
    if is_npn {
        let ap = c + rot(Vec2::new(10.0, 14.0), r);
        painter.line_segment([ap, ap + rot(Vec2::new(-5.0, 0.0), r)], Stroke::new(2.0, COMP_COLOR));
        painter.line_segment([ap, ap + rot(Vec2::new(0.0, -5.0), r)], Stroke::new(2.0, COMP_COLOR));
    } else {
        let ap = c + rot(Vec2::new(-6.0, 6.0), r);
        painter.line_segment([ap, ap + rot(Vec2::new(5.0, 0.0), r)], Stroke::new(2.0, COMP_COLOR));
        painter.line_segment([ap, ap + rot(Vec2::new(0.0, 5.0), r)], Stroke::new(2.0, COMP_COLOR));
    }
}

fn draw_mosfet(painter: &egui::Painter, c: Pos2, r: Rotation, is_nmos: bool) {
    let gate_pin = c + rot(Vec2::new(-20.0, 0.0), r);
    let drain_pin = c + rot(Vec2::new(20.0, -20.0), r);
    let source_pin = c + rot(Vec2::new(20.0, 20.0), r);
    painter.circle_stroke(c, 16.0, Stroke::new(1.5, COMP_COLOR));
    painter.line_segment([gate_pin, c + rot(Vec2::new(-8.0, 0.0), r)], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot(Vec2::new(-8.0, -10.0), r), c + rot(Vec2::new(-8.0, 10.0), r)], Stroke::new(2.5, COMP_COLOR));
    painter.line_segment([c + rot(Vec2::new(-4.0, -10.0), r), c + rot(Vec2::new(-4.0, -6.0), r)], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot(Vec2::new(-4.0, -2.0), r), c + rot(Vec2::new(-4.0, 2.0), r)], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot(Vec2::new(-4.0, 6.0), r), c + rot(Vec2::new(-4.0, 10.0), r)], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot(Vec2::new(-4.0, -8.0), r), c + rot(Vec2::new(12.0, -8.0), r)], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot(Vec2::new(12.0, -8.0), r), c + rot(Vec2::new(12.0, -20.0), r)], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot(Vec2::new(12.0, -20.0), r), drain_pin], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot(Vec2::new(-4.0, 8.0), r), c + rot(Vec2::new(12.0, 8.0), r)], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot(Vec2::new(12.0, 8.0), r), c + rot(Vec2::new(12.0, 20.0), r)], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot(Vec2::new(12.0, 20.0), r), source_pin], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot(Vec2::new(-4.0, 0.0), r), c + rot(Vec2::new(12.0, 0.0), r)], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot(Vec2::new(12.0, 0.0), r), c + rot(Vec2::new(12.0, 8.0), r)], Stroke::new(2.0, COMP_COLOR));
    let bulk_pt = c + rot(Vec2::new(-4.0, 0.0), r);
    if is_nmos {
        painter.line_segment([bulk_pt, bulk_pt + rot(Vec2::new(4.0, -3.0), r)], Stroke::new(2.0, COMP_COLOR));
        painter.line_segment([bulk_pt, bulk_pt + rot(Vec2::new(4.0, 3.0), r)], Stroke::new(2.0, COMP_COLOR));
    } else {
        let ap = c + rot(Vec2::new(4.0, 0.0), r);
        painter.line_segment([ap, ap + rot(Vec2::new(-4.0, -3.0), r)], Stroke::new(2.0, COMP_COLOR));
        painter.line_segment([ap, ap + rot(Vec2::new(-4.0, 3.0), r)], Stroke::new(2.0, COMP_COLOR));
    }
}

fn draw_opamp(painter: &egui::Painter, c: Pos2, r: Rotation) {
    let p1 = c + rot(Vec2::new(-16.0, -16.0), r);
    let p2 = c + rot(Vec2::new(-16.0, 16.0), r);
    let p3 = c + rot(Vec2::new(20.0, 0.0), r);
    painter.add(egui::Shape::convex_polygon(vec![p1, p2, p3], Color32::TRANSPARENT, Stroke::new(2.0, COMP_COLOR)));
    painter.line_segment([c + rot(Vec2::new(-16.0, 10.0), r), c + rot(Vec2::new(-24.0, 10.0), r)], Stroke::new(2.0, COMP_COLOR));
    painter.text(c + rot(Vec2::new(-10.0, 10.0), r), egui::Align2::LEFT_CENTER, "+", egui::FontId::monospace(10.0), COMP_COLOR);
    painter.line_segment([c + rot(Vec2::new(-16.0, -10.0), r), c + rot(Vec2::new(-24.0, -10.0), r)], Stroke::new(2.0, COMP_COLOR));
    painter.text(c + rot(Vec2::new(-10.0, -10.0), r), egui::Align2::LEFT_CENTER, "−", egui::FontId::monospace(10.0), COMP_COLOR);
    painter.line_segment([p3, c + rot(Vec2::new(28.0, 0.0), r)], Stroke::new(2.0, COMP_COLOR));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinate_transforms() {
        let origin = Pos2::new(100.0, 100.0);
        let world = Position::new(50.0, 20.0);
        let screen = world_to_screen(origin, world);
        assert_eq!(screen.x, 150.0);
        assert_eq!(screen.y, 120.0);
        let back = screen_to_world(origin, screen);
        assert_eq!(back.x, world.x);
        assert_eq!(back.y, world.y);
    }

    #[test]
    fn test_pin_positions_deg0() {
        let comp = ComponentInfo {
            id: "R1".into(), name: "R1".into(),
            kind: ComponentKind::Resistor(1000.0),
            node_a: NodeId(1), node_b: NodeId(2),
            pos: Position::new(200.0, 100.0),
            rotation: Rotation::Deg0,
        };
        let (a, b) = pin_positions(&comp);
        assert!((a.x - 180.0).abs() < 0.01);
        assert!((a.y - 100.0).abs() < 0.01);
        assert!((b.x - 220.0).abs() < 0.01);
        assert!((b.y - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_pin_positions_deg90() {
        let comp = ComponentInfo {
            id: "C1".into(), name: "C1".into(),
            kind: ComponentKind::Capacitor(1e-7),
            node_a: NodeId(1), node_b: NodeId(2),
            pos: Position::new(300.0, 150.0),
            rotation: Rotation::Deg90,
        };
        let (a, b) = pin_positions(&comp);
        // Deg90: (-20, 0) → (0, -20), (+20, 0) → (0, +20)
        assert!((a.x - 300.0).abs() < 0.01);
        assert!((a.y - 130.0).abs() < 0.01);
        assert!((b.x - 300.0).abs() < 0.01);
        assert!((b.y - 170.0).abs() < 0.01);
    }
}
