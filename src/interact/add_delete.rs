/*
 * Manual addition and deletion tools.
 * Handles placing new rooms, corridors, doors, markers, and text labels,
 * as well as deleting existing elements via pointer interaction.
 */

use super::{
    AddCorridorDrag, GridGeometry, HoverMarker, MarkerKind, centered_marker_origin,
    corridor_drag_at_pointer, door_edge_at_pointer, door_edges_for, marker_at_pointer,
    normalized_edge, pointer_to_grid, room_at_pointer, stair_at_pointer, text_at_pointer,
};
use crate::app::{DungeonApp, ManualDoorKind};
use crate::layout::{
    self, blocked_room_cells, corridor_cells, room_index_at_cell, rooms_overlap,
    shortest_path_cells,
};
use crate::ui::AddTool;
use eframe::egui;

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
