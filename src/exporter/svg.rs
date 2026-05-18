/*
 * SVG vector export logic.
 * Handles building SVG representations of dungeon levels, including
 * rooms, corridors, walls, doors, and markers.
 */

use super::types::{
    BG, CORRIDOR, END_MARKER, ExportGeometry, GRID, MONSTER_MARKER, ROOM, START_MARKER,
    TRAP_MARKER, WINDOW_COLOR, door_render_width, norm_edge,
};
use super::utils::{collect_scene_data, door_line_points, window_line_points};
use crate::layout::DungeonLayout;
use crate::ui::UiSettings;
use std::collections::HashSet;
use std::fmt::Write as _;

pub fn build_svg(layout: &DungeonLayout, settings: &UiSettings) -> String {
    let g = ExportGeometry::new(settings.cols, settings.rows);
    let wall_w = (g.cell / 2.5).max(1.0);
    let door_w = (g.cell / 5.0).max(1.0);

    let mut s = String::new();
    let _ = writeln!(
        s,
        "<svg xmlns='http://www.w3.org/2000/svg' width='{}' height='{}' viewBox='0 0 {} {}'>",
        g.width, g.height, g.width, g.height
    );
    let _ = writeln!(
        s,
        "<rect width='100%' height='100%' fill='rgb({},{},{})'/>",
        BG.0, BG.1, BG.2
    );

    if settings.export_show_grid {
        for c in 0..=g.cols {
            let x = g.left() + c as f32 * g.cell;
            let _ = writeln!(
                s,
                "<line x1='{x}' y1='{}' x2='{x}' y2='{}' stroke='rgb({},{},{})' stroke-width='1'/>",
                g.top(),
                g.top() + g.rows as f32 * g.cell,
                GRID.0,
                GRID.1,
                GRID.2
            );
        }
        for r in 0..=g.rows {
            let y = g.top() + r as f32 * g.cell;
            let _ = writeln!(
                s,
                "<line x1='{}' y1='{y}' x2='{}' y2='{y}' stroke='rgb({},{},{})' stroke-width='1'/>",
                g.left(),
                g.left() + g.cols as f32 * g.cell,
                GRID.0,
                GRID.1,
                GRID.2
            );
        }
    }

    let scene = collect_scene_data(layout, settings);

    for &(x, y) in &scene.corridor_cells {
        let px = g.left() + x as f32 * g.cell;
        let py = g.top() + y as f32 * g.cell;
        let _ = writeln!(
            s,
            "<rect x='{px}' y='{py}' width='{}' height='{}' fill='rgb({},{},{})'/>",
            g.cell, g.cell, CORRIDOR.0, CORRIDOR.1, CORRIDOR.2
        );
        if settings.colorblind_mode {
            let (cx, cy) = g.cell_center(x, y);
            let _ = writeln!(
                s,
                "<circle cx='{cx}' cy='{cy}' r='{}' fill='black'/>",
                g.cell * 0.14
            );
        }
    }

    for room in &layout.rooms {
        let px = g.left() + room.x as f32 * g.cell;
        let py = g.top() + room.y as f32 * g.cell;
        let _ = writeln!(
            s,
            "<rect x='{px}' y='{py}' width='{}' height='{}' fill='rgb({},{},{})'/>",
            room.width as f32 * g.cell,
            room.height as f32 * g.cell,
            ROOM.0,
            ROOM.1,
            ROOM.2
        );
        if settings.colorblind_mode {
            for x in room.x..(room.x + room.width) {
                for y in room.y..(room.y + room.height) {
                    let rx = g.left() + x as f32 * g.cell + 2.0;
                    let ry = g.top() + y as f32 * g.cell + 2.0;
                    let rr = g.cell - 4.0;
                    let _ = writeln!(
                        s,
                        "<line x1='{rx}' y1='{ry}' x2='{}' y2='{}' stroke='black' stroke-width='1.5'/>",
                        rx + rr,
                        ry + rr
                    );
                    let _ = writeln!(
                        s,
                        "<line x1='{}' y1='{ry}' x2='{rx}' y2='{}' stroke='black' stroke-width='1.5'/>",
                        rx + rr,
                        ry + rr
                    );
                }
            }
        }
    }

    append_svg_walls(
        &mut s,
        &g,
        &scene.corridor_cells,
        Some(&scene.corridor_edges),
        &scene.door_edges,
        wall_w,
    );
    append_svg_walls(
        &mut s,
        &g,
        &scene.room_cells,
        Some(&scene.room_edges),
        &scene.door_edges,
        wall_w,
    );

    for door in &layout.doors {
        let (color, dash) = if settings.colorblind_mode {
            if door.secret {
                ("black", Some("7,3,1,3,1,4"))
            } else if door.archway {
                ("black", Some("2,4"))
            } else if door.locked {
                ("black", Some("6,4"))
            } else {
                ("black", Some("14,7"))
            }
        } else if door.secret {
            ("rgb(170,80,170)", None)
        } else if door.archway {
            ("rgb(230,140,60)", None)
        } else if door.locked {
            ("rgb(220,70,70)", None)
        } else {
            ("rgb(80,200,120)", None)
        };

        if let Some((x1, y1, x2, y2)) =
            door_line_points(&g, door.from, door.to, door_render_width(door))
        {
            if let Some(pattern) = dash {
                let _ = writeln!(
                    s,
                    "<line x1='{x1}' y1='{y1}' x2='{x2}' y2='{y2}' stroke='{color}' stroke-width='{door_w}' stroke-dasharray='{pattern}'/>"
                );
            } else {
                let _ = writeln!(
                    s,
                    "<line x1='{x1}' y1='{y1}' x2='{x2}' y2='{y2}' stroke='{color}' stroke-width='{door_w}'/>"
                );
            }
        }
    }

    for window in &layout.windows {
        let dash = settings.colorblind_mode.then_some("7,3,1,4");
        if let Some((x1, y1, x2, y2)) = window_line_points(&g, window) {
            if let Some(pattern) = dash {
                let _ = writeln!(
                    s,
                    "<line x1='{x1}' y1='{y1}' x2='{x2}' y2='{y2}' stroke='rgb({},{},{})' stroke-width='{door_w}' stroke-dasharray='{pattern}'/>",
                    WINDOW_COLOR.0, WINDOW_COLOR.1, WINDOW_COLOR.2
                );
            } else {
                let _ = writeln!(
                    s,
                    "<line x1='{x1}' y1='{y1}' x2='{x2}' y2='{y2}' stroke='rgb({},{},{})' stroke-width='{door_w}'/>",
                    WINDOW_COLOR.0, WINDOW_COLOR.1, WINDOW_COLOR.2
                );
            }
        }
    }

    append_svg_area_marker_group(
        &mut s,
        &g,
        &layout.start_markers,
        "S",
        START_MARKER,
        settings.colorblind_mode,
    );
    append_svg_area_marker_group(
        &mut s,
        &g,
        &layout.end_markers,
        "E",
        END_MARKER,
        settings.colorblind_mode,
    );
    append_svg_area_marker_group(
        &mut s,
        &g,
        &layout.trap_markers,
        "T",
        TRAP_MARKER,
        settings.colorblind_mode,
    );
    append_svg_area_marker_group(
        &mut s,
        &g,
        &layout.monster_markers,
        "M",
        MONSTER_MARKER,
        settings.colorblind_mode,
    );

    // Draw stairs.
    for stair in &layout.stairs {
        let x = g.left() + stair.cell.0 as f32 * g.cell;
        let y = g.top() + stair.cell.1 as f32 * g.cell;
        let w = stair.width as f32 * g.cell;
        let h = stair.height as f32 * g.cell;

        let fill = if settings.colorblind_mode {
            "rgb(180,180,180)"
        } else {
            "rgb(200,150,50)"
        };
        let _ = writeln!(
            s,
            "<rect x='{x}' y='{y}' width='{w}' height='{h}' fill='{fill}' fill-opacity='0.35' stroke='rgb(150,100,30)' stroke-width='2'/>"
        );
        let steps = (stair.height as f32 * 2.0).round() as usize;
        let step_h = h / steps as f32;
        for i in 0..steps {
            let sy = y + (i as f32 * step_h) + step_h * 0.5;
            let _ = writeln!(
                s,
                "<line x1='{x1}' y1='{sy}' x2='{x2}' y2='{sy}' stroke='rgb(150,100,30)' stroke-width='1.5'/>",
                x1 = x + 2.0,
                x2 = x + w - 2.0
            );
        }
    }

    s.push_str("</svg>\n");
    s
}

