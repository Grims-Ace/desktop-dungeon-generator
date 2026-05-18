/*
 * Core data structures for the dungeon layout.
 * Defines structs for Rooms, Corridors, Doors, Windows, Stairs,
 * TextLabels, and the master DungeonLayout container.
 */

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Room {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

impl Room {
    // Return the center cell of the room.
    pub fn center_cell(&self) -> (usize, usize) {
        (self.x + (self.width / 2), self.y + (self.height / 2))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Corridor {
    #[allow(dead_code)]
    pub id: u64,
    pub start_room_id: usize,
    pub end_room_id: usize,
    pub path: Vec<(usize, usize)>,
    pub width: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Door {
    pub from: (usize, usize),
    pub to: (usize, usize),
    pub width: usize,
    pub span_width: bool,
    pub locked: bool,
    pub archway: bool,
    pub secret: bool,
    pub manual: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Window {
    pub cell: (usize, usize),
    pub side: WindowSide,
    pub width: usize,
    pub span_width: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextLabel {
    pub cell: (usize, usize),
    pub text: String,
    pub font_size: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AreaMarker {
    pub cell: (usize, usize),
    pub size: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Staircase {
    pub cell: (usize, usize),
    pub width: usize,
    pub height: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowSide {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DoorSettings {
    pub frequency_percent: usize,
    pub room_hallway_percent: usize,
    pub locked_percent: usize,
    pub secret_percent: usize,
    pub allow_middle_corridor_doors: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WindowSettings {
    pub enabled: bool,
    pub min_width: usize,
    pub max_width: usize,
    pub frequency_percent: usize,
    pub room_hallway_percent: usize,
    pub allow_internal_windows: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DungeonLayout {
    pub rooms: Vec<Room>,
    pub corridors: Vec<Corridor>,
    pub doors: Vec<Door>,
    pub windows: Vec<Window>,
    pub text_labels: Vec<TextLabel>,
    pub start_markers: Vec<AreaMarker>,
    pub end_markers: Vec<AreaMarker>,
    pub trap_markers: Vec<AreaMarker>,
    pub monster_markers: Vec<AreaMarker>,
    pub packed_rooms: bool,
    pub stairs: Vec<Staircase>,
}

impl DungeonLayout {
    // Build an empty layout for the current packing mode.
    pub(crate) fn empty(packed_rooms: bool) -> Self {
        Self {
            packed_rooms,
            ..Self::default()
        }
    }

    // Build a layout from generated rooms and corridors.
    pub(crate) fn from_generated_parts(
        rooms: Vec<Room>,
        corridors: Vec<Corridor>,
        packed_rooms: bool,
    ) -> Self {
        Self {
            rooms,
            corridors,
            packed_rooms,
            ..Self::default()
        }
    }
}
