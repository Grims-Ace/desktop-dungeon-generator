/*
 * Path rerouting helpers for corridor dragging and room resizing.
 */

use crate::layout;
use std::collections::HashSet;

// Reroutes path through cell.
pub fn reroute_path_through_cell(
    original_path: &[(usize, usize)],
    original_cell: (usize, usize),
    target_cell: (usize, usize),
    cols: usize,
    rows: usize,
    blocked_room_cells_set: &HashSet<(usize, usize)>,
) -> Option<Vec<(usize, usize)>> {
    let (&start, &end) = (original_path.first()?, original_path.last()?);
    if start == end {
        return None;
    }

    let mut blocked = blocked_room_cells_set.clone();
    blocked.insert(original_cell);
    blocked.remove(&start);
    blocked.remove(&end);
    blocked.remove(&target_cell);

    let first_leg = crate::layout::shortest_path_cells(start, target_cell, cols, rows, &blocked)?;

    let mut blocked_second = blocked.clone();
    for &cell in first_leg
        .iter()
        .skip(1)
        .take(first_leg.len().saturating_sub(2))
    {
        if cell != target_cell && cell != end {
            blocked_second.insert(cell);
        }
    }

    let second_leg =
        crate::layout::shortest_path_cells(target_cell, end, cols, rows, &blocked_second).or_else(
            || crate::layout::shortest_path_cells(target_cell, end, cols, rows, &blocked),
        )?;

    let mut full_path = first_leg;
    for cell in second_leg.into_iter().skip(1) {
        if full_path.last().copied() != Some(cell) {
            full_path.push(cell);
        }
    }

    simplify_path_loops(&mut full_path);

    if full_path.len() < 2 || !full_path.contains(&target_cell) {
        return None;
    }

    if original_cell != start && original_cell != end && full_path.contains(&original_cell) {
        return None;
    }

    Some(full_path)
}

// Simplifies path loops.
pub fn simplify_path_loops(path: &mut Vec<(usize, usize)>) {
    let mut out = Vec::new();
    for &cell in path.iter() {
        if let Some(pos) = out.iter().position(|&c| c == cell) {
            out.truncate(pos + 1);
        } else {
            out.push(cell);
        }
    }
    *path = out;
}

// Resizes hit cells.
pub fn resize_hit_cells(
    room: &layout::Room,
    room_idx: usize,
    rooms: &[layout::Room],
    cols: usize,
    rows: usize,
) -> HashSet<(usize, usize)> {
    let mut cells = HashSet::new();
    let corners = [
        (room.x as isize, room.y as isize, -1isize, -1isize),
        (
            (room.x + room.width - 1) as isize,
            room.y as isize,
            1isize,
            -1isize,
        ),
        (
            room.x as isize,
            (room.y + room.height - 1) as isize,
            -1isize,
            1isize,
        ),
        (
            (room.x + room.width - 1) as isize,
            (room.y + room.height - 1) as isize,
            1isize,
            1isize,
        ),
    ];

    for (cx, cy, ox, oy) in corners {
        let candidates = [
            (cx, cy),
            (cx - ox, cy),
            (cx, cy - oy),
            (cx - ox, cy - oy),
            (cx + ox, cy),
            (cx, cy + oy),
            (cx + ox, cy + oy),
        ];

        for (x, y) in candidates {
            if x < 0 || y < 0 || x >= cols as isize || y >= rows as isize {
                continue;
            }

            let cell = (x as usize, y as usize);
            let in_other_room = rooms.iter().enumerate().any(|(idx, other_room)| {
                idx != room_idx
                    && cell.0 >= other_room.x
                    && cell.0 < other_room.x + other_room.width
                    && cell.1 >= other_room.y
                    && cell.1 < other_room.y + other_room.height
            });
            if !in_other_room {
                cells.insert(cell);
            }
        }
    }

    cells
}
