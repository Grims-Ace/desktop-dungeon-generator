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
use crate::layout::{self, rects_overlap};
use crate::ui::AddTool;
use eframe::egui;
use egui::{Color32, Stroke};

// Handles secondary-button resize gestures for resizable layout elements.
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
                ResizeState::Room(ref state) => update_room_ghost(app, state, cell),
                ResizeState::Text(ref state) => update_text_ghost(app, state, cell),
                ResizeState::Staircase(ref state) => update_staircase_ghost(app, state, cell),
            }
        }

        if ctx.input(|i| i.pointer.button_released(egui::PointerButton::Secondary)) {
            match state {
                ResizeState::Room(ref state) => apply_room_resize(app, state),
                ResizeState::Text(ref state) => apply_text_resize(app, state),
                ResizeState::Staircase(ref state) => apply_staircase_resize(app, state),
            }
            app.resize_state = None;
        }
        return true;
    }

    false
}

// Starts text resize.
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
        current_font_size: label.font_size,
    })
}

// Starts a room resize from a pointer position.
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
        original_room: original_room.clone(),
        current_room: original_room,
        resize_corner,
    })
}

// Starts staircase resize.
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
        original_stair: original_stair.clone(),
        current_stair: original_stair,
        resize_corner,
    })
}

// Updates room ghost dimensions during resize.
pub fn update_room_ghost(
    app: &mut DungeonApp,
    state: &RoomResizeState,
    target_cell: (usize, usize),
) {
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

    if let Some(ResizeState::Room(ref mut s)) = app.resize_state {
        s.current_room = new_room;
    }
}

// Applies final room resize.
pub fn apply_room_resize(app: &mut DungeonApp, state: &RoomResizeState) {
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let room_idx = state.room_idx;
    let new_room = state.current_room.clone();
    let layout = &mut app.levels[app.settings.active_level_index];

    if let Some(room) = layout.rooms.get_mut(room_idx) {
        room.x = new_room.x;
        room.y = new_room.y;
        room.width = new_room.width;
        room.height = new_room.height;
        reroute_corridors_for_room(app, room_idx);
        app.refresh_doors();
    }
}

// Updates text ghost font size.
pub fn update_text_ghost(
    app: &mut DungeonApp,
    state: &TextResizeState,
    target_cell: (usize, usize),
) {
    let dx = target_cell.0 as isize - state.origin_cell.0 as isize;
    let dy = target_cell.1 as isize - state.origin_cell.1 as isize;
    let delta_cells = if dx.abs() >= dy.abs() { dx } else { -dy };
    let new_size = (state.original_font_size as isize + (delta_cells * 4)).clamp(8, 96);

    if let Some(ResizeState::Text(ref mut s)) = app.resize_state {
        s.current_font_size = new_size as u16;
    }
}

// Applies final text resize.
pub fn apply_text_resize(app: &mut DungeonApp, state: &TextResizeState) {
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let layout = &mut app.levels[app.settings.active_level_index];
    if let Some(label) = layout.text_labels.get_mut(state.text_idx) {
        label.font_size = state.current_font_size;
    }
}

// Updates staircase ghost dimensions.
pub fn update_staircase_ghost(
    app: &mut DungeonApp,
    state: &StaircaseResizeState,
    target_cell: (usize, usize),
) {
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

    if let Some(active_layout) = app.levels.get(app.settings.active_level_index) {
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
    }

    if let Some(ResizeState::Staircase(ref mut s)) = app.resize_state {
        s.current_stair.cell = new_stair_cell;
        s.current_stair.width = new_stair_width;
        s.current_stair.height = new_stair_height;
    }
}

// Applies final staircase resize.
pub fn apply_staircase_resize(app: &mut DungeonApp, state: &StaircaseResizeState) {
    let new_stair_cell = state.current_stair.cell;
    let new_stair_width = state.current_stair.width;
    let new_stair_height = state.current_stair.height;

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
}

