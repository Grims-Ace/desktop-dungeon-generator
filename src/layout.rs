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

pub fn generate_layout(cols: usize, rows: usize, target_room_count: usize, seed: u64) -> DungeonLayout {
    let mut rng = SimpleRng::new(seed ^ ((cols as u64) << 32) ^ rows as u64);
    let mut rooms = Vec::new();
    let mut corridors = Vec::new();

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

    for idx in 1..rooms.len() {
        let a = rooms[idx - 1].center_cell();
        let b = rooms[idx].center_cell();

        if rng.next_bool() {
            push_corridor_if_nonzero(&mut corridors, (a.0, a.1), (b.0, a.1));
            push_corridor_if_nonzero(&mut corridors, (b.0, a.1), (b.0, b.1));
        } else {
            push_corridor_if_nonzero(&mut corridors, (a.0, a.1), (a.0, b.1));
            push_corridor_if_nonzero(&mut corridors, (a.0, b.1), (b.0, b.1));
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

fn push_corridor_if_nonzero(
    corridors: &mut Vec<Corridor>,
    from: (usize, usize),
    to: (usize, usize),
) {
    if from != to {
        corridors.push(Corridor { from, to });
    }
}

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        let state = if seed == 0 { 0xA5A5_A5A5_1234_5678 } else { seed };
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
