/*
 * Raster image export logic.
 * Provides functions to render dungeon layouts into pixel buffers (Pixmaps)
 * and encode them as PNG, JPEG, or WebP images.
 */

use super::types::{
    BG, BLACK, CORRIDOR, Cell, CellEdge, DoorStyle, END_MARKER, ExportGeometry, GRID,
    MONSTER_MARKER, ROOM, STAIRS, START_MARKER, TRAP_MARKER, WHITE, WINDOW_COLOR,
    door_render_width, norm_edge,
};
use super::utils::{collect_scene_data, door_line_points, window_line_points};
use crate::layout::{AreaMarker, DungeonLayout, Room, Staircase, Window};
use crate::ui::UiSettings;
use image::{DynamicImage, RgbaImage, imageops::FilterType};
use std::collections::HashSet;
use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Rect, Stroke, StrokeDash, Transform};

// Renders a dungeon layout into a raster pixmap.
pub fn render_pixmap(layout: &DungeonLayout, settings: &UiSettings) -> Result<Pixmap, String> {
    let g = ExportGeometry::new(settings.cols, settings.rows);
    let scene = collect_scene_data(layout, settings);
    render_pixmap_with_scene(layout, settings, &g, &scene)
}

// Renders pixmap with scene.
pub fn render_pixmap_with_scene(
    layout: &DungeonLayout,
    settings: &UiSettings,
    g: &ExportGeometry,
    scene: &super::types::SceneData,
) -> Result<Pixmap, String> {
    let mut pixmap =
        Pixmap::new(g.width, g.height).ok_or_else(|| "Failed to allocate canvas".to_string())?;
    fill_bg(&mut pixmap, BG);

    let wall_w = snap_even_width((g.cell / 2.5).max(1.0));
    let door_w = (g.cell / 5.0).max(1.0).round();

    if settings.export_show_grid {
        draw_grid_color(&mut pixmap, g, GRID);
    }

    for &(x, y) in &scene.corridor_cells {
        fill_rect_cell(&mut pixmap, g, x, y, CORRIDOR);
        if settings.colorblind_mode {
            let (cx, cy) = g.cell_center(x, y);
            draw_dot(&mut pixmap, cx, cy, g.cell * 0.14, BLACK);
        }
    }

    for room in &layout.rooms {
        fill_rect_room(&mut pixmap, g, room, ROOM);
        if settings.colorblind_mode {
            draw_room_crosshatch(&mut pixmap, g, room, 1.5, BLACK);
        }
    }

    draw_cell_walls(
        &mut pixmap,
        g,
        &scene.corridor_cells,
        Some(&scene.corridor_edges),
        &scene.door_edges,
        wall_w,
        BLACK,
    );

    for room in &layout.rooms {
        draw_room_walls(&mut pixmap, g, room, &scene.door_edges, wall_w, BLACK);
    }

    for door in &layout.doors {
        let (color, style) = if settings.colorblind_mode {
            let s = if door.secret {
                DoorStyle::DashDotDot
            } else if door.archway {
                DoorStyle::Dotted
            } else if door.locked {
                DoorStyle::ShortDash
            } else {
                DoorStyle::LongDash
            };
            (BLACK, s)
        } else if door.secret {
            (super::types::SECRET_DOOR, DoorStyle::Solid)
        } else if door.archway {
            (super::types::ARCHWAY, DoorStyle::Solid)
        } else if door.locked {
            (super::types::LOCKED_DOOR, DoorStyle::Solid)
        } else {
            (super::types::OPEN_DOOR, DoorStyle::Solid)
        };
        draw_door(&mut pixmap, g, door, door_w, color, style);
    }

    for window in &layout.windows {
        let style = if settings.colorblind_mode {
            DoorStyle::DashDot
        } else {
            DoorStyle::Solid
        };
        draw_window(&mut pixmap, g, window, door_w, WINDOW_COLOR, style);
    }

    draw_area_marker_group(
        &mut pixmap,
        g,
        &layout.start_markers,
        "S",
        START_MARKER,
        settings.colorblind_mode,
    );
    draw_area_marker_group(
        &mut pixmap,
        g,
        &layout.end_markers,
        "E",
        END_MARKER,
        settings.colorblind_mode,
    );
    draw_area_marker_group(
        &mut pixmap,
        g,
        &layout.trap_markers,
        "T",
        TRAP_MARKER,
        settings.colorblind_mode,
    );
    draw_area_marker_group(
        &mut pixmap,
        g,
        &layout.monster_markers,
        "M",
        MONSTER_MARKER,
        settings.colorblind_mode,
    );

    draw_staircase_group(
        &mut pixmap,
        g,
        &layout.stairs,
        STAIRS,
        settings.colorblind_mode,
    );

    Ok(pixmap)
}

