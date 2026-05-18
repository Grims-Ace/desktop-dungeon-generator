/*
 * Marker and transition population.
 * Handles the placement of start/end points, traps, monsters,
 * and staircase transitions between dungeon levels.
 */

use super::super::types::{AreaMarker, Corridor, DungeonLayout, Room, Staircase};
use super::super::utils::room_index_at_cell;
use crate::seed;
use crate::ui::UiSettings;
use std::collections::HashSet;

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
        seed::derive_seed(settings.seed, 10_001),
    );
    let end_count = random_range_inclusive(
        settings.min_end_marker_count,
        settings.max_end_marker_count,
        seed::derive_seed(settings.seed, 10_002),
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
            seed::derive_seed(settings.seed, 11_000 + pair_idx as u64),
        ));
        layout.end_markers.push(marker_in_room(
            end_room,
            settings.min_end_marker_size,
            settings.max_end_marker_size,
            seed::derive_seed(settings.seed, 12_000 + pair_idx as u64),
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
        13_000,
        14_000,
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
        15_000,
        16_000,
    );

    layout = populate_random_traps(layout, settings);
    layout = populate_random_monsters(layout, settings);
    layout
}

pub fn random_range_inclusive(min: usize, max: usize, seed_value: u64) -> usize {
    let min = min.max(1);
    let max = max.max(min);
    let span = max - min + 1;
    min + (seed_value as usize % span)
}

pub fn marker_in_room(
    room: &Room,
    min_size: usize,
    max_size: usize,
    seed_value: u64,
) -> AreaMarker {
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
            seed_value as usize % (x_span + 1)
        };
    let y = room.y
        + if y_span == 0 {
            0
        } else {
            seed_value.rotate_left(19) as usize % (y_span + 1)
        };
    AreaMarker { cell: (x, y), size }
}

