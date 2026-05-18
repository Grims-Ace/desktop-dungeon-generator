/*
 * Main interaction module.
 * Provides the GridGeometry structure and hit-testing utilities,
 * orchestrating dragging, resizing, and manual layout adjustments.
 */

use crate::app::DungeonApp;
pub use crate::exporter::types::{
    door_edges_for, door_render_width, norm_edge as normalized_edge, window_render_width,
};
use crate::layout;
use crate::ui::AddTool;
use eframe::egui;
use egui::{Color32, Stroke};
use std::collections::HashSet;

pub mod add_delete;
pub mod drag;
pub mod resize;
pub mod types;

pub use add_delete::*;
pub use drag::*;
pub use resize::*;
pub use types::*;

pub struct GridGeometry {
    pub rect: egui::Rect,
    pub cell_size: f32,
    pub cols: usize,
    pub rows: usize,
}

// Draws the editable grid and returns its screen geometry.
pub fn draw_grid(
    painter: &egui::Painter,
    area: egui::Rect,
    cols: usize,
    rows: usize,
) -> GridGeometry {
    let cols = cols.max(1);
    let rows = rows.max(1);

    let cell_size = (area.width() / cols as f32).min(area.height() / rows as f32);
    let grid_width = cell_size * cols as f32;
    let grid_height = cell_size * rows as f32;

    let grid_rect = egui::Rect::from_min_size(
        egui::pos2(
            area.center().x - (grid_width * 0.5),
            area.center().y - (grid_height * 0.5),
        ),
        egui::vec2(grid_width, grid_height),
    );

    let stroke = Stroke::new(1.0, Color32::from_gray(160));
    painter.rect_stroke(grid_rect, 0.0, stroke, egui::StrokeKind::Middle);

    for col in 1..cols {
        let x = grid_rect.left() + (col as f32 * cell_size);
        painter.line_segment(
            [
                egui::pos2(x, grid_rect.top()),
                egui::pos2(x, grid_rect.bottom()),
            ],
            stroke,
        );
    }

    for row in 1..rows {
        let y = grid_rect.top() + (row as f32 * cell_size);
        painter.line_segment(
            [
                egui::pos2(grid_rect.left(), y),
                egui::pos2(grid_rect.right(), y),
            ],
            stroke,
        );
    }

    GridGeometry {
        rect: grid_rect,
        cell_size,
        cols,
        rows,
    }
}