// Fills the entire pixmap with a background color.
pub fn fill_bg(pixmap: &mut Pixmap, color: (u8, u8, u8, u8)) {
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.0, color.1, color.2, color.3);
    let Some(rect) = Rect::from_xywh(0.0, 0.0, pixmap.width() as f32, pixmap.height() as f32)
    else {
        return;
    };
    pixmap.fill_rect(rect, &paint, Transform::identity(), None);
}

// Draws grid color.
pub fn draw_grid_color(pixmap: &mut Pixmap, g: &ExportGeometry, color: (u8, u8, u8, u8)) {
    let left = g.left();
    let top = g.top();
    let width = g.cols as f32 * g.cell;
    let height = g.rows as f32 * g.cell;

    let mut paint = Paint::default();
    paint.set_color_rgba8(color.0, color.1, color.2, color.3);

    for c in 0..=g.cols {
        let x = (left + c as f32 * g.cell).round();
        let Some(rect) = Rect::from_xywh(x, top, 1.0, height) else {
            continue;
        };
        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
    }
    for r in 0..=g.rows {
        let y = (top + r as f32 * g.cell).round();
        let Some(rect) = Rect::from_xywh(left, y, width, 1.0) else {
            continue;
        };
        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
    }
}

// Fills rect cell.
pub fn fill_rect_cell(
    pixmap: &mut Pixmap,
    g: &ExportGeometry,
    col: usize,
    row: usize,
    color: (u8, u8, u8, u8),
) {
    let Some(rect) = g.cell_rect(col, row) else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.0, color.1, color.2, color.3);
    pixmap.fill_rect(rect, &paint, Transform::identity(), None);
}

// Fills rect room.
pub fn fill_rect_room(
    pixmap: &mut Pixmap,
    g: &ExportGeometry,
    room: &Room,
    color: (u8, u8, u8, u8),
) {
    let Some(rect) = Rect::from_xywh(
        g.left() + room.x as f32 * g.cell,
        g.top() + room.y as f32 * g.cell,
        room.width as f32 * g.cell,
        room.height as f32 * g.cell,
    ) else {
        return;
    };

    let mut paint = Paint::default();
    paint.set_color_rgba8(color.0, color.1, color.2, color.3);
    pixmap.fill_rect(rect, &paint, Transform::identity(), None);
}

// Fills rect marker.
pub fn fill_rect_marker(
    pixmap: &mut Pixmap,
    g: &ExportGeometry,
    marker: &AreaMarker,
    color: (u8, u8, u8, u8),
) {
    let Some(rect) = Rect::from_xywh(
        g.left() + marker.cell.0 as f32 * g.cell,
        g.top() + marker.cell.1 as f32 * g.cell,
        marker.size as f32 * g.cell,
        marker.size as f32 * g.cell,
    ) else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.0, color.1, color.2, color.3);
    pixmap.fill_rect(rect, &paint, Transform::identity(), None);
}