// Draws resize overlay.
pub fn draw_resize_overlay(app: &DungeonApp, painter: &egui::Painter, geometry: &GridGeometry) {
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let layout = &app.levels[app.settings.active_level_index];

    if let Some(state) = &app.resize_state {
        match state {
            ResizeState::Text(state) => {
                if let Some(label) = layout.text_labels.get(state.text_idx) {
                    let mut ghost_label = label.clone();
                    ghost_label.font_size = state.current_font_size;
                    let rect =
                        crate::rendering::text_label_rect(geometry, &ghost_label).expand(3.0);
                    painter.rect_stroke(
                        rect,
                        0.0,
                        Stroke::new(2.0_f32, Color32::from_gray(180)),
                        egui::StrokeKind::Middle,
                    );
                    let pos = crate::interact::cell_center(
                        geometry,
                        ghost_label.cell.0,
                        ghost_label.cell.1,
                    );
                    painter.text(
                        pos,
                        egui::Align2::CENTER_CENTER,
                        &ghost_label.text,
                        egui::FontId::proportional(ghost_label.font_size as f32),
                        Color32::from_gray(200).gamma_multiply(0.5),
                    );
                }
            }
            ResizeState::Room(state) => {
                crate::rendering::draw_layout(
                    painter,
                    geometry,
                    &crate::layout::DungeonLayout {
                        rooms: vec![state.current_room.clone()],
                        ..crate::layout::DungeonLayout::empty(false)
                    },
                    app.settings.colorblind_mode,
                    None,
                    None,
                    None,
                    0.5,
                );
                draw_room_resize_visuals(painter, geometry, &state.current_room, true);
            }
            ResizeState::Staircase(state) => {
                crate::rendering::draw_staircase(
                    painter,
                    geometry,
                    &state.current_stair,
                    app.settings.colorblind_mode,
                    false,
                    0.5,
                );
                draw_staircase_resize_visuals(painter, geometry, &state.current_stair, true);
            }
        }
        return;
    }

    if let Some(text_idx) = app.hover_text_idx {
        if let Some(label) = layout.text_labels.get(text_idx) {
            let rect = crate::rendering::text_label_rect(geometry, label).expand(3.0);
            painter.rect_stroke(
                rect,
                0.0,
                Stroke::new(2.0_f32, Color32::from_gray(180)),
                egui::StrokeKind::Middle,
            );
        }
    } else if let Some(room_idx) = app.hover_room_idx {
        if let Some(room) = layout.rooms.get(room_idx) {
            draw_room_resize_visuals(painter, geometry, room, false);
        }
    } else if let Some(stair_idx) = app.hover_stair_idx
        && let Some(stair) = layout.stairs.get(stair_idx)
    {
        draw_staircase_resize_visuals(painter, geometry, stair, false);
    }
}

// Draws room resize visuals.
pub fn draw_room_resize_visuals(
    painter: &egui::Painter,
    geometry: &GridGeometry,
    room: &layout::Room,
    show_handles: bool,
) {
    let color = Color32::from_gray(170);
    let stroke = Stroke::new(1.5_f32, color);

    let left = geometry.rect.left() + room.x as f32 * geometry.cell_size;
    let top = geometry.rect.top() + room.y as f32 * geometry.cell_size;
    let right = left + room.width as f32 * geometry.cell_size;
    let bottom = top + room.height as f32 * geometry.cell_size;

    let tl = egui::pos2(left, top);
    let tr = egui::pos2(right, top);
    let br = egui::pos2(right, bottom);
    let bl = egui::pos2(left, bottom);

    crate::rendering::draw_dashed_line(painter, tl, tr, stroke, 6.0, 6.0);
    crate::rendering::draw_dashed_line(painter, tr, br, stroke, 6.0, 6.0);
    crate::rendering::draw_dashed_line(painter, br, bl, stroke, 6.0, 6.0);
    crate::rendering::draw_dashed_line(painter, bl, tl, stroke, 6.0, 6.0);

    let font = egui::FontId::proportional(12.0);
    let center_top = egui::pos2((left + right) * 0.5, top - 10.0);
    let center_left = egui::pos2(left - 12.0, (top + bottom) * 0.5);
    painter.text(
        center_top,
        egui::Align2::CENTER_CENTER,
        "<->",
        font.clone(),
        color,
    );
    painter.text(center_left, egui::Align2::CENTER_CENTER, "^v", font, color);

    if show_handles {
        draw_resize_handle(painter, tl, ResizeCorner::TopLeft, color);
        draw_resize_handle(painter, tr, ResizeCorner::TopRight, color);
        draw_resize_handle(painter, br, ResizeCorner::BottomRight, color);
        draw_resize_handle(painter, bl, ResizeCorner::BottomLeft, color);
    }
}

