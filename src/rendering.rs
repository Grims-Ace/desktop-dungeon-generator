/*
 * Canvas drawing logic for the dungeon layout.
 * Provides functions to render rooms, corridors, walls, doors, windows,
 * and various markers using egui's Painter API.
 */

use crate::interact::{
    GridGeometry, HoverMarker, MarkerKind, cell_center, cell_rect, door_edges_for,
    door_render_width, marker_rect, normalized_edge, window_render_width,
};
use crate::layout::{DungeonLayout, corridor_cells};
use eframe::egui;
use egui::{Color32, Stroke};
use std::collections::HashSet;

// Draws the full dungeon layout into the interactive canvas.
pub fn draw_layout(
    painter: &egui::Painter,
    geometry: &GridGeometry,
    layout: &DungeonLayout,
    colorblind_mode: bool,
    hover_text_idx: Option<usize>,
    hover_marker: Option<HoverMarker>,
    hover_stair_idx: Option<usize>,
) {
    let wall_width_px = (geometry.cell_size / 2.5).max(1.0);
    let door_width_px = (geometry.cell_size / 5.0).max(1.0);

    let room_fill = Color32::from_rgb(70, 120, 160);
    let corridor_fill = Color32::from_rgb(210, 190, 120);
    let wall_color = Color32::BLACK;
    let corridor_cells_set = corridor_cells(layout, geometry.cols, geometry.rows);
    let corridor_edges_set = corridor_edges_from_cells(&corridor_cells_set);
    let mut room_cells_set = HashSet::new();
    let room_edges_set = room_edges_from_rooms(&layout.rooms);
    let mut door_edges_set = HashSet::new();

    for door in &layout.doors {
        for edge in door_edges_for(door, geometry.cols, geometry.rows) {
            door_edges_set.insert(edge);
        }
    }

    for &(col, row) in &corridor_cells_set {
        painter.rect_filled(cell_rect(geometry, col, row), 0.0, corridor_fill);
        if colorblind_mode {
            painter.circle_filled(
                cell_center(geometry, col, row),
                geometry.cell_size * 0.14,
                Color32::BLACK,
            );
        }
    }

    for &(col, row) in &corridor_cells_set {
        let rect = cell_rect(geometry, col, row);
        let right = (col + 1, row);
        let bottom = (col, row + 1);
        let left = col.checked_sub(1).map(|x| (x, row));
        let top = row.checked_sub(1).map(|y| (col, y));

        if col == 0
            || (!corridor_cells_set.contains(&(col - 1, row))
                && left
                    .map(|n| !door_edges_set.contains(&normalized_edge((col, row), n)))
                    .unwrap_or(true))
        {
            draw_wall_segment(
                painter,
                egui::pos2(rect.left(), rect.top()),
                egui::pos2(rect.left(), rect.bottom()),
                wall_width_px,
                wall_color,
            );
        }
        if (!corridor_cells_set.contains(&right)
            || !corridor_edges_set.contains(&normalized_edge((col, row), right)))
            && !door_edges_set.contains(&normalized_edge((col, row), right))
        {
            draw_wall_segment(
                painter,
                egui::pos2(rect.right(), rect.top()),
                egui::pos2(rect.right(), rect.bottom()),
                wall_width_px,
                wall_color,
            );
        }
        if row == 0
            || (!corridor_cells_set.contains(&(col, row - 1))
                && top
                    .map(|n| !door_edges_set.contains(&normalized_edge((col, row), n)))
                    .unwrap_or(true))
        {
            draw_wall_segment(
                painter,
                egui::pos2(rect.left(), rect.top()),
                egui::pos2(rect.right(), rect.top()),
                wall_width_px,
                wall_color,
            );
        }
        if (!corridor_cells_set.contains(&bottom)
            || !corridor_edges_set.contains(&normalized_edge((col, row), bottom)))
            && !door_edges_set.contains(&normalized_edge((col, row), bottom))
        {
            draw_wall_segment(
                painter,
                egui::pos2(rect.left(), rect.bottom()),
                egui::pos2(rect.right(), rect.bottom()),
                wall_width_px,
                wall_color,
            );
        }
    }

    for room in &layout.rooms {
        for x in room.x..(room.x + room.width) {
            for y in room.y..(room.y + room.height) {
                room_cells_set.insert((x, y));
            }
        }

        let left = geometry.rect.left() + room.x as f32 * geometry.cell_size;
        let top = geometry.rect.top() + room.y as f32 * geometry.cell_size;
        let right = left + room.width as f32 * geometry.cell_size;
        let bottom = top + room.height as f32 * geometry.cell_size;
        let room_rect = egui::Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, bottom));

        painter.rect_filled(room_rect, 0.0, room_fill);
        if colorblind_mode {
            draw_room_crosshatch(painter, geometry, room, Stroke::new(1.5, Color32::BLACK));
        }
    }

    for &(col, row) in &room_cells_set {
        let rect = cell_rect(geometry, col, row);
        let right = (col + 1, row);
        let bottom = (col, row + 1);
        let left = col.checked_sub(1).map(|x| (x, row));
        let top = row.checked_sub(1).map(|y| (col, y));

        let left_edge = col == 0
            || (!room_cells_set.contains(&(col - 1, row))
                || left
                    .map(|n| !room_edges_set.contains(&normalized_edge((col, row), n)))
                    .unwrap_or(true))
                && left
                    .map(|n| !door_edges_set.contains(&normalized_edge((col, row), n)))
                    .unwrap_or(true);
        if left_edge {
            draw_wall_segment(
                painter,
                egui::pos2(rect.left(), rect.top()),
                egui::pos2(rect.left(), rect.bottom()),
                wall_width_px,
                wall_color,
            );
        }
        if (!room_cells_set.contains(&right)
            || !room_edges_set.contains(&normalized_edge((col, row), right)))
            && !door_edges_set.contains(&normalized_edge((col, row), right))
        {
            draw_wall_segment(
                painter,
                egui::pos2(rect.right(), rect.top()),
                egui::pos2(rect.right(), rect.bottom()),
                wall_width_px,
                wall_color,
            );
        }
        let top_edge = row == 0
            || (!room_cells_set.contains(&(col, row - 1))
                || top
                    .map(|n| !room_edges_set.contains(&normalized_edge((col, row), n)))
                    .unwrap_or(true))
                && top
                    .map(|n| !door_edges_set.contains(&normalized_edge((col, row), n)))
                    .unwrap_or(true);
        if top_edge {
            draw_wall_segment(
                painter,
                egui::pos2(rect.left(), rect.top()),
                egui::pos2(rect.right(), rect.top()),
                wall_width_px,
                wall_color,
            );
        }
        if (!room_cells_set.contains(&bottom)
            || !room_edges_set.contains(&normalized_edge((col, row), bottom)))
            && !door_edges_set.contains(&normalized_edge((col, row), bottom))
        {
            draw_wall_segment(
                painter,
                egui::pos2(rect.left(), rect.bottom()),
                egui::pos2(rect.right(), rect.bottom()),
                wall_width_px,
                wall_color,
            );
        }
    }

    for door in &layout.doors {
        let color = if colorblind_mode {
            Color32::BLACK
        } else if door.secret {
            Color32::from_rgb(170, 80, 170)
        } else if door.archway {
            Color32::from_rgb(230, 140, 60)
        } else if door.locked {
            Color32::from_rgb(220, 70, 70)
        } else {
            Color32::from_rgb(80, 200, 120)
        };
        let style = if colorblind_mode {
            if door.secret {
                DoorLineStyle::DashDotDot
            } else if door.archway {
                DoorLineStyle::Dotted
            } else if door.locked {
                DoorLineStyle::ShortDash
            } else {
                DoorLineStyle::LongDash
            }
        } else {
            DoorLineStyle::Solid
        };
        draw_door_line(
            painter,
            geometry,
            door.from,
            door.to,
            door_render_width(door),
            Stroke::new(door_width_px, color),
            style,
        );
    }

    for window in &layout.windows {
        let style = if colorblind_mode {
            DoorLineStyle::DashDot
        } else {
            DoorLineStyle::Solid
        };
        draw_window_line(
            painter,
            geometry,
            window,
            Stroke::new(door_width_px, Color32::from_rgb(70, 130, 220)),
            style,
        );
    }

    for (idx, label) in layout.text_labels.iter().enumerate() {
        if hover_text_idx == Some(idx) {
            painter.rect_stroke(
                text_label_rect(geometry, label).expand(2.0),
                0.0,
                Stroke::new(2.0, Color32::from_rgb(255, 220, 120)),
                egui::StrokeKind::Middle,
            );
        }
        let pos = cell_center(geometry, label.cell.0, label.cell.1);
        let color = if colorblind_mode {
            Color32::BLACK
        } else {
            Color32::WHITE
        };
        painter.text(
            pos,
            egui::Align2::CENTER_CENTER,
            &label.text,
            egui::FontId::proportional(label.font_size as f32),
            color,
        );
    }

    draw_area_marker_group(
        painter,
        geometry,
        &layout.start_markers,
        "S",
        Color32::from_rgb(60, 220, 200),
        colorblind_mode,
        hover_marker,
        MarkerKind::Start,
    );
    draw_area_marker_group(
        painter,
        geometry,
        &layout.end_markers,
        "E",
        Color32::from_rgb(240, 90, 90),
        colorblind_mode,
        hover_marker,
        MarkerKind::End,
    );
    draw_area_marker_group(
        painter,
        geometry,
        &layout.trap_markers,
        "T",
        Color32::from_rgb(150, 80, 230),
        colorblind_mode,
        hover_marker,
        MarkerKind::Trap,
    );
    draw_area_marker_group(
        painter,
        geometry,
        &layout.monster_markers,
        "M",
        Color32::from_rgb(200, 50, 50),
        colorblind_mode,
        hover_marker,
        MarkerKind::Monster,
    );

    for (idx, stair) in layout.stairs.iter().enumerate() {
        draw_staircase(
            painter,
            geometry,
            stair,
            colorblind_mode,
            hover_stair_idx == Some(idx),
        );
    }
}

