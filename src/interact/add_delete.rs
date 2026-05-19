/*
 * Manual addition and deletion tools.
 * Handles placing new rooms, corridors, doors, markers, and text labels,
 * as well as deleting existing elements via pointer interaction.
 */

use super::{
    AddCorridorDrag, GridGeometry, HoverMarker, MarkerKind, cell_center, centered_marker_origin,
    corridor_drag_at_pointer, door_edge_at_pointer, door_edges_for, marker_at_pointer, marker_rect,
    normalized_edge, pointer_to_grid, room_at_pointer, stair_at_pointer, text_at_pointer,
};
use crate::app::{DungeonApp, ManualDoorKind};
use crate::layout::{
    self, blocked_room_cells, corridor_cells, room_index_at_cell, rooms_overlap,
    shortest_path_cells,
};
use crate::ui::AddTool;
use eframe::egui;
use egui::{Color32, Stroke};

// Handles add tool.
pub fn handle_add_tool(app: &mut DungeonApp, response: &egui::Response, geometry: &GridGeometry) {
    if app.settings.add_tool != AddTool::None && response.secondary_clicked() {
        app.settings.add_tool = AddTool::None;
        app.add_corridor_drag = None;
        return;
    }

    match app.settings.add_tool {
        AddTool::Room => {
            app.add_corridor_drag = None;
            if response.clicked()
                && let Some(pointer_pos) = response.interact_pointer_pos()
                && let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry)
            {
                let cell = (grid_x.floor() as usize, grid_y.floor() as usize);
                add_room_at_cell(app, cell);
            }
        }
        AddTool::Corridor => {
            if response.drag_started()
                && let Some(pointer_pos) = response.interact_pointer_pos()
                && let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry)
            {
                let cell = (grid_x.floor() as usize, grid_y.floor() as usize);
                app.add_corridor_drag = Some(AddCorridorDrag {
                    start_cell: cell,
                    current_cell: cell,
                });
            }

            if response.dragged()
                && let Some(pointer_pos) = response.interact_pointer_pos()
                && let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry)
                && let Some(drag) = &mut app.add_corridor_drag
            {
                drag.current_cell = (grid_x.floor() as usize, grid_y.floor() as usize);
            }

            if response.drag_stopped()
                && let Some(drag) = app.add_corridor_drag.take()
            {
                add_corridor_between(app, drag.start_cell, drag.current_cell);
            }
        }
        AddTool::Staircase => {
            app.add_corridor_drag = None;
            if response.clicked()
                && let Some(pointer_pos) = response.interact_pointer_pos()
                && let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry)
            {
                let cell = (grid_x.floor() as usize, grid_y.floor() as usize);
                add_staircase_at_cell(app, cell);
            }
        }
        AddTool::Archway | AddTool::Door | AddTool::LockedDoor | AddTool::SecretDoor => {
            app.add_corridor_drag = None;
            if response.clicked()
                && let Some(pointer_pos) = response.interact_pointer_pos()
            {
                let kind = match app.settings.add_tool {
                    AddTool::Archway => ManualDoorKind::Archway,
                    AddTool::Door => ManualDoorKind::Regular,
                    AddTool::LockedDoor => ManualDoorKind::Locked,
                    AddTool::SecretDoor => ManualDoorKind::Secret,
                    _ => unreachable!(),
                };
                add_door_at_pointer(app, pointer_pos, geometry, kind);
            }
        }
        AddTool::Text => {
            app.add_corridor_drag = None;
            if response.clicked()
                && let Some(pointer_pos) = response.interact_pointer_pos()
                && let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry)
            {
                let cell = (grid_x.floor() as usize, grid_y.floor() as usize);
                add_text_at_cell(app, cell);
            }
        }
        AddTool::StartMarker
        | AddTool::EndMarker
        | AddTool::TrapMarker
        | AddTool::MonsterMarker => {
            app.add_corridor_drag = None;
            if response.clicked()
                && let Some(pointer_pos) = response.interact_pointer_pos()
                && let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry)
            {
                let cell = (grid_x.floor() as usize, grid_y.floor() as usize);
                let kind = match app.settings.add_tool {
                    AddTool::StartMarker => MarkerKind::Start,
                    AddTool::EndMarker => MarkerKind::End,
                    AddTool::TrapMarker => MarkerKind::Trap,
                    AddTool::MonsterMarker => MarkerKind::Monster,
                    _ => unreachable!(),
                };
                add_marker_at_cell(app, cell, kind);
            }
        }
        AddTool::None => {
            app.add_corridor_drag = None;
        }
    }
}

