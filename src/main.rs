mod layout;
mod seed;
mod settings;
mod ui;

use std::collections::HashSet;

use eframe::egui;
use egui::{Color32, Stroke};
use layout::{DungeonLayout, generate_layout};
use ui::{UiSettings, draw_side_panel};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();

    eframe::run_native(
        "Desktop Dungeon Generator",
        options,
        Box::new(|_cc| Ok(Box::new(DungeonApp::default()))),
    )
}

struct DungeonApp {
    settings: UiSettings,
    layout: DungeonLayout,
    room_drag: Option<RoomDragState>,
}

impl Default for DungeonApp {
    fn default() -> Self {
        let settings = settings::load_settings().unwrap_or_default();
        let layout = generate_layout(
            settings.cols,
            settings.rows,
            settings.room_count,
            settings.seed,
            settings.min_room_size,
            settings.max_room_size,
            settings.square_rooms_only,
            settings.corridor_randomness,
            settings.dead_end_rooms_percent,
        );
        Self {
            settings,
            layout,
            room_drag: None,
        }
    }
}

impl Drop for DungeonApp {
    fn drop(&mut self) {
        if let Err(err) = settings::save_settings(&self.settings) {
            eprintln!("Failed to save settings: {err}");
        }
    }
}

impl eframe::App for DungeonApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let panel_result = draw_side_panel(ctx, &mut self.settings);

        if self.settings.min_room_size > self.settings.max_room_size {
            self.settings.max_room_size = self.settings.min_room_size;
        }

        if panel_result.reset_clicked {
            self.regenerate_layout();
        }

        if panel_result.settings_changed {
            self.regenerate_layout();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_size_before_wrap();
            let (response, painter) = ui.allocate_painter(available, egui::Sense::click_and_drag());
            let canvas = response.rect.shrink(12.0);
            let geometry = draw_grid(&painter, canvas, self.settings.cols, self.settings.rows);
            self.handle_room_drag(&response, &geometry);
            draw_layout(&painter, &geometry, &self.layout);
        });
    }
}

impl DungeonApp {
    fn regenerate_layout(&mut self) {
        self.room_drag = None;
        self.layout = generate_layout(
            self.settings.cols,
            self.settings.rows,
            self.settings.room_count,
            self.settings.seed,
            self.settings.min_room_size,
            self.settings.max_room_size,
            self.settings.square_rooms_only,
            self.settings.corridor_randomness,
            self.settings.dead_end_rooms_percent,
        );
    }

    fn handle_room_drag(&mut self, response: &egui::Response, geometry: &GridGeometry) {
        if response.drag_started()
            && let Some(pointer_pos) = response.interact_pointer_pos()
            && let Some((room_idx, offset_x, offset_y)) =
                self.room_at_pointer(pointer_pos, geometry)
        {
            self.room_drag = Some(RoomDragState {
                room_idx,
                offset_x,
                offset_y,
            });
        }

        if response.dragged()
            && let Some(pointer_pos) = response.interact_pointer_pos()
            && let Some(drag) = self.room_drag
        {
            self.drag_room_to_pointer(drag, pointer_pos, geometry);
        }

        if response.drag_stopped() || !response.ctx.input(|i| i.pointer.primary_down()) {
            self.room_drag = None;
        }
    }

    fn room_at_pointer(
        &self,
        pointer_pos: egui::Pos2,
        geometry: &GridGeometry,
    ) -> Option<(usize, f32, f32)> {
        let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;

        for (room_idx, room) in self.layout.rooms.iter().enumerate().rev() {
            let room_x = room.x as f32;
            let room_y = room.y as f32;
            let room_right = room_x + room.width as f32;
            let room_bottom = room_y + room.height as f32;

            if grid_x >= room_x && grid_x < room_right && grid_y >= room_y && grid_y < room_bottom {
                return Some((room_idx, grid_x - room_x, grid_y - room_y));
            }
        }

        None
    }

    fn drag_room_to_pointer(
        &mut self,
        drag: RoomDragState,
        pointer_pos: egui::Pos2,
        geometry: &GridGeometry,
    ) {
        let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry) else {
            return;
        };

        if drag.room_idx >= self.layout.rooms.len() {
            self.room_drag = None;
            return;
        }

