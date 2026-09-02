/*
 * Grid geometry and coordinate helpers.
 * Owns screen/grid conversion and rectangle helpers used by interaction code.
 */

use crate::layout;
use eframe::egui;
use egui::{Color32, Stroke};

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

    let stroke = Stroke::new(1.0_f32, Color32::from_gray(160));
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

// Converts to grid.
pub fn pointer_to_grid(pointer_pos: egui::Pos2, geometry: &GridGeometry) -> Option<(f32, f32)> {
    if !geometry.rect.contains(pointer_pos) {
        return None;
    }

    let x = (pointer_pos.x - geometry.rect.left()) / geometry.cell_size;
    let y = (pointer_pos.y - geometry.rect.top()) / geometry.cell_size;
    Some((x, y))
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
