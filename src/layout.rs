use std::collections::HashSet;

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
    pub from: (usize, usize),
    pub to: (usize, usize),
}

#[derive(Debug, Clone, Default)]
pub struct DungeonLayout {
    pub rooms: Vec<Room>,
    pub corridors: Vec<Corridor>,
}

pub fn generate_layout(
    cols: usize,
    rows: usize,
    target_room_count: usize,
    seed: u64,
) -> DungeonLayout {
    let mut rng = SimpleRng::new(seed ^ ((cols as u64) << 32) ^ rows as u64);
    let mut rooms = Vec::new();
    let mut corridors = Vec::new();
    let mut occupied_corridor_cells = HashSet::new();

    let min_room_size = 2;
    if cols < min_room_size || rows < min_room_size || target_room_count == 0 {
        return DungeonLayout { rooms, corridors };
    }

    let max_room_width = (cols / 3).clamp(min_room_size, cols.min(12));
    let max_room_height = (rows / 3).clamp(min_room_size, rows.min(12));
    let max_attempts = target_room_count.saturating_mul(40).max(50);

    for _ in 0..max_attempts {
        if rooms.len() >= target_room_count {
            break;
        }

        let width = rng.range_inclusive(min_room_size, max_room_width);
        let height = rng.range_inclusive(min_room_size, max_room_height);
        let x = rng.range_inclusive(0, cols - width);
        let y = rng.range_inclusive(0, rows - height);

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

    if !rooms.is_empty() {
        let mut connected = vec![false; rooms.len()];
        connected[0] = true;

        loop {
            let mut progress = false;
            let connected_indices: Vec<usize> = connected
                .iter()
                .enumerate()
                .filter_map(|(idx, is_connected)| if *is_connected { Some(idx) } else { None })
                .collect();

            for room_idx in 0..rooms.len() {
                if connected[room_idx] {
                    continue;
                }

                let mut anchors = connected_indices.clone();
                anchors.sort_by_key(|anchor_idx| {
                    manhattan_distance(
                        rooms[*anchor_idx].center_cell(),
                        rooms[room_idx].center_cell(),
                    )
                });

                let mut did_connect = false;

                for &anchor_idx in &anchors {
                    if try_connect_rooms(
                        rooms[anchor_idx].center_cell(),
                        rooms[room_idx].center_cell(),
                        true,
                        &mut rng,
                        &mut occupied_corridor_cells,
                        &mut corridors,
                    ) {
                        did_connect = true;
                        break;
                    }
                }

                if !did_connect {
                    for &anchor_idx in &anchors {
                        if try_connect_rooms(
                            rooms[anchor_idx].center_cell(),
                            rooms[room_idx].center_cell(),
                            false,
                            &mut rng,
                            &mut occupied_corridor_cells,
                            &mut corridors,
                        ) {
                            did_connect = true;
                            break;
                        }
                    }
                }

                if did_connect {
                    connected[room_idx] = true;
                    progress = true;
                }
            }

            if connected.iter().all(|is_connected| *is_connected) || !progress {
                break;
            }
        }
    }

    DungeonLayout { rooms, corridors }
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

fn make_l_path(
    from: (usize, usize),
    to: (usize, usize),
    horizontal_first: bool,
) -> Vec<(usize, usize)> {
    let mut path = Vec::new();
    if horizontal_first {
        append_segment_cells(&mut path, from, (to.0, from.1));
        append_segment_cells(&mut path, (to.0, from.1), to);
    } else {
        append_segment_cells(&mut path, from, (from.0, to.1));
        append_segment_cells(&mut path, (from.0, to.1), to);
    }
    path
}

fn append_segment_cells(path: &mut Vec<(usize, usize)>, from: (usize, usize), to: (usize, usize)) {
    if from.0 == to.0 {
        let x = from.0;
        let start = from.1.min(to.1);
        let end = from.1.max(to.1);
        for y in start..=end {
            if path.last().copied() != Some((x, y)) {
                path.push((x, y));
            }
        }
    } else if from.1 == to.1 {
        let y = from.1;
        let start = from.0.min(to.0);
        let end = from.0.max(to.0);
        for x in start..=end {
            if path.last().copied() != Some((x, y)) {
                path.push((x, y));
            }
        }
    }
}

fn try_connect_rooms(
    from: (usize, usize),
    to: (usize, usize),
    enforce_gap: bool,
    rng: &mut SimpleRng,
    occupied: &mut HashSet<(usize, usize)>,
    corridors: &mut Vec<Corridor>,
) -> bool {
    let horizontal_first = rng.next_bool();
    let first_try = make_l_path(from, to, horizontal_first);
    let second_try = make_l_path(from, to, !horizontal_first);

    try_place_path(&first_try, occupied, corridors, enforce_gap)
        || try_place_path(&second_try, occupied, corridors, enforce_gap)
}

fn try_place_path(
    path: &[(usize, usize)],
    occupied: &mut HashSet<(usize, usize)>,
    corridors: &mut Vec<Corridor>,
    enforce_gap: bool,
) -> bool {
    if path.len() < 2 || (enforce_gap && !has_required_corridor_gap(path, occupied)) {
        return false;
    }

    for cell in path {
        occupied.insert(*cell);
    }

    for segment in path.windows(2) {
        corridors.push(Corridor {
            from: segment[0],
            to: segment[1],
        });
    }

    true
}

fn has_required_corridor_gap(path: &[(usize, usize)], occupied: &HashSet<(usize, usize)>) -> bool {
    for &(x, y) in path {
        let x = x as isize;
        let y = y as isize;
        for dx in -1..=1 {
            for dy in -1..=1 {
                let nx = x + dx;
                let ny = y + dy;
                if nx < 0 || ny < 0 {
                    continue;
                }
                if occupied.contains(&(nx as usize, ny as usize)) {
                    return false;
                }
            }
        }
    }
    true
}

fn manhattan_distance(a: (usize, usize), b: (usize, usize)) -> usize {
    a.0.abs_diff(b.0) + a.1.abs_diff(b.1)
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

    fn next_bool(&mut self) -> bool {
        (self.next_u32() & 1) == 0
    }

    fn range_inclusive(&mut self, min: usize, max: usize) -> usize {
        if min >= max {
            return min;
        }
        let width = max - min + 1;
        min + (self.next_u32() as usize % width)
    }
}