pub fn append_svg_walls(
    out: &mut String,
    g: &ExportGeometry,
    cells: &HashSet<(usize, usize)>,
    connected_edges: Option<&HashSet<((usize, usize), (usize, usize))>>,
    door_edges: &HashSet<((usize, usize), (usize, usize))>,
    width: f32,
) {
    for &(col, row) in cells {
        let left = g.left() + col as f32 * g.cell;
        let top = g.top() + row as f32 * g.cell;
        let right = left + g.cell;
        let bottom = top + g.cell;

        let right_cell = (col + 1, row);
        let bottom_cell = (col, row + 1);

        let left_edge = col == 0
            || (!cells.contains(&(col - 1, row))
                && !door_edges.contains(&norm_edge((col, row), (col - 1, row))));

        let right_connected = connected_edges
            .map(|set| set.contains(&norm_edge((col, row), right_cell)))
            .unwrap_or(true);
        let right_edge = (!cells.contains(&right_cell) || !right_connected)
            && !door_edges.contains(&norm_edge((col, row), right_cell));

        let top_edge = row == 0
            || (!cells.contains(&(col, row - 1))
                && !door_edges.contains(&norm_edge((col, row), (col, row - 1))));

        let bottom_connected = connected_edges
            .map(|set| set.contains(&norm_edge((col, row), bottom_cell)))
            .unwrap_or(true);
        let bottom_edge = (!cells.contains(&bottom_cell) || !bottom_connected)
            && !door_edges.contains(&norm_edge((col, row), bottom_cell));

        if left_edge {
            let _ = writeln!(
                out,
                "<rect x='{}' y='{}' width='{width}' height='{}' fill='black'/>",
                left - (width * 0.5),
                top - (width * 0.5),
                (bottom - top) + width
            );
        }
        if right_edge {
            let _ = writeln!(
                out,
                "<rect x='{}' y='{}' width='{width}' height='{}' fill='black'/>",
                right - (width * 0.5),
                top - (width * 0.5),
                (bottom - top) + width
            );
        }
        if top_edge {
            let _ = writeln!(
                out,
                "<rect x='{}' y='{}' width='{}' height='{width}' fill='black'/>",
                left - (width * 0.5),
                top - (width * 0.5),
                (right - left) + width
            );
        }
        if bottom_edge {
            let _ = writeln!(
                out,
                "<rect x='{}' y='{}' width='{}' height='{width}' fill='black'/>",
                left - (width * 0.5),
                bottom - (width * 0.5),
                (right - left) + width
            );
        }
    }
}

