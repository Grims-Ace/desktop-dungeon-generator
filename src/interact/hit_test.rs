/*
 * Hit-testing and hover target detection for interactive dungeon editing.
 */

use super::{
    CorridorDragState, GridGeometry, HoverMarker, MarkerDragState, MarkerKind, StaircaseDragState,
    door_edges_for, marker_contains_cell, normalized_edge, pointer_to_grid, resize_hit_cells,
};
use crate::app::DungeonApp;
use eframe::egui;

// Handles update hover targets.
pub fn update_hover_targets(app: &mut DungeonApp, ctx: &egui::Context, geometry: &GridGeometry) {
    if app.drag_state.is_some()
        || app.resize_state.is_some()
        || app.add_corridor_drag.is_some()
        || app.add_door_drag.is_some()
        || app.selection_box_drag.is_some()
        || app.multi_drag_state.is_some()
        || app.multi_resize_state.is_some()
    {
        return;
    }

    let Some(pointer_pos) = ctx.pointer_hover_pos() else {
        app.clear_hover_targets();
        return;
    };
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let layout = &app.levels[app.settings.active_level_index];

    if let Some(edge) = door_edge_at_pointer(app, pointer_pos, geometry) {
        let target = normalized_edge(edge.0, edge.1);
        app.hover_door_idx = layout.doors.iter().position(|d| {
            door_edges_for(d, app.settings.cols, app.settings.rows)
                .iter()
                .any(|e| normalized_edge(e.0, e.1) == target)
        });
        if app.hover_door_idx.is_some() {
            app.hover_room_idx = None;
            app.hover_corridor_idx = None;
            app.hover_text_idx = None;
            app.hover_marker = None;
            return;
        }
    }

    app.hover_text_idx = text_at_pointer(app, pointer_pos, geometry);
    if app.hover_text_idx.is_some() {
        app.hover_room_idx = None;
        app.hover_corridor_idx = None;
        app.hover_door_idx = None;
        app.hover_marker = None;
        return;
    }

    app.hover_marker = marker_at_pointer(app, pointer_pos, geometry);
    if app.hover_marker.is_some() {
        app.hover_room_idx = None;
        app.hover_corridor_idx = None;
        app.hover_door_idx = None;
        app.hover_stair_idx = None;
        return;
    }

    app.hover_stair_idx = stair_at_pointer(app, pointer_pos, geometry);
    if app.hover_stair_idx.is_some() {
        app.hover_room_idx = None;
        app.hover_corridor_idx = None;
        app.hover_door_idx = None;
        app.hover_marker = None;
        return;
    }

    if let Some((room_idx, _, _)) = room_at_pointer(app, pointer_pos, geometry) {
        app.hover_room_idx = Some(room_idx);
        app.hover_corridor_idx = None;
        app.hover_door_idx = None;
        app.hover_marker = None;
        println!("DEBUG: Hovering over room {}", room_idx);
    } else {
        app.hover_room_idx = None;
        if let Some(corridor_drag) = corridor_drag_at_pointer(app, pointer_pos, geometry) {
            app.hover_corridor_idx = Some(corridor_drag.corridor_index);
            app.hover_door_idx = None;
            app.hover_marker = None;
        } else {
            app.hover_corridor_idx = None;
            app.hover_door_idx = None;
            app.hover_marker = None;
        }
    }
}