// Draws area marker.
pub fn draw_area_marker(
    pixmap: &mut Pixmap,
    g: &ExportGeometry,
    marker: &AreaMarker,
    label: &str,
    color: (u8, u8, u8, u8),
    colorblind_mode: bool,
) {
    let overlay = (
        color.0,
        color.1,
        color.2,
        if colorblind_mode { 56 } else { 90 },
    );
    fill_rect_marker(pixmap, g, marker, overlay);

    let x = g.left() + marker.cell.0 as f32 * g.cell;
    let y = g.top() + marker.cell.1 as f32 * g.cell;
    let w = marker.size as f32 * g.cell;
    let h = marker.size as f32 * g.cell;
    draw_wall_segment(pixmap, x, y, x + w, y, 3.0, color);
    draw_wall_segment(pixmap, x, y + h, x + w, y + h, 3.0, color);
    draw_wall_segment(pixmap, x, y, x, y + h, 3.0, color);
    draw_wall_segment(pixmap, x + w, y, x + w, y + h, 3.0, color);

    let symbol = if colorblind_mode { BLACK } else { WHITE };
    if label == "S" {
        draw_line(
            pixmap,
            x + w * 0.25,
            y + h * 0.75,
            x + w * 0.5,
            y + h * 0.25,
            4.0,
            symbol,
            DoorStyle::Solid,
        );
        draw_line(
            pixmap,
            x + w * 0.5,
            y + h * 0.25,
            x + w * 0.75,
            y + h * 0.75,
            4.0,
            symbol,
            DoorStyle::Solid,
        );
    } else if label == "E" {
        draw_line(
            pixmap,
            x + w * 0.25,
            y + h * 0.25,
            x + w * 0.75,
            y + h * 0.75,
            4.0,
            symbol,
            DoorStyle::Solid,
        );
        draw_line(
            pixmap,
            x + w * 0.75,
            y + h * 0.25,
            x + w * 0.25,
            y + h * 0.75,
            4.0,
            symbol,
            DoorStyle::Solid,
        );
    } else if label == "T" {
        // Horizontal bar
        draw_line(
            pixmap,
            x + w * 0.2,
            y + h * 0.25,
            x + w * 0.8,
            y + h * 0.25,
            4.0,
            symbol,
            DoorStyle::Solid,
        );
        // Vertical bar
        draw_line(
            pixmap,
            x + w * 0.5,
            y + h * 0.25,
            x + w * 0.5,
            y + h * 0.75,
            4.0,
            symbol,
            DoorStyle::Solid,
        );
    } else if label == "M" {
        draw_line(
            pixmap,
            x + w * 0.2,
            y + h * 0.75,
            x + w * 0.2,
            y + h * 0.25,
            4.0,
            symbol,
            DoorStyle::Solid,
        );
        draw_line(
            pixmap,
            x + w * 0.2,
            y + h * 0.25,
            x + w * 0.5,
            y + h * 0.5,
            4.0,
            symbol,
            DoorStyle::Solid,
        );
        draw_line(
            pixmap,
            x + w * 0.5,
            y + h * 0.5,
            x + w * 0.8,
            y + h * 0.25,
            4.0,
            symbol,
            DoorStyle::Solid,
        );
        draw_line(
            pixmap,
            x + w * 0.8,
            y + h * 0.25,
            x + w * 0.8,
            y + h * 0.75,
            4.0,
            symbol,
            DoorStyle::Solid,
        );
    }
}

// Fills rect staircase.
pub fn fill_rect_staircase(
    pixmap: &mut Pixmap,
    g: &ExportGeometry,
    staircase: &Staircase,
    color: (u8, u8, u8, u8),
) {
    let Some(rect) = Rect::from_xywh(
        g.left() + staircase.cell.0 as f32 * g.cell,
        g.top() + staircase.cell.1 as f32 * g.cell,
        staircase.width as f32 * g.cell,
        staircase.height as f32 * g.cell,
    ) else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.0, color.1, color.2, color.3);
    pixmap.fill_rect(rect, &paint, Transform::identity(), None);
}

