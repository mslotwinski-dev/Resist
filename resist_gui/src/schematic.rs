use std::collections::HashMap;

use eframe::egui;
use eframe::egui::{Color32, Pos2, Stroke, Vec2};

use crate::sim_state::{
    ComponentInfo, ComponentKind, EditorMode, PinRef, Position, Rotation, SelectedEntity, SimState,
};

const GRID_SIZE: f32 = 36.0;
const PIN_RADIUS: f32 = 6.0;

const GRID_COLOR: Color32 = Color32::from_rgb(42, 44, 52);
const WIRE_COLOR: Color32 = Color32::from_rgb(102, 220, 130);
const COMP_COLOR: Color32 = Color32::from_rgb(204, 224, 244);
const TEXT_COLOR: Color32 = Color32::from_rgb(190, 198, 215);
const SELECT_COLOR: Color32 = Color32::from_rgb(255, 170, 82);
const PIN_COLOR: Color32 = Color32::from_rgb(120, 182, 255);
const HOT_PIN_COLOR: Color32 = Color32::from_rgb(255, 206, 84);

pub fn draw_schematic(ui: &mut egui::Ui, sim: &mut SimState) {
    let (response, painter) =
        ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
    let origin = response.rect.left_top() + Vec2::new(24.0, 24.0);

    draw_grid(&painter, response.rect);

    let mut pin_positions: HashMap<PinRef, Pos2> = HashMap::new();
    for comp in &sim.layout.components {
        let pin_count = comp.kind.pin_count();
        for pin_index in 0..pin_count {
            let pin_ref = PinRef {
                component_id: comp.id.clone(),
                pin_index,
            };
            pin_positions.insert(pin_ref, pin_screen_pos_from_origin(origin, comp, pin_index));
        }
    }

    let hover_pin = response
        .hover_pos()
        .and_then(|pos| nearest_pin(pos, &pin_positions));

    for wire in &sim.layout.wires {
        if let (Some(a), Some(b)) = (pin_positions.get(&wire.from), pin_positions.get(&wire.to)) {
            draw_orthogonal_wire(&painter, *a, *b, WIRE_COLOR, 2.0);
        }
    }

    if let (Some(start), Some(cursor)) = (
        sim.pending_wire
            .as_ref()
            .and_then(|p| pin_positions.get(p))
            .copied(),
        response.hover_pos(),
    ) {
        draw_orthogonal_wire(
            &painter,
            start,
            cursor,
            Color32::from_rgb(84, 168, 242),
            1.5,
        );
    }

    for comp in &sim.layout.components {
        let center = world_to_screen(origin, comp.pos);
        let is_selected = matches!(&sim.selection, SelectedEntity::Component(id) if id == &comp.id);

        let stroke = if is_selected {
            Stroke::new(2.0, SELECT_COLOR)
        } else {
            Stroke::new(1.8, COMP_COLOR)
        };

        draw_component(&painter, comp, center, stroke);

        painter.text(
            center + Vec2::new(0.0, -GRID_SIZE * 0.95),
            egui::Align2::CENTER_CENTER,
            format!("{} ({})", comp.name, value_unit(comp)),
            egui::FontId::proportional(13.0),
            if is_selected {
                SELECT_COLOR
            } else {
                TEXT_COLOR
            },
        );
    }

    for (pin_ref, pin_pos) in &pin_positions {
        let is_hot =
            hover_pin.as_ref() == Some(pin_ref) || sim.pending_wire.as_ref() == Some(pin_ref);
        painter.circle_filled(
            *pin_pos,
            PIN_RADIUS,
            if is_hot { HOT_PIN_COLOR } else { PIN_COLOR },
        );
    }

    handle_pointer_interaction(response, sim, origin, &pin_positions, hover_pin);

    ui.horizontal(|ui| {
        ui.label(match sim.editor_mode {
            EditorMode::Select => "Mode: Select/Drag",
            EditorMode::Wire => "Mode: Wire",
        });
        if let Some(pin) = &sim.pending_wire {
            ui.colored_label(
                HOT_PIN_COLOR,
                format!("wire start: {}:{}", pin.component_id, pin.pin_index + 1),
            );
        }
    });
}