// Deletes at pointer.
pub fn delete_at_pointer(app: &mut DungeonApp, pointer_pos: egui::Pos2, geometry: &GridGeometry) {
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }

    if let Some(idx) = text_at_pointer(app, pointer_pos, geometry) {
        app.push_undo_snapshot();
        let layout = &mut app.levels[app.settings.active_level_index];
        if idx < layout.text_labels.len() {
            layout.text_labels.remove(idx);
        }
        return;
    }

    if let Some(marker) = marker_at_pointer(app, pointer_pos, geometry) {
        app.push_undo_snapshot();
        let layout = &mut app.levels[app.settings.active_level_index];
        match marker {
            HoverMarker::Start(idx) => {
                if idx < layout.start_markers.len() {
                    layout.start_markers.remove(idx);
                }
            }
            HoverMarker::End(idx) => {
                if idx < layout.end_markers.len() {
                    layout.end_markers.remove(idx);
                }
            }
            HoverMarker::Trap(idx) => {
                if idx < layout.trap_markers.len() {
                    layout.trap_markers.remove(idx);
                }
            }
            HoverMarker::Monster(idx) => {
                if idx < layout.monster_markers.len() {
                    layout.monster_markers.remove(idx);
                }
            }
        }
        return;
    }

    if let Some(edge) = door_edge_at_pointer(app, pointer_pos, geometry) {
        let target = normalized_edge(edge.0, edge.1);
        let active_index = app.settings.active_level_index;
        if let Some(idx) = app.levels[active_index].doors.iter().position(|d| {
            door_edges_for(d, app.settings.cols, app.settings.rows)
                .iter()
                .any(|e| normalized_edge(e.0, e.1) == target)
        }) {
            app.push_undo_snapshot();
            let layout = &mut app.levels[app.settings.active_level_index];
            if !layout.doors[idx].manual {
                for edge in door_edges_for(&layout.doors[idx], app.settings.cols, app.settings.rows)
                {
                    app.suppressed_auto_door_edges.insert(edge);
                }
                layout.doors.remove(idx);
                return;
            }

            layout.doors.remove(idx);
            return;
        }
    }

    if let Some(stair_idx) = stair_at_pointer(app, pointer_pos, geometry) {
        let original_cell = {
            let layout = &app.levels[app.settings.active_level_index];
            layout.stairs[stair_idx].cell
        };

        app.push_undo_snapshot();
        if app.settings.sync_stairs_across_levels {
            for layout in &mut app.levels {
                layout.stairs.retain(|s| s.cell != original_cell);
            }
        } else {
            let current_idx = app.settings.active_level_index;
            let level_indices = vec![
                Some(current_idx),
                current_idx.checked_sub(1),
                if current_idx + 1 < app.levels.len() {
                    Some(current_idx + 1)
                } else {
                    None
                },
            ];

            for idx in level_indices.into_iter().flatten() {
                app.levels[idx].stairs.retain(|s| s.cell != original_cell);
            }
        }
        return;
    }

    if let Some((room_idx, _, _)) = room_at_pointer(app, pointer_pos, geometry) {
        app.push_undo_snapshot();
        delete_room(app, room_idx);
        return;
    }

    if let Some(corridor_drag) = corridor_drag_at_pointer(app, pointer_pos, geometry) {
        let active_index = app.settings.active_level_index;
        if corridor_drag.corridor_index < app.levels[active_index].corridors.len() {
            app.push_undo_snapshot();
            let layout = &mut app.levels[app.settings.active_level_index];
            layout.corridors.remove(corridor_drag.corridor_index);
            app.drag_state = None;
            app.refresh_doors();
        }
    }
}