// Handles update hover targets.
pub fn update_hover_targets(app: &mut DungeonApp, ctx: &egui::Context, geometry: &GridGeometry) {
    if app.drag_state.is_some() || app.resize_state.is_some() || app.add_corridor_drag.is_some() {
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
    let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;
    let clicked = (grid_x.floor() as usize, grid_y.floor() as usize);
    if app.settings.active_level_index >= app.levels.len() {
        return None;
    }
    let layout = &app.levels[app.settings.active_level_index];

    layout
        .text_labels
        .iter()
        .position(|label| label.cell == clicked)
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

    let corridor_index = layout
        .corridors
        .iter()
        .position(|corridor| corridor.path.contains(&clicked))?;

    Some(CorridorDragState {
        corridor_index,
        original_path: layout.corridors[corridor_index].path.clone(),
        original_cell: clicked,
    })
}

// Converts to grid.
pub fn pointer_to_grid(pointer_pos: egui::Pos2, geometry: &GridGeometry) -> Option<(f32, f32)> {
    if !geometry.rect.contains(pointer_pos) {
        return None;
    }

    let x = (pointer_pos.x - geometry.rect.left()) / geometry.cell_size;
    let y = (pointer_pos.y - geometry.rect.top()) / geometry.cell_size;
    Some((x, y))
}

// Reroutes path through cell.
pub fn reroute_path_through_cell(
    original_path: &[(usize, usize)],
    original_cell: (usize, usize),
    target_cell: (usize, usize),
    cols: usize,
    rows: usize,
    blocked_room_cells_set: &HashSet<(usize, usize)>,
) -> Option<Vec<(usize, usize)>> {
    let (&start, &end) = (original_path.first()?, original_path.last()?);
    if start == end {
        return None;
    }

    let mut blocked = blocked_room_cells_set.clone();
    blocked.insert(original_cell);
    blocked.remove(&start);
    blocked.remove(&end);
    blocked.remove(&target_cell);

    let first_leg = crate::layout::shortest_path_cells(start, target_cell, cols, rows, &blocked)?;

    let mut blocked_second = blocked.clone();
    for &cell in first_leg
        .iter()
        .skip(1)
        .take(first_leg.len().saturating_sub(2))
    {
        if cell != target_cell && cell != end {
            blocked_second.insert(cell);
        }
    }

    let second_leg =
        crate::layout::shortest_path_cells(target_cell, end, cols, rows, &blocked_second).or_else(
            || crate::layout::shortest_path_cells(target_cell, end, cols, rows, &blocked),
        )?;

    let mut full_path = first_leg;
    for cell in second_leg.into_iter().skip(1) {
        if full_path.last().copied() != Some(cell) {
            full_path.push(cell);
        }
    }

    simplify_path_loops(&mut full_path);

    if full_path.len() < 2 || !full_path.contains(&target_cell) {
        return None;
    }

    if original_cell != start && original_cell != end && full_path.contains(&original_cell) {
        return None;
    }

    Some(full_path)
}

// Simplifies path loops.
pub fn simplify_path_loops(path: &mut Vec<(usize, usize)>) {
    let mut out = Vec::new();
    for &cell in path.iter() {
        if let Some(pos) = out.iter().position(|&c| c == cell) {
            out.truncate(pos + 1);
        } else {
            out.push(cell);
        }
    }
    *path = out;
}

// Resizes hit cells.
pub fn resize_hit_cells(
    room: &layout::Room,
    room_idx: usize,
    rooms: &[layout::Room],
    cols: usize,
    rows: usize,
) -> HashSet<(usize, usize)> {
    let mut cells = HashSet::new();
    let corners = [
        (room.x as isize, room.y as isize, -1isize, -1isize),
        (
            (room.x + room.width - 1) as isize,
            room.y as isize,
            1isize,
            -1isize,
        ),
        (
            room.x as isize,
            (room.y + room.height - 1) as isize,
            -1isize,
            1isize,
        ),
        (
            (room.x + room.width - 1) as isize,
            (room.y + room.height - 1) as isize,
            1isize,
            1isize,
        ),
    ];

    for (cx, cy, ox, oy) in corners {
        let candidates = [
            (cx, cy),
            (cx - ox, cy),
            (cx, cy - oy),
            (cx - ox, cy - oy),
            (cx + ox, cy),
            (cx, cy + oy),
            (cx + ox, cy + oy),
        ];

        for (x, y) in candidates {
            if x < 0 || y < 0 || x >= cols as isize || y >= rows as isize {
                continue;
            }

            let cell = (x as usize, y as usize);
            let in_other_room = rooms.iter().enumerate().any(|(idx, other_room)| {
                idx != room_idx
                    && cell.0 >= other_room.x
                    && cell.0 < other_room.x + other_room.width
                    && cell.1 >= other_room.y
                    && cell.1 < other_room.y + other_room.height
            });
            if !in_other_room {
                cells.insert(cell);
            }
        }
    }

    cells
}

// Returns the screen center point for a grid cell.
pub fn cell_center(geometry: &GridGeometry, col: usize, row: usize) -> egui::Pos2 {
    egui::pos2(
        geometry.rect.left() + (col as f32 + 0.5) * geometry.cell_size,
        geometry.rect.top() + (row as f32 + 0.5) * geometry.cell_size,
    )
}

// Returns the screen rectangle for a grid cell.
pub fn cell_rect(geometry: &GridGeometry, col: usize, row: usize) -> egui::Rect {
    let left = geometry.rect.left() + col as f32 * geometry.cell_size;
    let top = geometry.rect.top() + row as f32 * geometry.cell_size;
    egui::Rect::from_min_size(
        egui::pos2(left, top),
        egui::vec2(geometry.cell_size, geometry.cell_size),
    )
}

// Computes the screen rectangle occupied by an area marker.
pub fn marker_rect(marker: &layout::AreaMarker, geometry: &GridGeometry) -> egui::Rect {
    let left = geometry.rect.left() + marker.cell.0 as f32 * geometry.cell_size;
    let top = geometry.rect.top() + marker.cell.1 as f32 * geometry.cell_size;
    egui::Rect::from_min_size(
        egui::pos2(left, top),
        egui::vec2(
            marker.size as f32 * geometry.cell_size,
            marker.size as f32 * geometry.cell_size,
        ),
    )
}

// Computes contains cell.
pub fn marker_contains_cell(marker: Option<&layout::AreaMarker>, cell: (usize, usize)) -> bool {
    let Some(marker) = marker else {
        return false;
    };
    cell.0 >= marker.cell.0
        && cell.0 < marker.cell.0 + marker.size
        && cell.1 >= marker.cell.1
        && cell.1 < marker.cell.1 + marker.size
}

// Computes marker origin.
pub fn centered_marker_origin(
    cell: (usize, usize),
    size: usize,
    cols: usize,
    rows: usize,
) -> (usize, usize) {
    let size = size.clamp(1, 10).min(cols.max(1)).min(rows.max(1));
    let half = size / 2;
    let max_x = cols.saturating_sub(size);
    let max_y = rows.saturating_sub(size);
    (
        cell.0.saturating_sub(half).min(max_x),
        cell.1.saturating_sub(half).min(max_y),
    )
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
                    let rect = crate::rendering::text_label_rect(geometry, label).expand(3.0);
                    painter.rect_stroke(
                        rect,
                        0.0,
                        Stroke::new(2.0, Color32::from_gray(180)),
                        egui::StrokeKind::Middle,
                    );
                }
            }
            ResizeState::Room(state) => {
                if let Some(room) = layout.rooms.get(state.room_idx) {
                    draw_room_resize_visuals(painter, geometry, room, true);
                }
            }
            ResizeState::Staircase(state) => {
                if let Some(stair) = layout.stairs.get(state.stair_idx) {
                    draw_staircase_resize_visuals(painter, geometry, stair, true);
                }
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
                Stroke::new(2.0, Color32::from_gray(180)),
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
    let stroke = Stroke::new(1.5, color);

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
    let stroke = Stroke::new(1.5, color);

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
    painter.rect_stroke(rect, 2.0, Stroke::new(1.0, color), egui::StrokeKind::Middle);

    let center = corner;
    let stroke = Stroke::new(1.0, color);
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