fn handle_pointer_interaction(
    response: egui::Response,
    sim: &mut SimState,
    origin: Pos2,
    pin_positions: &HashMap<PinRef, Pos2>,
    hover_pin: Option<PinRef>,
) {
    if response.clicked() {
        match sim.editor_mode {
            EditorMode::Wire => {
                if let Some(pin) = hover_pin {
                    if let Some(start) = sim.pending_wire.clone() {
                        if start != pin {
                            let exists = sim.layout.wires.iter().any(|w| {
                                (w.from == start && w.to == pin) || (w.from == pin && w.to == start)
                            });
                            if !exists {
                                sim.layout.wires.push(crate::sim_state::Wire {
                                    from: start,
                                    to: pin.clone(),
                                });
                            }
                        }
                        sim.pending_wire = None;
                    } else {
                        sim.pending_wire = Some(pin);
                    }
                } else {
                    sim.pending_wire = None;
                }
            }
            EditorMode::Select => {
                if let Some(pin) = hover_pin {
                    if let Some(nodes) = sim.last_component_nodes.get(&pin.component_id) {
                        if let Some(node) = nodes.get(pin.pin_index) {
                            sim.selection = SelectedEntity::Node(*node);
                        }
                    }
                } else if let Some(pos) = response.hover_pos() {
                    if let Some(comp_id) = hit_component(pos, sim, origin) {
                        sim.selection = SelectedEntity::Component(comp_id);
                    } else {
                        sim.selection = SelectedEntity::None;
                    }
                }
            }
        }
    }

    if sim.editor_mode == EditorMode::Select {
        if response.drag_started() {
            if let Some(pos) = response.interact_pointer_pos() {
                sim.dragging_component = hit_component(pos, sim, origin);
            }
        }

        if response.dragged() {
            if let (Some(id), Some(pos)) = (
                sim.dragging_component.clone(),
                response.interact_pointer_pos(),
            ) {
                if let Some(comp) = sim.layout.components.iter_mut().find(|c| c.id == id) {
                    comp.pos = screen_to_world(origin, pos);
                }
            }
        }

        if response.drag_stopped() {
            sim.dragging_component = None;
        }
    }

    if let Some(pos) = response.hover_pos() {
        if nearest_pin(pos, pin_positions).is_some() {
            response.ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
        } else if hit_component(pos, sim, origin).is_some() {
            response.ctx.set_cursor_icon(egui::CursorIcon::Grab);
        }
    }
}

fn value_unit(comp: &ComponentInfo) -> String {
    match comp.kind {
        ComponentKind::Resistor => format!("{:.3} Ohm", comp.value),
        ComponentKind::Capacitor => format!("{:.3e} F", comp.value),
        ComponentKind::Inductor => format!("{:.3e} H", comp.value),
        ComponentKind::VoltageSource => format!("{:.3} V", comp.value),
        ComponentKind::CurrentSource => format!("{:.3e} A", comp.value),
        ComponentKind::FunctionalVoltageSource => {
            if let Some(expr) = &comp.expression {
                format!("V(t) = {}", expr)
            } else {
                "V(t) undefined".to_string()
            }
        }
        ComponentKind::FunctionalCurrentSource => {
            if let Some(expr) = &comp.expression {
                format!("I(t) = {}", expr)
            } else {
                "I(t) undefined".to_string()
            }
        }
        ComponentKind::Ground => "0 V".to_string(),
    }
}

fn nearest_pin(pos: Pos2, pin_positions: &HashMap<PinRef, Pos2>) -> Option<PinRef> {
    pin_positions
        .iter()
        .filter_map(|(pin, pin_pos)| {
            let d = pos.distance(*pin_pos);
            if d <= 10.0 {
                Some((pin.clone(), d))
            } else {
                None
            }
        })
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .map(|(pin, _)| pin)
}

fn hit_component(pos: Pos2, sim: &SimState, origin: Pos2) -> Option<String> {
    sim.layout
        .components
        .iter()
        .find(|comp| component_rect(world_to_screen(origin, comp.pos), comp.kind).contains(pos))
        .map(|c| c.id.clone())
}

fn component_rect(center: Pos2, kind: ComponentKind) -> egui::Rect {
    let size = match kind {
        ComponentKind::Ground => Vec2::new(24.0, 24.0),
        _ => Vec2::new(60.0, 36.0),
    };
    egui::Rect::from_center_size(center, size)
}

