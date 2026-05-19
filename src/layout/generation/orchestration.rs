/*
 * Top-level procedural generation orchestration.
 * Coordinates room placement, corridor routing, doors, and windows.
 */

use super::super::types::{Corridor, DoorSettings, DungeonLayout, Room, WindowSettings};
use super::super::utils::{SimpleRng, blocked_room_cells, noisy_path, shortest_path_cells};
use crate::seed;

use super::{
    apply_doors, apply_windows, build_room_connection_edges, generate_room_sizes,
    place_packed_rooms, random_centers,
};

// Generates a complete dungeon layout from generation settings.
#[allow(clippy::too_many_arguments)]
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
    let corridors = Vec::new();

    if cols < 2 || rows < 2 || target_room_count == 0 {
        return DungeonLayout::empty(pack_rooms_without_corridors);
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
        return DungeonLayout::empty(pack_rooms_without_corridors);
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
        let mut layout = DungeonLayout::from_generated_parts(rooms, corridors, true);
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
            .all(|existing| !crate::layout::utils::overlaps_with_padding(&candidate, existing, 1))
        {
            rooms.push(candidate);
        }
    }

    if rooms.len() < 2 {
        return DungeonLayout::from_generated_parts(rooms, corridors, false);
    }

    let mut min_width = min_corridor_width.max(1);
    let max_grid_width = cols.max(1).min(rows.max(1));
    let max_width = max_corridor_width.max(min_width).min(max_grid_width);
    min_width = min_width.min(max_width);
    let centers: Vec<(usize, usize)> = rooms.iter().map(Room::center_cell).collect();
    let target_dead_end_rooms = ((rooms.len() * dead_end_room_percent.min(50)) + 50) / 100;
    let room_edges =
        build_room_connection_edges(&centers, randomness, target_dead_end_rooms, &mut graph_rng);

    let mut corridors = Vec::new();
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

    let mut layout = DungeonLayout::from_generated_parts(rooms, corridors, false);
    apply_doors(&mut layout, seed, door_settings, cols, rows);
    apply_windows(&mut layout, seed, window_settings, cols, rows);
    layout
}