// Draws a staircase marker into the raster export.
pub fn draw_staircase(
    pixmap: &mut Pixmap,
    g: &ExportGeometry,
    staircase: &Staircase,
    color: (u8, u8, u8, u8),
    colorblind_mode: bool,
) {
    let fill_color = if colorblind_mode {
        (180, 180, 180, 90)
    } else {
        (color.0, color.1, color.2, 90)
    };

    let Some(rect) = Rect::from_xywh(
        g.left() + staircase.cell.0 as f32 * g.cell,
        g.top() + staircase.cell.1 as f32 * g.cell,
        staircase.width as f32 * g.cell,
        staircase.height as f32 * g.cell,
    ) else {
        return;
    };

    let mut paint = Paint::default();
    paint.set_color_rgba8(fill_color.0, fill_color.1, fill_color.2, fill_color.3);
    pixmap.fill_rect(rect, &paint, Transform::identity(), None);

    let stroke_color = if colorblind_mode {
        BLACK
    } else {
        (150, 100, 30, 255)
    };
    let x = rect.left();
    let y = rect.top();
    let w = rect.width();
    let h = rect.height();

    draw_wall_segment(pixmap, x, y, x + w, y, 2.0, stroke_color);
    draw_wall_segment(pixmap, x, y + h, x + w, y + h, 2.0, stroke_color);
    draw_wall_segment(pixmap, x, y, x, y + h, 2.0, stroke_color);
    draw_wall_segment(pixmap, x + w, y, x + w, y + h, 2.0, stroke_color);

    let steps = (staircase.height as f32 * 2.0).round() as usize;
    let step_h = h / steps as f32;
    for i in 0..steps {
        let sy = (y + (i as f32 * step_h) + step_h * 0.5).round();
        draw_line(
            pixmap,
            x + 2.0,
            sy,
            x + w - 2.0,
            sy,
            1.5,
            stroke_color,
            DoorStyle::Solid,
        );
    }
}

// Draws staircase group.
pub fn draw_staircase_group(
    pixmap: &mut Pixmap,
    g: &ExportGeometry,
    stairs: &[Staircase],
    color: (u8, u8, u8, u8),
    colorblind_mode: bool,
) {
    for staircase in stairs {
        draw_staircase(pixmap, g, staircase, color, colorblind_mode);
    }
}

// Draws area marker group.
pub fn draw_area_marker_group(
    pixmap: &mut Pixmap,
    g: &ExportGeometry,
    markers: &[AreaMarker],
    label: &str,
    color: (u8, u8, u8, u8),
    colorblind_mode: bool,
) {
    for marker in markers {
        draw_area_marker(pixmap, g, marker, label, color, colorblind_mode);
    }
}

// Draws a filled circular dot at a pixel position.
pub fn draw_dot(pixmap: &mut Pixmap, cx: f32, cy: f32, radius: f32, color: (u8, u8, u8, u8)) {
    let mut pb = PathBuilder::new();
    pb.push_circle(cx, cy, radius);
    let Some(path) = pb.finish() else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.0, color.1, color.2, color.3);
    pixmap.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

// Draws room crosshatch.
pub fn draw_room_crosshatch(
    pixmap: &mut Pixmap,
    g: &ExportGeometry,
    room: &Room,
    width: f32,
    color: (u8, u8, u8, u8),
) {
    for x in room.x..(room.x + room.width) {
        for y in room.y..(room.y + room.height) {
            if let Some(rect) = g.cell_rect(x, y) {
                let pad = 2.0;
                draw_line(
                    pixmap,
                    rect.left() + pad,
                    rect.top() + pad,
                    rect.right() - pad,
                    rect.bottom() - pad,
                    width,
                    color,
                    DoorStyle::Solid,
                );
                draw_line(
                    pixmap,
                    rect.right() - pad,
                    rect.top() + pad,
                    rect.left() + pad,
                    rect.bottom() - pad,
                    width,
                    color,
                    DoorStyle::Solid,
                );
            }
        }
    }
}