fn draw_grid(painter: &egui::Painter, rect: egui::Rect) {
    let mut x = rect.left();
    while x < rect.right() {
        let mut y = rect.top();
        while y < rect.bottom() {
            painter.circle_filled(Pos2::new(x, y), 1.0, GRID_COLOR);
            y += GRID_SIZE;
        }
        x += GRID_SIZE;
    }
}

fn draw_orthogonal_wire(
    painter: &egui::Painter,
    start: Pos2,
    end: Pos2,
    color: Color32,
    width: f32,
) {
    let elbow = Pos2::new(end.x, start.y);
    painter.line_segment([start, elbow], Stroke::new(width, color));
    painter.line_segment([elbow, end], Stroke::new(width, color));
}

fn draw_component(painter: &egui::Painter, comp: &ComponentInfo, center: Pos2, stroke: Stroke) {
    let pin0 = pin_screen_pos_from_center(center, comp, 0);
    match comp.kind {
        ComponentKind::Ground => {
            painter.line_segment([pin0, pin0 + Vec2::new(0.0, 8.0)], stroke);
            painter.line_segment(
                [pin0 + Vec2::new(-10.0, 8.0), pin0 + Vec2::new(10.0, 8.0)],
                stroke,
            );
            painter.line_segment(
                [pin0 + Vec2::new(-6.0, 13.0), pin0 + Vec2::new(6.0, 13.0)],
                stroke,
            );
            painter.line_segment(
                [pin0 + Vec2::new(-3.0, 18.0), pin0 + Vec2::new(3.0, 18.0)],
                stroke,
            );
        }
        ComponentKind::Resistor => {
            let pin1 = pin_screen_pos_from_center(center, comp, 1);
            painter.line_segment([pin0, pin1], stroke);
            painter.rect_stroke(
                component_rect(center, comp.kind),
                2.0,
                stroke,
                egui::StrokeKind::Middle,
            );
        }
        ComponentKind::Capacitor => {
            let pin1 = pin_screen_pos_from_center(center, comp, 1);
            let dir = (pin1 - pin0).normalized();
            let perp = Vec2::new(-dir.y, dir.x) * 12.0;
            let c = pin0 + (pin1 - pin0) * 0.5;
            let p0 = c - dir * 6.0;
            let p1 = c + dir * 6.0;
            painter.line_segment([pin0, p0], stroke);
            painter.line_segment([pin1, p1], stroke);
            painter.line_segment([p0 - perp, p0 + perp], stroke);
            painter.line_segment([p1 - perp, p1 + perp], stroke);
        }
        ComponentKind::Inductor => {
            let pin1 = pin_screen_pos_from_center(center, comp, 1);
            painter.line_segment([pin0, pin1], stroke);
            for i in 0..3 {
                let t = (i as f32 + 0.5) / 3.0;
                let c = pin0 + (pin1 - pin0) * t;
                painter.circle_stroke(c, 6.0, stroke);
            }
        }
        ComponentKind::VoltageSource => {
            let pin1 = pin_screen_pos_from_center(center, comp, 1);
            let c = pin0 + (pin1 - pin0) * 0.5;
            let dir = (pin1 - pin0).normalized();
            painter.line_segment([pin0, c - dir * 13.0], stroke);
            painter.line_segment([pin1, c + dir * 13.0], stroke);
            painter.circle_stroke(c, 13.0, stroke);
            painter.text(
                c - dir * 6.0,
                egui::Align2::CENTER_CENTER,
                "+",
                egui::FontId::proportional(14.0),
                COMP_COLOR,
            );
            painter.text(
                c + dir * 6.0,
                egui::Align2::CENTER_CENTER,
                "-",
                egui::FontId::proportional(14.0),
                COMP_COLOR,
            );
        }
        ComponentKind::CurrentSource => {
            let pin1 = pin_screen_pos_from_center(center, comp, 1);
            let c = pin0 + (pin1 - pin0) * 0.5;
            let dir = (pin1 - pin0).normalized();
            let perp = Vec2::new(-dir.y, dir.x) * 3.5;
            painter.line_segment([pin0, c - dir * 13.0], stroke);
            painter.line_segment([pin1, c + dir * 13.0], stroke);
            painter.circle_stroke(c, 13.0, stroke);
            painter.line_segment([c - dir * 6.0, c + dir * 6.0], stroke);
            painter.line_segment([c + dir * 6.0, c + dir * 1.0 + perp], stroke);
            painter.line_segment([c + dir * 6.0, c + dir * 1.0 - perp], stroke);
        }
        ComponentKind::FunctionalVoltageSource => {
            let pin1 = pin_screen_pos_from_center(center, comp, 1);
            let c = pin0 + (pin1 - pin0) * 0.5;
            let dir = (pin1 - pin0).normalized();
            painter.line_segment([pin0, c - dir * 13.0], stroke);
            painter.line_segment([pin1, c + dir * 13.0], stroke);
            painter.circle_stroke(c, 13.0, stroke);
            painter.text(
                c - dir * 6.0,
                egui::Align2::CENTER_CENTER,
                "~",
                egui::FontId::proportional(14.0),
                COMP_COLOR,
            );
            painter.text(
                c + dir * 6.0,
                egui::Align2::CENTER_CENTER,
                "f",
                egui::FontId::proportional(12.0),
                COMP_COLOR,
            );
        }
        ComponentKind::FunctionalCurrentSource => {
            let pin1 = pin_screen_pos_from_center(center, comp, 1);
            let c = pin0 + (pin1 - pin0) * 0.5;
            let dir = (pin1 - pin0).normalized();
            let perp = Vec2::new(-dir.y, dir.x) * 3.5;
            painter.line_segment([pin0, c - dir * 13.0], stroke);
            painter.line_segment([pin1, c + dir * 13.0], stroke);
            painter.circle_stroke(c, 13.0, stroke);
            painter.line_segment([c - dir * 6.0, c + dir * 6.0], stroke);
            painter.line_segment([c + dir * 6.0, c + dir * 1.0 + perp], stroke);
            painter.line_segment([c + dir * 6.0, c + dir * 1.0 - perp], stroke);
            painter.text(
                c + dir * 3.0,
                egui::Align2::CENTER_CENTER,
                "f",
                egui::FontId::proportional(10.0),
                COMP_COLOR,
            );
        }
    }
}

