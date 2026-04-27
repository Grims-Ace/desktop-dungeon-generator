use crate::layout::{self, DungeonLayout};
use crate::seed;
use crate::ui::UiSettings;
use std::collections::HashSet;

// Import room_index_at_cell from layout module.
use crate::layout::room_index_at_cell;

const START_COUNT_STREAM: u64 = 10_001;
const END_COUNT_STREAM: u64 = 10_002;
const START_ROOM_STREAM_BASE: u64 = 11_000;
const END_ROOM_STREAM_BASE: u64 = 12_000;
const EXTRA_START_ROOM_STREAM_BASE: u64 = 13_000;
const EXTRA_START_MARKER_STREAM_BASE: u64 = 14_000;
const EXTRA_END_ROOM_STREAM_BASE: u64 = 15_000;
const EXTRA_END_MARKER_STREAM_BASE: u64 = 16_000;
const TRAP_ROOM_STREAM_BASE: u64 = 17_000;
const TRAP_CORRIDOR_STREAM_BASE: u64 = 18_000;
const MONSTER_ROOM_STREAM_BASE: u64 = 19_000;
const MONSTER_CORRIDOR_STREAM_BASE: u64 = 20_000;

// Populate generated start/end markers after the core dungeon layout exists.
pub fn populate_random_markers(mut layout: DungeonLayout, settings: &UiSettings) -> DungeonLayout {
    layout.start_markers.clear();
    layout.end_markers.clear();
    layout.trap_markers.clear();
    layout.monster_markers.clear();

    if layout.rooms.is_empty() {
        return layout;
    }

    let start_count = random_range_inclusive(
        settings.min_start_marker_count,
        settings.max_start_marker_count,
        seed::derive_seed(settings.seed, START_COUNT_STREAM),
    );
    let end_count = random_range_inclusive(
        settings.min_end_marker_count,
        settings.max_end_marker_count,
        seed::derive_seed(settings.seed, END_COUNT_STREAM),
    );
    let pair_count = start_count.min(end_count);

    let mut available_start_rooms: Vec<usize> = (0..layout.rooms.len()).collect();
    let mut available_end_rooms: Vec<usize> = (0..layout.rooms.len()).collect();

    for pair_idx in 0..pair_count {
        let (start_room_idx, end_room_idx) =
            farthest_room_pair(&layout.rooms, &available_start_rooms, &available_end_rooms);
        let start_room = &layout.rooms[start_room_idx];
        let end_room = &layout.rooms[end_room_idx];
        layout.start_markers.push(marker_in_room(
            start_room,
            settings.min_start_marker_size,
            settings.max_start_marker_size,
            seed::derive_seed(settings.seed, START_ROOM_STREAM_BASE + pair_idx as u64),
        ));
        layout.end_markers.push(marker_in_room(
            end_room,
            settings.min_end_marker_size,
            settings.max_end_marker_size,
            seed::derive_seed(settings.seed, END_ROOM_STREAM_BASE + pair_idx as u64),
        ));
        consume_room(&mut available_start_rooms, start_room_idx);
        consume_room(&mut available_end_rooms, end_room_idx);
    }

    assign_extra_markers(
        &mut layout.start_markers,
        &layout.rooms,
        &mut available_start_rooms,
        pair_count,
        start_count,
        settings.min_start_marker_size,
        settings.max_start_marker_size,
        settings.seed,
        EXTRA_START_ROOM_STREAM_BASE,
        EXTRA_START_MARKER_STREAM_BASE,
    );
    assign_extra_markers(
        &mut layout.end_markers,
        &layout.rooms,
        &mut available_end_rooms,
        pair_count,
        end_count,
        settings.min_end_marker_size,
        settings.max_end_marker_size,
        settings.seed,
        EXTRA_END_ROOM_STREAM_BASE,
        EXTRA_END_MARKER_STREAM_BASE,
    );

    layout = populate_random_traps(layout, settings);
    layout = populate_random_monsters(layout, settings);

    layout
}

// Manual marker placement uses the configured minimum size for the chosen marker type.
pub fn manual_marker_size(settings: &UiSettings, is_start: bool) -> usize {
    if is_start {
        settings
            .min_start_marker_size
            .min(settings.max_start_marker_size)
    } else {
        settings
            .min_end_marker_size
            .min(settings.max_end_marker_size)
    }
    .clamp(1, 10)
}

pub fn manual_trap_marker_size(_settings: &UiSettings) -> usize {
    1
}

pub fn manual_monster_marker_size(_settings: &UiSettings) -> usize {
    1
}