// Draws room walls.
pub fn draw_room_walls(
    pixmap: &mut Pixmap,
    g: &ExportGeometry,
    room: &Room,
    door_edges: &HashSet<CellEdge>,
    width: f32,
    color: (u8, u8, u8, u8),
) {
    for rx in 0..room.width {
        for ry in 0..room.height {
            let col = room.x + rx;
            let row = room.y + ry;
            let left = (col, row);
            let right = (col + 1, row);
            let top = (col, row);
            let bottom = (col, row + 1);

            if rx == 0 {
                let neighbor = col.checked_sub(1).map(|x| (x, row));
                let suppressed = neighbor
                    .map(|n| door_edges.contains(&norm_edge(left, n)))
                    .unwrap_or(false);
                if !suppressed {
                    if let Some(rect) = g.cell_rect(col, row) {
                        draw_wall_segment(
                            pixmap,
                            rect.left(),
                            rect.top(),
                            rect.left(),
                            rect.bottom(),
                            width,
                            color,
                        );
                    }
                }
            }

            if rx == room.width - 1 {
                let suppressed = door_edges.contains(&norm_edge(left, right));
                if !suppressed {
                    if let Some(rect) = g.cell_rect(col, row) {
                        draw_wall_segment(
                            pixmap,
                            rect.right(),
                            rect.top(),
                            rect.right(),
                            rect.bottom(),
                            width,
                            color,
                        );
                    }
                }
            }

            if ry == 0 {
                let neighbor = row.checked_sub(1).map(|y| (col, y));
                let suppressed = neighbor
                    .map(|n| door_edges.contains(&norm_edge(top, n)))
                    .unwrap_or(false);
                if !suppressed {
                    if let Some(rect) = g.cell_rect(col, row) {
                        draw_wall_segment(
                            pixmap,
                            rect.left(),
                            rect.top(),
                            rect.right(),
                            rect.top(),
                            width,
                            color,
                        );
                    }
                }
            }

            if ry == room.height - 1 {
                let suppressed = door_edges.contains(&norm_edge(top, bottom));
                if !suppressed {
                    if let Some(rect) = g.cell_rect(col, row) {
                        draw_wall_segment(
                            pixmap,
                            rect.left(),
                            rect.bottom(),
                            rect.right(),
                            rect.bottom(),
                            width,
                            color,
                        );
                    }
                }
            }
        }
    }
}

// Draws cell walls.
pub fn draw_cell_walls(
    pixmap: &mut Pixmap,
    g: &ExportGeometry,
    cells: &HashSet<Cell>,
    connected_edges: Option<&HashSet<CellEdge>>,
    door_edges: &HashSet<CellEdge>,
    width: f32,
    color: (u8, u8, u8, u8),
) {
    for &(col, row) in cells {
        if let Some(rect) = g.cell_rect(col, row) {
            let right = (col + 1, row);
            let bottom = (col, row + 1);

            let should_left = col == 0
                || (!cells.contains(&(col - 1, row))
                    && !door_edges.contains(&norm_edge((col, row), (col - 1, row))));

            let right_connected = connected_edges
                .map(|set| set.contains(&norm_edge((col, row), right)))
                .unwrap_or(true);
            let should_right = (!cells.contains(&right) || !right_connected)
                && !door_edges.contains(&norm_edge((col, row), right));

            let should_top = row == 0
                || (!cells.contains(&(col, row - 1))
                    && !door_edges.contains(&norm_edge((col, row), (col, row - 1))));

            let bottom_connected = connected_edges
                .map(|set| set.contains(&norm_edge((col, row), bottom)))
                .unwrap_or(true);
            let should_bottom = (!cells.contains(&bottom) || !bottom_connected)
                && !door_edges.contains(&norm_edge((col, row), bottom));

            if should_left {
                draw_wall_segment(
                    pixmap,
                    rect.left(),
                    rect.top(),
                    rect.left(),
                    rect.bottom(),
                    width,
                    color,
                );
            }
            if should_right {
                draw_wall_segment(
                    pixmap,
                    rect.right(),
                    rect.top(),
                    rect.right(),
                    rect.bottom(),
                    width,
                    color,
                );
            }
            if should_top {
                draw_wall_segment(
                    pixmap,
                    rect.left(),
                    rect.top(),
                    rect.right(),
                    rect.top(),
                    width,
                    color,
                );
            }
            if should_bottom {
                draw_wall_segment(
                    pixmap,
                    rect.left(),
                    rect.bottom(),
                    rect.right(),
                    rect.bottom(),
                    width,
                    color,
                );
            }
        }
    }
}

