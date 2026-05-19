/*
 * Drag-and-drop interaction logic.
 * Handles the movement of rooms, text labels, markers, corridors, and staircases.
 */

use super::{
    CorridorDragState, DragState, GridGeometry, MarkerDragState, MarkerKind, RoomDragState,
    StaircaseDragState, TextDragState, cell_center, corridor_drag_at_pointer, door_render_width,
    marker_drag_at_pointer, pointer_to_grid, reroute_path_through_cell, room_at_pointer,
    stair_drag_at_pointer, text_at_pointer,
};
use crate::app::DungeonApp;
use crate::layout::{blocked_room_cells, shortest_path_cells};
use crate::ui::AddTool;
use eframe::egui;
use egui::{Color32, Stroke};

// Handles active drag gestures for rooms, corridors, markers, stairs, and text.
pub fn handle_drag(
    app: &mut DungeonApp,
    ctx: &egui::Context,
    response: &egui::Response,
    geometry: &GridGeometry,
) {
    if !ctx.input(|i| i.pointer.primary_down()) && !response.drag_started() {
        return;
    }
    if app.settings.add_tool != AddTool::None {
        return;
    }
    if response.drag_started()
        && let Some(pointer_pos) = response.interact_pointer_pos()
        && let Some(text_idx) = text_at_pointer(app, pointer_pos, geometry)
    {
        app.push_undo_snapshot();
        app.drag_state = Some(DragState::Text(TextDragState { text_idx }));
    } else if response.drag_started()
        && let Some(pointer_pos) = response.interact_pointer_pos()
        && let Some(marker_drag) = marker_drag_at_pointer(app, pointer_pos, geometry)
    {
        app.push_undo_snapshot();
        app.drag_state = Some(DragState::Marker(marker_drag));
    } else if response.drag_started()
        && let Some(pointer_pos) = response.interact_pointer_pos()
        && let Some(stair_drag) = stair_drag_at_pointer(app, pointer_pos, geometry)
    {
        app.push_undo_snapshot();
        app.drag_state = Some(DragState::Staircase(stair_drag));
    } else if response.drag_started()
        && let Some(pointer_pos) = response.interact_pointer_pos()
        && let Some(corridor_drag) = corridor_drag_at_pointer(app, pointer_pos, geometry)
    {
        app.push_undo_snapshot();
        app.drag_state = Some(DragState::Corridor(corridor_drag));
    } else if response.drag_started()
        && let Some(pointer_pos) = response.interact_pointer_pos()
        && let Some((room_idx, offset_x, offset_y)) = room_at_pointer(app, pointer_pos, geometry)
    {
        app.push_undo_snapshot();
        app.drag_state = Some(DragState::Room(RoomDragState {
            room_idx,
            offset_x,
            offset_y,
        }));
    }

    if response.dragged()
        && let Some(pointer_pos) = response.interact_pointer_pos()
        && let Some(drag_state) = app.drag_state.clone()
    {
        match drag_state {
            DragState::Room(drag) => drag_room_to_pointer(app, drag, pointer_pos, geometry),
            DragState::Text(drag) => drag_text_to_pointer(app, drag, pointer_pos, geometry),
            DragState::Marker(drag) => drag_marker_to_pointer(app, drag, pointer_pos, geometry),
            DragState::Corridor(drag) => drag_corridor_to_pointer(app, drag, pointer_pos, geometry),
            DragState::Staircase(drag) => drag_stair_to_pointer(app, drag, pointer_pos, geometry),
        }
    }

    if response.drag_stopped() || !response.ctx.input(|i| i.pointer.primary_down()) {
        app.drag_state = None;
    }
}

// Drags room to pointer.
pub fn drag_room_to_pointer(
    app: &mut DungeonApp,
    drag: RoomDragState,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) {
    let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry) else {
        return;
    };

    if app.settings.active_level_index >= app.levels.len() {
        return;
    }

    if drag.room_idx >= app.levels[app.settings.active_level_index].rooms.len() {
        app.drag_state = None;
        return;
    }

    let (width, height, old_x, old_y) = {
        let room = &app.levels[app.settings.active_level_index].rooms[drag.room_idx];
        (room.width, room.height, room.x, room.y)
    };
    let max_x = app.settings.cols.saturating_sub(width);
    let max_y = app.settings.rows.saturating_sub(height);

    let target_x = (grid_x - drag.offset_x).floor().max(0.0) as usize;
    let target_y = (grid_y - drag.offset_y).floor().max(0.0) as usize;
    let new_x = target_x.min(max_x);
    let new_y = target_y.min(max_y);

    if new_x == old_x && new_y == old_y {
        return;
    }

    app.levels[app.settings.active_level_index].rooms[drag.room_idx].x = new_x;
    app.levels[app.settings.active_level_index].rooms[drag.room_idx].y = new_y;
    reroute_corridors_for_room(app, drag.room_idx);
    app.refresh_doors();
}

// Drags text to pointer.
pub fn drag_text_to_pointer(
    app: &mut DungeonApp,
    drag: TextDragState,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) {
    let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry) else {
        return;
    };

    if app.settings.active_level_index >= app.levels.len() {
        return;
    }

    if drag.text_idx
        >= app.levels[app.settings.active_level_index]
            .text_labels
            .len()
    {
        app.drag_state = None;
        return;
    }

    let target = (
        (grid_x.floor().max(0.0) as usize).min(app.settings.cols.saturating_sub(1)),
        (grid_y.floor().max(0.0) as usize).min(app.settings.rows.saturating_sub(1)),
    );

    app.levels[app.settings.active_level_index].text_labels[drag.text_idx].cell = target;
}