// Pick a deterministic inclusive random value from a seed-derived stream.
fn random_range_inclusive(min: usize, max: usize, seed_value: u64) -> usize {
    let min = min.max(1);
    let max = max.max(min);
    let span = max - min + 1;
    min + (seed_value as usize % span)
}

// Fit a marker inside a room and choose a deterministic offset within that room.
fn marker_in_room(
    room: &layout::Room,
    min_size: usize,
    max_size: usize,
    seed_value: u64,
) -> layout::AreaMarker {
    let room_limit = room.width.min(room.height).max(1);
    let min_size = min_size.clamp(1, 10).min(room_limit);
    let max_size = max_size.clamp(min_size, 10).min(room_limit);
    let size = random_range_inclusive(min_size, max_size, seed_value.rotate_left(7));
    let x_span = room.width.saturating_sub(size);
    let y_span = room.height.saturating_sub(size);
    let x = room.x
        + if x_span == 0 {
            0
        } else {
            (seed_value as usize) % (x_span + 1)
        };
    let y = room.y
        + if y_span == 0 {
            0
        } else {
            (seed_value.rotate_left(19) as usize) % (y_span + 1)
        };
    layout::AreaMarker { cell: (x, y), size }
}

fn assign_extra_markers(
    markers: &mut Vec<layout::AreaMarker>,
    rooms: &[layout::Room],
    available_rooms: &mut Vec<usize>,
    start_idx: usize,
    end_idx: usize,
    min_size: usize,
    max_size: usize,
    seed_base: u64,
    room_stream_base: u64,
    marker_stream_base: u64,
) {
    for idx in start_idx..end_idx {
        let room_idx = pick_room_index(
            available_rooms,
            rooms.len(),
            seed::derive_seed(seed_base, room_stream_base + idx as u64),
        );
        markers.push(marker_in_room(
            &rooms[room_idx],
            min_size,
            max_size,
            seed::derive_seed(seed_base, marker_stream_base + idx as u64),
        ));
        consume_room(available_rooms, room_idx);
    }
}

// Match each start/end pair to the farthest available room combination.
fn farthest_room_pair(
    rooms: &[layout::Room],
    available_start_rooms: &[usize],
    available_end_rooms: &[usize],
) -> (usize, usize) {
    let start_rooms = if available_start_rooms.is_empty() {
        (0..rooms.len()).collect::<Vec<_>>()
    } else {
        available_start_rooms.to_vec()
    };
    let end_rooms = if available_end_rooms.is_empty() {
        (0..rooms.len()).collect::<Vec<_>>()
    } else {
        available_end_rooms.to_vec()
    };

    let mut best = (start_rooms[0], end_rooms[0]);
    let mut best_dist = 0usize;
    for &start_idx in &start_rooms {
        for &end_idx in &end_rooms {
            if rooms.len() > 1 && start_idx == end_idx {
                continue;
            }
            let dist = room_distance_sq(&rooms[start_idx], &rooms[end_idx]);
            if dist > best_dist {
                best = (start_idx, end_idx);
                best_dist = dist;
            }
        }
    }
    best
}

fn room_distance_sq(a: &layout::Room, b: &layout::Room) -> usize {
    let ac = a.center_cell();
    let bc = b.center_cell();
    let dx = ac.0.abs_diff(bc.0);
    let dy = ac.1.abs_diff(bc.1);
    dx * dx + dy * dy
}

// Remove a room from the available set once it has been consumed by a marker assignment.
fn consume_room(available_rooms: &mut Vec<usize>, room_idx: usize) {
    if let Some(pos) = available_rooms.iter().position(|&idx| idx == room_idx) {
        available_rooms.remove(pos);
    }
}

// Fall back to any room when all candidates for a side have been exhausted.
fn pick_room_index(available_rooms: &[usize], room_count: usize, seed_value: u64) -> usize {
    if !available_rooms.is_empty() {
        available_rooms[seed_value as usize % available_rooms.len()]
    } else {
        seed_value as usize % room_count.max(1)
    }
}

