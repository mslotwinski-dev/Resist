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

pub const GRID_SIZE: f32 = 40.0;

pub fn world_to_screen(origin: Pos2, world: Position) -> Pos2 {
    Pos2::new(
        origin.x + (world.x as f32 * GRID_SIZE),
        origin.y + (world.y as f32 * GRID_SIZE)
    )
}

pub fn screen_to_world(origin: Pos2, screen: Pos2) -> Position {
    Position::new(
        ((screen.x - origin.x) / GRID_SIZE).round() as i32,
        ((screen.y - origin.y) / GRID_SIZE).round() as i32
    )
}

fn rot(v: (i32, i32), r: Rotation) -> (i32, i32) {
    match r {
        Rotation::Deg0 => v,
        Rotation::Deg90 => (-v.1, v.0),
        Rotation::Deg180 => (-v.0, -v.1),
        Rotation::Deg270 => (v.1, -v.0),
    }
}

fn rot_f(v: Vec2, r: Rotation) -> Vec2 {
    match r {
        Rotation::Deg0 => v,
        Rotation::Deg90 => Vec2::new(-v.y, v.x),
        Rotation::Deg180 => Vec2::new(-v.x, -v.y),
        Rotation::Deg270 => Vec2::new(v.y, -v.x),
    }
}

/// Compute the pin grid-positions and their explicit Escape Directions (Facing).
/// Returns a list of `(grid_position, direction)` where direction is a grid integer `(i32, i32)`.
fn pin_escapes(comp: &ComponentInfo) -> Vec<(Position, (i32, i32))> {
    let c = (comp.pos.x, comp.pos.y);
    let r = comp.rotation;

    match &comp.kind {
        ComponentKind::Bjt { is_npn: _ } => {
            // C: (c.x, c.y - 1), B: (c.x - 1, c.y), E: (c.x, c.y + 1)
            let mut pins = vec![];
            for (p, dir) in [
                ((c.0, c.1 - 1), (0, -1)), // Collector (Up)
                ((c.0 - 1, c.1), (-1, 0)), // Base (Left)
                ((c.0, c.1 + 1), (0, 1)),  // Emitter (Down)
            ] {
                let offset = (p.0 - c.0, p.1 - c.1);
                let rotated_pos = rot(offset, r);
                let rotated_dir = rot(dir, r);
                pins.push((Position::new(c.0 + rotated_pos.0, c.1 + rotated_pos.1), rotated_dir));
            }
            pins
        }
        ComponentKind::Mosfet { is_nmos: _ } => {
            // D, G, S, Bulk
            let mut pins = vec![];
            for (p, dir) in [
                ((c.0, c.1 - 1), (0, -1)), // Drain (Up)
                ((c.0 - 1, c.1), (-1, 0)), // Gate (Left)
                ((c.0, c.1 + 1), (0, 1)),  // Source (Down)
                ((c.0 + 1, c.1), (1, 0)),  // Bulk (Right)
            ] {
                let offset = (p.0 - c.0, p.1 - c.1);
                let rotated_pos = rot(offset, r);
                let rotated_dir = rot(dir, r);
                pins.push((Position::new(c.0 + rotated_pos.0, c.1 + rotated_pos.1), rotated_dir));
            }
            pins
        }
        ComponentKind::OpAmp => {
            // Out, GND, In(+), In(-)
            let mut pins = vec![];
            for (p, dir) in [
                ((c.0 + 1, c.1), (1, 0)),      // Out (Right)
                ((c.0, c.1 + 1), (0, 1)),      // GND (Down)
                ((c.0 - 1, c.1 + 1), (-1, 0)), // In+ (Left Bottom)
                ((c.0 - 1, c.1 - 1), (-1, 0)), // In- (Left Top)
            ] {
                let offset = (p.0 - c.0, p.1 - c.1);
                let rotated_pos = rot(offset, r);
                let rotated_dir = rot(dir, r);
                pins.push((Position::new(c.0 + rotated_pos.0, c.1 + rotated_pos.1), rotated_dir));
            }
            pins
        }
        ComponentKind::VoltageSource(_) | ComponentKind::CurrentSource(_) | ComponentKind::TransientSource => {
            // Vertical 2-pin sources
            let mut pins = vec![];
            for (p, dir) in [
                ((c.0, c.1 - 1), (0, -1)), // Pin A (Up)
                ((c.0, c.1 + 1), (0, 1)),  // Pin B (Down)
            ] {
                let offset = (p.0 - c.0, p.1 - c.1);
                let rotated_pos = rot(offset, r);
                let rotated_dir = rot(dir, r);
                pins.push((Position::new(c.0 + rotated_pos.0, c.1 + rotated_pos.1), rotated_dir));
            }
            pins
        }
        _ => {
            // Standard Horizontal 2-pin components (Resistor, Cap, Inductor, Diode)
            let mut pins = vec![];
            for (p, dir) in [
                ((c.0 - 1, c.1), (-1, 0)), // Pin A (Left)
                ((c.0 + 1, c.1), (1, 0)),  // Pin B (Right)
            ] {
                let offset = (p.0 - c.0, p.1 - c.1);
                let rotated_pos = rot(offset, r);
                let rotated_dir = rot(dir, r);
                pins.push((Position::new(c.0 + rotated_pos.0, c.1 + rotated_pos.1), rotated_dir));
            }
            pins
        }
    }
}