// Computes edge at pointer.
pub fn door_edge_at_pointer(
    app: &DungeonApp,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) -> Option<((usize, usize), (usize, usize))> {
    let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;
    let cell_x = grid_x.floor() as isize;
    let cell_y = grid_y.floor() as isize;
    if cell_x < 0
        || cell_y < 0
        || cell_x >= app.settings.cols as isize
        || cell_y >= app.settings.rows as isize
    {
        return None;
    }

    let fx = grid_x - grid_x.floor();
    let fy = grid_y - grid_y.floor();
    let left = fx;
    let right = 1.0 - fx;
    let top = fy;
    let bottom = 1.0 - fy;

    let mut candidates = [
        (left, (cell_x - 1, cell_y), (cell_x, cell_y)),
        (right, (cell_x + 1, cell_y), (cell_x, cell_y)),
        (top, (cell_x, cell_y - 1), (cell_x, cell_y)),
        (bottom, (cell_x, cell_y + 1), (cell_x, cell_y)),
    ];
    candidates.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let (dist, a, b) = candidates[0];
    if dist > 0.2 {
        return None;
    }

    let (ax, ay) = a;
    let (bx, by) = b;
    if ax < 0
        || ay < 0
        || bx < 0
        || by < 0
        || ax >= app.settings.cols as isize
        || bx >= app.settings.cols as isize
        || ay >= app.settings.rows as isize
        || by >= app.settings.rows as isize
    {
        return None;
    }

    Some(((ax as usize, ay as usize), (bx as usize, by as usize)))
}

// Computes at pointer.
pub fn room_at_pointer(
    app: &DungeonApp,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) -> Option<(usize, f32, f32)> {
    let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;

    if app.settings.active_level_index >= app.levels.len() {
        return None;
    }
    let layout = &app.levels[app.settings.active_level_index];

    for (room_idx, room) in layout.rooms.iter().enumerate().rev() {
        let room_x = room.x as f32;
        let room_y = room.y as f32;
        let room_right = room_x + room.width as f32;
        let room_bottom = room_y + room.height as f32;

        if grid_x >= room_x && grid_x < room_right && grid_y >= room_y && grid_y < room_bottom {
            return Some((room_idx, grid_x - room_x, grid_y - room_y));
        }
    }

    None
}

// Computes at pointer.
pub fn text_at_pointer(
    app: &DungeonApp,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) -> Option<usize> {
    if app.settings.active_level_index >= app.levels.len() {
        return None;
    }
    let layout = &app.levels[app.settings.active_level_index];

    for (idx, label) in layout.text_labels.iter().enumerate().rev() {
        let rect = crate::rendering::text_label_rect(geometry, label);
        if rect.contains(pointer_pos) {
            return Some(idx);
        }
    }

    None
}

// Computes at pointer.
pub fn marker_at_pointer(
    app: &DungeonApp,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) -> Option<HoverMarker> {
    let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;
    let clicked = (grid_x.floor() as usize, grid_y.floor() as usize);
    if app.settings.active_level_index >= app.levels.len() {
        return None;
    }
    let layout = &app.levels[app.settings.active_level_index];

    layout
        .start_markers
        .iter()
        .enumerate()
        .find(|(_, marker)| marker_contains_cell(Some(marker), clicked))
        .map(|(idx, _)| HoverMarker::Start(idx))
        .or_else(|| {
            layout
                .end_markers
                .iter()
                .enumerate()
                .find(|(_, marker)| marker_contains_cell(Some(marker), clicked))
                .map(|(idx, _)| HoverMarker::End(idx))
        })
        .or_else(|| {
            layout
                .trap_markers
                .iter()
                .enumerate()
                .find(|(_, marker)| marker_contains_cell(Some(marker), clicked))
                .map(|(idx, _)| HoverMarker::Trap(idx))
        })
        .or_else(|| {
            layout
                .monster_markers
                .iter()
                .enumerate()
                .find(|(_, marker)| marker_contains_cell(Some(marker), clicked))
                .map(|(idx, _)| HoverMarker::Monster(idx))
        })
}

