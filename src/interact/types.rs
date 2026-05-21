/*
 * Core interaction state types and enums.
 * Defines the various states for dragging and resizing dungeon elements.
 */

use crate::layout;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct RoomDragState {
    pub room_idx: usize,
    pub offset_x: f32,
    pub offset_y: f32,
}

#[derive(Debug, Clone)]
pub struct TextDragState {
    pub text_idx: usize,
}

#[derive(Debug, Clone)]
pub struct MarkerDragState {
    pub kind: MarkerKind,
    pub marker_idx: usize,
    pub offset_x: f32,
    pub offset_y: f32,
}

#[derive(Debug, Clone)]
pub struct CorridorDragState {
    pub corridor_index: usize,
    pub original_path: Vec<(usize, usize)>,
    pub original_cell: (usize, usize),
}

#[derive(Debug, Clone)]
pub struct AddCorridorDrag {
    pub start_cell: (usize, usize),
    pub current_cell: (usize, usize),
}

#[derive(Debug, Clone)]
pub struct AddDoorDrag {
    pub painted_edges: HashSet<((usize, usize), (usize, usize))>,
    pub snapshot_taken: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum ResizeCorner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarkerKind {
    Start,
    End,
    Trap,
    Monster,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoverMarker {
    Start(usize),
    End(usize),
    Trap(usize),
    Monster(usize),
}

#[derive(Debug, Clone)]
pub struct RoomResizeState {
    pub room_idx: usize,
    pub original_room: layout::Room,
    pub current_room: layout::Room,
    pub resize_corner: ResizeCorner,
}

#[derive(Debug, Clone)]
pub enum DragState {
    Room(RoomDragState),
    Text(TextDragState),
    Marker(MarkerDragState),
    Corridor(CorridorDragState),
    Staircase(StaircaseDragState),
}

#[derive(Debug, Clone)]
pub struct StaircaseDragState {
    pub stair_idx: usize,
    pub offset_x: f32,
    pub offset_y: f32,
    pub original_cell: (usize, usize),
}

#[derive(Debug, Clone)]
pub struct TextResizeState {
    pub text_idx: usize,
    pub origin_cell: (usize, usize),
    pub original_font_size: u16,
    pub current_font_size: u16,
}

#[derive(Debug, Clone)]
pub struct StaircaseResizeState {
    pub stair_idx: usize,
    pub original_stair: layout::Staircase,
    pub current_stair: layout::Staircase,
    pub resize_corner: ResizeCorner,
}

#[derive(Debug, Clone)]
pub enum ResizeState {
    Room(RoomResizeState),
    Text(TextResizeState),
    Staircase(StaircaseResizeState),
}

// ---- Multi-selection types ----

/// Identifies a single selectable item in the dungeon layout.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SelectedItem {
    Room(usize),
    Text(usize),
    Staircase(usize),
    Marker(MarkerKind, usize),
}

/// State while the user is rubber-band selecting (left-click drag on empty space).
#[derive(Debug, Clone)]
pub struct SelectionBoxDrag {
    /// Grid-space start corner of the selection rectangle.
    pub start_cell: (f32, f32),
    /// Grid-space current corner of the selection rectangle (tracks the pointer).
    pub current_cell: (f32, f32),
}

/// State while dragging a multi-selection with the left mouse button.
#[derive(Debug, Clone)]
pub struct MultiDragState {
    /// Grid-space position when the drag started (used to compute delta).
    pub start_grid: (f32, f32),
    /// Snapshot of each selected item's original position at drag start.
    pub origins: Vec<(SelectedItem, (isize, isize))>,
}

/// State while resizing a multi-selection with the right mouse button.
#[derive(Debug, Clone)]
pub struct MultiResizeState {
    /// Grid-space position when the resize started.
    pub start_grid: (f32, f32),
    /// Snapshot of each selected item's original size at resize start.
    /// For rooms/staircases: (x, y, width, height).
    /// For text: (cell_x, cell_y, font_size as width, 0).
    pub origins: Vec<(SelectedItem, (isize, isize, isize, isize))>,
}
