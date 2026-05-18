/*
 * Room connection graph and corridor pathing.
 * Builds the logical connections between rooms and generates the
 * physical grid paths for corridors.
 */

use super::super::types::Room;
use super::super::utils::{
    SimpleRng, manhattan_distance, normalized_cell_edge, shared_boundary_edges, shuffle_indices,
};
use std::collections::HashSet;

pub fn build_room_connection_edges(
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

pub fn ordered_core_rooms(
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

pub fn push_unique_room_edge(
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

pub fn room_exit_edge(
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

pub fn room_collision_edges(
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

pub fn shared_room_boundaries(
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