#[allow(clippy::too_many_arguments)]
pub fn assign_extra_markers(
    markers: &mut Vec<AreaMarker>,
    rooms: &[Room],
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

pub fn farthest_room_pair(
    rooms: &[Room],
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

pub fn room_distance_sq(a: &Room, b: &Room) -> usize {
    let ac = a.center_cell();
    let bc = b.center_cell();
    let dx = ac.0.abs_diff(bc.0);
    let dy = ac.1.abs_diff(bc.1);
    dx * dx + dy * dy
}

pub fn consume_room(available_rooms: &mut Vec<usize>, room_idx: usize) {
    if let Some(pos) = available_rooms.iter().position(|&idx| idx == room_idx) {
        available_rooms.remove(pos);
    }
}

pub fn pick_room_index(available_rooms: &[usize], room_count: usize, seed_value: u64) -> usize {
    if !available_rooms.is_empty() {
        available_rooms[seed_value as usize % available_rooms.len()]
    } else {
        seed_value as usize % room_count.max(1)
    }
}

pub fn populate_random_traps(mut layout: DungeonLayout, settings: &UiSettings) -> DungeonLayout {
    layout.trap_markers = populate_random_area_markers(
        &layout,
        settings.seed,
        settings.trap_frequency_percent,
        settings.min_traps_per_area,
        settings.max_traps_per_area,
        17_000,
        18_000,
    );
    layout
}

pub fn populate_random_monsters(mut layout: DungeonLayout, settings: &UiSettings) -> DungeonLayout {
    layout.monster_markers = populate_random_area_markers(
        &layout,
        settings.seed,
        settings.monster_frequency_percent,
        settings.min_monsters_per_area,
        settings.max_monsters_per_area,
        19_000,
        20_000,
    );
    layout
}

pub fn populate_random_area_markers(
    layout: &DungeonLayout,
    master_seed: u64,
    frequency_percent: usize,
    min_count: usize,
    max_count: usize,
    room_stream_base: u64,
    corridor_stream_base: u64,
) -> Vec<AreaMarker> {
    if frequency_percent == 0 {
        return Vec::new();
    }

    let mut markers = Vec::new();

    for (idx, room) in layout.rooms.iter().enumerate() {
        let area_seed = seed::derive_seed(master_seed, room_stream_base + idx as u64);
        append_room_markers(
            &mut markers,
            room,
            area_seed,
            frequency_percent,
            min_count,
            max_count,
        );
    }

    for (idx, corridor) in layout.corridors.iter().enumerate() {
        let area_seed = seed::derive_seed(master_seed, corridor_stream_base + idx as u64);
        append_corridor_markers(
            &mut markers,
            corridor,
            area_seed,
            frequency_percent,
            min_count,
            max_count,
        );
    }

    markers
}

pub fn append_room_markers(
    markers: &mut Vec<AreaMarker>,
    room: &Room,
    area_seed: u64,
    frequency_percent: usize,
    min_count: usize,
    max_count: usize,
) {
    if !passes_frequency_roll(area_seed, frequency_percent) {
        return;
    }

    let count = random_range_inclusive(min_count, max_count, area_seed.rotate_left(13));
    for offset in 0..count {
        markers.push(marker_in_room(
            room,
            1,
            1,
            area_seed.wrapping_add(offset as u64).rotate_right(7),
        ));
    }
}

pub fn append_corridor_markers(
    markers: &mut Vec<AreaMarker>,
    corridor: &Corridor,
    area_seed: u64,
    frequency_percent: usize,
    min_count: usize,
    max_count: usize,
) {
    if !passes_frequency_roll(area_seed, frequency_percent) {
        return;
    }

    let count = random_range_inclusive(min_count, max_count, area_seed.rotate_left(13));
    for offset in 0..count {
        if let Some(cell) =
            pick_random_corridor_cell(corridor, area_seed.wrapping_add(offset as u64))
        {
            markers.push(AreaMarker { cell, size: 1 });
        }
    }
}

pub fn passes_frequency_roll(area_seed: u64, frequency_percent: usize) -> bool {
    (area_seed % 100) < frequency_percent as u64
}

pub fn pick_random_corridor_cell(corridor: &Corridor, seed_value: u64) -> Option<(usize, usize)> {
    if corridor.path.is_empty() {
        return None;
    }
    let idx = seed_value as usize % corridor.path.len();
    Some(corridor.path[idx])
}

pub fn populate_stairs(
    mut layouts: Vec<DungeonLayout>,
    settings: &UiSettings,
) -> Vec<(DungeonLayout, bool)> {
    if layouts.is_empty() {
        return Vec::new();
    }

    for layout in &mut layouts {
        layout.stairs.clear();
    }

    if settings.min_stairs_per_level == 0 && settings.max_stairs_per_level == 0 {
        return layouts.into_iter().map(|layout| (layout, false)).collect();
    }

    let num_levels = layouts.len();
    let num_gaps = num_levels.saturating_sub(1);
    let sets_to_gen = if num_gaps == 0 { 1 } else { num_gaps };
    let mut all_stair_sets: Vec<Vec<Staircase>> = Vec::new();

    if settings.sync_stairs_across_levels {
        let stair_count = get_stair_count(settings, 0);
        let synced_stairs = pick_stair_positions(&layouts[0], settings, stair_count, 0);
        for _ in 0..sets_to_gen {
            all_stair_sets.push(synced_stairs.clone());
        }
    } else {
        for gap_idx in 0..sets_to_gen {
            let stair_count = get_stair_count(settings, gap_idx);
            let layout_idx = gap_idx.min(num_levels - 1);
            let stairs =
                pick_stair_positions(&layouts[layout_idx], settings, stair_count, gap_idx as u64);
            all_stair_sets.push(stairs);
        }
    }

    if num_levels == 1 {
        layouts[0].stairs.extend(all_stair_sets[0].clone());
    } else {
        for gap_idx in 0..num_gaps {
            let stairs = &all_stair_sets[gap_idx];
            layouts[gap_idx].stairs.extend(stairs.clone());
            layouts[gap_idx + 1].stairs.extend(stairs.clone());
        }
    }

    for layout in &mut layouts {
        let mut seen = HashSet::new();
        layout.stairs.retain(|stair| seen.insert(stair.cell));
    }

    let mut result = Vec::new();
    for mut layout in layouts {
        let mut modified = false;
        let stairs = layout.stairs.clone();
        for stair in &stairs {
            if ensure_stair_in_room(&mut layout, stair) {
                modified = true;
            }
        }
        result.push((layout, modified));
    }

    result
}

pub fn get_stair_count(settings: &UiSettings, gap_idx: usize) -> usize {
    if settings.min_stairs_per_level == settings.max_stairs_per_level {
        settings.min_stairs_per_level
    } else {
        let range_seed = seed::derive_seed(settings.seed, 0x0002_A3B4_u64 + gap_idx as u64);
        (range_seed as usize % (settings.max_stairs_per_level - settings.min_stairs_per_level + 1))
            + settings.min_stairs_per_level
    }
}

pub fn ensure_stair_in_room(layout: &mut DungeonLayout, stair: &Staircase) -> bool {
    let stair_x = stair.cell.0;
    let stair_y = stair.cell.1;
    let stair_width = stair.width;
    let stair_height = stair.height;

    for room in &layout.rooms {
        if stair_x >= room.x
            && stair_y >= room.y
            && (stair_x + stair_width) <= room.x + room.width
            && (stair_y + stair_height) <= room.y + room.height
        {
            return false;
        }
    }

    let mut nearest_room_idx = None;
    let mut min_dist = usize::MAX;

    for (idx, room) in layout.rooms.iter().enumerate() {
        let dist = room_to_stair_min_dist(room, stair);
        if dist < min_dist {
            min_dist = dist;
            nearest_room_idx = Some(idx);
        }
    }

    if let Some(idx) = nearest_room_idx
        && min_dist <= 5
    {
        let room = &mut layout.rooms[idx];
        let new_x = room.x.min(stair_x);
        let new_y = room.y.min(stair_y);
        let new_right = (room.x + room.width).max(stair_x + stair_width);
        let new_bottom = (room.y + room.height).max(stair_y + stair_height);

        room.x = new_x;
        room.y = new_y;
        room.width = new_right - new_x;
        room.height = new_bottom - new_y;
        true
    } else {
        layout.rooms.push(Room {
            x: stair_x,
            y: stair_y,
            width: stair_width,
            height: stair_height,
        });
        true
    }
}

pub fn room_to_stair_min_dist(room: &Room, stair: &Staircase) -> usize {
    let dx = if stair.cell.0 + stair.width <= room.x {
        room.x - (stair.cell.0 + stair.width)
    } else {
        stair.cell.0.saturating_sub(room.x + room.width)
    };

    let dy = if stair.cell.1 + stair.height <= room.y {
        room.y - (stair.cell.1 + stair.height)
    } else {
        stair.cell.1.saturating_sub(room.y + room.height)
    };

    dx + dy
}

pub fn pick_stair_positions(
    layout: &DungeonLayout,
    settings: &UiSettings,
    count: usize,
    seed_offset: u64,
) -> Vec<Staircase> {
    if layout.rooms.is_empty() || count == 0 {
        return Vec::new();
    }

    let mut valid_cells = Vec::new();
    for room in &layout.rooms {
        let blocked_bottom_row =
            has_start_or_end_on_bottom_row(room, &layout.start_markers, &layout.end_markers);
        if !blocked_bottom_row {
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

    let mut shuffled = valid_cells.clone();
    shuffle_with_seed(
        &mut shuffled,
        settings.seed.wrapping_add(21_000).wrapping_add(seed_offset),
    );

    let max_size = settings.max_stair_width.max(settings.max_stair_height);
    let mut stair_cells = Vec::new();
    let mut used_cells = HashSet::new();

    for cell in shuffled {
        if stair_cells.len() >= count || used_cells.contains(&cell) {
            continue;
        }

        if let Some(room_idx) = room_index_at_cell(&layout.rooms, cell) {
            let room = &layout.rooms[room_idx];
            let available_w = room.x + room.width - cell.0;
            let available_h = room.y + room.height - cell.1;
            if available_w >= settings.min_stair_width && available_h >= settings.min_stair_height {
                stair_cells.push(cell);
                for dx in 0..max_size {
                    for dy in 0..max_size {
                        used_cells.insert((cell.0 + dx, cell.1 + dy));
                    }
                }
            }
        }
    }

    stair_cells
        .into_iter()
        .enumerate()
        .map(|(idx, cell)| {
            let (width, height) = random_stair_size(
                settings,
                seed::derive_seed(settings.seed, 21_000 + idx as u64 + seed_offset * 1000),
            );
            Staircase {
                cell,
                width,
                height,
            }
        })
        .collect()
}

pub fn has_start_or_end_on_bottom_row(
    room: &Room,
    start_markers: &[AreaMarker],
    end_markers: &[AreaMarker],
) -> bool {
    let bottom_row = room.y + room.height - 1;
    for marker in start_markers.iter().chain(end_markers.iter()) {
        let marker_bottom = marker.cell.1 + marker.size;
        if marker.cell.1 <= bottom_row && marker_bottom > bottom_row {
            return true;
        }
    }
    false
}

pub fn random_stair_size(settings: &UiSettings, seed_value: u64) -> (usize, usize) {
    let w_span = settings
        .max_stair_width
        .saturating_sub(settings.min_stair_width)
        + 1;
    let h_span = settings
        .max_stair_height
        .saturating_sub(settings.min_stair_height)
        + 1;
    let width = settings.min_stair_width + (seed_value as usize % w_span);
    let height = settings.min_stair_height + (seed_value.rotate_left(17) as usize % h_span);
    (width, height)
}

pub fn shuffle_with_seed<T>(items: &mut [T], seed: u64) {
    let mut rng = StairShuffleRng::new(seed);
    for idx in (1..items.len()).rev() {
        let swap_idx = rng.next_u32() as usize % (idx + 1);
        items.swap(idx, swap_idx);
    }
}

pub struct StairShuffleRng {
    pub state: u64,
}

impl StairShuffleRng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.state >> 32) as u32
    }
}
