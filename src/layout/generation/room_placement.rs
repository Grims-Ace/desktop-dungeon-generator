/*
 * Room placement and sizing algorithms.
 * Handles the generation of room dimensions and their spatial arrangement,
 * including packed room placement.
 */

use super::super::types::Room;
use super::super::utils::{SimpleRng, rooms_overlap, rooms_touch, shuffle_indices};
use std::collections::VecDeque;

// Generates room sizes.
pub fn generate_room_sizes(
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

// Generates random room center points inside the grid.
pub fn random_centers(
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

// Places packed rooms.
pub fn place_packed_rooms(
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

// Computes the order used to place packed rooms.
pub fn placement_order(room_count: usize, room_edges: &[(usize, usize)]) -> Vec<usize> {
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

    for (idx, was_visited) in visited.iter().enumerate().take(room_count) {
        if !was_visited {
            order.push(idx);
        }
    }

    order
}

// Tries to place packed room.
pub fn try_place_packed_room(
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

// Builds room candidates.
pub fn packed_room_candidates(
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

// Shuffles rooms with the deterministic layout RNG.
pub fn shuffle_rooms(rooms: &mut [Room], rng: &mut SimpleRng) {
    if rooms.len() <= 1 {
        return;
    }
    for i in (1..rooms.len()).rev() {
        let j = rng.range_inclusive(0, i);
        rooms.swap(i, j);
    }
}