// Draws a staircase marker into the interactive canvas.
pub fn draw_staircase(
    painter: &egui::Painter,
    geometry: &GridGeometry,
    stair: &crate::layout::Staircase,
    colorblind_mode: bool,
    is_hovered: bool,
) {
    let left = geometry.rect.left() + stair.cell.0 as f32 * geometry.cell_size;
    let top = geometry.rect.top() + stair.cell.1 as f32 * geometry.cell_size;
    let right = left + stair.width as f32 * geometry.cell_size;
    let bottom = top + stair.height as f32 * geometry.cell_size;
    let rect = egui::Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, bottom));

    let fill = if is_hovered {
        Color32::from_rgb(220, 180, 80).gamma_multiply(0.5)
    } else if colorblind_mode {
        Color32::from_rgb(180, 180, 180).gamma_multiply(0.4)
    } else {
        Color32::from_rgb(200, 150, 50).gamma_multiply(0.35)
    };
    painter.rect_filled(rect, 4.0, fill);

    let stroke_color = if is_hovered {
        Color32::WHITE
    } else if colorblind_mode {
        Color32::BLACK
    } else {
        Color32::from_rgb(200, 150, 50)
    };
    painter.rect_stroke(
        rect,
        4.0,
        Stroke::new(if is_hovered { 3.0 } else { 2.0 }, stroke_color),
        egui::StrokeKind::Middle,
    );

    let steps = (stair.height as f32 * 2.0).round() as usize;
    let step_height = (rect.height() / (steps as f32)).max(1.0);
    let line_color = if colorblind_mode {
        Color32::BLACK
    } else {
        Color32::from_rgb(150, 100, 30)
    };
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