pub fn append_svg_area_marker(
    out: &mut String,
    g: &ExportGeometry,
    marker: &crate::layout::AreaMarker,
    label: &str,
    color: (u8, u8, u8, u8),
    colorblind_mode: bool,
) {
    let x = g.left() + marker.cell.0 as f32 * g.cell;
    let y = g.top() + marker.cell.1 as f32 * g.cell;
    let w = marker.size as f32 * g.cell;
    let h = marker.size as f32 * g.cell;
    let alpha = if colorblind_mode { 0.22 } else { 0.35 };
    let _ = writeln!(
        out,
        "<rect x='{x}' y='{y}' width='{w}' height='{h}' fill='rgb({},{},{})' fill-opacity='{alpha}' stroke='rgb({},{},{})' stroke-width='3'/>",
        color.0, color.1, color.2, color.0, color.1, color.2
    );
    let symbol = if colorblind_mode { "black" } else { "white" };
    if label == "S" {
        let _ = writeln!(
            out,
            "<polyline points='{},{} {},{} {},{}' fill='none' stroke='{}' stroke-width='4' stroke-linecap='round' stroke-linejoin='round'/>",
            x + w * 0.25,
            y + h * 0.75,
            x + w * 0.5,
            y + h * 0.25,
            x + w * 0.75,
            y + h * 0.75,
            symbol
        );
    } else if label == "E" {
        let _ = writeln!(
            out,
            "<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='{}' stroke-width='4' stroke-linecap='round'/>",
            x + w * 0.25,
            y + h * 0.25,
            x + w * 0.75,
            y + h * 0.75,
            symbol
        );
        let _ = writeln!(
            out,
            "<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='{}' stroke-width='4' stroke-linecap='round'/>",
            x + w * 0.75,
            y + h * 0.25,
            x + w * 0.25,
            y + h * 0.75,
            symbol
        );
    } else if label == "T" {
        // Horizontal bar
        let _ = writeln!(
            out,
            "<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='{}' stroke-width='4' stroke-linecap='round'/>",
            x + w * 0.2,
            y + h * 0.25,
            x + w * 0.8,
            y + h * 0.25,
            symbol
        );
        // Vertical bar
        let _ = writeln!(
            out,
            "<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='{}' stroke-width='4' stroke-linecap='round'/>",
            x + w * 0.5,
            y + h * 0.25,
            x + w * 0.5,
            y + h * 0.75,
            symbol
        );
    } else if label == "M" {
        let _ = writeln!(
            out,
            "<polyline points='{},{} {},{} {},{} {},{} {},{}' fill='none' stroke='{}' stroke-width='4' stroke-linecap='round' stroke-linejoin='round'/>",
            x + w * 0.2,
            y + h * 0.75,
            x + w * 0.2,
            y + h * 0.25,
            x + w * 0.5,
            y + h * 0.5,
            x + w * 0.8,
            y + h * 0.25,
            x + w * 0.8,
            y + h * 0.75,
            symbol
        );
    }
}

pub fn append_svg_area_marker_group(
    out: &mut String,
    g: &ExportGeometry,
    markers: &[crate::layout::AreaMarker],
    label: &str,
    color: (u8, u8, u8, u8),
    colorblind_mode: bool,
) {
    for marker in markers {
        append_svg_area_marker(out, g, marker, label, color, colorblind_mode);
    }
}
