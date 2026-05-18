/*
 * Door and window placement logic.
 * Handles the automatic generation and arrangement of doors and windows
 * between rooms and corridors.
 */

use super::super::types::{
    Door, DoorSettings, DungeonLayout, Room, Window, WindowSettings, WindowSide,
};
use super::super::utils::{
    SimpleRng, corridor_cells, normalized_cell_edge, room_index_at_cell, shared_opening_width,
};
use super::connections::{room_collision_edges, room_exit_edge, shared_room_boundaries};
use crate::seed;
use std::collections::HashSet;

// Applies generated doors and archways to room-corridor transitions.
pub fn apply_doors(
    layout: &mut DungeonLayout,
    seed: u64,
    settings: DoorSettings,
    cols: usize,
    rows: usize,
) {
    layout.doors.clear();

    if layout.packed_rooms {
        apply_packed_room_doors(layout, seed, settings);
        return;
    }

    let mut rng = SimpleRng::new(seed::derive_seed(seed, 0xD005_5EED_u64));
    let base = (settings.frequency_percent.min(100) as f32) / 100.0;
    let room_hall = (settings.room_hallway_percent.min(100) as f32) / 100.0;
    let locked = (settings.locked_percent.min(100) as f32) / 100.0;

    let corridor_cells = corridor_cells(layout, cols, rows);
    let mut seen_edges = HashSet::new();
    let door_chance = base * room_hall;
    for corridor in &layout.corridors {
        if corridor.path.len() < 2 {
            continue;
        }

        let start_room = &layout.rooms[corridor.start_room_id];
        let end_room = &layout.rooms[corridor.end_room_id];

        if let Some(edge) = room_exit_edge(&corridor.path, start_room, true)
            && seen_edges.insert(edge)
        {
            let place_door = door_chance > 0.0 && rng.next_f32() <= door_chance;
            layout.doors.push(Door {
                from: edge.0,
                to: edge.1,
                width: corridor.width.max(1),
                span_width: true,
                locked: place_door && rng.next_f32() <= locked,
                archway: !place_door,
                secret: false,
                manual: false,
            });
        }

        if let Some(edge) = room_exit_edge(&corridor.path, end_room, false)
            && seen_edges.insert(edge)
        {
            let place_door = door_chance > 0.0 && rng.next_f32() <= door_chance;
            layout.doors.push(Door {
                from: edge.0,
                to: edge.1,
                width: corridor.width.max(1),
                span_width: true,
                locked: place_door && rng.next_f32() <= locked,
                archway: !place_door,
                secret: false,
                manual: false,
            });
        }

        for (room_idx, room) in layout.rooms.iter().enumerate() {
            if room_idx == corridor.start_room_id || room_idx == corridor.end_room_id {
                continue;
            }

            for edge in room_collision_edges(&corridor.path, room) {
                if seen_edges.insert(edge) {
                    let place_door = door_chance > 0.0 && rng.next_f32() <= door_chance;
                    layout.doors.push(Door {
                        from: edge.0,
                        to: edge.1,
                        width: corridor.width.max(1),
                        span_width: true,
                        locked: place_door && rng.next_f32() <= locked,
                        archway: !place_door,
                        secret: false,
                        manual: false,
                    });
                }
            }
        }
    }

    if settings.allow_middle_corridor_doors {
        for &(x, y) in &corridor_cells {
            let right = (x + 1, y);
            let bottom = (x, y + 1);
            if x + 1 < cols && corridor_cells.contains(&right) {
                let edge = normalized_cell_edge((x, y), right);
                if seen_edges.insert(edge) && base > 0.0 && rng.next_f32() <= base {
                    layout.doors.push(Door {
                        from: edge.0,
                        to: edge.1,
                        width: 1,
                        span_width: false,
                        locked: rng.next_f32() <= locked,
                        archway: false,
                        secret: false,
                        manual: false,
                    });
                }
            }
            if y + 1 < rows && corridor_cells.contains(&bottom) {
                let edge = normalized_cell_edge((x, y), bottom);
                if seen_edges.insert(edge) && base > 0.0 && rng.next_f32() <= base {
                    layout.doors.push(Door {
                        from: edge.0,
                        to: edge.1,
                        width: 1,
                        span_width: false,
                        locked: rng.next_f32() <= locked,
                        archway: false,
                        secret: false,
                        manual: false,
                    });
                }
            }
        }
    }

    apply_secret_doors(layout, seed, settings);
}