// Computes label rect.
pub fn text_label_rect(geometry: &GridGeometry, label: &crate::layout::TextLabel) -> egui::Rect {
    let center = cell_center(geometry, label.cell.0, label.cell.1);
    let font_size = label.font_size as f32;
    let width =
        (label.text.chars().count().max(1) as f32 * font_size * 0.6).max(geometry.cell_size * 0.5);
    let height = font_size.max(geometry.cell_size * 0.35);
    egui::Rect::from_center_size(center, egui::vec2(width, height))
}

// Draws area marker.
pub fn draw_area_marker(
    painter: &egui::Painter,
    geometry: &GridGeometry,
    marker: &crate::layout::AreaMarker,
    label: &str,
    base_color: Color32,
    colorblind_mode: bool,
    hovered: bool,
) {
    let rect = marker_rect(marker, geometry);
    let fill = base_color.gamma_multiply(if colorblind_mode { 0.22 } else { 0.35 });
    let stroke_color = if hovered {
        Color32::from_rgb(255, 220, 120)
    } else if colorblind_mode {
        Color32::BLACK
    } else {
        base_color
    };
    let text_color = if colorblind_mode {
        Color32::BLACK
    } else {
        Color32::WHITE
    };
    let font_size = (geometry.cell_size * marker.size as f32 * 0.5).clamp(14.0, 72.0);

    painter.rect_filled(rect, 4.0, fill);
    painter.rect_stroke(
        rect,
        4.0,
        Stroke::new(if hovered { 3.0 } else { 2.0 }, stroke_color),
        egui::StrokeKind::Middle,
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(font_size),
        text_color,
    );
}