// Draws wall segment.
pub fn draw_wall_segment(
    pixmap: &mut Pixmap,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    width: f32,
    color: (u8, u8, u8, u8),
) {
    let width = width.round().max(1.0);
    let half = width * 0.5;
    let x1 = x1.round();
    let y1 = y1.round();
    let x2 = x2.round();
    let y2 = y2.round();
    let rect = if (x1 - x2).abs() <= f32::EPSILON {
        let x = x1.min(x2);
        let y0 = y1.min(y2);
        let y1 = y1.max(y2);
        Rect::from_xywh(x - half, y0 - half, width, (y1 - y0) + width)
    } else {
        let y = y1.min(y2);
        let x0 = x1.min(x2);
        let x1 = x1.max(x2);
        Rect::from_xywh(x0 - half, y - half, (x1 - x0) + width, width)
    };

    let Some(rect) = rect else {
        return;
    };

    let mut paint = Paint::default();
    paint.set_color_rgba8(color.0, color.1, color.2, color.3);
    pixmap.fill_rect(rect, &paint, Transform::identity(), None);
}

// Snaps even width.
pub fn snap_even_width(value: f32) -> f32 {
    let mut w = value.round().max(1.0);
    if (w as u32) % 2 == 1 {
        w += 1.0;
    }
    w
}

// Draws a styled door line onto the pixmap.
pub fn draw_door(
    pixmap: &mut Pixmap,
    g: &ExportGeometry,
    door: &crate::layout::Door,
    width: f32,
    color: (u8, u8, u8, u8),
    style: DoorStyle,
) {
    let Some((x1, y1, x2, y2)) = door_line_points(g, door.from, door.to, door_render_width(door))
    else {
        return;
    };
    draw_line(pixmap, x1, y1, x2, y2, width, color, style);
}

// Draws a styled window line onto the pixmap.
pub fn draw_window(
    pixmap: &mut Pixmap,
    g: &ExportGeometry,
    window: &Window,
    width: f32,
    color: (u8, u8, u8, u8),
    style: DoorStyle,
) {
    let Some((x1, y1, x2, y2)) = window_line_points(g, window) else {
        return;
    };
    draw_line(pixmap, x1, y1, x2, y2, width, color, style);
}

// Draws a solid or patterned line segment.
#[allow(clippy::too_many_arguments)]
pub fn draw_line(
    pixmap: &mut Pixmap,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    width: f32,
    color: (u8, u8, u8, u8),
    style: DoorStyle,
) {
    match style {
        DoorStyle::Solid => stroke_path(pixmap, x1, y1, x2, y2, width, color, None),
        DoorStyle::LongDash => stroke_path(pixmap, x1, y1, x2, y2, width, color, Some((14.0, 7.0))),
        DoorStyle::ShortDash => stroke_path(pixmap, x1, y1, x2, y2, width, color, Some((6.0, 4.0))),
        DoorStyle::Dotted => dotted_line(pixmap, x1, y1, x2, y2, width, color),
        DoorStyle::DashDot => stroke_path(pixmap, x1, y1, x2, y2, width, color, Some((7.0, 3.0))),
        DoorStyle::DashDotDot => {
            stroke_path(pixmap, x1, y1, x2, y2, width, color, Some((7.0, 3.0)))
        }
    }
}