// Applies packed room doors.
pub fn apply_packed_room_doors(layout: &mut DungeonLayout, seed: u64, settings: DoorSettings) {
    let mut rng = SimpleRng::new(seed::derive_seed(seed, 0xD005_5EED_u64));
    let door_chance = ((settings.frequency_percent.min(100) as f32) / 100.0)
        * ((settings.room_hallway_percent.min(100) as f32) / 100.0);
    let locked = (settings.locked_percent.min(100) as f32) / 100.0;

    for (a_idx, b_idx, shared_edges) in shared_room_boundaries(&layout.rooms) {
        let edge_idx = if shared_edges.len() <= 1 {
            0
        } else {
            rng.range_inclusive(0, shared_edges.len() - 1)
        };
        let edge = shared_edges[edge_idx];
        let place_door = door_chance > 0.0 && rng.next_f32() <= door_chance;

        let width = shared_opening_width(
            &layout.rooms[a_idx],
            &layout.rooms[b_idx],
            shared_edges.len(),
            1,
        );
        layout.doors.push(Door {
            from: edge.0,
            to: edge.1,
            width,
            span_width: width > 1,
            locked: place_door && rng.next_f32() <= locked,
            archway: !place_door,
            secret: false,
            manual: false,
        });
    }

    apply_secret_doors(layout, seed, settings);
}

// Applies secret doors.
pub fn apply_secret_doors(layout: &mut DungeonLayout, seed: u64, settings: DoorSettings) {
    let secret_percent = settings.secret_percent.min(100);
    let chance = (secret_percent as f32) / 100.0;
    if chance <= 0.0 || layout.rooms.is_empty() || layout.doors.is_empty() {
        return;
    }

    let mut rng = SimpleRng::new(seed::derive_seed(seed, 0x5EC2_E700_u64));
    let room_door_map = room_to_door_indices(layout);
    let eligible_secret_doors = (0..layout.doors.len())
        .filter(|&door_idx| {
            let door = &layout.doors[door_idx];
            !door.archway
                && !door.locked
                && door_is_secret_eligible_for_rooms(layout, &room_door_map, door_idx)
        })
        .collect::<HashSet<_>>();

    for (door_idx, door) in layout.doors.iter_mut().enumerate() {
        if !eligible_secret_doors.contains(&door_idx) {
            continue;
        }

        if rng.next_f32() <= chance {
            door.locked = false;
            door.archway = false;
            door.secret = true;
        }
    }
}

// Computes is secret eligible for rooms.
pub fn door_is_secret_eligible_for_rooms(
    layout: &DungeonLayout,
    room_door_map: &[Vec<usize>],
    door_idx: usize,
) -> bool {
    for door_indices in room_door_map {
        if !door_indices.contains(&door_idx) {
            continue;
        }

        for &other_door_idx in door_indices {
            if other_door_idx == door_idx {
                continue;
            }

            if !door_directly_connects_rooms(layout, other_door_idx) {
                return false;
            }
        }
    }

    true
}

// Computes to door indices.
pub fn room_to_door_indices(layout: &DungeonLayout) -> Vec<Vec<usize>> {
    let mut room_door_map = vec![Vec::new(); layout.rooms.len()];
    for (door_idx, door) in layout.doors.iter().enumerate() {
        for (room_idx, room) in layout.rooms.iter().enumerate() {
            if door_touches_room(door, room) {
                room_door_map[room_idx].push(door_idx);
            }
        }
    }
    room_door_map
}