// Draws area marker group.
#[allow(clippy::too_many_arguments)]
pub fn draw_area_marker_group(
    painter: &egui::Painter,
    geometry: &GridGeometry,
    markers: &[crate::layout::AreaMarker],
    label: &str,
    base_color: Color32,
    colorblind_mode: bool,
    hover_marker: Option<HoverMarker>,
    kind: MarkerKind,
) {
    for (idx, marker) in markers.iter().enumerate() {
        let hovered = match (kind, hover_marker) {
            (MarkerKind::Start, Some(HoverMarker::Start(hover_idx))) => hover_idx == idx,
            (MarkerKind::End, Some(HoverMarker::End(hover_idx))) => hover_idx == idx,
            (MarkerKind::Trap, Some(HoverMarker::Trap(hover_idx))) => hover_idx == idx,
            (MarkerKind::Monster, Some(HoverMarker::Monster(hover_idx))) => hover_idx == idx,
            _ => false,
        };
        draw_area_marker(
            painter,
            geometry,
            marker,
            label,
            base_color,
            colorblind_mode,
            hovered,
        );
    }
}

// Draws wall segment.
pub fn draw_wall_segment(
    painter: &egui::Painter,
    from: egui::Pos2,
    to: egui::Pos2,
    width: f32,
    color: Color32,
) {
    let half = width * 0.5;
    if (from.x - to.x).abs() <= f32::EPSILON {
        let x = from.x.min(to.x);
        let y0 = from.y.min(to.y);
        let y1 = from.y.max(to.y);
        let rect = egui::Rect::from_min_max(
            egui::pos2(x - half, y0 - half),
            egui::pos2(x + half, y1 + half),
        );
        painter.rect_filled(rect, 0.0, color);
    } else {
        let y = from.y.min(to.y);
        let x0 = from.x.min(to.x);
        let x1 = from.x.max(to.x);
        let rect = egui::Rect::from_min_max(
            egui::pos2(x0 - half, y - half),
            egui::pos2(x1 + half, y + half),
        );
        painter.rect_filled(rect, 0.0, color);
    }
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
            edges.insert(normalized_edge((col, row), right));
        }
        if corridor_cells.contains(&bottom) {
            edges.insert(normalized_edge((col, row), bottom));
        }
    }
    edges
}

// Computes edges from rooms.
pub fn room_edges_from_rooms(
    rooms: &[crate::layout::Room],
) -> HashSet<((usize, usize), (usize, usize))> {
    let mut edges = HashSet::new();
    for room in rooms {
        let x_end = room.x + room.width;
        let y_end = room.y + room.height;
        for x in room.x..x_end {
            for y in room.y..y_end {
                if x + 1 < x_end {
                    edges.insert(normalized_edge((x, y), (x + 1, y)));
                }
                if y + 1 < y_end {
                    edges.insert(normalized_edge((x, y), (x, y + 1)));
                }
            }
        }
    }
    edges
}

#[derive(Debug, Clone, Copy)]
pub enum DoorLineStyle {
    Solid,
    LongDash,
    ShortDash,
    Dotted,
    DashDot,
    DashDotDot,
}

// Draws door line.
pub fn draw_door_line(
    painter: &egui::Painter,
    geometry: &GridGeometry,
    a: (usize, usize),
    b: (usize, usize),
    width_cells: usize,
    stroke: Stroke,
    style: DoorLineStyle,
) {
    if a.0.abs_diff(b.0) + a.1.abs_diff(b.1) != 1 {
        return;
    }

    let width_cells = width_cells.max(1);
    let min_offset = -((width_cells as isize - 1) / 2);
    let max_offset = width_cells as isize / 2;

    if a.0 != b.0 {
        let x = geometry.rect.left() + (a.0.max(b.0) as f32) * geometry.cell_size;
        let row = a.1 as isize;
        let y0_cell = (row + min_offset).clamp(0, geometry.rows.saturating_sub(1) as isize);
        let y1_cell = (row + max_offset).clamp(0, geometry.rows.saturating_sub(1) as isize);
        let y0 = geometry.rect.top() + y0_cell as f32 * geometry.cell_size;
        let y1 = geometry.rect.top() + (y1_cell as f32 + 1.0) * geometry.cell_size;
        draw_styled_line(painter, egui::pos2(x, y0), egui::pos2(x, y1), stroke, style);
    } else {
        let y = geometry.rect.top() + (a.1.max(b.1) as f32) * geometry.cell_size;
        let col = a.0 as isize;
        let x0_cell = (col + min_offset).clamp(0, geometry.cols.saturating_sub(1) as isize);
        let x1_cell = (col + max_offset).clamp(0, geometry.cols.saturating_sub(1) as isize);
        let x0 = geometry.rect.left() + x0_cell as f32 * geometry.cell_size;
        let x1 = geometry.rect.left() + (x1_cell as f32 + 1.0) * geometry.cell_size;
        draw_styled_line(painter, egui::pos2(x0, y), egui::pos2(x1, y), stroke, style);
    }
}