// Adds room at cell.
pub fn add_room_at_cell(app: &mut DungeonApp, cell: (usize, usize)) {
    app.push_undo_snapshot();
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let layout = &mut app.levels[app.settings.active_level_index];

    let width = app.settings.add_room_width.max(1);
    let height = app.settings.add_room_height.max(1);
    if cell.0 >= app.settings.cols || cell.1 >= app.settings.rows {
        return;
    }
    let max_w = app.settings.cols.saturating_sub(cell.0);
    let max_h = app.settings.rows.saturating_sub(cell.1);
    let width = width.min(max_w);
    let height = height.min(max_h);
    if width == 0 || height == 0 {
        return;
    }

    let new_room = layout::Room {
        x: cell.0,
        y: cell.1,
        width,
        height,
    };

    if layout
        .rooms
        .iter()
        .any(|room| rooms_overlap(&new_room, room))
    {
        return;
    }

    layout.rooms.push(new_room);
    app.refresh_doors();
}

// Adds corridor between.
pub fn add_corridor_between(app: &mut DungeonApp, start: (usize, usize), end: (usize, usize)) {
    app.push_undo_snapshot();
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let layout = &mut app.levels[app.settings.active_level_index];

    if start == end {
        return;
    }

    let start_room = room_index_at_cell(&layout.rooms, start);
    let end_room = room_index_at_cell(&layout.rooms, end);
    let (Some(start_room), Some(end_room)) = (start_room, end_room) else {
        return;
    };

    let blocked = blocked_room_cells(&layout.rooms, &[start_room, end_room]);

    let Some(path) =
        shortest_path_cells(start, end, app.settings.cols, app.settings.rows, &blocked)
    else {
        return;
    };

    let next_id = layout
        .corridors
        .iter()
        .map(|c| c.id)
        .max()
        .unwrap_or(0)
        .wrapping_add(1);
    let width = app.settings.add_corridor_width.max(1);

    layout.corridors.push(layout::Corridor {
        id: next_id,
        start_room_id: start_room,
        end_room_id: end_room,
        path,
        width,
    });
    app.refresh_doors();
}

// Adds text at cell.
pub fn add_text_at_cell(app: &mut DungeonApp, cell: (usize, usize)) {
    app.push_undo_snapshot();
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let layout = &mut app.levels[app.settings.active_level_index];

    if cell.0 >= app.settings.cols || cell.1 >= app.settings.rows {
        return;
    }

    let text = app.settings.add_text_value.trim().to_string();
    if text.is_empty() {
        return;
    }

    if let Some(existing) = layout
        .text_labels
        .iter_mut()
        .find(|label| label.cell == cell)
    {
        existing.text = text;
        return;
    }

    layout.text_labels.push(layout::TextLabel {
        cell,
        text,
        font_size: app.settings.add_text_font_size,
    });
}

// Adds marker at cell.
pub fn add_marker_at_cell(app: &mut DungeonApp, cell: (usize, usize), kind: MarkerKind) {
    app.push_undo_snapshot();
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let layout = &mut app.levels[app.settings.active_level_index];

    if cell.0 >= app.settings.cols || cell.1 >= app.settings.rows {
        return;
    }

    let size = app.settings.add_marker_size.max(1);
    let origin = centered_marker_origin(cell, size, app.settings.cols, app.settings.rows);
    let marker = layout::AreaMarker { cell: origin, size };

    match kind {
        MarkerKind::Start => layout.start_markers.push(marker),
        MarkerKind::End => layout.end_markers.push(marker),
        MarkerKind::Trap => layout.trap_markers.push(marker),
        MarkerKind::Monster => layout.monster_markers.push(marker),
    }
}

// Adds staircase at cell.
pub fn add_staircase_at_cell(app: &mut DungeonApp, cell: (usize, usize)) {
    app.push_undo_snapshot();
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }

    if cell.0 >= app.settings.cols || cell.1 >= app.settings.rows {
        return;
    }

    let width = app.settings.add_stair_width.max(1);
    let height = app.settings.add_stair_height.max(1);
    let max_x = app.settings.cols.saturating_sub(width);
    let max_y = app.settings.rows.saturating_sub(height);
    let origin_x = cell.0.min(max_x);
    let origin_y = cell.1.min(max_y);

    let stair = layout::Staircase {
        cell: (origin_x, origin_y),
        width,
        height,
    };

    if app.settings.sync_stairs_across_levels {
        for layout in &mut app.levels {
            layout.stairs.push(stair.clone());
        }
    } else {
        let active_idx = app.settings.active_level_index;
        let layout = &mut app.levels[active_idx];
        layout.stairs.push(stair.clone());

        if active_idx + 1 < app.levels.len() {
            let above_layout = &mut app.levels[active_idx + 1];
            above_layout.stairs.push(stair);
        } else if active_idx > 0 {
            let below_layout = &mut app.levels[active_idx - 1];
            below_layout.stairs.push(stair);
        }
    }
}

