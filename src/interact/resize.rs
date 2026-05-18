/*
 * Resize interaction logic.
 * Handles resizing of rooms, text labels, and staircases using secondary click + drag.
 */

use super::{
    GridGeometry, ResizeCorner, ResizeState, RoomResizeState, StaircaseResizeState,
    TextResizeState, pointer_to_grid, reroute_corridors_for_room, resize_room_at_pointer,
    stair_at_pointer, text_at_pointer,
};
use crate::app::DungeonApp;
use crate::layout::{rects_overlap, rooms_overlap};
use crate::ui::AddTool;
use eframe::egui;

pub fn handle_resize(
    app: &mut DungeonApp,
    ctx: &egui::Context,
    response: &egui::Response,
    geometry: &GridGeometry,
) -> bool {
    if app.settings.add_tool != AddTool::None {
        return false;
    }

    if ctx.input(|i| i.pointer.secondary_pressed())
        && app.resize_state.is_none()
        && let Some(pointer_pos) = response.interact_pointer_pos()
    {
        if let Some(text_idx) = text_at_pointer(app, pointer_pos, geometry) {
            if let Some(state) = start_text_resize(app, text_idx) {
                app.push_undo_snapshot();
                app.resize_state = Some(ResizeState::Text(state));
                return true;
            }
        } else if let Some(stair_idx) = stair_at_pointer(app, pointer_pos, geometry) {
            if let Some(state) = start_staircase_resize(app, stair_idx, pointer_pos, geometry) {
                app.push_undo_snapshot();
                app.resize_state = Some(ResizeState::Staircase(state));
                return true;
            }
        } else if let Some(room_idx) = resize_room_at_pointer(app, pointer_pos, geometry)
            && let Some(state) = start_resize(app, room_idx, pointer_pos, geometry)
        {
            app.push_undo_snapshot();
            app.resize_state = Some(ResizeState::Room(state));
            return true;
        }
    }

    if let Some(state) = app.resize_state.clone() {
        if response.dragged_by(egui::PointerButton::Secondary)
            && ctx.input(|i| i.pointer.secondary_down())
            && let Some(pointer_pos) = response.interact_pointer_pos()
            && let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry)
        {
            let cell = (grid_x.floor() as usize, grid_y.floor() as usize);
            match state {
                ResizeState::Room(state) => resize_room(app, &state, cell),
                ResizeState::Text(state) => resize_text(app, &state, cell),
                ResizeState::Staircase(state) => resize_staircase(app, &state, cell),
            }
        }

        if ctx.input(|i| i.pointer.button_released(egui::PointerButton::Secondary)) {
            app.resize_state = None;
        }
        return true;
    }

    false
}

pub fn start_text_resize(app: &DungeonApp, text_idx: usize) -> Option<TextResizeState> {
    if app.settings.active_level_index >= app.levels.len() {
        return None;
    }
    let layout = &app.levels[app.settings.active_level_index];

    let label = layout.text_labels.get(text_idx)?;
    Some(TextResizeState {
        text_idx,
        origin_cell: label.cell,
        original_font_size: label.font_size,
    })
}

pub fn start_resize(
    app: &DungeonApp,
    room_idx: usize,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) -> Option<RoomResizeState> {
    if app.settings.active_level_index >= app.levels.len() {
        return None;
    }
    let layout = &app.levels[app.settings.active_level_index];

    let original_room = layout.rooms.get(room_idx)?.clone();
    let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;
    let center_x = original_room.x as f32 + original_room.width as f32 * 0.5;
    let center_y = original_room.y as f32 + original_room.height as f32 * 0.5;
    let pointer_x = grid_x;
    let pointer_y = grid_y;

    let resize_corner = match (pointer_x >= center_x, pointer_y >= center_y) {
        (false, false) => ResizeCorner::TopLeft,
        (true, false) => ResizeCorner::TopRight,
        (false, true) => ResizeCorner::BottomLeft,
        (true, true) => ResizeCorner::BottomRight,
    };

    Some(RoomResizeState {
        room_idx,
        original_room,
        resize_corner,
    })
}

pub fn start_staircase_resize(
    app: &DungeonApp,
    stair_idx: usize,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) -> Option<StaircaseResizeState> {
    if app.settings.active_level_index >= app.levels.len() {
        return None;
    }
    let layout = &app.levels[app.settings.active_level_index];

    let original_stair = layout.stairs.get(stair_idx)?.clone();
    let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;
    let center_x = original_stair.cell.0 as f32 + original_stair.width as f32 * 0.5;
    let center_y = original_stair.cell.1 as f32 + original_stair.height as f32 * 0.5;
    let pointer_x = grid_x;
    let pointer_y = grid_y;

    let resize_corner = match (pointer_x >= center_x, pointer_y >= center_y) {
        (false, false) => ResizeCorner::TopLeft,
        (true, false) => ResizeCorner::TopRight,
        (false, true) => ResizeCorner::BottomLeft,
        (true, true) => ResizeCorner::BottomRight,
    };

    Some(StaircaseResizeState {
        stair_idx,
        original_stair,
        resize_corner,
    })
}

