/*
 * Core types and constants for the export system.
 * Defines shared color constants, geometry helpers, and data structures
 * used by both SVG and raster exporters.
 */

use crate::layout::{Door, DungeonLayout, Window};
use std::collections::HashSet;
use std::path::PathBuf;
use tiny_skia::Rect;

pub const BG: (u8, u8, u8, u8) = (24, 24, 26, 255);
pub const GRID: (u8, u8, u8, u8) = (130, 130, 130, 255);
pub const ROOM: (u8, u8, u8, u8) = (70, 120, 160, 255);
pub const CORRIDOR: (u8, u8, u8, u8) = (210, 190, 120, 255);
pub const BLACK: (u8, u8, u8, u8) = (0, 0, 0, 255);
pub const OPEN_DOOR: (u8, u8, u8, u8) = (80, 200, 120, 255);
pub const LOCKED_DOOR: (u8, u8, u8, u8) = (220, 70, 70, 255);
pub const SECRET_DOOR: (u8, u8, u8, u8) = (170, 80, 170, 255);
pub const ARCHWAY: (u8, u8, u8, u8) = (230, 140, 60, 255);
pub const WINDOW_COLOR: (u8, u8, u8, u8) = (70, 130, 220, 255);
pub const WHITE: (u8, u8, u8, u8) = (255, 255, 255, 255);
pub const START_MARKER: (u8, u8, u8, u8) = (60, 220, 200, 255);
pub const END_MARKER: (u8, u8, u8, u8) = (240, 90, 90, 255);
pub const TRAP_MARKER: (u8, u8, u8, u8) = (150, 80, 230, 255);
pub const MONSTER_MARKER: (u8, u8, u8, u8) = (200, 50, 50, 255);
pub const STAIRS: (u8, u8, u8, u8) = (200, 150, 50, 255);

pub enum ExportTarget {
    File(PathBuf),
    Folder(PathBuf),
}

#[derive(Debug, Clone, Copy)]
pub struct ExportProgress {
    pub completed: usize,
    pub total: usize,
}

pub enum ExportEvent {
    Progress(ExportProgress),
    Finished(Result<PathBuf, String>),
}

#[derive(Debug, Clone, Copy)]
pub enum DoorStyle {
    Solid,
    LongDash,
    ShortDash,
    Dotted,
    DashDot,
    DashDotDot,
}

#[derive(Debug, Clone, Copy)]
pub struct ExportGeometry {
    pub cols: usize,
    pub rows: usize,
    pub cell: f32,
    pub pad: f32,
    pub width: u32,
    pub height: u32,
}

impl ExportGeometry {
    pub fn new(cols: usize, rows: usize) -> Self {
        let cell = 64.0;
        let pad = 0.0;
        let width = (cols as f32 * cell).round() as u32;
        let height = (rows as f32 * cell).round() as u32;
        Self {
            cols,
            rows,
            cell,
            pad,
            width,
            height,
        }
    }

    pub fn left(&self) -> f32 {
        self.pad
    }

    pub fn top(&self) -> f32 {
        self.pad
    }

    pub fn cell_rect(&self, col: usize, row: usize) -> Option<Rect> {
        Rect::from_xywh(
            self.left() + col as f32 * self.cell,
            self.top() + row as f32 * self.cell,
            self.cell,
            self.cell,
        )
    }

    pub fn cell_center(&self, col: usize, row: usize) -> (f32, f32) {
        (
            self.left() + (col as f32 + 0.5) * self.cell,
            self.top() + (row as f32 + 0.5) * self.cell,
        )
    }
}

#[derive(Default)]
pub struct SceneData {
    pub corridor_cells: HashSet<(usize, usize)>,
    pub corridor_edges: HashSet<((usize, usize), (usize, usize))>,
    pub room_cells: HashSet<(usize, usize)>,
    pub room_edges: HashSet<((usize, usize), (usize, usize))>,
    pub door_edges: HashSet<((usize, usize), (usize, usize))>,
}

#[derive(Debug, Clone, Copy)]
pub enum MaskTask {
    Floors,
    Walls,
    Doors,
    Archways,
    LockedDoors,
    SecretDoors,
    Windows,
    StartMarkers,
    EndMarkers,
    TrapMarkers,
    MonsterMarkers,
    Stairs,
    Grid,
}

#[derive(Debug, Clone, Copy)]
pub enum CompositeTask {
    Mask(MaskTask),
    Composite,
}

pub struct CompositeExportContext<'a> {
    pub layout: &'a DungeonLayout,
    pub folder: PathBuf,
    pub geometry: ExportGeometry,
    pub scene: SceneData,
    pub wall_width: f32,
    pub door_width: f32,
}

pub fn norm_edge(a: (usize, usize), b: (usize, usize)) -> ((usize, usize), (usize, usize)) {
    if a <= b { (a, b) } else { (b, a) }
}

pub fn door_edges_for(
    door: &Door,
    cols: usize,
    rows: usize,
) -> Vec<((usize, usize), (usize, usize))> {
    let mut edges = Vec::new();
    if door.from.0.abs_diff(door.to.0) + door.from.1.abs_diff(door.to.1) != 1 {
        return edges;
    }

    let width = door_render_width(door).max(1) as isize;
    let min_offset = -((width - 1) / 2);
    let max_offset = width / 2;

    if door.from.0 != door.to.0 {
        let y = door.from.1 as isize;
        for dy in min_offset..=max_offset {
            let ny = y + dy;
            if ny < 0 || ny >= rows as isize {
                continue;
            }
            let a = (door.from.0, ny as usize);
            let b = (door.to.0, ny as usize);
            edges.push(norm_edge(a, b));
        }
    } else {
        let x = door.from.0 as isize;
        for dx in min_offset..=max_offset {
            let nx = x + dx;
            if nx < 0 || nx >= cols as isize {
                continue;
            }
            let a = (nx as usize, door.from.1);
            let b = (nx as usize, door.to.1);
            edges.push(norm_edge(a, b));
        }
    }

    edges
}

pub fn door_render_width(door: &Door) -> usize {
    if door.span_width {
        door.width.max(1)
    } else {
        1
    }
}

pub fn window_render_width(window: &Window) -> usize {
    if window.span_width {
        window.width.max(1)
    } else {
        1
    }
}