// Deletes a room and removes dependent generated elements.
pub fn delete_room(app: &mut DungeonApp, room_idx: usize) {
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let layout = &mut app.levels[app.settings.active_level_index];

    if room_idx >= layout.rooms.len() {
        return;
    }
    layout.rooms.remove(room_idx);
    layout
        .corridors
        .retain(|c| c.start_room_id != room_idx && c.end_room_id != room_idx);
    for corridor in &mut layout.corridors {
        if corridor.start_room_id > room_idx {
            corridor.start_room_id -= 1;
        }
        if corridor.end_room_id > room_idx {
            corridor.end_room_id -= 1;
        }
    }
    app.drag_state = None;
    app.refresh_doors();
}

// Adds door at pointer.
pub fn add_door_at_pointer(
    app: &mut DungeonApp,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
    kind: ManualDoorKind,
) {
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }

    let Some(edge) = door_edge_at_pointer(app, pointer_pos, geometry) else {
        return;
    };

    app.push_undo_snapshot();
    let layout = &mut app.levels[app.settings.active_level_index];

    let target = normalized_edge(edge.0, edge.1);
    if let Some(idx) = layout.doors.iter().position(|d| {
        door_edges_for(d, app.settings.cols, app.settings.rows)
            .iter()
            .any(|e| normalized_edge(e.0, e.1) == target)
    }) {
        let edges = door_edges_for(&layout.doors[idx], app.settings.cols, app.settings.rows);
        let mut spawned = Vec::new();
        for edge in edges {
            if normalized_edge(edge.0, edge.1) == target {
                continue;
            }
            if layout
                .doors
                .iter()
                .any(|d| normalized_edge(d.from, d.to) == normalized_edge(edge.0, edge.1))
            {
                continue;
            }
            spawned.push(layout::Door {
                from: edge.0,
                to: edge.1,
                width: 1,
                span_width: false,
                locked: false,
                archway: true,
                secret: false,
                manual: true,
            });
        }
        let door = &mut layout.doors[idx];
        door.locked = kind == ManualDoorKind::Locked;
        door.archway = kind == ManualDoorKind::Archway;
        door.secret = kind == ManualDoorKind::Secret;
        door.width = 1;
        door.span_width = false;
        door.manual = true;
        for edge in door_edges_for(door, app.settings.cols, app.settings.rows) {
            app.suppressed_auto_door_edges.remove(&edge);
        }
        layout.doors.extend(spawned);
        return;
    }

    let corridor_cells = corridor_cells(layout, app.settings.cols, app.settings.rows);
    let a_room = room_index_at_cell(&layout.rooms, edge.0);
    let b_room = room_index_at_cell(&layout.rooms, edge.1);
    let a_in_corridor = corridor_cells.contains(&edge.0);
    let b_in_corridor = corridor_cells.contains(&edge.1);

    if !manual_door_edge_allowed(a_room, b_room, a_in_corridor, b_in_corridor) {
        return;
    }

    layout.doors.push(layout::Door {
        from: edge.0,
        to: edge.1,
        width: 1,
        span_width: false,
        locked: kind == ManualDoorKind::Locked,
        archway: kind == ManualDoorKind::Archway,
        secret: kind == ManualDoorKind::Secret,
        manual: true,
    });
    app.suppressed_auto_door_edges.remove(&target);
}

