use crate::layout::{self, DungeonLayout};
use crate::seed;
use crate::ui::UiSettings;

const START_COUNT_STREAM: u64 = 10_001;
const END_COUNT_STREAM: u64 = 10_002;
const START_ROOM_STREAM_BASE: u64 = 11_000;
const END_ROOM_STREAM_BASE: u64 = 12_000;
const EXTRA_START_ROOM_STREAM_BASE: u64 = 13_000;
const EXTRA_START_MARKER_STREAM_BASE: u64 = 14_000;
const EXTRA_END_ROOM_STREAM_BASE: u64 = 15_000;
const EXTRA_END_MARKER_STREAM_BASE: u64 = 16_000;

// Populate generated start/end markers after the core dungeon layout exists.
pub fn populate_random_markers(
    mut layout: DungeonLayout,
    settings: &UiSettings,
) -> DungeonLayout {
    layout.start_markers.clear();
    layout.end_markers.clear();

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

    layout
}

// Manual marker placement uses the configured minimum size for the chosen marker type.
pub fn manual_marker_size(settings: &UiSettings, is_start: bool) -> usize {
    if is_start {
        settings
            .min_start_marker_size
            .min(settings.max_start_marker_size)
    } else {
        settings.min_end_marker_size.min(settings.max_end_marker_size)
    }
    .clamp(1, 10)
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