// Handles secret room ids.
pub fn secret_room_ids(layout: &DungeonLayout) -> HashSet<usize> {
    let mut secret_rooms = HashSet::new();
    for door in layout.doors.iter().filter(|door| door.secret) {
        for (room_idx, room) in layout.rooms.iter().enumerate() {
            if door_touches_room(door, room) {
                secret_rooms.insert(room_idx);
            }
        }
    }
    secret_rooms
}

// Computes touches room.
pub fn door_touches_room(door: &Door, room: &Room) -> bool {
    cell_in_room(door.from, room) || cell_in_room(door.to, room)
}

// Computes directly connects rooms.
pub fn door_directly_connects_rooms(layout: &DungeonLayout, door_idx: usize) -> bool {
    let Some(door) = layout.doors.get(door_idx) else {
        return false;
    };

    let touching_rooms = layout
        .rooms
        .iter()
        .enumerate()
        .filter_map(|(room_idx, room)| door_touches_room(door, room).then_some(room_idx))
        .collect::<Vec<_>>();

    touching_rooms.len() >= 2
}

// Builds in room.
pub fn cell_in_room(cell: (usize, usize), room: &Room) -> bool {
    cell.0 >= room.x
        && cell.0 < room.x + room.width
        && cell.1 >= room.y
        && cell.1 < room.y + room.height
}

// Applies generated windows to eligible room boundaries.
pub fn apply_windows(
    layout: &mut DungeonLayout,
    seed: u64,
    settings: WindowSettings,
    cols: usize,
    rows: usize,
) {
    layout.windows.clear();

    if !settings.enabled || cols == 0 || rows == 0 {
        return;
    }

    let mut min_width = settings.min_width.clamp(1, 5);
    let max_width = settings.max_width.clamp(min_width, 5);
    min_width = min_width.min(max_width);

    let mut rng = SimpleRng::new(seed::derive_seed(seed, 0xA117_0055_u64));
    let base = (settings.frequency_percent.min(100) as f32) / 100.0;
    let internal_ratio = (settings.room_hallway_percent.min(100) as f32) / 100.0;

    for segment in collect_window_segments(layout, cols, rows, settings.allow_internal_windows) {
        let chance = if segment.internal {
            if !settings.allow_internal_windows {
                continue;
            }
            base * internal_ratio
        } else {
            base * (1.0 - internal_ratio)
        };

        if chance <= 0.0 || rng.next_f32() > chance {
            continue;
        }

        let max_segment_width = max_width.min(segment.cells.len()).max(1);
        let width = rng.range_inclusive(min_width.min(max_segment_width), max_segment_width);
        let cell = select_segment_cell(&segment.cells, width, &mut rng);
        layout.windows.push(Window {
            cell,
            side: segment.side,
            width,
            span_width: width > 1,
        });
    }
}

