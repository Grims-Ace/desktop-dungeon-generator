/*
 * Shared utilities for the export module.
 * Provides functions for scene data collection and common geometric
 * calculations used by both SVG and raster rendering paths.
 */

use super::types::{ExportGeometry, SceneData, door_edges_for, norm_edge, window_render_width};
use crate::layout::{DungeonLayout, Room, Window, WindowSide, corridor_cells};
use crate::ui::UiSettings;
use std::collections::HashSet;

// Collects scene data.
pub fn collect_scene_data(layout: &DungeonLayout, settings: &UiSettings) -> SceneData {
    let mut scene = SceneData::default();

    for door in &layout.doors {
        for edge in door_edges_for(door, settings.cols, settings.rows) {
            scene.door_edges.insert(edge);
        }
    }

    scene.corridor_cells = corridor_cells(layout, settings.cols, settings.rows);
    scene.corridor_edges = corridor_edges_from_cells(&scene.corridor_cells);

    for room in &layout.rooms {
        for x in room.x..(room.x + room.width) {
            for y in room.y..(room.y + room.height) {
                scene.room_cells.insert((x, y));
            }
        }
    }
    scene.room_edges = room_edges_from_rooms(&layout.rooms);

    scene
}

// Computes edges from cells.
pub fn corridor_edges_from_cells(
    corridor_cells: &HashSet<(usize, usize)>,
) -> HashSet<((usize, usize), (usize, usize))> {
    let mut edges = HashSet::new();
    for &(col, row) in corridor_cells {
        let right = (col + 1, row);
        let bottom = (col, row + 1);
        if corridor_cells.contains(&right) {
            edges.insert(norm_edge((col, row), right));
        }
        if corridor_cells.contains(&bottom) {
            edges.insert(norm_edge((col, row), bottom));
        }
    }
    edges
}

// Computes edges from rooms.
pub fn room_edges_from_rooms(rooms: &[Room]) -> HashSet<((usize, usize), (usize, usize))> {
    let mut edges = HashSet::new();
    for room in rooms {
        let x_end = room.x + room.width;
        let y_end = room.y + room.height;
        for x in room.x..x_end {
            for y in room.y..y_end {
                if x + 1 < x_end {
                    edges.insert(norm_edge((x, y), (x + 1, y)));
                }
                if y + 1 < y_end {
                    edges.insert(norm_edge((x, y), (x, y + 1)));
                }
            }
        }
    }
    edges
}

// Computes line points.
pub fn door_line_points(
    g: &ExportGeometry,
    a: (usize, usize),
    b: (usize, usize),
    width_cells: usize,
) -> Option<(f32, f32, f32, f32)> {
    if a.0.abs_diff(b.0) + a.1.abs_diff(b.1) != 1 {
        return None;
    }

    let width_cells = width_cells.max(1) as isize;
    let min_offset = -((width_cells - 1) / 2);
    let max_offset = width_cells / 2;

    if a.0 != b.0 {
        let x = g.left() + (a.0.max(b.0) as f32) * g.cell;
        let row = a.1 as isize;
        let y0_cell = (row + min_offset).clamp(0, g.rows.saturating_sub(1) as isize);
        let y1_cell = (row + max_offset).clamp(0, g.rows.saturating_sub(1) as isize);
        let y0 = g.top() + y0_cell as f32 * g.cell;
        let y1 = g.top() + (y1_cell as f32 + 1.0) * g.cell;
        Some((x, y0, x, y1))
    } else {
        let y = g.top() + (a.1.max(b.1) as f32) * g.cell;
        let col = a.0 as isize;
        let x0_cell = (col + min_offset).clamp(0, g.cols.saturating_sub(1) as isize);
        let x1_cell = (col + max_offset).clamp(0, g.cols.saturating_sub(1) as isize);
        let x0 = g.left() + x0_cell as f32 * g.cell;
        let x1 = g.left() + (x1_cell as f32 + 1.0) * g.cell;
        Some((x0, y, x1, y))
    }
}

// Computes line points.
pub fn window_line_points(g: &ExportGeometry, window: &Window) -> Option<(f32, f32, f32, f32)> {
    let width_cells = window_render_width(window).max(1) as isize;
    let min_offset = -((width_cells - 1) / 2);
    let max_offset = width_cells / 2;

    match window.side {
        WindowSide::Left => {
            let x = g.left() + window.cell.0 as f32 * g.cell;
            let row = window.cell.1 as isize;
            let y0_cell = (row + min_offset).clamp(0, g.rows.saturating_sub(1) as isize);
            let y1_cell = (row + max_offset).clamp(0, g.rows.saturating_sub(1) as isize);
            let y0 = g.top() + y0_cell as f32 * g.cell;
            let y1 = g.top() + (y1_cell as f32 + 1.0) * g.cell;
            Some((x, y0, x, y1))
        }
        WindowSide::Right => {
            let x = g.left() + (window.cell.0 as f32 + 1.0) * g.cell;
            let row = window.cell.1 as isize;
            let y0_cell = (row + min_offset).clamp(0, g.rows.saturating_sub(1) as isize);
            let y1_cell = (row + max_offset).clamp(0, g.rows.saturating_sub(1) as isize);
            let y0 = g.top() + y0_cell as f32 * g.cell;
            let y1 = g.top() + (y1_cell as f32 + 1.0) * g.cell;
            Some((x, y0, x, y1))
        }
        WindowSide::Top => {
            let y = g.top() + window.cell.1 as f32 * g.cell;
            let col = window.cell.0 as isize;
            let x0_cell = (col + min_offset).clamp(0, g.cols.saturating_sub(1) as isize);
            let x1_cell = (col + max_offset).clamp(0, g.cols.saturating_sub(1) as isize);
            let x0 = g.left() + x0_cell as f32 * g.cell;
            let x1 = g.left() + (x1_cell as f32 + 1.0) * g.cell;
            Some((x0, y, x1, y))
        }
        WindowSide::Bottom => {
            let y = g.top() + (window.cell.1 as f32 + 1.0) * g.cell;
            let col = window.cell.0 as isize;
            let x0_cell = (col + min_offset).clamp(0, g.cols.saturating_sub(1) as isize);
            let x1_cell = (col + max_offset).clamp(0, g.cols.saturating_sub(1) as isize);
            let x0 = g.left() + x0_cell as f32 * g.cell;
            let x1 = g.left() + (x1_cell as f32 + 1.0) * g.cell;
            Some((x0, y, x1, y))
        }
    }
}