use std::collections::HashSet;

/// Strict Integer Grid A* Router 
fn a_star_grid_route(start: (i32, i32), end: (i32, i32), obstacles: &HashSet<(i32, i32)>) -> Vec<(i32, i32)> {
    let heuristic = |&(x, y): &(i32, i32)| {
        ((x - end.0).abs() + (y - end.1).abs()) as u32
    };

    let successors = |&node: &(i32, i32)| {
        let mut succs = Vec::new();
        let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        for &d in &dirs {
            let next_pos = (node.0 + d.0, node.1 + d.1);
            // We allow stepping on the exact end pin even if it coincidentally falls on an obstacle bound
            if next_pos == end || !obstacles.contains(&next_pos) {
                // Cost is always 1 step
                succs.push((next_pos, 1));
            }
        }
        succs
    };

    let success = |node: &(i32, i32)| *node == end;

    if let Some((path, _cost)) = pathfinding::directed::astar::astar(
        &start, successors, heuristic, success
    ) {
        path
    } else {
        // Fallback L-Shape (Direct Manhattan channel)
        vec![start, (end.0, start.1), end]
    }
}

// The Z-router replaces A*.

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
        let step = GRID_SIZE;
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

    // ── 2. Build net map: NodeId → Vec<(screen Pos2, facing Grid Vec)> ──────
    let mut net_map: HashMap<NodeId, Vec<(Pos2, (i32, i32))>> = HashMap::new();

    for comp in &layout.components {
        let pins = pin_escapes(comp);
        // Map each NodeId in comp.pins to its computed screen position and direction
        for (i, &node_id) in comp.pins.iter().enumerate() {
            if i < pins.len() {
                let screen_pos = world_to_screen(origin, pins[i].0);
                net_map.entry(node_id).or_default().push((screen_pos, pins[i].1));
            }
        }
    }

    // Keep track of all segments drawn to place junction dots later
    let mut undirected_segments: HashSet<(NodeId, i32, i32, i32, i32)> = HashSet::new();

    // Helper to draw orthogonal line segments
    let mut draw_segment = |net_id: NodeId, p1: Pos2, p2: Pos2| {
        let snap = |v: f32| v.round() as i32;
        let mut ds_push = |p_a: Pos2, p_b: Pos2| {
            let x1 = snap(p_a.x); let y1 = snap(p_a.y);
            let x2 = snap(p_b.x); let y2 = snap(p_b.y);
            if x1 == x2 && y1 == y2 { return; }
            let (sx1, sy1, sx2, sy2) = if (x1, y1) < (x2, y2) {
                (x1, y1, x2, y2)
            } else {
                (x2, y2, x1, y1)
            };
            undirected_segments.insert((net_id, sx1, sy1, sx2, sy2));
        };

        if (p1.x - p2.x).abs() > 0.1 && (p1.y - p2.y).abs() > 0.1 {
            // Force Manhattan
            let corner = Pos2::new(p2.x, p1.y);
            painter.line_segment([p1, corner], Stroke::new(2.0, WIRE_COLOR));
            painter.line_segment([corner, p2], Stroke::new(2.0, WIRE_COLOR));
            ds_push(p1, corner);
            ds_push(corner, p2);
        } else if (p1.x - p2.x).abs() > 0.1 || (p1.y - p2.y).abs() > 0.1 {
            // Already orthogonal and not zero length
            painter.line_segment([p1, p2], Stroke::new(2.0, WIRE_COLOR));
            ds_push(p1, p2);
        }
    };

    // ── 2.5 Extract Obstacle Rectangles for Pathfinding ─────────────
    // Wires cannot route through the solid bodies of components.
    let mut obstacles: HashSet<(i32, i32)> = HashSet::new();

    for comp in &layout.components {
        let (cx, cy) = (comp.pos.x, comp.pos.y);
        
        // Define exact Grid cell blocks for different components to block off
        let block_cells = match &comp.kind {
            ComponentKind::Resistor(_) | ComponentKind::Capacitor(_) | ComponentKind::Inductor(_) | ComponentKind::Diode => {
                // Horizontal 2-unit (blocks only the center 0, 0 local cell)
                vec![(0, 0)]
            }
            ComponentKind::VoltageSource(_) | ComponentKind::CurrentSource(_) | ComponentKind::TransientSource => {
                // Vertical 2-unit (blocks only center 0, 0)
                vec![(0, 0)]
            }
            ComponentKind::Bjt { .. } | ComponentKind::Mosfet { .. } => {
                // Blocks a 2x2 area 
                vec![
                    (0, 0), (-1, 0), // core vertical and base/gate joint
                    (0, -1), (0, 1)  // top bottom half
                ]
            }
            ComponentKind::OpAmp => {
                // Op Amp is a solid 2x2.
                vec![
                    (0, 0), (1, 0),
                    (0, 1), (1, 1),
                    (-1, 0), (-1, 1) // extend polygon block left
                ]
            }
        };

        for offset in block_cells {
            let rot_off = rot(offset, comp.rotation);
            obstacles.insert((cx + rot_off.0, cy + rot_off.1));
        }
    }

    // ── 3. Grid A* Routing ───────────────
    for (&node_id, pins) in &net_map {
        if pins.len() < 2 && node_id != NodeId::GROUND { continue; }
        if node_id == NodeId::GROUND {
            // GND pins get ground symbols instead of wires
            continue;
        }

        // Sort pins left-to-right to easily chain them point-to-point
        let mut sorted_pins = pins.clone();
        sorted_pins.sort_by(|a, b| a.0.x.partial_cmp(&b.0.x).unwrap().then(a.0.y.partial_cmp(&b.0.y).unwrap()));

        for i in 0..sorted_pins.len() - 1 {
            let (target_p1_screen, dir1) = sorted_pins[i];
            let (target_p2_screen, dir2) = sorted_pins[i + 1];

            let p1 = screen_to_world(origin, target_p1_screen);
            let p2 = screen_to_world(origin, target_p2_screen);

            let esc_p1 = (p1.x + dir1.0, p1.y + dir1.1);
            let esc_p2 = (p2.x + dir2.0, p2.y + dir2.1);
            
            // Draw stub lines visually
            draw_segment(node_id, target_p1_screen, world_to_screen(origin, Position::new(esc_p1.0, esc_p1.1)));
            draw_segment(node_id, target_p2_screen, world_to_screen(origin, Position::new(esc_p2.0, esc_p2.1)));

            if esc_p1 == esc_p2 { continue; }

            // 2. A* pathfind exactly from Escape Point to Escape Point
            let path = a_star_grid_route(esc_p1, esc_p2, &obstacles);

            for j in 0..path.len() - 1 {
                let p_a = world_to_screen(origin, Position::new(path[j].0, path[j].1));
                let p_b = world_to_screen(origin, Position::new(path[j + 1].0, path[j + 1].1));
                draw_segment(node_id, p_a, p_b);
            }
        }
    }

    // ── 4. Draw Junction Dots ───────────────────────────────────────
    // A point needs a dot if 3 or more segments OF THE SAME NET share its coordinate
    let mut point_counts: HashMap<(NodeId, i32, i32), usize> = HashMap::new();

    for (net_id, x1, y1, x2, y2) in &undirected_segments {
        *point_counts.entry((*net_id, *x1, *y1)).or_default() += 1;
        *point_counts.entry((*net_id, *x2, *y2)).or_default() += 1;
    }

    for (pt, count) in point_counts {
        if count >= 3 {
             let pos = Pos2::new(pt.1 as f32, pt.2 as f32);
             painter.circle_filled(pos, 3.5, JUNCTION_COLOR);
        }
    }

    // ── 5. Draw ground symbols at every GND pin ─────────────────────
    if let Some(gnd_pins) = net_map.get(&NodeId::GROUND) {
        for (pin_pos, _dir) in gnd_pins {
            draw_ground_symbol(&painter, *pin_pos, Vec2::new(0.0, 1.0));
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
        let label_offset = rot_f(Vec2::new(0.0, -25.0), comp.rotation);
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

/// Draw the standard 3-line ground symbol pointing outward along `dir` vector.
fn draw_ground_symbol(painter: &egui::Painter, pos: Pos2, dir: Vec2) {
    let perp = Vec2::new(-dir.y, dir.x);
    // Vertical stub down
    painter.line_segment(
        [pos, pos + dir * 10.0],
        Stroke::new(2.0, GND_COLOR),
    );
    // Three horizontal lines of decreasing width
    painter.line_segment(
        [pos + dir * 10.0 - perp * 10.0, pos + dir * 10.0 + perp * 10.0],
        Stroke::new(2.0, GND_COLOR),
    );
    painter.line_segment(
        [pos + dir * 14.0 - perp * 6.0, pos + dir * 14.0 + perp * 6.0],
        Stroke::new(2.0, GND_COLOR),
    );
    painter.line_segment(
        [pos + dir * 18.0 - perp * 3.0, pos + dir * 18.0 + perp * 3.0],
        Stroke::new(2.0, GND_COLOR),
    );
}

fn draw_resistor(painter: &egui::Painter, c: Pos2, r: Rotation) {
    let p1 = c + rot_f(Vec2::new(-GRID_SIZE, 0.0), r);
    let p2 = c + rot_f(Vec2::new(GRID_SIZE, 0.0), r);
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
    let gap = rot_f(Vec2::new(6.0, 0.0), r);
    let plate = rot_f(Vec2::new(0.0, 16.0), r);
    let p1 = c - rot_f(Vec2::new(GRID_SIZE, 0.0), r);
    let p2 = c + rot_f(Vec2::new(GRID_SIZE, 0.0), r);
    painter.line_segment([p1, c - gap], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([p2, c + gap], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c - gap - plate, c - gap + plate], Stroke::new(3.0, COMP_COLOR));
    painter.line_segment([c + gap - plate, c + gap + plate], Stroke::new(3.0, COMP_COLOR));
}

fn draw_inductor(painter: &egui::Painter, c: Pos2, r: Rotation) {
    let p1 = c - rot_f(Vec2::new(GRID_SIZE, 0.0), r);
    let p2 = c + rot_f(Vec2::new(GRID_SIZE, 0.0), r);
    painter.line_segment([p1, p2], Stroke::new(2.5, COMP_COLOR));
    for i in 0..3 {
        let frac = (i as f32 + 0.5) / 3.0;
        let mid = p1 + (p2 - p1) * frac;
        painter.circle_stroke(mid, 5.0, Stroke::new(1.5, COMP_COLOR));
    }
}

fn draw_voltage_source(painter: &egui::Painter, c: Pos2, r: Rotation) {
    let p1 = c + rot_f(Vec2::new(0.0, -GRID_SIZE), r);
    let p2 = c + rot_f(Vec2::new(0.0, GRID_SIZE), r);
    let edge1 = c + rot_f(Vec2::new(0.0, -14.0), r);
    let edge2 = c + rot_f(Vec2::new(0.0, 14.0), r);
    painter.line_segment([p1, edge1], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([p2, edge2], Stroke::new(2.0, COMP_COLOR));
    painter.circle_stroke(c, 14.0, Stroke::new(2.0, COMP_COLOR));
    painter.text(c + rot_f(Vec2::new(0.0, -6.0), r), egui::Align2::CENTER_CENTER, "+", egui::FontId::monospace(12.0), COMP_COLOR);
    painter.text(c + rot_f(Vec2::new(0.0, 6.0), r), egui::Align2::CENTER_CENTER, "−", egui::FontId::monospace(12.0), COMP_COLOR);
}

fn draw_current_source(painter: &egui::Painter, c: Pos2, r: Rotation) {
    let p1 = c + rot_f(Vec2::new(0.0, -GRID_SIZE), r);
    let p2 = c + rot_f(Vec2::new(0.0, GRID_SIZE), r);
    let edge1 = c + rot_f(Vec2::new(0.0, -14.0), r);
    let edge2 = c + rot_f(Vec2::new(0.0, 14.0), r);
    painter.line_segment([p1, edge1], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([p2, edge2], Stroke::new(2.0, COMP_COLOR));
    painter.circle_stroke(c, 14.0, Stroke::new(2.0, COMP_COLOR));
    let arrow_dir = rot_f(Vec2::new(0.0, 1.0), r);
    painter.line_segment([c - arrow_dir * 6.0, c + arrow_dir * 6.0], Stroke::new(2.0, COMP_COLOR));
    let head1 = c + arrow_dir * 6.0 + rot_f(Vec2::new(-4.0, -4.0), r);
    let head2 = c + arrow_dir * 6.0 + rot_f(Vec2::new(4.0, -4.0), r);
    painter.line_segment([c + arrow_dir * 6.0, head1], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + arrow_dir * 6.0, head2], Stroke::new(2.0, COMP_COLOR));
}

fn draw_diode(painter: &egui::Painter, c: Pos2, r: Rotation) {
    let p1 = c - rot_f(Vec2::new(GRID_SIZE, 0.0), r);
    let p2 = c + rot_f(Vec2::new(GRID_SIZE, 0.0), r);
    let mid_a = c - rot_f(Vec2::new(8.0, 0.0), r);
    let mid_c = c + rot_f(Vec2::new(8.0, 0.0), r);
    painter.line_segment([p1, mid_a], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([p2, mid_c], Stroke::new(2.0, COMP_COLOR));
    let perp = rot_f(Vec2::new(0.0, 8.0), r);
    let tri = [mid_a + perp, mid_a - perp, mid_c];
    painter.add(egui::Shape::convex_polygon(tri.to_vec(), COMP_COLOR, Stroke::NONE));
    painter.line_segment([mid_c + perp, mid_c - perp], Stroke::new(2.5, COMP_COLOR));
}

fn draw_bjt(painter: &egui::Painter, c: Pos2, r: Rotation, is_npn: bool) {
    let base_pin = c + rot_f(Vec2::new(-GRID_SIZE, 0.0), r);
    let coll_pin = c + rot_f(Vec2::new(0.0, -GRID_SIZE), r);
    let emit_pin = c + rot_f(Vec2::new(0.0, GRID_SIZE), r);
    painter.circle_stroke(c, 16.0, Stroke::new(1.5, COMP_COLOR));
    
    // Base straight line into base plate
    painter.line_segment([base_pin, c + rot_f(Vec2::new(-6.0, 0.0), r)], Stroke::new(2.0, COMP_COLOR));
    
    // Base plate
    painter.line_segment([c + rot_f(Vec2::new(-6.0, -10.0), r), c + rot_f(Vec2::new(-6.0, 10.0), r)], Stroke::new(2.5, COMP_COLOR));
    
    // Collector angled line to a corner, then straight up to the grid pin without gap
    painter.line_segment([c + rot_f(Vec2::new(-6.0, -6.0), r), c + rot_f(Vec2::new(0.0, -20.0), r)], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot_f(Vec2::new(0.0, -20.0), r), coll_pin], Stroke::new(2.0, COMP_COLOR));
    
    // Emitter angled line
    painter.line_segment([c + rot_f(Vec2::new(-6.0, 6.0), r), c + rot_f(Vec2::new(0.0, 20.0), r)], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot_f(Vec2::new(0.0, 20.0), r), emit_pin], Stroke::new(2.0, COMP_COLOR));
    
    if is_npn {
        let ap = c + rot_f(Vec2::new(0.0, 18.0), r);
        painter.line_segment([ap, ap + rot_f(Vec2::new(-5.0, -4.0), r)], Stroke::new(2.0, COMP_COLOR));
        painter.line_segment([ap, ap + rot_f(Vec2::new(-3.0, -6.0), r)], Stroke::new(2.0, COMP_COLOR));
    } else {
        let ap = c + rot_f(Vec2::new(-4.0, 10.0), r);
        painter.line_segment([ap, ap + rot_f(Vec2::new(3.0, 6.0), r)], Stroke::new(2.0, COMP_COLOR));
        painter.line_segment([ap, ap + rot_f(Vec2::new(5.0, 4.0), r)], Stroke::new(2.0, COMP_COLOR));
    }
}

fn draw_mosfet(painter: &egui::Painter, c: Pos2, r: Rotation, is_nmos: bool) {
    let gate_pin = c + rot_f(Vec2::new(-GRID_SIZE, 0.0), r);
    let drain_pin = c + rot_f(Vec2::new(0.0, -GRID_SIZE), r);
    let source_pin = c + rot_f(Vec2::new(0.0, GRID_SIZE), r);
    let bulk_pin = c + rot_f(Vec2::new(GRID_SIZE, 0.0), r);
    painter.circle_stroke(c, 16.0, Stroke::new(1.5, COMP_COLOR));
    painter.line_segment([gate_pin, c + rot_f(Vec2::new(-8.0, 0.0), r)], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot_f(Vec2::new(-8.0, -10.0), r), c + rot_f(Vec2::new(-8.0, 10.0), r)], Stroke::new(2.5, COMP_COLOR));
    painter.line_segment([c + rot_f(Vec2::new(-4.0, -10.0), r), c + rot_f(Vec2::new(-4.0, -6.0), r)], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot_f(Vec2::new(-4.0, -2.0), r), c + rot_f(Vec2::new(-4.0, 2.0), r)], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot_f(Vec2::new(-4.0, 6.0), r), c + rot_f(Vec2::new(-4.0, 10.0), r)], Stroke::new(2.0, COMP_COLOR));
    
    painter.line_segment([c + rot_f(Vec2::new(-4.0, -8.0), r), c + rot_f(Vec2::new(6.0, -8.0), r)], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot_f(Vec2::new(6.0, -8.0), r), drain_pin], Stroke::new(2.0, COMP_COLOR));
    
    painter.line_segment([c + rot_f(Vec2::new(-4.0, 8.0), r), c + rot_f(Vec2::new(6.0, 8.0), r)], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot_f(Vec2::new(6.0, 8.0), r), source_pin], Stroke::new(2.0, COMP_COLOR));
    
    painter.line_segment([c + rot_f(Vec2::new(-4.0, 0.0), r), bulk_pin], Stroke::new(2.0, COMP_COLOR));
    
    let bulk_pt = c + rot_f(Vec2::new(-4.0, 0.0), r);
    if is_nmos {
        painter.line_segment([bulk_pt, bulk_pt + rot_f(Vec2::new(4.0, -3.0), r)], Stroke::new(2.0, COMP_COLOR));
        painter.line_segment([bulk_pt, bulk_pt + rot_f(Vec2::new(4.0, 3.0), r)], Stroke::new(2.0, COMP_COLOR));
    } else {
        let ap = c + rot_f(Vec2::new(4.0, 0.0), r);
        painter.line_segment([ap, ap + rot_f(Vec2::new(-4.0, -3.0), r)], Stroke::new(2.0, COMP_COLOR));
        painter.line_segment([ap, ap + rot_f(Vec2::new(-4.0, 3.0), r)], Stroke::new(2.0, COMP_COLOR));
    }
}