        let (old_x, old_y, width, height) = {
            let room = &self.layout.rooms[drag.room_idx];
            (room.x, room.y, room.width, room.height)
        };
        let max_x = self.settings.cols.saturating_sub(width);
        let max_y = self.settings.rows.saturating_sub(height);

        let target_x = (grid_x - drag.offset_x).floor().max(0.0) as usize;
        let target_y = (grid_y - drag.offset_y).floor().max(0.0) as usize;
        let new_x = target_x.min(max_x);
        let new_y = target_y.min(max_y);

        if new_x == old_x && new_y == old_y {
            return;
        }

        self.layout.rooms[drag.room_idx].x = new_x;
        self.layout.rooms[drag.room_idx].y = new_y;
        self.adjust_corridors_for_room_move(old_x, old_y, width, height, new_x, new_y);
    }

    fn adjust_corridors_for_room_move(
        &mut self,
        old_x: usize,
        old_y: usize,
        width: usize,
        height: usize,
        new_x: usize,
        new_y: usize,
    ) {
        let dx = new_x as isize - old_x as isize;
        let dy = new_y as isize - old_y as isize;
        if dx == 0 && dy == 0 {
            return;
        }

        let room_cells = RoomCells {
            x: old_x,
            y: old_y,
            width,
            height,
        };

        for corridor in &mut self.layout.corridors {
            if room_cells.contains(corridor.from) {
                corridor.from = shift_cell(
                    corridor.from,
                    dx,
                    dy,
                    self.settings.cols,
                    self.settings.rows,
                );
            }
            if room_cells.contains(corridor.to) {
                corridor.to =
                    shift_cell(corridor.to, dx, dy, self.settings.cols, self.settings.rows);
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct RoomDragState {
    room_idx: usize,
    offset_x: f32,
    offset_y: f32,
}

struct RoomCells {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

impl RoomCells {
    fn contains(&self, cell: (usize, usize)) -> bool {
        let (cx, cy) = cell;
        cx >= self.x && cx < self.x + self.width && cy >= self.y && cy < self.y + self.height
    }
}

struct GridGeometry {
    rect: egui::Rect,
    cell_size: f32,
}

fn draw_grid(painter: &egui::Painter, area: egui::Rect, cols: usize, rows: usize) -> GridGeometry {
    let cols = cols.max(1);
    let rows = rows.max(1);

    let cell_size = (area.width() / cols as f32).min(area.height() / rows as f32);
    let grid_width = cell_size * cols as f32;
    let grid_height = cell_size * rows as f32;

    let grid_rect = egui::Rect::from_min_size(
        egui::pos2(
            area.center().x - (grid_width * 0.5),
            area.center().y - (grid_height * 0.5),
        ),
        egui::vec2(grid_width, grid_height),
    );

    let stroke = Stroke::new(1.0, Color32::from_gray(160));
    painter.rect_stroke(grid_rect, 0.0, stroke, egui::StrokeKind::Middle);

    for col in 1..cols {
        let x = grid_rect.left() + (col as f32 * cell_size);
        painter.line_segment(
            [
                egui::pos2(x, grid_rect.top()),
                egui::pos2(x, grid_rect.bottom()),
            ],
            stroke,
        );
    }

    for row in 1..rows {
        let y = grid_rect.top() + (row as f32 * cell_size);
        painter.line_segment(
            [
                egui::pos2(grid_rect.left(), y),
                egui::pos2(grid_rect.right(), y),
            ],
            stroke,
        );
    }

    GridGeometry {
        rect: grid_rect,
        cell_size,
    }
}

fn draw_layout(painter: &egui::Painter, geometry: &GridGeometry, layout: &DungeonLayout) {
    let room_fill = Color32::from_rgb(70, 120, 160);
    let corridor_fill = Color32::from_rgb(210, 190, 120);
    let outline_stroke = Stroke::new(3.0, Color32::BLACK);
    let mut corridor_cells_set = HashSet::new();
    let mut corridor_edges_set = HashSet::new();

    for corridor in &layout.corridors {
        let cells = corridor_cells(corridor.from, corridor.to);
        for cell in &cells {
            corridor_cells_set.insert(*cell);
        }
        for pair in cells.windows(2) {
            corridor_edges_set.insert(normalized_edge(pair[0], pair[1]));
        }
    }

    for &(col, row) in &corridor_cells_set {
        painter.rect_filled(cell_rect(geometry, col, row), 0.0, corridor_fill);
    }

    for &(col, row) in &corridor_cells_set {
        let rect = cell_rect(geometry, col, row);
        let right = (col + 1, row);
        let bottom = (col, row + 1);

        if col == 0 || !corridor_cells_set.contains(&(col - 1, row)) {
            painter.line_segment(
                [
                    egui::pos2(rect.left(), rect.top()),
                    egui::pos2(rect.left(), rect.bottom()),
                ],
                outline_stroke,
            );
        }
        if !corridor_cells_set.contains(&right)
            || !corridor_edges_set.contains(&normalized_edge((col, row), right))
        {
            painter.line_segment(
                [
                    egui::pos2(rect.right(), rect.top()),
                    egui::pos2(rect.right(), rect.bottom()),
                ],
                outline_stroke,
            );
        }
        if row == 0 || !corridor_cells_set.contains(&(col, row - 1)) {
            painter.line_segment(
                [
                    egui::pos2(rect.left(), rect.top()),
                    egui::pos2(rect.right(), rect.top()),
                ],
                outline_stroke,
            );
        }
        if !corridor_cells_set.contains(&bottom)
            || !corridor_edges_set.contains(&normalized_edge((col, row), bottom))
        {
            painter.line_segment(
                [
                    egui::pos2(rect.left(), rect.bottom()),
                    egui::pos2(rect.right(), rect.bottom()),
                ],
                outline_stroke,
            );
        }
    }

    for room in &layout.rooms {
        let left = geometry.rect.left() + room.x as f32 * geometry.cell_size;
        let top = geometry.rect.top() + room.y as f32 * geometry.cell_size;
        let right = left + room.width as f32 * geometry.cell_size;
        let bottom = top + room.height as f32 * geometry.cell_size;
        let room_rect = egui::Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, bottom));

        painter.rect_filled(room_rect, 0.0, room_fill);
        painter.rect_stroke(room_rect, 0.0, outline_stroke, egui::StrokeKind::Middle);
    }
}

fn corridor_cells(from: (usize, usize), to: (usize, usize)) -> Vec<(usize, usize)> {
    let mut cells = Vec::new();
    if from.0 == to.0 {
        let x = from.0;
        let start = from.1.min(to.1);
        let end = from.1.max(to.1);
        for y in start..=end {
            cells.push((x, y));
        }
    } else if from.1 == to.1 {
        let y = from.1;
        let start = from.0.min(to.0);
        let end = from.0.max(to.0);
        for x in start..=end {
            cells.push((x, y));
        }
    } else {
        // If an endpoint drift makes a segment diagonal, render it as an L path.
        let corner = (to.0, from.1);
        cells.extend(corridor_cells(from, corner));
        for cell in corridor_cells(corner, to) {
            if cells.last().copied() != Some(cell) {
                cells.push(cell);
            }
        }
    }
    cells
}

fn cell_rect(geometry: &GridGeometry, col: usize, row: usize) -> egui::Rect {
    let left = geometry.rect.left() + col as f32 * geometry.cell_size;
    let top = geometry.rect.top() + row as f32 * geometry.cell_size;
    egui::Rect::from_min_size(
        egui::pos2(left, top),
        egui::vec2(geometry.cell_size, geometry.cell_size),
    )
}

fn normalized_edge(a: (usize, usize), b: (usize, usize)) -> ((usize, usize), (usize, usize)) {
    if a <= b { (a, b) } else { (b, a) }
}

fn pointer_to_grid(pointer_pos: egui::Pos2, geometry: &GridGeometry) -> Option<(f32, f32)> {
    if !geometry.rect.contains(pointer_pos) {
        return None;
    }

    let x = (pointer_pos.x - geometry.rect.left()) / geometry.cell_size;
    let y = (pointer_pos.y - geometry.rect.top()) / geometry.cell_size;
    Some((x, y))
}

fn shift_cell(
    cell: (usize, usize),
    dx: isize,
    dy: isize,
    cols: usize,
    rows: usize,
) -> (usize, usize) {
    let x = (cell.0 as isize + dx).clamp(0, cols.saturating_sub(1) as isize) as usize;
    let y = (cell.1 as isize + dy).clamp(0, rows.saturating_sub(1) as isize) as usize;
    (x, y)
}