// Draws window line.
pub fn draw_window_line(
    painter: &egui::Painter,
    geometry: &GridGeometry,
    window: &crate::layout::Window,
    stroke: Stroke,
    style: DoorLineStyle,
) {
    let width_cells = window_render_width(window).max(1) as isize;
    let min_offset = -((width_cells - 1) / 2);
    let max_offset = width_cells / 2;

    match window.side {
        crate::layout::WindowSide::Left => {
            let x = geometry.rect.left() + window.cell.0 as f32 * geometry.cell_size;
            let row = window.cell.1 as isize;
            let y0_cell = (row + min_offset).clamp(0, geometry.rows.saturating_sub(1) as isize);
            let y1_cell = (row + max_offset).clamp(0, geometry.rows.saturating_sub(1) as isize);
            let y0 = geometry.rect.top() + y0_cell as f32 * geometry.cell_size;
            let y1 = geometry.rect.top() + (y1_cell as f32 + 1.0) * geometry.cell_size;
            draw_styled_line(painter, egui::pos2(x, y0), egui::pos2(x, y1), stroke, style);
        }
        crate::layout::WindowSide::Right => {
            let x = geometry.rect.left() + (window.cell.0 as f32 + 1.0) * geometry.cell_size;
            let row = window.cell.1 as isize;
            let y0_cell = (row + min_offset).clamp(0, geometry.rows.saturating_sub(1) as isize);
            let y1_cell = (row + max_offset).clamp(0, geometry.rows.saturating_sub(1) as isize);
            let y0 = geometry.rect.top() + y0_cell as f32 * geometry.cell_size;
            let y1 = geometry.rect.top() + (y1_cell as f32 + 1.0) * geometry.cell_size;
            draw_styled_line(painter, egui::pos2(x, y0), egui::pos2(x, y1), stroke, style);
        }
        crate::layout::WindowSide::Top => {
            let y = geometry.rect.top() + window.cell.1 as f32 * geometry.cell_size;
            let col = window.cell.0 as isize;
            let x0_cell = (col + min_offset).clamp(0, geometry.cols.saturating_sub(1) as isize);
            let x1_cell = (col + max_offset).clamp(0, geometry.cols.saturating_sub(1) as isize);
            let x0 = geometry.rect.left() + x0_cell as f32 * geometry.cell_size;
            let x1 = geometry.rect.left() + (x1_cell as f32 + 1.0) * geometry.cell_size;
            draw_styled_line(painter, egui::pos2(x0, y), egui::pos2(x1, y), stroke, style);
        }
        crate::layout::WindowSide::Bottom => {
            let y = geometry.rect.top() + (window.cell.1 as f32 + 1.0) * geometry.cell_size;
            let col = window.cell.0 as isize;
            let x0_cell = (col + min_offset).clamp(0, geometry.cols.saturating_sub(1) as isize);
            let x1_cell = (col + max_offset).clamp(0, geometry.cols.saturating_sub(1) as isize);
            let x0 = geometry.rect.left() + x0_cell as f32 * geometry.cell_size;
            let x1 = geometry.rect.left() + (x1_cell as f32 + 1.0) * geometry.cell_size;
            draw_styled_line(painter, egui::pos2(x0, y), egui::pos2(x1, y), stroke, style);
        }
    }
}

