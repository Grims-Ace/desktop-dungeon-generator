use std::collections::{HashSet, VecDeque};

use crate::seed;

#[derive(Debug, Clone)]
pub struct Room {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

impl Room {
    pub fn center_cell(&self) -> (usize, usize) {
        (self.x + (self.width / 2), self.y + (self.height / 2))
    }
}

#[derive(Debug, Clone)]
pub struct Corridor {
    #[allow(dead_code)]
    pub id: u64,
    pub start_room_id: usize,
    pub end_room_id: usize,
    pub path: Vec<(usize, usize)>,
    pub width: usize,
}

#[derive(Debug, Clone)]
pub struct Door {
    pub from: (usize, usize),
    pub to: (usize, usize),
    pub width: usize,
    pub locked: bool,
    pub archway: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct DoorSettings {
    pub frequency_percent: usize,
    pub room_hallway_percent: usize,
    pub locked_percent: usize,
    pub allow_middle_corridor_doors: bool,
}

#[derive(Debug, Clone, Default)]
pub struct DungeonLayout {
    pub rooms: Vec<Room>,
    pub corridors: Vec<Corridor>,
    pub doors: Vec<Door>,
}

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
    door_settings: DoorSettings,
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
        };
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
        };
    }

    let randomness = (corridor_randomness_percent.min(100) as f32) / 100.0;
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

        let path = if randomness <= 0.001 {
            shortest_path_cells(start, end, cols, rows, &HashSet::new())
        } else {
            Some(noisy_path(
                start,
                end,
                cols,
                rows,
                randomness,
                &mut path_rng,
            ))
        }
        .or_else(|| shortest_path_cells(start, end, cols, rows, &HashSet::new()));

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
    };
    apply_doors(&mut layout, seed, door_settings, cols, rows);
    layout
}

pub fn apply_doors(
    layout: &mut DungeonLayout,
    seed: u64,
    settings: DoorSettings,
    cols: usize,
    rows: usize,
) {
    layout.doors.clear();

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
                    locked: place_door && rng.next_f32() <= locked,
                    archway: !place_door,
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
                    locked: place_door && rng.next_f32() <= locked,
                    archway: !place_door,
                });
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
                        locked: rng.next_f32() <= locked,
                        archway: false,
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
                        locked: rng.next_f32() <= locked,
                        archway: false,
                    });
                }
            }
        }
    }
}

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
    }

    cells
}

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

fn shuffle_indices(indices: &mut [usize], rng: &mut SimpleRng) {
    if indices.len() <= 1 {
        return;
    }
    for i in (1..indices.len()).rev() {
        let j = rng.range_inclusive(0, i);
        indices.swap(i, j);
    }
}

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

fn noisy_path(
    start: (usize, usize),
    end: (usize, usize),
    cols: usize,
    rows: usize,
    randomness: f32,
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
        && let Some(tail) = shortest_path_cells(current, end, cols, rows, &HashSet::new())
    {
        for &cell in tail.iter().skip(1) {
            path.push(cell);
        }
    }

    path
}

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

fn manhattan_distance(a: (usize, usize), b: (usize, usize)) -> usize {
    a.0.abs_diff(b.0) + a.1.abs_diff(b.1)
}

fn normalized_cell_edge(a: (usize, usize), b: (usize, usize)) -> ((usize, usize), (usize, usize)) {
    if a <= b { (a, b) } else { (b, a) }
}

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        let state = if seed == 0 {
            0xA5A5_A5A5_1234_5678
        } else {
            seed
        };
        Self { state }
    }

    fn next_u32(&mut self) -> u32 {
        self.state ^= self.state >> 12;
        self.state ^= self.state << 25;
        self.state ^= self.state >> 27;
        (self.state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
    }

    fn next_f32(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }

    fn range_inclusive(&mut self, min: usize, max: usize) -> usize {
        if min >= max {
            return min;
        }
        let width = max - min + 1;
        min + (self.next_u32() as usize % width)
    }
}