// Draws staircase resize visuals.
pub fn draw_staircase_resize_visuals(
    painter: &egui::Painter,
    geometry: &GridGeometry,
    stair: &layout::Staircase,
    show_handles: bool,
) {
    let color = Color32::from_gray(170);
    let stroke = Stroke::new(1.5_f32, color);

    let left = geometry.rect.left() + stair.cell.0 as f32 * geometry.cell_size;
    let top = geometry.rect.top() + stair.cell.1 as f32 * geometry.cell_size;
    let right = left + stair.width as f32 * geometry.cell_size;
    let bottom = top + stair.height as f32 * geometry.cell_size;

    let tl = egui::pos2(left, top);
    let tr = egui::pos2(right, top);
    let br = egui::pos2(right, bottom);
    let bl = egui::pos2(left, bottom);

    crate::rendering::draw_dashed_line(painter, tl, tr, stroke, 6.0, 6.0);
    crate::rendering::draw_dashed_line(painter, tr, br, stroke, 6.0, 6.0);
    crate::rendering::draw_dashed_line(painter, br, bl, stroke, 6.0, 6.0);
    crate::rendering::draw_dashed_line(painter, bl, tl, stroke, 6.0, 6.0);

    if show_handles {
        draw_resize_handle(painter, tl, ResizeCorner::TopLeft, color);
        draw_resize_handle(painter, tr, ResizeCorner::TopRight, color);
        draw_resize_handle(painter, br, ResizeCorner::BottomRight, color);
        draw_resize_handle(painter, bl, ResizeCorner::BottomLeft, color);
    }
}

// Draws resize handle.
pub fn draw_resize_handle(
    painter: &egui::Painter,
    corner: egui::Pos2,
    which: ResizeCorner,
    color: Color32,
) {
    let size = 10.0;
    let half = size * 0.5;
    let rect = egui::Rect::from_center_size(corner, egui::vec2(size, size));
    painter.rect_filled(rect, 2.0, Color32::from_gray(30));
    painter.rect_stroke(rect, 2.0, Stroke::new(1.0_f32, color), egui::StrokeKind::Middle);

    let center = corner;
    let stroke = Stroke::new(1.0_f32, color);
    match which {
        ResizeCorner::TopLeft => {
            painter.line_segment(
                [center, egui::pos2(center.x - half + 2.0, center.y)],
                stroke,
            );
            painter.line_segment(
                [center, egui::pos2(center.x, center.y - half + 2.0)],
                stroke,
            );
        }
        ResizeCorner::TopRight => {
            painter.line_segment(
                [center, egui::pos2(center.x + half - 2.0, center.y)],
                stroke,
            );
            painter.line_segment(
                [center, egui::pos2(center.x, center.y - half + 2.0)],
                stroke,
            );
        }
        ResizeCorner::BottomLeft => {
            painter.line_segment(
                [center, egui::pos2(center.x - half + 2.0, center.y)],
                stroke,
            );
            painter.line_segment(
                [center, egui::pos2(center.x, center.y + half - 2.0)],
                stroke,
            );
        }
        ResizeCorner::BottomRight => {
            painter.line_segment(
                [center, egui::pos2(center.x + half - 2.0, center.y)],
                stroke,
            );
            painter.line_segment(
                [center, egui::pos2(center.x, center.y + half - 2.0)],
                stroke,
            );
        }
    }
}