// Reroutes corridors for room.
pub fn reroute_corridors_for_room(app: &mut DungeonApp, room_idx: usize) {
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let layout = &mut app.levels[app.settings.active_level_index];

    if layout.packed_rooms {
        return;
    }

    for corridor in &mut layout.corridors {
        if corridor.start_room_id != room_idx && corridor.end_room_id != room_idx {
            continue;
        }

        let start = layout.rooms[corridor.start_room_id].center_cell();
        let end = layout.rooms[corridor.end_room_id].center_cell();
        let blocked = blocked_room_cells(
            &layout.rooms,
            &[corridor.start_room_id, corridor.end_room_id],
        );
        if let Some(path) =
            shortest_path_cells(start, end, app.settings.cols, app.settings.rows, &blocked)
        {
            corridor.path = path;
        }
    }
}

// Drags corridor to pointer.
pub fn drag_corridor_to_pointer(
    app: &mut DungeonApp,
    drag: CorridorDragState,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) {
    let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry) else {
        return;
    };

    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let layout = &mut app.levels[app.settings.active_level_index];

    if drag.corridor_index >= layout.corridors.len() {
        app.drag_state = None;
        return;
    }
    let corridor = &layout.corridors[drag.corridor_index];

    let target = (
        (grid_x.floor().max(0.0) as usize).min(app.settings.cols.saturating_sub(1)),
        (grid_y.floor().max(0.0) as usize).min(app.settings.rows.saturating_sub(1)),
    );

    let Some(new_path) = reroute_path_through_cell(
        &drag.original_path,
        drag.original_cell,
        target,
        app.settings.cols,
        app.settings.rows,
        &blocked_room_cells(
            &layout.rooms,
            &[corridor.start_room_id, corridor.end_room_id],
        ),
    ) else {
        return;
    };

    layout.corridors[drag.corridor_index].path = new_path;
    app.refresh_doors();
}

// Drags marker to pointer.
pub fn drag_marker_to_pointer(
    app: &mut DungeonApp,
    drag: MarkerDragState,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) {
    let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry) else {
        return;
    };

    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let layout = &mut app.levels[app.settings.active_level_index];

    let markers = match drag.kind {
        MarkerKind::Start => &mut layout.start_markers,
        MarkerKind::End => &mut layout.end_markers,
        MarkerKind::Trap => &mut layout.trap_markers,
        MarkerKind::Monster => &mut layout.monster_markers,
    };
    let Some(marker) = markers.get_mut(drag.marker_idx) else {
        app.drag_state = None;
        return;
    };

    let max_x = app.settings.cols.saturating_sub(marker.size);
    let max_y = app.settings.rows.saturating_sub(marker.size);
    let target_x = (grid_x - drag.offset_x).floor().max(0.0) as usize;
    let target_y = (grid_y - drag.offset_y).floor().max(0.0) as usize;
    marker.cell = (target_x.min(max_x), target_y.min(max_y));
}

// Drags stair to pointer.
pub fn drag_stair_to_pointer(
    app: &mut DungeonApp,
    drag: StaircaseDragState,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) {
    let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry) else {
        return;
    };

    if app.settings.active_level_index >= app.levels.len() {
        return;
    }

    let (stair_width, stair_height) = {
        let layout = &app.levels[app.settings.active_level_index];
        let Some(stair) = layout.stairs.get(drag.stair_idx) else {
            return;
        };
        (stair.width, stair.height)
    };

    let max_x = app.settings.cols.saturating_sub(stair_width);
    let max_y = app.settings.rows.saturating_sub(stair_height);
    let target_x = (grid_x - drag.offset_x).floor().max(0.0) as usize;
    let target_y = (grid_y - drag.offset_y).floor().max(0.0) as usize;
    let new_cell = (target_x.min(max_x), target_y.min(max_y));

    if app.settings.sync_stairs_across_levels {
        for layout in &mut app.levels {
            for stair in &mut layout.stairs {
                if stair.cell == drag.original_cell {
                    stair.cell = new_cell;
                }
            }
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
            let layout = &mut app.levels[idx];
            for stair in &mut layout.stairs {
                if stair.cell == drag.original_cell {
                    stair.cell = new_cell;
                }
            }
        }
    }

    if let Some(DragState::Staircase(ref mut d)) = app.drag_state {
        d.original_cell = new_cell;
    }
}

// Draws corridor hover overlay.
pub fn draw_corridor_hover_overlay(
    app: &DungeonApp,
    painter: &egui::Painter,
    geometry: &GridGeometry,
) {
    let Some(idx) = app.hover_corridor_idx else {
        return;
    };
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let layout = &app.levels[app.settings.active_level_index];

    let Some(corridor) = layout.corridors.get(idx) else {
        return;
    };

    let color = Color32::from_gray(170);
    let stroke = Stroke::new(2.0, color);
    for pair in corridor.path.windows(2) {
        let a = cell_center(geometry, pair[0].0, pair[0].1);
        let b = cell_center(geometry, pair[1].0, pair[1].1);
        crate::rendering::draw_dashed_line(painter, a, b, stroke, 6.0, 6.0);
    }
}

// Draws door hover overlay.
pub fn draw_door_hover_overlay(app: &DungeonApp, painter: &egui::Painter, geometry: &GridGeometry) {
    let Some(idx) = app.hover_door_idx else {
        return;
    };
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let layout = &app.levels[app.settings.active_level_index];

    let Some(door) = layout.doors.get(idx) else {
        return;
    };
    let color = Color32::from_gray(200);
    let stroke = Stroke::new(2.0, color);
    crate::rendering::draw_door_line(
        painter,
        geometry,
        door.from,
        door.to,
        door_render_width(door),
        stroke,
        crate::rendering::DoorLineStyle::LongDash,
    );
}
