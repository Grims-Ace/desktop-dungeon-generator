mod layout;
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
    seed: u64,
    layout: DungeonLayout,
}

impl Default for DungeonApp {
    fn default() -> Self {
        let settings = UiSettings::default();
        let seed = 1;
        let layout = generate_layout(
            settings.cols,
            settings.rows,
            settings.room_count,
            seed,
            settings.min_room_size,
            settings.max_room_size,
            settings.square_rooms_only,
            settings.corridor_randomness,
            settings.dead_end_rooms_percent,
        );

        Self {
            settings,
            seed,
            layout,
        }
    }
}

impl eframe::App for DungeonApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let panel_result = draw_side_panel(ctx, &mut self.settings);

        if self.settings.min_room_size > self.settings.max_room_size {
            self.settings.max_room_size = self.settings.min_room_size;
        }

        if panel_result.generate_clicked {
            self.seed = self.seed.wrapping_add(1);
            self.regenerate_layout();
        }

        if panel_result.settings_changed {
            self.regenerate_layout();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_size_before_wrap();
            let (response, painter) = ui.allocate_painter(available, egui::Sense::hover());
            let canvas = response.rect.shrink(12.0);
            let geometry = draw_grid(&painter, canvas, self.settings.cols, self.settings.rows);
            draw_layout(&painter, &geometry, &self.layout);
        });
    }
}

impl DungeonApp {
    fn regenerate_layout(&mut self) {
        self.layout = generate_layout(
            self.settings.cols,
            self.settings.rows,
            self.settings.room_count,
            self.seed,
            self.settings.min_room_size,
            self.settings.max_room_size,
            self.settings.square_rooms_only,
            self.settings.corridor_randomness,
            self.settings.dead_end_rooms_percent,
        );
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
        corridor_edges_set.insert(normalized_edge(corridor.from, corridor.to));
        for (col, row) in corridor_cells(corridor.from, corridor.to) {
            corridor_cells_set.insert((col, row));
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