pub fn resize_room(app: &mut DungeonApp, state: &RoomResizeState, target_cell: (usize, usize)) {
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let layout = &mut app.levels[app.settings.active_level_index];

    let cols = app.settings.cols.max(1);
    let rows = app.settings.rows.max(1);

    let tx = target_cell.0.min(cols - 1) as isize;
    let ty = target_cell.1.min(rows - 1) as isize;
    let original = &state.original_room;

    let left = original.x as isize;
    let top = original.y as isize;
    let right = (original.x + original.width - 1) as isize;
    let bottom = (original.y + original.height - 1) as isize;
    let (x0, x1, y0, y1) = match state.resize_corner {
        ResizeCorner::TopLeft => (tx.min(right), right, ty.min(bottom), bottom),
        ResizeCorner::TopRight => (left, tx.max(left), ty.min(bottom), bottom),
        ResizeCorner::BottomLeft => (tx.min(right), right, top, ty.max(top)),
        ResizeCorner::BottomRight => (left, tx.max(left), top, ty.max(top)),
    };

    let new_room = crate::layout::Room {
        x: x0 as usize,
        y: y0 as usize,
        width: (x1 - x0 + 1) as usize,
        height: (y1 - y0 + 1) as usize,
    };

    if new_room.width == 0 || new_room.height == 0 {
        return;
    }

    if new_room.x + new_room.width > cols || new_room.y + new_room.height > rows {
        return;
    }

    if layout
        .rooms
        .iter()
        .enumerate()
        .any(|(idx, room)| idx != state.room_idx && rooms_overlap(&new_room, room))
    {
        return;
    }

    if let Some(room) = layout.rooms.get_mut(state.room_idx) {
        room.x = new_room.x;
        room.y = new_room.y;
        room.width = new_room.width;
        room.height = new_room.height;
        reroute_corridors_for_room(app, state.room_idx);
        app.refresh_doors();
    }
}

pub fn resize_text(app: &mut DungeonApp, state: &TextResizeState, target_cell: (usize, usize)) {
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let layout = &mut app.levels[app.settings.active_level_index];

    let Some(label) = layout.text_labels.get_mut(state.text_idx) else {
        app.resize_state = None;
        return;
    };

    let dx = target_cell.0 as isize - state.origin_cell.0 as isize;
    let dy = target_cell.1 as isize - state.origin_cell.1 as isize;
    let delta_cells = if dx.abs() >= dy.abs() { dx } else { -dy };
    let new_size = (state.original_font_size as isize + (delta_cells * 4)).clamp(8, 96);
    label.font_size = new_size as u16;
}

pub fn resize_staircase(
    app: &mut DungeonApp,
    state: &StaircaseResizeState,
    target_cell: (usize, usize),
) {
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }

    let cols = app.settings.cols.max(1);
    let rows = app.settings.rows.max(1);

    let tx = target_cell.0.min(cols - 1) as isize;
    let ty = target_cell.1.min(rows - 1) as isize;
    let original = &state.original_stair;

    let left = original.cell.0 as isize;
    let top = original.cell.1 as isize;
    let right = (original.cell.0 + original.width - 1) as isize;
    let bottom = (original.cell.1 + original.height - 1) as isize;

    let (x0, x1, y0, y1) = match state.resize_corner {
        ResizeCorner::TopLeft => (tx.min(right), right, ty.min(bottom), bottom),
        ResizeCorner::TopRight => (left, tx.max(left), ty.min(bottom), bottom),
        ResizeCorner::BottomLeft => (tx.min(right), right, top, ty.max(top)),
        ResizeCorner::BottomRight => (left, tx.max(left), top, ty.max(top)),
    };

    let new_stair_cell = (x0 as usize, y0 as usize);
    let new_stair_width = (x1 - x0 + 1) as usize;
    let new_stair_height = (y1 - y0 + 1) as usize;

    if new_stair_width == 0 || new_stair_height == 0 {
        return;
    }

    let active_layout = &app.levels[app.settings.active_level_index];
    if active_layout.stairs.iter().enumerate().any(|(idx, stair)| {
        idx != state.stair_idx
            && rects_overlap(
                new_stair_cell.0,
                new_stair_cell.1,
                new_stair_width,
                new_stair_height,
                stair.cell.0,
                stair.cell.1,
                stair.width,
                stair.height,
            )
    }) {
        return;
    }

    if app.settings.sync_stairs_across_levels {
        for layout in &mut app.levels {
            for stair in &mut layout.stairs {
                if stair.cell == state.original_stair.cell {
                    stair.cell = new_stair_cell;
                    stair.width = new_stair_width;
                    stair.height = new_stair_height;
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
                if stair.cell == state.original_stair.cell {
                    stair.cell = new_stair_cell;
                    stair.width = new_stair_width;
                    stair.height = new_stair_height;
                }
            }
        }
    }

    if let Some(ResizeState::Staircase(ref mut s)) = app.resize_state {
        s.original_stair.cell = new_stair_cell;
        s.original_stair.width = new_stair_width;
        s.original_stair.height = new_stair_height;
    }
}
