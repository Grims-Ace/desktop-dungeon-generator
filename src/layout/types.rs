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

impl Corridor {
    // Return all cells belonging to this corridor given the grid size and room cells to exclude.
    pub fn cells(
        &self,
        cols: usize,
        rows: usize,
        room_cells: &std::collections::HashSet<(usize, usize)>,
    ) -> std::collections::HashSet<(usize, usize)> {
        let mut cells = std::collections::HashSet::new();
        if cols == 0 || rows == 0 {
            return cells;
        }

        let width = self.width.max(1);
        let min_offset = -((width as isize - 1) / 2);
        let max_offset = width as isize / 2;

        if self.path.len() == 1 {
            let (x, y) = self.path[0];
            for dy in min_offset..=max_offset {
                let ny = y as isize + dy;
                if ny < 0 || ny >= rows as isize {
                    continue;
                }
                let cell = (x, ny as usize);
                if !room_cells.contains(&cell) {
                    cells.insert(cell);
                }
            }
            return cells;
        }

        for pair in self.path.windows(2) {
            let a = pair[0];
            let b = pair[1];
            if a == b {
                continue;
            }

            if a.0 != b.0 {
                let x0 = a.0.min(b.0);
                let x1 = a.0.max(b.0);
                let y = a.1 as isize;
                for x in x0..=x1 {
                    for dy in min_offset..=max_offset {
                        let ny = y + dy;
                        if ny < 0 || ny >= rows as isize {
                            continue;
                        }
                        let cell = (x, ny as usize);
                        if !room_cells.contains(&cell) {
                            cells.insert(cell);
                        }
                    }
                }
            } else {
                let y0 = a.1.min(b.1);
                let y1 = a.1.max(b.1);
                let x = a.0 as isize;
                for y in y0..=y1 {
                    for dx in min_offset..=max_offset {
                        let nx = x + dx;
                        if nx < 0 || nx >= cols as isize {
                            continue;
                        }
                        let cell = (nx as usize, y);
                        if !room_cells.contains(&cell) {
                            cells.insert(cell);
                        }
                    }
                }
            }
        }

        for turn in self.path.windows(3) {
            let prev = turn[0];
            let corner = turn[1];
            let next = turn[2];
            let incoming = (
                corner.0 as isize - prev.0 as isize,
                corner.1 as isize - prev.1 as isize,
            );
            let outgoing = (
                next.0 as isize - corner.0 as isize,
                next.1 as isize - corner.1 as isize,
            );
            if incoming == outgoing {
                continue;
            }

            for dx in min_offset..=max_offset {
                let nx = corner.0 as isize + dx;
                if nx < 0 || nx >= cols as isize {
                    continue;
                }
                for dy in min_offset..=max_offset {
                    let ny = corner.1 as isize + dy;
                    if ny < 0 || ny >= rows as isize {
                        continue;
                    }

                    let cell = (nx as usize, ny as usize);
                    if !room_cells.contains(&cell) {
                        cells.insert(cell);
                    }
                }
            }
        }

        cells
    }
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
