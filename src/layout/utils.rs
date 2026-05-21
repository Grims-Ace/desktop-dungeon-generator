/*
 * Geometric utilities and low-level layout helpers.
 * Provides logic for Manhattan distance, room overlap testing,
 * BFS-based shortest pathfinding, and custom deterministic RNG.
 */

use super::types::{DungeonLayout, Room};
use std::collections::{HashSet, VecDeque};

pub struct SimpleRng {
    pub state: u64,
}

impl SimpleRng {
    // Create a small deterministic RNG with a fallback seed.
    pub fn new(seed: u64) -> Self {
        let state = if seed == 0 {
            0xA5A5_A5A5_1234_5678
        } else {
            seed
        };
        Self { state }
    }

    // Return the next random u32.
    pub fn next_u32(&mut self) -> u32 {
        self.state ^= self.state >> 12;
        self.state ^= self.state << 25;
        self.state ^= self.state >> 27;
        (self.state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
    }

    // Return the next random f32 in [0,1].
    pub fn next_f32(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }

    // Generate a random usize between min and max inclusive.
    pub fn range_inclusive(&mut self, min: usize, max: usize) -> usize {
        if min >= max {
            return min;
        }
        let width = max - min + 1;
        min + (self.next_u32() as usize % width)
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
        for cell in corridor.cells(cols, rows, &room_cells) {
            cells.insert(cell);
        }
    }

    cells
}

// Computes index at cell.
pub fn room_index_at_cell(rooms: &[Room], cell: (usize, usize)) -> Option<usize> {
    rooms.iter().position(|room| {
        cell.0 >= room.x
            && cell.0 < room.x + room.width
            && cell.1 >= room.y
            && cell.1 < room.y + room.height
    })
}

// Normalize a cell edge ordering.
pub fn normalized_cell_edge(
    a: (usize, usize),
    b: (usize, usize),
) -> ((usize, usize), (usize, usize)) {
    if a <= b { (a, b) } else { (b, a) }
}

// Compute Manhattan distance between two grid cells.
pub fn manhattan_distance(a: (usize, usize), b: (usize, usize)) -> usize {
    a.0.abs_diff(b.0) + a.1.abs_diff(b.1)
}

// Test whether two rooms overlap with extra padding.
pub fn overlaps_with_padding(a: &Room, b: &Room, padding: usize) -> bool {
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

// Checks whether rects overlap.
#[allow(clippy::too_many_arguments)]
pub fn rects_overlap(
    ax: usize,
    ay: usize,
    aw: usize,
    ah: usize,
    bx: usize,
    by: usize,
    bw: usize,
    bh: usize,
) -> bool {
    let a_right = ax + aw;
    let a_bottom = ay + ah;
    let b_right = bx + bw;
    let b_bottom = by + bh;

    ax < b_right && a_right > bx && ay < b_bottom && a_bottom > by
}

// Checks whether rooms overlap.
pub fn rooms_overlap(a: &Room, b: &Room) -> bool {
    rects_overlap(a.x, a.y, a.width, a.height, b.x, b.y, b.width, b.height)
}

// Checks whether rooms touch.
pub fn rooms_touch(a: &Room, b: &Room) -> bool {
    !shared_boundary_edges(a, b).is_empty()
}

// Finds boundary edges.
pub fn shared_boundary_edges(a: &Room, b: &Room) -> Vec<((usize, usize), (usize, usize))> {
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

// Finds opening width.
pub fn shared_opening_width(a: &Room, b: &Room, span: usize, default_width: usize) -> usize {
    let max_width = if a.x + a.width == b.x || b.x + b.width == a.x {
        a.height.min(b.height)
    } else {
        a.width.min(b.width)
    };
    default_width.max(1).min(span).min(max_width.max(1))
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
pub fn noisy_path(
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

// Shuffle any mutable slice in place using the provided RNG.
pub fn shuffle_items<T>(items: &mut [T], rng: &mut SimpleRng) {
    if items.len() <= 1 {
        return;
    }
    for i in (1..items.len()).rev() {
        let j = rng.range_inclusive(0, i);
        items.swap(i, j);
    }
}

// Shuffle indices in place using the provided RNG.
pub fn shuffle_indices(indices: &mut [usize], rng: &mut SimpleRng) {
    shuffle_items(indices, rng);
}