pub fn world_to_screen(origin: Pos2, world: Position) -> Pos2 {
    Pos2::new(
        origin.x + world.x as f32 * GRID_SIZE,
        origin.y + world.y as f32 * GRID_SIZE,
    )
}

pub fn screen_to_world(origin: Pos2, screen: Pos2) -> Position {
    Position::new(
        ((screen.x - origin.x) / GRID_SIZE).round() as i32,
        ((screen.y - origin.y) / GRID_SIZE).round() as i32,
    )
}

fn pin_screen_pos_from_center(center: Pos2, comp: &ComponentInfo, pin_index: usize) -> Pos2 {
    let offset = pin_offset(comp.kind, comp.rotation, pin_index);
    center + Vec2::new(offset.0 as f32 * GRID_SIZE, offset.1 as f32 * GRID_SIZE)
}

fn pin_screen_pos_from_origin(origin: Pos2, comp: &ComponentInfo, pin_index: usize) -> Pos2 {
    let center = world_to_screen(origin, comp.pos);
    pin_screen_pos_from_center(center, comp, pin_index)
}

fn pin_offset(kind: ComponentKind, rotation: Rotation, pin_index: usize) -> (i32, i32) {
    let base = match kind {
        ComponentKind::Ground => vec![(0, 0)],
        _ => vec![(-1, 0), (1, 0)],
    };

    let raw = base.get(pin_index).copied().unwrap_or((0, 0));
    rotate_grid(raw, rotation)
}

fn rotate_grid(v: (i32, i32), rotation: Rotation) -> (i32, i32) {
    match rotation {
        Rotation::Deg0 => v,
        Rotation::Deg90 => (v.1, -v.0),
        Rotation::Deg180 => (-v.0, -v.1),
        Rotation::Deg270 => (-v.1, v.0),
    }
}

pub fn component_pin_positions(origin: Pos2, comp: &ComponentInfo) -> Vec<Pos2> {
    (0..comp.kind.pin_count())
        .map(|i| pin_screen_pos_from_origin(origin, comp, i))
        .collect()
}

pub fn component_pin_positions_world(comp: &ComponentInfo) -> Vec<Position> {
    (0..comp.kind.pin_count())
        .map(|i| {
            let o = pin_offset(comp.kind, comp.rotation, i);
            Position::new(comp.pos.x + o.0, comp.pos.y + o.1)
        })
        .collect()
}
