/*
 * Core interaction state types and enums.
 * Defines the various states for dragging and resizing dungeon elements.
 */

use crate::layout;

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

#[derive(Debug, Clone, Copy)]
pub enum ResizeCorner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

#[derive(Debug, Clone)]
pub struct StaircaseResizeState {
    pub stair_idx: usize,
    pub original_stair: layout::Staircase,
    pub resize_corner: ResizeCorner,
}

#[derive(Debug, Clone)]
pub enum ResizeState {
    Room(RoomResizeState),
    Text(TextResizeState),
    Staircase(StaircaseResizeState),
}