// Checks whether manual door edge allowed.
pub fn manual_door_edge_allowed(
    a_room: Option<usize>,
    b_room: Option<usize>,
    a_in_corridor: bool,
    b_in_corridor: bool,
) -> bool {
    if matches!((a_room, b_room), (Some(a), Some(b)) if a == b) {
        return false;
    }

    let a_is_feature = a_room.is_some() || a_in_corridor;
    let b_is_feature = b_room.is_some() || b_in_corridor;
    a_is_feature || b_is_feature
}

// Draws add overlay.
pub fn draw_add_overlay(app: &DungeonApp, painter: &egui::Painter, geometry: &GridGeometry) {
    if app.settings.add_tool != AddTool::Corridor {
        return;
    }
    let Some(drag) = &app.add_corridor_drag else {
        return;
    };

    let start = cell_center(geometry, drag.start_cell.0, drag.start_cell.1);
    let end = cell_center(geometry, drag.current_cell.0, drag.current_cell.1);
    let stroke = Stroke::new(2.0, Color32::WHITE);
    painter.circle_filled(start, 3.0, Color32::WHITE);
    crate::rendering::draw_dashed_line(painter, start, end, stroke, 8.0, 6.0);
}

// Draws add tool ghost.
pub fn draw_add_tool_ghost(app: &DungeonApp, painter: &egui::Painter, geometry: &GridGeometry) {
    let tool = app.settings.add_tool;
    if tool == AddTool::None || tool == AddTool::Corridor {
        return;
    }

    let Some(pointer_pos) = painter.ctx().pointer_hover_pos() else {
        return;
    };
    let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry) else {
        return;
    };
    let cell = (grid_x.floor() as usize, grid_y.floor() as usize);

    let ghost_alpha = 0.35;

    match tool {
        AddTool::Room => {
            let width = app.settings.add_room_width.max(1);
            let height = app.settings.add_room_height.max(1);
            let max_x = app.settings.cols.saturating_sub(cell.0);
            let max_y = app.settings.rows.saturating_sub(cell.1);
            let w = width.min(max_x).max(1);
            let h = height.min(max_y).max(1);

            let left = geometry.rect.left() + cell.0 as f32 * geometry.cell_size;
            let top = geometry.rect.top() + cell.1 as f32 * geometry.cell_size;
            let right = left + w as f32 * geometry.cell_size;
            let bottom = top + h as f32 * geometry.cell_size;
            let rect = egui::Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, bottom));

            let fill = Color32::from_rgb(70, 120, 160).gamma_multiply(ghost_alpha);
            painter.rect_filled(rect, 4.0, fill);
            painter.rect_stroke(
                rect,
                4.0,
                Stroke::new(
                    1.5,
                    Color32::from_rgb(150, 200, 240).gamma_multiply(ghost_alpha + 0.3),
                ),
                egui::StrokeKind::Middle,
            );
        }
        AddTool::Text => {
            let text = app.settings.add_text_value.trim();
            if text.is_empty() {
                return;
            }
            let font_size = app.settings.add_text_font_size.max(8);
            let rect = crate::rendering::text_label_rect(
                geometry,
                &layout::TextLabel {
                    cell,
                    text: text.to_string(),
                    font_size,
                },
            );

            let fill = Color32::from_rgb(50, 50, 70).gamma_multiply(ghost_alpha);
            painter.rect_filled(rect.expand(4.0), 4.0, fill);
            painter.rect_stroke(
                rect.expand(4.0),
                4.0,
                Stroke::new(
                    1.0,
                    Color32::from_rgb(180, 180, 220).gamma_multiply(ghost_alpha + 0.2),
                ),
                egui::StrokeKind::Middle,
            );

            let color = Color32::WHITE.gamma_multiply(ghost_alpha + 0.1);
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::proportional(font_size as f32),
                color,
            );
        }
        AddTool::StartMarker
        | AddTool::EndMarker
        | AddTool::TrapMarker
        | AddTool::MonsterMarker => {
            let size = app.settings.add_marker_size.max(1);
            let origin = centered_marker_origin(cell, size, app.settings.cols, app.settings.rows);

            let rect = marker_rect(&layout::AreaMarker { cell: origin, size }, geometry);

            let base_color = match tool {
                AddTool::StartMarker => Color32::from_rgb(60, 220, 200),
                AddTool::EndMarker => Color32::from_rgb(240, 90, 90),
                AddTool::TrapMarker => Color32::from_rgb(150, 80, 230),
                AddTool::MonsterMarker => Color32::from_rgb(200, 50, 50),
                _ => Color32::WHITE,
            };

            let fill = base_color.gamma_multiply(ghost_alpha);
            painter.rect_filled(rect, 4.0, fill);
            painter.rect_stroke(
                rect,
                4.0,
                Stroke::new(
                    1.5,
                    Color32::from_rgb(255, 255, 255).gamma_multiply(ghost_alpha + 0.2),
                ),
                egui::StrokeKind::Middle,
            );

            let label = match tool {
                AddTool::StartMarker => "S",
                AddTool::EndMarker => "E",
                AddTool::TrapMarker => "T",
                AddTool::MonsterMarker => "M",
                _ => "?",
            };
            let text_color = Color32::WHITE.gamma_multiply(ghost_alpha + 0.15);
            let font_size = (geometry.cell_size * size as f32 * 0.5).clamp(14.0, 72.0);
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(font_size),
                text_color,
            );
        }
        AddTool::Staircase => {
            let width = app.settings.add_stair_width.max(1);
            let height = app.settings.add_stair_height.max(1);
            let max_x = app.settings.cols.saturating_sub(cell.0);
            let max_y = app.settings.rows.saturating_sub(cell.1);
            let w = width.min(max_x).max(1);
            let h = height.min(max_y).max(1);

            let left = geometry.rect.left() + cell.0 as f32 * geometry.cell_size;
            let top = geometry.rect.top() + cell.1 as f32 * geometry.cell_size;
            let right = left + w as f32 * geometry.cell_size;
            let bottom = top + h as f32 * geometry.cell_size;
            let rect = egui::Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, bottom));

            let fill = Color32::from_rgb(200, 150, 50).gamma_multiply(ghost_alpha);
            painter.rect_filled(rect, 4.0, fill);
            painter.rect_stroke(
                rect,
                4.0,
                Stroke::new(
                    1.5,
                    Color32::from_rgb(220, 180, 80).gamma_multiply(ghost_alpha + 0.2),
                ),
                egui::StrokeKind::Middle,
            );

            let steps = (h as f32 * 2.0).round() as usize;
            let step_height = (rect.height() / (steps as f32)).max(1.0);
            let line_color = Color32::from_rgb(150, 100, 30).gamma_multiply(ghost_alpha + 0.15);
            for i in 0..steps {
                let y = rect.top() + (i as f32 * step_height) + step_height * 0.5;
                painter.line_segment(
                    [
                        egui::pos2(rect.left() + 2.0, y),
                        egui::pos2(rect.right() - 2.0, y),
                    ],
                    Stroke::new(1.5, line_color),
                );
            }
        }
        AddTool::Archway | AddTool::Door | AddTool::LockedDoor | AddTool::SecretDoor => {
            if let Some((edge_a, edge_b)) = door_edge_at_pointer(app, pointer_pos, geometry) {
                let door_width_cells = 1;
                let style = match tool {
                    AddTool::Archway => crate::rendering::DoorLineStyle::Dotted,
                    AddTool::Door => crate::rendering::DoorLineStyle::LongDash,
                    AddTool::LockedDoor => crate::rendering::DoorLineStyle::ShortDash,
                    AddTool::SecretDoor => crate::rendering::DoorLineStyle::DashDotDot,
                    _ => crate::rendering::DoorLineStyle::Solid,
                };
                let color = match tool {
                    AddTool::Archway => Color32::from_rgb(230, 140, 60),
                    AddTool::Door => Color32::from_rgb(80, 200, 120),
                    AddTool::LockedDoor => Color32::from_rgb(220, 70, 70),
                    AddTool::SecretDoor => Color32::from_rgb(170, 80, 170),
                    _ => Color32::WHITE,
                };
                let ghost_color = color.gamma_multiply(ghost_alpha);
                crate::rendering::draw_door_line(
                    painter,
                    geometry,
                    edge_a,
                    edge_b,
                    door_width_cells,
                    Stroke::new((geometry.cell_size / 5.0).max(1.0), ghost_color),
                    style,
                );
            }
        }
        AddTool::None | AddTool::Corridor => {}
    }
}
