use std::collections::{HashSet, VecDeque};

use crate::seed;

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Corridor {
    #[allow(dead_code)]
    pub id: u64,
    pub start_room_id: usize,
    pub end_room_id: usize,
    pub path: Vec<(usize, usize)>,
    pub width: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Window {
    pub cell: (usize, usize),
    pub side: WindowSide,
    pub width: usize,
    pub span_width: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextLabel {
    pub cell: (usize, usize),
    pub text: String,
    pub font_size: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaMarker {
    pub cell: (usize, usize),
    pub size: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowSide {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy)]
pub struct DoorSettings {
    pub frequency_percent: usize,
    pub room_hallway_percent: usize,
    pub locked_percent: usize,
    pub secret_percent: usize,
    pub allow_middle_corridor_doors: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct WindowSettings {
    pub enabled: bool,
    pub min_width: usize,
    pub max_width: usize,
    pub frequency_percent: usize,
    pub room_hallway_percent: usize,
    pub allow_internal_windows: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DungeonLayout {
    pub rooms: Vec<Room>,
    pub corridors: Vec<Corridor>,
    pub doors: Vec<Door>,
    pub windows: Vec<Window>,
    pub text_labels: Vec<TextLabel>,
    pub start_marker: Option<AreaMarker>,
    pub end_marker: Option<AreaMarker>,
    pub packed_rooms: bool,
}

impl Default for DungeonLayout {
    fn default() -> Self {
        Self {
            rooms: Vec::new(),
            corridors: Vec::new(),
            doors: Vec::new(),
            windows: Vec::new(),
            text_labels: Vec::new(),
            start_marker: None,
            end_marker: None,
            packed_rooms: false,
        }
    }
}

// Collect all room cells except those belonging to excluded room ids.
pub fn blocked_room_cells(rooms: &[Room], excluded_room_ids: &[usize]) -> HashSet<(usize, usize)> {
    let excluded: HashSet<usize> = excluded_room_ids.iter().copied().collect();
    let mut blocked = HashSet::new();

    for (room_idx, room) in rooms.iter().enumerate() {
        if excluded.contains(&room_idx) {
            continue;
        }

        for x in room.x..(room.x + room.width) {
            for y in room.y..(room.y + room.height) {
                blocked.insert((x, y));
            }
        }
    }

    blocked
}

// Build a layout based on settings and a derived deterministic seed.
pub fn generate_layout(
    cols: usize,
    rows: usize,
    target_room_count: usize,
    seed: u64,
    min_room_size: usize,
    max_room_size: usize,
    square_rooms_only: bool,
    min_corridor_width: usize,
    max_corridor_width: usize,
    corridor_randomness_percent: usize,
    dead_end_room_percent: usize,
    pack_rooms_without_corridors: bool,
    door_settings: DoorSettings,
    window_settings: WindowSettings,
) -> DungeonLayout {
    let layout_salt = ((cols as u64) << 48)
        ^ ((rows as u64) << 32)
        ^ ((target_room_count as u64) << 16)
        ^ (min_room_size as u64)
        ^ ((max_room_size as u64) << 8)
        ^ ((min_corridor_width as u64) << 24)
        ^ ((max_corridor_width as u64) << 28)
        ^ ((corridor_randomness_percent as u64) << 56)
        ^ ((dead_end_room_percent as u64) << 40);
    let base_seed = seed::derive_seed(seed, layout_salt);
    let mut room_rng = SimpleRng::new(seed::derive_seed(base_seed, 1));
    let mut graph_rng = SimpleRng::new(seed::derive_seed(base_seed, 2));
    let mut path_rng = SimpleRng::new(seed::derive_seed(base_seed, 3));
    let mut corridor_rng = SimpleRng::new(seed::derive_seed(base_seed, 4));

    let mut rooms = Vec::new();
    let mut corridors = Vec::new();

    if cols < 2 || rows < 2 || target_room_count == 0 {
        return DungeonLayout {
            rooms,
            corridors,
            doors: Vec::new(),
            windows: Vec::new(),
            text_labels: Vec::new(),
            start_marker: None,
            end_marker: None,
            packed_rooms: pack_rooms_without_corridors,
        };
    }

    let mut min_size = min_room_size.max(2);
    let mut max_size = max_room_size.max(min_size);

    if square_rooms_only {
        let hard_max = cols.min(rows);
        min_size = min_size.min(hard_max);
        max_size = max_size.min(hard_max);
    } else {
        min_size = min_size.min(cols.min(rows));
        max_size = max_size.min(cols.max(rows));
    }

    if min_size == 0 || max_size < min_size {
        return DungeonLayout {
            rooms,
            corridors,
            doors: Vec::new(),
            windows: Vec::new(),
            text_labels: Vec::new(),
            start_marker: None,
            end_marker: None,
            packed_rooms: pack_rooms_without_corridors,
        };
    }

    let room_sizes = generate_room_sizes(
        target_room_count,
        cols,
        rows,
        min_size,
        max_size,
        square_rooms_only,
        &mut room_rng,
    );
    let room_count = room_sizes.len();
    let centers = random_centers(room_count, cols, rows, &mut graph_rng);
    let randomness = (corridor_randomness_percent.min(100) as f32) / 100.0;
    let target_dead_end_rooms = ((room_count * dead_end_room_percent.min(50)) + 50) / 100;
    let room_edges =
        build_room_connection_edges(&centers, randomness, target_dead_end_rooms, &mut graph_rng);

    if pack_rooms_without_corridors {
        rooms = place_packed_rooms(&room_sizes, &room_edges, cols, rows, &mut room_rng);
        let mut layout = DungeonLayout {
            rooms,
            corridors,
            doors: Vec::new(),
            windows: Vec::new(),
            text_labels: Vec::new(),
            start_marker: None,
            end_marker: None,
            packed_rooms: true,
        };
        apply_doors(&mut layout, seed, door_settings, cols, rows);
        apply_windows(&mut layout, seed, window_settings, cols, rows);
        return layout;
    }

    let max_attempts = target_room_count.saturating_mul(40).max(50);

    for _ in 0..max_attempts {
        if rooms.len() >= target_room_count {
            break;
        }

        let (width, height) = if square_rooms_only {
            let side = room_rng.range_inclusive(min_size, max_size.min(cols.min(rows)));
            (side, side)
        } else {
            let width_max = max_size.min(cols);
            let height_max = max_size.min(rows);
            if min_size > width_max || min_size > height_max {
                continue;
            }
            (
                room_rng.range_inclusive(min_size, width_max),
                room_rng.range_inclusive(min_size, height_max),
            )
        };

        if width > cols || height > rows {
            continue;
        }

        let x = room_rng.range_inclusive(0, cols - width);
        let y = room_rng.range_inclusive(0, rows - height);
        let candidate = Room {
            x,
            y,
            width,
            height,
        };

        if rooms
            .iter()
            .all(|existing| !overlaps_with_padding(&candidate, existing, 1))
        {
            rooms.push(candidate);
        }
    }

    if rooms.len() < 2 {
        return DungeonLayout {
            rooms,
            corridors,
            doors: Vec::new(),
            windows: Vec::new(),
            text_labels: Vec::new(),
            start_marker: None,
            end_marker: None,
            packed_rooms: false,
        };
    }

    let mut min_width = min_corridor_width.max(1);
    let max_grid_width = cols.max(1).min(rows.max(1));
    let max_width = max_corridor_width.max(min_width).min(max_grid_width);
    min_width = min_width.min(max_width);
    let centers: Vec<(usize, usize)> = rooms.iter().map(Room::center_cell).collect();
    let target_dead_end_rooms = ((rooms.len() * dead_end_room_percent.min(50)) + 50) / 100;
    let room_edges =
        build_room_connection_edges(&centers, randomness, target_dead_end_rooms, &mut graph_rng);

    let mut next_corridor_id = 1u64;
    for (start_room_id, end_room_id) in room_edges {
        let start = centers[start_room_id];
        let end = centers[end_room_id];
        let blocked = blocked_room_cells(&rooms, &[start_room_id, end_room_id]);

        let path = if randomness <= 0.001 {
            shortest_path_cells(start, end, cols, rows, &blocked)
        } else {
            Some(noisy_path(
                start,
                end,
                cols,
                rows,
                randomness,
                &blocked,
                &mut path_rng,
            ))
        }
        .or_else(|| shortest_path_cells(start, end, cols, rows, &blocked));

        if let Some(path) = path
            && path.len() >= 2
        {
            corridors.push(Corridor {
                id: next_corridor_id,
                start_room_id,
                end_room_id,
                path,
                width: corridor_rng.range_inclusive(min_width, max_width),
            });
            next_corridor_id = next_corridor_id.wrapping_add(1);
        }
    }

    let mut layout = DungeonLayout {
        rooms,
        corridors,
        doors: Vec::new(),
        windows: Vec::new(),
        text_labels: Vec::new(),
        start_marker: None,
        end_marker: None,
        packed_rooms: false,
    };
    apply_doors(&mut layout, seed, door_settings, cols, rows);
    apply_windows(&mut layout, seed, window_settings, cols, rows);
    layout
}

// Populate layout doors based on corridor edges and door settings.
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

        if let Some(edge) = room_exit_edge(&corridor.path, start_room, true) {
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

        if let Some(edge) = room_exit_edge(&corridor.path, end_room, false) {
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

fn apply_packed_room_doors(layout: &mut DungeonLayout, seed: u64, settings: DoorSettings) {
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

fn apply_secret_doors(layout: &mut DungeonLayout, seed: u64, settings: DoorSettings) {
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

fn door_is_secret_eligible_for_rooms(
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

fn room_to_door_indices(layout: &DungeonLayout) -> Vec<Vec<usize>> {
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

fn secret_room_ids(layout: &DungeonLayout) -> HashSet<usize> {
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

fn door_touches_room(door: &Door, room: &Room) -> bool {
    cell_in_room(door.from, room) || cell_in_room(door.to, room)
}

fn door_directly_connects_rooms(layout: &DungeonLayout, door_idx: usize) -> bool {
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

fn cell_in_room(cell: (usize, usize), room: &Room) -> bool {
    cell.0 >= room.x
        && cell.0 < room.x + room.width
        && cell.1 >= room.y
        && cell.1 < room.y + room.height
}

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
struct WindowSegment {
    cells: Vec<(usize, usize)>,
    side: WindowSide,
    internal: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowTarget {
    Exterior,
    Corridor,
    Room(usize),
}

fn collect_window_segments(
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

fn collect_room_side_segments(
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

fn push_window_segment(
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

fn outward_neighbor(
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

fn select_segment_cell(
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

// Find the edge where a corridor path exits a room.
fn room_exit_edge(
    path: &[(usize, usize)],
    room: &Room,
    from_start: bool,
) -> Option<((usize, usize), (usize, usize))> {
    let in_room = |cell: (usize, usize)| {
        cell.0 >= room.x
            && cell.0 < room.x + room.width
            && cell.1 >= room.y
            && cell.1 < room.y + room.height
    };

    if from_start {
        for pair in path.windows(2) {
            if in_room(pair[0]) && !in_room(pair[1]) {
                return Some(normalized_cell_edge(pair[0], pair[1]));
            }
        }
    } else {
        for pair in path.windows(2).rev() {
            if in_room(pair[1]) && !in_room(pair[0]) {
                return Some(normalized_cell_edge(pair[0], pair[1]));
            }
        }
    }

    None
}

// Find every edge where a corridor path crosses into or out of a room.
fn room_collision_edges(
    path: &[(usize, usize)],
    room: &Room,
) -> Vec<((usize, usize), (usize, usize))> {
    let in_room = |cell: (usize, usize)| {
        cell.0 >= room.x
            && cell.0 < room.x + room.width
            && cell.1 >= room.y
            && cell.1 < room.y + room.height
    };

    let mut edges = Vec::new();
    let mut seen = HashSet::new();
    for pair in path.windows(2) {
        let a_in_room = in_room(pair[0]);
        let b_in_room = in_room(pair[1]);
        if a_in_room == b_in_room {
            continue;
        }

        let edge = normalized_cell_edge(pair[0], pair[1]);
        if seen.insert(edge) {
            edges.push(edge);
        }
    }

    edges
}

fn shared_room_boundaries(
    rooms: &[Room],
) -> Vec<(usize, usize, Vec<((usize, usize), (usize, usize))>)> {
    let mut boundaries = Vec::new();
    for a_idx in 0..rooms.len() {
        for b_idx in (a_idx + 1)..rooms.len() {
            let shared_edges = shared_boundary_edges(&rooms[a_idx], &rooms[b_idx]);
            if !shared_edges.is_empty() {
                boundaries.push((a_idx, b_idx, shared_edges));
            }
        }
    }
    boundaries
}

fn shared_boundary_edges(a: &Room, b: &Room) -> Vec<((usize, usize), (usize, usize))> {
    let mut edges = Vec::new();

    if a.x + a.width == b.x || b.x + b.width == a.x {
        let left = if a.x < b.x { a } else { b };
        let right = if a.x < b.x { b } else { a };
        let y0 = left.y.max(right.y);
        let y1 = (left.y + left.height).min(right.y + right.height);
        for y in y0..y1 {
            edges.push(normalized_cell_edge(
                (left.x + left.width - 1, y),
                (right.x, y),
            ));
        }
    }

    if a.y + a.height == b.y || b.y + b.height == a.y {
        let top = if a.y < b.y { a } else { b };
        let bottom = if a.y < b.y { b } else { a };
        let x0 = top.x.max(bottom.x);
        let x1 = (top.x + top.width).min(bottom.x + bottom.width);
        for x in x0..x1 {
            edges.push(normalized_cell_edge(
                (x, top.y + top.height - 1),
                (x, bottom.y),
            ));
        }
    }

    edges
}

fn room_index_at_cell(rooms: &[Room], cell: (usize, usize)) -> Option<usize> {
    rooms.iter().position(|room| {
        cell.0 >= room.x
            && cell.0 < room.x + room.width
            && cell.1 >= room.y
            && cell.1 < room.y + room.height
    })
}

// Compute corridor cells while excluding room cells.
pub fn corridor_cells(layout: &DungeonLayout, cols: usize, rows: usize) -> HashSet<(usize, usize)> {
    let mut cells = HashSet::new();
    if cols == 0 || rows == 0 {
        return cells;
    }

    let mut room_cells = HashSet::new();
    for room in &layout.rooms {
        for x in room.x..(room.x + room.width) {
            for y in room.y..(room.y + room.height) {
                room_cells.insert((x, y));
            }
        }
    }

    for corridor in &layout.corridors {
        let width = corridor.width.max(1);
        let min_offset = -((width as isize - 1) / 2);
        let max_offset = width as isize / 2;
        if corridor.path.len() == 1 {
            let (x, y) = corridor.path[0];
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
            continue;
        }

        for pair in corridor.path.windows(2) {
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

        for turn in corridor.path.windows(3) {
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
    }

    cells
}

fn generate_room_sizes(
    target_room_count: usize,
    cols: usize,
    rows: usize,
    min_size: usize,
    max_size: usize,
    square_rooms_only: bool,
    rng: &mut SimpleRng,
) -> Vec<(usize, usize)> {
    let mut sizes = Vec::new();
    let max_attempts = target_room_count.saturating_mul(40).max(50);

    for _ in 0..max_attempts {
        if sizes.len() >= target_room_count {
            break;
        }

        let (width, height) = if square_rooms_only {
            let side = rng.range_inclusive(min_size, max_size.min(cols.min(rows)));
            (side, side)
        } else {
            let width_max = max_size.min(cols);
            let height_max = max_size.min(rows);
            if min_size > width_max || min_size > height_max {
                continue;
            }
            (
                rng.range_inclusive(min_size, width_max),
                rng.range_inclusive(min_size, height_max),
            )
        };

        if width <= cols && height <= rows {
            sizes.push((width, height));
        }
    }

    sizes
}

fn random_centers(
    count: usize,
    cols: usize,
    rows: usize,
    rng: &mut SimpleRng,
) -> Vec<(usize, usize)> {
    let mut centers = Vec::with_capacity(count);
    for _ in 0..count {
        centers.push((
            rng.range_inclusive(0, cols.saturating_sub(1)),
            rng.range_inclusive(0, rows.saturating_sub(1)),
        ));
    }
    centers
}

fn place_packed_rooms(
    room_sizes: &[(usize, usize)],
    room_edges: &[(usize, usize)],
    cols: usize,
    rows: usize,
    rng: &mut SimpleRng,
) -> Vec<Room> {
    if room_sizes.is_empty() || cols == 0 || rows == 0 {
        return Vec::new();
    }

    let mut placed: Vec<Option<Room>> = vec![None; room_sizes.len()];
    let (first_w, first_h) = room_sizes[0];
    if first_w > cols || first_h > rows {
        return Vec::new();
    }

    placed[0] = Some(Room {
        x: (cols.saturating_sub(first_w)) / 2,
        y: (rows.saturating_sub(first_h)) / 2,
        width: first_w,
        height: first_h,
    });

    let mut order = placement_order(room_sizes.len(), room_edges);
    if !order.contains(&0) {
        order.insert(0, 0);
    }

    for room_idx in order.into_iter().skip(1) {
        let Some(room) =
            try_place_packed_room(room_idx, room_sizes, room_edges, &placed, cols, rows, rng)
        else {
            continue;
        };
        placed[room_idx] = Some(room);
    }

    for room_idx in 0..room_sizes.len() {
        if placed[room_idx].is_some() {
            continue;
        }
        if let Some(room) =
            try_place_packed_room(room_idx, room_sizes, room_edges, &placed, cols, rows, rng)
        {
            placed[room_idx] = Some(room);
        }
    }

    placed.into_iter().flatten().collect()
}

fn placement_order(room_count: usize, room_edges: &[(usize, usize)]) -> Vec<usize> {
    if room_count == 0 {
        return Vec::new();
    }

    let mut adjacency = vec![Vec::new(); room_count];
    for &(a, b) in room_edges {
        adjacency[a].push(b);
        adjacency[b].push(a);
    }

    let mut visited = vec![false; room_count];
    let mut queue = VecDeque::new();
    let mut order = Vec::with_capacity(room_count);
    queue.push_back(0);
    visited[0] = true;

    while let Some(idx) = queue.pop_front() {
        order.push(idx);
        for &next in &adjacency[idx] {
            if !visited[next] {
                visited[next] = true;
                queue.push_back(next);
            }
        }
    }

    for idx in 0..room_count {
        if !visited[idx] {
            order.push(idx);
        }
    }

    order
}

fn try_place_packed_room(
    room_idx: usize,
    room_sizes: &[(usize, usize)],
    room_edges: &[(usize, usize)],
    placed: &[Option<Room>],
    cols: usize,
    rows: usize,
    rng: &mut SimpleRng,
) -> Option<Room> {
    let (width, height) = *room_sizes.get(room_idx)?;
    if width > cols || height > rows {
        return None;
    }

    let mut anchors: Vec<usize> = room_edges
        .iter()
        .filter_map(|&(a, b)| {
            if a == room_idx && placed.get(b)?.is_some() {
                Some(b)
            } else if b == room_idx && placed.get(a)?.is_some() {
                Some(a)
            } else {
                None
            }
        })
        .collect();

    if anchors.is_empty() {
        anchors = placed
            .iter()
            .enumerate()
            .filter_map(|(idx, room)| room.as_ref().map(|_| idx))
            .collect();
    }

    shuffle_indices(&mut anchors, rng);

    for anchor_idx in anchors {
        let Some(anchor) = placed.get(anchor_idx).and_then(|room| room.as_ref()) else {
            continue;
        };
        let mut candidates = packed_room_candidates(anchor, width, height, cols, rows);
        shuffle_rooms(&mut candidates, rng);

        for candidate in candidates {
            if placed
                .iter()
                .flatten()
                .all(|existing| !rooms_overlap(&candidate, existing))
                && placed
                    .iter()
                    .flatten()
                    .any(|existing| rooms_touch(&candidate, existing))
            {
                return Some(candidate);
            }
        }
    }

    None
}

fn packed_room_candidates(
    anchor: &Room,
    width: usize,
    height: usize,
    cols: usize,
    rows: usize,
) -> Vec<Room> {
    let mut candidates = Vec::new();

    let min_y = anchor.y.saturating_sub(height.saturating_sub(1));
    let max_y = (anchor.y + anchor.height).saturating_sub(1);
    for y in min_y..=max_y {
        candidates.push(Room {
            x: anchor.x.saturating_sub(width),
            y,
            width,
            height,
        });
        candidates.push(Room {
            x: anchor.x + anchor.width,
            y,
            width,
            height,
        });
    }

    let min_x = anchor.x.saturating_sub(width.saturating_sub(1));
    let max_x = (anchor.x + anchor.width).saturating_sub(1);
    for x in min_x..=max_x {
        candidates.push(Room {
            x,
            y: anchor.y.saturating_sub(height),
            width,
            height,
        });
        candidates.push(Room {
            x,
            y: anchor.y + anchor.height,
            width,
            height,
        });
    }

    candidates.retain(|room| room.x + room.width <= cols && room.y + room.height <= rows);
    candidates
}

fn shuffle_rooms(rooms: &mut [Room], rng: &mut SimpleRng) {
    if rooms.len() <= 1 {
        return;
    }
    for i in (1..rooms.len()).rev() {
        let j = rng.range_inclusive(0, i);
        rooms.swap(i, j);
    }
}

fn rooms_overlap(a: &Room, b: &Room) -> bool {
    let a_right = a.x + a.width;
    let a_bottom = a.y + a.height;
    let b_right = b.x + b.width;
    let b_bottom = b.y + b.height;

    a.x < b_right && a_right > b.x && a.y < b_bottom && a_bottom > b.y
}

fn rooms_touch(a: &Room, b: &Room) -> bool {
    !shared_boundary_edges(a, b).is_empty()
}

fn shared_opening_width(a: &Room, b: &Room, span: usize, default_width: usize) -> usize {
    let max_width = if a.x + a.width == b.x || b.x + b.width == a.x {
        a.height.min(b.height)
    } else {
        a.width.min(b.width)
    };
    default_width.max(1).min(span).min(max_width.max(1))
}

// Create a connected graph of room-to-room edges with optional dead ends.
fn build_room_connection_edges(
    centers: &[(usize, usize)],
    randomness: f32,
    target_dead_end_rooms: usize,
    rng: &mut SimpleRng,
) -> Vec<(usize, usize)> {
    if centers.len() < 2 {
        return Vec::new();
    }

    let room_count = centers.len();
    let max_dead_ends = room_count / 2;
    let desired_dead_ends = target_dead_end_rooms.min(max_dead_ends);
    let core_count = (room_count - desired_dead_ends).max(1);

    let mut room_indices: Vec<usize> = (0..room_count).collect();
    shuffle_indices(&mut room_indices, rng);

    let mut core_rooms = room_indices[..core_count].to_vec();
    let leaf_rooms = room_indices[core_count..].to_vec();
    core_rooms = ordered_core_rooms(&core_rooms, centers, randomness, rng);

    let mut edges = Vec::new();
    let mut edge_set = HashSet::new();

    if core_rooms.len() >= 2 {
        for pair in core_rooms.windows(2) {
            push_unique_room_edge(pair[0], pair[1], &mut edges, &mut edge_set);
        }

        if core_rooms.len() >= 3 {
            push_unique_room_edge(
                core_rooms[core_rooms.len() - 1],
                core_rooms[0],
                &mut edges,
                &mut edge_set,
            );
        }
    }

    for leaf in leaf_rooms {
        let mut best_anchor = core_rooms[0];
        let mut best_score = f32::INFINITY;

        for &core in &core_rooms {
            let dist = manhattan_distance(centers[leaf], centers[core]) as f32;
            let score = (dist * (1.0 - 0.8 * randomness)) + (rng.next_f32() * 30.0 * randomness);
            if score < best_score {
                best_score = score;
                best_anchor = core;
            }
        }

        push_unique_room_edge(leaf, best_anchor, &mut edges, &mut edge_set);
    }

    edges
}

// Order core rooms to create a reasonable loop backbone.
fn ordered_core_rooms(
    core_rooms: &[usize],
    centers: &[(usize, usize)],
    randomness: f32,
    rng: &mut SimpleRng,
) -> Vec<usize> {
    if core_rooms.len() <= 2 {
        return core_rooms.to_vec();
    }

    let mut remaining = core_rooms.to_vec();
    let start_idx = rng.range_inclusive(0, remaining.len() - 1);
    let mut ordered = vec![remaining.swap_remove(start_idx)];

    while !remaining.is_empty() {
        let last = *ordered.last().unwrap_or(&remaining[0]);
        let mut best_idx = 0usize;
        let mut best_score = f32::INFINITY;

        for (idx, candidate) in remaining.iter().enumerate() {
            let dist = manhattan_distance(centers[last], centers[*candidate]) as f32;
            let score = (dist * (1.0 - 0.85 * randomness)) + (rng.next_f32() * 20.0 * randomness);
            if score < best_score {
                best_score = score;
                best_idx = idx;
            }
        }

        ordered.push(remaining.swap_remove(best_idx));
    }

    ordered
}

// Insert a room edge only if it has not been added yet.
fn push_unique_room_edge(
    a: usize,
    b: usize,
    edges: &mut Vec<(usize, usize)>,
    edge_set: &mut HashSet<(usize, usize)>,
) {
    if a == b {
        return;
    }
    let normalized = if a < b { (a, b) } else { (b, a) };
    if edge_set.insert(normalized) {
        edges.push((a, b));
    }
}

// Shuffle indices in place using the provided RNG.
fn shuffle_indices(indices: &mut [usize], rng: &mut SimpleRng) {
    if indices.len() <= 1 {
        return;
    }
    for i in (1..indices.len()).rev() {
        let j = rng.range_inclusive(0, i);
        indices.swap(i, j);
    }
}

// Compute the shortest grid path between two cells using BFS.
pub fn shortest_path_cells(
    start: (usize, usize),
    end: (usize, usize),
    cols: usize,
    rows: usize,
    blocked: &HashSet<(usize, usize)>,
) -> Option<Vec<(usize, usize)>> {
    if start == end {
        return Some(vec![start]);
    }
    if blocked.contains(&start) || blocked.contains(&end) {
        return None;
    }

    let total = cols.saturating_mul(rows);
    if total == 0 {
        return None;
    }

    let index = |p: (usize, usize)| -> usize { p.1 * cols + p.0 };
    let coord = |idx: usize| -> (usize, usize) { (idx % cols, idx / cols) };

    let start_idx = index(start);
    let end_idx = index(end);

    let mut queue = VecDeque::new();
    let mut visited = vec![false; total];
    let mut parent: Vec<Option<usize>> = vec![None; total];

    visited[start_idx] = true;
    queue.push_back(start_idx);

    while let Some(current) = queue.pop_front() {
        if current == end_idx {
            break;
        }

        let (x, y) = coord(current);
        let neighbors = [
            x.checked_sub(1).map(|nx| (nx, y)),
            (x + 1 < cols).then_some((x + 1, y)),
            y.checked_sub(1).map(|ny| (x, ny)),
            (y + 1 < rows).then_some((x, y + 1)),
        ];

        for neighbor in neighbors.into_iter().flatten() {
            if blocked.contains(&neighbor) {
                continue;
            }
            let n_idx = index(neighbor);
            if !visited[n_idx] {
                visited[n_idx] = true;
                parent[n_idx] = Some(current);
                queue.push_back(n_idx);
            }
        }
    }

    if !visited[end_idx] {
        return None;
    }

    let mut path = Vec::new();
    let mut current = end_idx;
    path.push(coord(current));

    while let Some(prev) = parent[current] {
        current = prev;
        path.push(coord(current));
    }

    path.reverse();
    Some(path)
}

// Generate a noisy path biased toward the target cell.
fn noisy_path(
    start: (usize, usize),
    end: (usize, usize),
    cols: usize,
    rows: usize,
    randomness: f32,
    blocked: &HashSet<(usize, usize)>,
    rng: &mut SimpleRng,
) -> Vec<(usize, usize)> {
    if start == end {
        return vec![start];
    }

    let mut path = vec![start];
    let mut visited = HashSet::new();
    visited.insert(start);

    let mut current = start;
    let mut prev_dir = (0isize, 0isize);
    let max_steps = cols.saturating_mul(rows).max(32);

    for _ in 0..max_steps {
        if current == end {
            break;
        }

        let mut neighbors = Vec::with_capacity(4);
        let (x, y) = current;
        if x > 0 {
            neighbors.push((x - 1, y));
        }
        if x + 1 < cols {
            neighbors.push((x + 1, y));
        }
        if y > 0 {
            neighbors.push((x, y - 1));
        }
        if y + 1 < rows {
            neighbors.push((x, y + 1));
        }

        if neighbors.is_empty() {
            break;
        }

        let mut best = neighbors[0];
        let mut best_score = f32::INFINITY;

        for &candidate in &neighbors {
            if blocked.contains(&candidate) && candidate != end {
                continue;
            }

            let step_dir = (
                candidate.0 as isize - current.0 as isize,
                candidate.1 as isize - current.1 as isize,
            );

            let dist = manhattan_distance(candidate, end) as f32;
            let progress_weight = 1.0 - (0.85 * randomness);
            let revisit_penalty = if visited.contains(&candidate) {
                2.5 + (2.0 * randomness)
            } else {
                0.0
            };
            let turn_penalty = if prev_dir == (0, 0) || prev_dir == step_dir {
                0.0
            } else {
                0.6 - (0.35 * randomness)
            };
            let noise = rng.next_f32() * 8.0 * randomness;
            let score = (dist * progress_weight) + revisit_penalty + turn_penalty + noise;

            if score < best_score {
                best_score = score;
                best = candidate;
            }
        }

        prev_dir = (
            best.0 as isize - current.0 as isize,
            best.1 as isize - current.1 as isize,
        );
        current = best;
        path.push(current);
        visited.insert(current);
    }

    if current != end
        && let Some(tail) = shortest_path_cells(current, end, cols, rows, blocked)
    {
        for &cell in tail.iter().skip(1) {
            path.push(cell);
        }
    }

    path
}

// Test whether two rooms overlap with extra padding.
fn overlaps_with_padding(a: &Room, b: &Room, padding: usize) -> bool {
    let a_left = a.x.saturating_sub(padding);
    let a_top = a.y.saturating_sub(padding);
    let a_right = a.x + a.width + padding;
    let a_bottom = a.y + a.height + padding;

    let b_left = b.x;
    let b_top = b.y;
    let b_right = b.x + b.width;
    let b_bottom = b.y + b.height;

    a_left < b_right && a_right > b_left && a_top < b_bottom && a_bottom > b_top
}

// Compute Manhattan distance between two grid cells.
fn manhattan_distance(a: (usize, usize), b: (usize, usize)) -> usize {
    a.0.abs_diff(b.0) + a.1.abs_diff(b.1)
}

// Normalize a cell edge ordering.
fn normalized_cell_edge(a: (usize, usize), b: (usize, usize)) -> ((usize, usize), (usize, usize)) {
    if a <= b { (a, b) } else { (b, a) }
}

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    // Create a small deterministic RNG with a fallback seed.
    fn new(seed: u64) -> Self {
        let state = if seed == 0 {
            0xA5A5_A5A5_1234_5678
        } else {
            seed
        };
        Self { state }
    }

    // Return the next random u32.
    fn next_u32(&mut self) -> u32 {
        self.state ^= self.state >> 12;
        self.state ^= self.state << 25;
        self.state ^= self.state >> 27;
        (self.state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
    }

    // Return the next random f32 in [0,1].
    fn next_f32(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }

    // Generate a random usize between min and max inclusive.
    fn range_inclusive(&mut self, min: usize, max: usize) -> usize {
        if min >= max {
            return min;
        }
        let width = max - min + 1;
        min + (self.next_u32() as usize % width)
    }
}