// Strokes a path segment with an optional dash pattern.
#[allow(clippy::too_many_arguments)]
pub fn stroke_path(
    pixmap: &mut Pixmap,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    width: f32,
    color: (u8, u8, u8, u8),
    dash: Option<(f32, f32)>,
) {
    let mut pb = PathBuilder::new();
    pb.move_to(x1, y1);
    pb.line_to(x2, y2);
    let Some(path) = pb.finish() else {
        return;
    };

    let mut paint = Paint::default();
    paint.set_color_rgba8(color.0, color.1, color.2, color.3);

    let mut stroke = Stroke {
        width,
        ..Default::default()
    };
    if let Some((dash_len, gap_len)) = dash {
        stroke.dash = StrokeDash::new(vec![dash_len, gap_len], 0.0);
    }

    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
}

// Handles dotted line.
pub fn dotted_line(
    pixmap: &mut Pixmap,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    width: f32,
    color: (u8, u8, u8, u8),
) {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 0.0 {
        return;
    }
    let ux = dx / len;
    let uy = dy / len;

    let mut d = 0.0_f32;
    let step = 6.0;
    let radius = (width * 0.35).max(1.0);
    while d <= len {
        draw_dot(pixmap, x1 + ux * d, y1 + uy * d, radius, color);
        d += step;
    }
}

// Handles raster image from pixmap.
pub fn raster_image_from_pixmap(
    pixmap: &Pixmap,
    settings: &UiSettings,
) -> Result<DynamicImage, String> {
    let rgba = RgbaImage::from_raw(pixmap.width(), pixmap.height(), pixmap.data().to_vec())
        .ok_or_else(|| "Failed to build image buffer".to_string())?;
    let mut img = DynamicImage::ImageRgba8(rgba);

    let target = raster_target_size(settings);
    if img.width() != target.0 || img.height() != target.1 {
        img = img.resize_exact(target.0, target.1, FilterType::Lanczos3);
    }

    Ok(img)
}

// Handles raster mask image from pixmap.
pub fn raster_mask_image_from_pixmap(
    pixmap: &Pixmap,
    settings: &UiSettings,
) -> Result<DynamicImage, String> {
    let rgba = RgbaImage::from_raw(pixmap.width(), pixmap.height(), pixmap.data().to_vec())
        .ok_or_else(|| "Failed to build image buffer".to_string())?;
    let mut img = DynamicImage::ImageRgba8(rgba);

    let target = raster_target_size(settings);
    if img.width() != target.0 || img.height() != target.1 {
        img = img.resize_exact(target.0, target.1, FilterType::Nearest);
    }

    let mut out = img.to_rgba8();
    for pixel in out.pixels_mut() {
        let on = pixel[0] > 127 || pixel[1] > 127 || pixel[2] > 127;
        let v = if on { 255 } else { 0 };
        *pixel = image::Rgba([v, v, v, 255]);
    }

    Ok(DynamicImage::ImageRgba8(out))
}

// Handles raster target size.
pub fn raster_target_size(settings: &UiSettings) -> (u32, u32) {
    let width = settings.export_width.clamp(1, 10_000);
    let mut height = settings.export_height.clamp(1, 10_000);

    if !settings.allow_export_aspect_change {
        let cols = settings.cols.max(1) as f32;
        let rows = settings.rows.max(1) as f32;
        let ratio = cols / rows;
        height = ((width as f32) / ratio).round() as u32;
        height = height.clamp(1, 10_000);
    }

    (width, height)
}