#[derive(Debug, Clone)]
pub struct WindowSegment {
    pub cells: Vec<(usize, usize)>,
    pub side: WindowSide,
    pub internal: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowTarget {
    Exterior,
    Corridor,
    Room(usize),
}

// Collects window segments.
pub fn collect_window_segments(
    layout: &DungeonLayout,
    cols: usize,
    rows: usize,
    allow_internal_windows: bool,
) -> Vec<WindowSegment> {
    let corridor_cells = corridor_cells(layout, cols, rows);
    let secret_rooms = secret_room_ids(layout);
    let mut segments = Vec::new();

    for (room_idx, room) in layout.rooms.iter().enumerate() {
        if secret_rooms.contains(&room_idx) {
            continue;
        }
        collect_room_side_segments(
            room_idx,
            room,
            layout,
            &corridor_cells,
            cols,
            rows,
            allow_internal_windows,
            WindowSide::Left,
            &mut segments,
        );
        collect_room_side_segments(
            room_idx,
            room,
            layout,
            &corridor_cells,
            cols,
            rows,
            allow_internal_windows,
            WindowSide::Right,
            &mut segments,
        );
        collect_room_side_segments(
            room_idx,
            room,
            layout,
            &corridor_cells,
            cols,
            rows,
            allow_internal_windows,
            WindowSide::Top,
            &mut segments,
        );
        collect_room_side_segments(
            room_idx,
            room,
            layout,
            &corridor_cells,
            cols,
            rows,
            allow_internal_windows,
            WindowSide::Bottom,
            &mut segments,
        );
    }

    segments
}

// Collects room side segments.
#[allow(clippy::too_many_arguments)]
pub fn collect_room_side_segments(
    room_idx: usize,
    room: &Room,
    layout: &DungeonLayout,
    corridor_cells: &HashSet<(usize, usize)>,
    cols: usize,
    rows: usize,
    allow_internal_windows: bool,
    side: WindowSide,
    segments: &mut Vec<WindowSegment>,
) {
    let cells: Vec<(usize, usize)> = match side {
        WindowSide::Left => (room.y..(room.y + room.height))
            .map(|y| (room.x, y))
            .collect(),
        WindowSide::Right => (room.y..(room.y + room.height))
            .map(|y| (room.x + room.width - 1, y))
            .collect(),
        WindowSide::Top => (room.x..(room.x + room.width))
            .map(|x| (x, room.y))
            .collect(),
        WindowSide::Bottom => (room.x..(room.x + room.width))
            .map(|x| (x, room.y + room.height - 1))
            .collect(),
    };

    let mut current_target = None;
    let mut current_cells = Vec::new();

    for cell in cells {
        let Some(neighbor) = outward_neighbor(cell, side, cols, rows) else {
            push_window_segment(current_target, side, &mut current_cells, segments);
            current_target = Some(WindowTarget::Exterior);
            current_cells.push(cell);
            continue;
        };

        let target = if let Some(other_room_idx) = room_index_at_cell(&layout.rooms, neighbor) {
            if other_room_idx == room_idx || !allow_internal_windows || other_room_idx < room_idx {
                None
            } else {
                Some(WindowTarget::Room(other_room_idx))
            }
        } else if corridor_cells.contains(&neighbor) {
            allow_internal_windows.then_some(WindowTarget::Corridor)
        } else {
            Some(WindowTarget::Exterior)
        };

        if current_target != target {
            push_window_segment(current_target, side, &mut current_cells, segments);
            current_target = target;
        }

        if target.is_some() {
            current_cells.push(cell);
        }
    }

    push_window_segment(current_target, side, &mut current_cells, segments);
}

// Pushes window segment.
pub fn push_window_segment(
    target: Option<WindowTarget>,
    side: WindowSide,
    cells: &mut Vec<(usize, usize)>,
    segments: &mut Vec<WindowSegment>,
) {
    let Some(target) = target else {
        cells.clear();
        return;
    };
    if cells.is_empty() {
        return;
    }

    segments.push(WindowSegment {
        cells: std::mem::take(cells),
        side,
        internal: !matches!(target, WindowTarget::Exterior),
    });
}

// Finds the grid cell outside a room side.
pub fn outward_neighbor(
    cell: (usize, usize),
    side: WindowSide,
    cols: usize,
    rows: usize,
) -> Option<(usize, usize)> {
    match side {
        WindowSide::Left => cell.0.checked_sub(1).map(|x| (x, cell.1)),
        WindowSide::Right => (cell.0 + 1 < cols).then_some((cell.0 + 1, cell.1)),
        WindowSide::Top => cell.1.checked_sub(1).map(|y| (cell.0, y)),
        WindowSide::Bottom => (cell.1 + 1 < rows).then_some((cell.0, cell.1 + 1)),
    }
}

// Selects segment cell.
pub fn select_segment_cell(
    cells: &[(usize, usize)],
    width: usize,
    rng: &mut SimpleRng,
) -> (usize, usize) {
    let width = width.max(1);
    let min_offset = -((width as isize - 1) / 2);
    let max_offset = width as isize / 2;
    let low = (-min_offset) as usize;
    let high = cells.len().saturating_sub(1 + max_offset.max(0) as usize);
    let idx = if low <= high {
        rng.range_inclusive(low, high)
    } else {
        cells.len() / 2
    };
    cells[idx]
}