// Draws styled line.
pub fn draw_styled_line(
    painter: &egui::Painter,
    from: egui::Pos2,
    to: egui::Pos2,
    stroke: Stroke,
    style: DoorLineStyle,
) {
    match style {
        DoorLineStyle::Solid => {
            painter.line_segment([from, to], stroke);
        }
        DoorLineStyle::LongDash => draw_dashed_line(painter, from, to, stroke, 14.0, 7.0),
        DoorLineStyle::ShortDash => draw_dashed_line(painter, from, to, stroke, 6.0, 4.0),
        DoorLineStyle::Dotted => draw_dotted_line(painter, from, to, stroke),
        DoorLineStyle::DashDot => draw_dash_dot_line(painter, from, to, stroke),
        DoorLineStyle::DashDotDot => draw_dash_dot_dot_line(painter, from, to, stroke),
    }
}

// Draws dashed line.
pub fn draw_dashed_line(
    painter: &egui::Painter,
    from: egui::Pos2,
    to: egui::Pos2,
    stroke: Stroke,
    dash_len: f32,
    gap_len: f32,
) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 0.0 {
        return;
    }
    let ux = dx / len;
    let uy = dy / len;

    let mut dist = 0.0_f32;
    while dist < len {
        let seg_start = dist;
        let seg_end = (dist + dash_len).min(len);
        let p0 = egui::pos2(from.x + ux * seg_start, from.y + uy * seg_start);
        let p1 = egui::pos2(from.x + ux * seg_end, from.y + uy * seg_end);
        painter.line_segment([p0, p1], stroke);
        dist += dash_len + gap_len;
    }
}

// Draws dotted line.
pub fn draw_dotted_line(painter: &egui::Painter, from: egui::Pos2, to: egui::Pos2, stroke: Stroke) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 0.0 {
        return;
    }
    let ux = dx / len;
    let uy = dy / len;
    let step = 5.0_f32;
    let radius = (stroke.width * 0.35).max(1.0);

    let mut dist = 0.0_f32;
    while dist <= len {
        let p = egui::pos2(from.x + ux * dist, from.y + uy * dist);
        painter.circle_filled(p, radius, stroke.color);
        dist += step;
    }
}

// Draws dash dot line.
pub fn draw_dash_dot_line(
    painter: &egui::Painter,
    from: egui::Pos2,
    to: egui::Pos2,
    stroke: Stroke,
) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 0.0 {
        return;
    }

    let ux = dx / len;
    let uy = dy / len;
    let mut dist = 0.0_f32;
    while dist < len {
        let dash_end = (dist + 7.0).min(len);
        let p0 = egui::pos2(from.x + ux * dist, from.y + uy * dist);
        let p1 = egui::pos2(from.x + ux * dash_end, from.y + uy * dash_end);
        painter.line_segment([p0, p1], stroke);
        dist = dash_end + 3.0;
        if dist >= len {
            break;
        }
        let dot = egui::pos2(from.x + ux * dist, from.y + uy * dist);
        painter.circle_filled(dot, (stroke.width * 0.35).max(1.0), stroke.color);
        dist += 4.0;
    }
}

// Draws dash dot dot line.
pub fn draw_dash_dot_dot_line(
    painter: &egui::Painter,
    from: egui::Pos2,
    to: egui::Pos2,
    stroke: Stroke,
) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 0.0 {
        return;
    }

    let ux = dx / len;
    let uy = dy / len;
    let radius = (stroke.width * 0.35).max(1.0);
    let mut dist = 0.0_f32;
    while dist < len {
        let dash_end = (dist + 7.0).min(len);
        let p0 = egui::pos2(from.x + ux * dist, from.y + uy * dist);
        let p1 = egui::pos2(from.x + ux * dash_end, from.y + uy * dash_end);
        painter.line_segment([p0, p1], stroke);
        dist = dash_end + 3.0;
        if dist >= len {
            break;
        }
        let dot1 = egui::pos2(from.x + ux * dist, from.y + uy * dist);
        painter.circle_filled(dot1, radius, stroke.color);
        dist += 3.0;
        if dist >= len {
            break;
        }
        let dot2 = egui::pos2(from.x + ux * dist, from.y + uy * dist);
        painter.circle_filled(dot2, radius, stroke.color);
        dist += 4.0;
    }
}

// Draws room crosshatch.
pub fn draw_room_crosshatch(
    painter: &egui::Painter,
    geometry: &GridGeometry,
    room: &crate::layout::Room,
    stroke: Stroke,
) {
    for x in room.x..(room.x + room.width) {
        for y in room.y..(room.y + room.height) {
            let rect = cell_rect(geometry, x, y).shrink(2.0);
            painter.line_segment([rect.left_top(), rect.right_bottom()], stroke);
            painter.line_segment([rect.right_top(), rect.left_bottom()], stroke);
        }
    }
}