// Computes drag at pointer.
pub fn marker_drag_at_pointer(
    app: &DungeonApp,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) -> Option<MarkerDragState> {
    let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;
    if app.settings.active_level_index >= app.levels.len() {
        return None;
    }
    let layout = &app.levels[app.settings.active_level_index];

    match marker_at_pointer(app, pointer_pos, geometry)? {
        HoverMarker::Start(marker_idx) => {
            let marker = layout.start_markers.get(marker_idx)?;
            Some(MarkerDragState {
                kind: MarkerKind::Start,
                marker_idx,
                offset_x: grid_x - marker.cell.0 as f32,
                offset_y: grid_y - marker.cell.1 as f32,
            })
        }
        HoverMarker::End(marker_idx) => {
            let marker = layout.end_markers.get(marker_idx)?;
            Some(MarkerDragState {
                kind: MarkerKind::End,
                marker_idx,
                offset_x: grid_x - marker.cell.0 as f32,
                offset_y: grid_y - marker.cell.1 as f32,
            })
        }
        HoverMarker::Trap(marker_idx) => {
            let marker = layout.trap_markers.get(marker_idx)?;
            Some(MarkerDragState {
                kind: MarkerKind::Trap,
                marker_idx,
                offset_x: grid_x - marker.cell.0 as f32,
                offset_y: grid_y - marker.cell.1 as f32,
            })
        }
        HoverMarker::Monster(marker_idx) => {
            let marker = layout.monster_markers.get(marker_idx)?;
            Some(MarkerDragState {
                kind: MarkerKind::Monster,
                marker_idx,
                offset_x: grid_x - marker.cell.0 as f32,
                offset_y: grid_y - marker.cell.1 as f32,
            })
        }
    }
}

// Handles stair at pointer.
pub fn stair_at_pointer(
    app: &DungeonApp,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) -> Option<usize> {
    let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;
    let clicked = (grid_x.floor() as usize, grid_y.floor() as usize);
    if app.settings.active_level_index >= app.levels.len() {
        return None;
    }
    let layout = &app.levels[app.settings.active_level_index];

    layout.stairs.iter().position(|stair| {
        clicked.0 >= stair.cell.0
            && clicked.0 < stair.cell.0 + stair.width
            && clicked.1 >= stair.cell.1
            && clicked.1 < stair.cell.1 + stair.height
    })
}

// Handles stair drag at pointer.
pub fn stair_drag_at_pointer(
    app: &DungeonApp,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) -> Option<StaircaseDragState> {
    let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;
    if app.settings.active_level_index >= app.levels.len() {
        return None;
    }
    let stair_idx = stair_at_pointer(app, pointer_pos, geometry)?;
    let layout = &app.levels[app.settings.active_level_index];
    let stair = layout.stairs.get(stair_idx)?;

    Some(StaircaseDragState {
        stair_idx,
        offset_x: grid_x - stair.cell.0 as f32,
        offset_y: grid_y - stair.cell.1 as f32,
        original_cell: stair.cell,
    })
}

// Resizes room at pointer.
pub fn resize_room_at_pointer(
    app: &DungeonApp,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) -> Option<usize> {
    if let Some((room_idx, _, _)) = room_at_pointer(app, pointer_pos, geometry) {
        return Some(room_idx);
    }

    let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;
    let clicked = (
        grid_x.floor().max(0.0) as usize,
        grid_y.floor().max(0.0) as usize,
    );

    if app.settings.active_level_index >= app.levels.len() {
        return None;
    }
    let layout = &app.levels[app.settings.active_level_index];

    for (room_idx, room) in layout.rooms.iter().enumerate().rev() {
        if resize_hit_cells(room, room_idx, &layout.rooms, geometry.cols, geometry.rows)
            .contains(&clicked)
        {
            return Some(room_idx);
        }
    }

    None
}

// Computes drag at pointer.
pub fn corridor_drag_at_pointer(
    app: &DungeonApp,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) -> Option<CorridorDragState> {
    let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;
    let clicked = (grid_x.floor() as usize, grid_y.floor() as usize);
    if app.settings.active_level_index >= app.levels.len() {
        return None;
    }
    let layout = &app.levels[app.settings.active_level_index];

    let mut room_cells = std::collections::HashSet::new();
    for room in &layout.rooms {
        for x in room.x..(room.x + room.width) {
            for y in room.y..(room.y + room.height) {
                room_cells.insert((x, y));
            }
        }
    }

    let corridor_index = layout.corridors.iter().position(|corridor| {
        corridor
            .cells(app.settings.cols, app.settings.rows, &room_cells)
            .contains(&clicked)
    })?;

    Some(CorridorDragState {
        corridor_index,
        original_path: layout.corridors[corridor_index].path.clone(),
        original_cell: clicked,
    })
}