fn draw_opamp(painter: &egui::Painter, c: Pos2, r: Rotation) {
    let p1 = c + rot_f(Vec2::new(-16.0, -16.0), r);
    let p2 = c + rot_f(Vec2::new(-16.0, 16.0), r);
    let p3 = c + rot_f(Vec2::new(20.0, 0.0), r);
    painter.add(egui::Shape::convex_polygon(vec![p1, p2, p3], Color32::TRANSPARENT, Stroke::new(2.0, COMP_COLOR)));
    
    painter.text(c + rot_f(Vec2::new(-10.0, 10.0), r), egui::Align2::CENTER_CENTER, "+", egui::FontId::monospace(10.0), COMP_COLOR);
    painter.text(c + rot_f(Vec2::new(-10.0, -10.0), r), egui::Align2::CENTER_CENTER, "-", egui::FontId::monospace(10.0), COMP_COLOR);
    
    // Wire out to the Grid endpoints physically
    let in_n = c + rot_f(Vec2::new(-GRID_SIZE, -GRID_SIZE), r);
    let in_p = c + rot_f(Vec2::new(-GRID_SIZE, GRID_SIZE), r);
    let out_p = c + rot_f(Vec2::new(GRID_SIZE, 0.0), r);
    let gnd_p = c + rot_f(Vec2::new(0.0, GRID_SIZE), r);

    painter.line_segment([in_n, c + rot_f(Vec2::new(-16.0, -GRID_SIZE), r)], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot_f(Vec2::new(-16.0, -GRID_SIZE), r), c + rot_f(Vec2::new(-16.0, -10.0), r)], Stroke::new(2.0, COMP_COLOR));

    painter.line_segment([in_p, c + rot_f(Vec2::new(-16.0, GRID_SIZE), r)], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot_f(Vec2::new(-16.0, GRID_SIZE), r), c + rot_f(Vec2::new(-16.0, 10.0), r)], Stroke::new(2.0, COMP_COLOR));

    painter.line_segment([p3, out_p], Stroke::new(2.0, COMP_COLOR));
    painter.line_segment([c + rot_f(Vec2::new(0.0, 10.0), r), gnd_p], Stroke::new(2.0, COMP_COLOR));
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