pub fn populate_random_traps(mut layout: DungeonLayout, settings: &UiSettings) -> DungeonLayout {
    if settings.trap_frequency_percent == 0 {
        return layout;
    }

    let mut trap_markers = Vec::new();

    // Rooms
    for (idx, room) in layout.rooms.iter().enumerate() {
        let seed = seed::derive_seed(settings.seed, TRAP_ROOM_STREAM_BASE + idx as u64);
        if (seed % 100) < settings.trap_frequency_percent as u64 {
            let count = random_range_inclusive(
                settings.min_traps_per_area,
                settings.max_traps_per_area,
                seed.rotate_left(13),
            );
            for i in 0..count {
                trap_markers.push(marker_in_room(
                    room,
                    1,
                    1,
                    seed.wrapping_add(i as u64).rotate_right(7),
                ));
            }
        }
    }

    // Corridors
    for (idx, corridor) in layout.corridors.iter().enumerate() {
        let seed = seed::derive_seed(settings.seed, TRAP_CORRIDOR_STREAM_BASE + idx as u64);
        if (seed % 100) < settings.trap_frequency_percent as u64 {
            let count = random_range_inclusive(
                settings.min_traps_per_area,
                settings.max_traps_per_area,
                seed.rotate_left(13),
            );
            for i in 0..count {
                if let Some(cell) = pick_random_corridor_cell(corridor, seed.wrapping_add(i as u64))
                {
                    trap_markers.push(layout::AreaMarker { cell, size: 1 });
                }
            }
        }
    }

    layout.trap_markers = trap_markers;
    layout
}

pub fn populate_random_monsters(mut layout: DungeonLayout, settings: &UiSettings) -> DungeonLayout {
    if settings.monster_frequency_percent == 0 {
        return layout;
    }

    let mut monster_markers = Vec::new();

    // Rooms
    for (idx, room) in layout.rooms.iter().enumerate() {
        let seed = seed::derive_seed(settings.seed, MONSTER_ROOM_STREAM_BASE + idx as u64);
        if (seed % 100) < settings.monster_frequency_percent as u64 {
            let count = random_range_inclusive(
                settings.min_monsters_per_area,
                settings.max_monsters_per_area,
                seed.rotate_left(13),
            );
            for i in 0..count {
                monster_markers.push(marker_in_room(
                    room,
                    1,
                    1,
                    seed.wrapping_add(i as u64).rotate_right(7),
                ));
            }
        }
    }

    // Corridors
    for (idx, corridor) in layout.corridors.iter().enumerate() {
        let seed = seed::derive_seed(settings.seed, MONSTER_CORRIDOR_STREAM_BASE + idx as u64);
        if (seed % 100) < settings.monster_frequency_percent as u64 {
            let count = random_range_inclusive(
                settings.min_monsters_per_area,
                settings.max_monsters_per_area,
                seed.rotate_left(13),
            );
            for i in 0..count {
                if let Some(cell) = pick_random_corridor_cell(corridor, seed.wrapping_add(i as u64))
                {
                    monster_markers.push(layout::AreaMarker { cell, size: 1 });
                }
            }
        }
    }

    layout.monster_markers = monster_markers;
    layout
}

fn pick_random_corridor_cell(
    corridor: &layout::Corridor,
    seed_value: u64,
) -> Option<(usize, usize)> {
    if corridor.path.is_empty() {
        return None;
    }
    let idx = (seed_value as usize) % corridor.path.len();
    Some(corridor.path[idx])
}

// Constant for stair marker stream base
const STAIR_MARKER_STREAM_BASE: u64 = 21_000;

// Populate stairs (and optionally elevators) across all levels after markers are placed.
pub fn populate_stairs(
    mut layouts: Vec<DungeonLayout>,
    settings: &UiSettings,
) -> Vec<DungeonLayout> {
    if settings.min_stairs_per_level == 0 && settings.max_stairs_per_level == 0 {
        return layouts;
    }

    let stair_count = if settings.min_stairs_per_level == settings.max_stairs_per_level {
        settings.min_stairs_per_level
    } else {
        let range_seed = seed::derive_seed(settings.seed, 0x2A_3B_4_u64);
        (range_seed as usize % (settings.max_stairs_per_level - settings.min_stairs_per_level + 1))
            + settings.min_stairs_per_level
    };

    if stair_count == 0 {
        return layouts;
    }

    let is_elevator = settings.windows_enabled;

    // If syncing stairs, pick the same stair positions for all levels.
    let mut sync_stair_cells: Vec<layout::Staircase> = Vec::new();
    if settings.sync_stairs_across_levels && !layouts.is_empty() {
        let base_settings = settings.clone();
        let base_layout = layouts[0].clone();
        sync_stair_cells =
            pick_stair_positions(&base_layout, &base_settings, stair_count, is_elevator);
    }

    for (level_idx, layout) in layouts.iter_mut().enumerate() {
        if settings.sync_stairs_across_levels && !sync_stair_cells.is_empty() {
            // Use the synced stair positions.
            layout.stairs = sync_stair_cells
                .iter()
                .map(|stair| layout::Staircase {
                    cell: stair.cell,
                    size: random_stair_size(
                        settings,
                        seed::derive_seed(
                            settings.seed,
                            STAIR_MARKER_STREAM_BASE + level_idx as u64,
                        ),
                    ),
                    is_elevator,
                })
                .collect();
        } else {
            // Generate independent stair positions for this level.
            layout.stairs = pick_stair_positions(layout, settings, stair_count, is_elevator);
        }
    }

    layouts
}

// Pick random cells within rooms that are valid for stair placement and return Staircase objects.
fn pick_stair_positions(
    layout: &DungeonLayout,
    settings: &UiSettings,
    count: usize,
    is_elevator: bool,
) -> Vec<layout::Staircase> {
    if layout.rooms.is_empty() || count == 0 {
        return Vec::new();
    }

    // Collect valid room cells (rooms without start/end markers on bottom row).
    let mut valid_cells: Vec<(usize, usize)> = Vec::new();
    for room in &layout.rooms {
        let has_start_end_bottom =
            has_start_or_end_on_bottom_row(room, &layout.start_markers, &layout.end_markers);
        if !has_start_end_bottom {
            for x in room.x..(room.x + room.width) {
                for y in room.y..(room.y + room.height) {
                    valid_cells.push((x, y));
                }
            }
        }
    }

    if valid_cells.is_empty() {
        return Vec::new();
    }

    // Shuffle valid cells deterministically based on seed.
    let mut shuffled = valid_cells.clone();
    shuffle_with_seed(
        &mut shuffled,
        settings.seed.wrapping_add(STAIR_MARKER_STREAM_BASE),
    );

    let max_size = settings.max_stair_width.max(settings.max_stair_height);
    let mut stair_cells: Vec<(usize, usize)> = Vec::new();
    let mut used_cells: HashSet<(usize, usize)> = HashSet::new();

    for cell in shuffled {
        if stair_cells.len() >= count {
            break;
        }
        if used_cells.contains(&cell) {
            continue;
        }
        // Check if this cell can accommodate a stair of at least min size.
        let min_w = settings.min_stair_width;
        let min_h = settings.min_stair_height;
        let room_idx = room_index_at_cell(&layout.rooms, cell);
        if let Some(ri) = room_idx {
            let room = &layout.rooms[ri];
            let available_w = room.x + room.width - cell.0;
            let available_h = room.y + room.height - cell.1;
            if available_w >= min_w && available_h >= min_h {
                stair_cells.push(cell);
                // Mark occupied cells to avoid overlap.
                for dx in 0..max_size {
                    for dy in 0..max_size {
                        used_cells.insert((cell.0 + dx, cell.1 + dy));
                    }
                }
            }
        }
    }

    // Convert cells to Staircase objects with random sizes.
    stair_cells
        .into_iter()
        .enumerate()
        .map(|(i, cell)| layout::Staircase {
            cell,
            size: random_stair_size(
                settings,
                seed::derive_seed(settings.seed, STAIR_MARKER_STREAM_BASE + i as u64),
            ),
            is_elevator,
        })
        .collect()
}

// Check if a room has a start or end marker on its bottom row.
fn has_start_or_end_on_bottom_row(
    room: &layout::Room,
    start_markers: &[layout::AreaMarker],
    end_markers: &[layout::AreaMarker],
) -> bool {
    let bottom_row = room.y + room.height - 1;
    for marker in start_markers.iter().chain(end_markers.iter()) {
        let marker_bottom = marker.cell.1 + marker.size;
        if marker.cell.1 <= bottom_row && marker_bottom > bottom_row {
            // Marker overlaps with bottom row.
            return true;
        }
    }
    false
}

// Pick a random stair size within settings range.
fn random_stair_size(settings: &UiSettings, seed_value: u64) -> usize {
    let min_size = settings.min_stair_width.min(settings.min_stair_height);
    let max_size = settings.max_stair_width.max(settings.max_stair_height);
    let span = max_size - min_size + 1;
    min_size + (seed_value as usize % span)
}

// Shuffle a vector deterministically using a seed.
fn shuffle_with_seed<T: Clone>(vec: &mut [T], seed: u64) {
    let mut rng = SimpleRng::new(seed);
    for i in (1..vec.len()).rev() {
        let j = (rng.next_u32() as usize) % (i + 1);
        vec.swap(i, j);
    }
}

// Simple RNG for deterministic shuffling.
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        // Simple LCG for deterministic shuffling.
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.state >> 32) as u32
    }
}
