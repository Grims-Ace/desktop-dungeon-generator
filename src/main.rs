mod layout;

use std::collections::HashSet;

use eframe::egui;
use egui::{Color32, Stroke};
use layout::{DungeonLayout, generate_layout};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();

    eframe::run_native(
        "Desktop Dungeon Generator",
        options,
        Box::new(|_cc| Ok(Box::new(DungeonApp::default()))),
    )
}

struct DungeonApp {
    cols: usize,
    rows: usize,
    room_count: usize,
    min_room_size: usize,
    max_room_size: usize,
    square_rooms_only: bool,
    corridor_randomness: usize,
    dead_end_rooms_percent: usize,
    seed: u64,
    layout: DungeonLayout,
}

impl Default for DungeonApp {
    fn default() -> Self {
        let cols = 24;
        let rows = 18;
        let room_count = 8;
        let min_room_size = 2;
        let max_room_size = 6;
        let square_rooms_only = false;
        let corridor_randomness = 0;
        let dead_end_rooms_percent = 0;
        let seed = 1;
        let layout = generate_layout(
            cols,
            rows,
            room_count,
            seed,
            min_room_size,
            max_room_size,
            square_rooms_only,
            corridor_randomness,
            dead_end_rooms_percent,
        );

        Self {
            cols,
            rows,
            room_count,
            min_room_size,
            max_room_size,
            square_rooms_only,
            corridor_randomness,
            dead_end_rooms_percent,
            seed,
            layout,
        }
    }
}

impl eframe::App for DungeonApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut settings_changed = false;

        egui::SidePanel::left("options_panel")
            .resizable(true)
            .min_width(180.0)
            .default_width(260.0)
            .show_separator_line(true)
            .show(ctx, |ui| {
                ui.heading("Grid Options");
                ui.add_space(8.0);

                ui.label("Columns");
                ui.horizontal(|ui| {
                    settings_changed |= ui
                        .add(egui::Slider::new(&mut self.cols, 6..=100).show_value(false))
                        .changed();
                    settings_changed |= ui
                        .add(
                            egui::DragValue::new(&mut self.cols)
                                .speed(1.0)
                                .range(2..=500),
                        )
                        .changed();
                });

                ui.add_space(8.0);
                ui.label("Rows");
                ui.horizontal(|ui| {
                    settings_changed |= ui
                        .add(egui::Slider::new(&mut self.rows, 6..=100).show_value(false))
                        .changed();
                    settings_changed |= ui
                        .add(
                            egui::DragValue::new(&mut self.rows)
                                .speed(1.0)
                                .range(2..=500),
                        )
                        .changed();
                });

                ui.add_space(12.0);
                ui.heading("Dungeon");
                ui.add_space(8.0);
                ui.label("Room Count");
                ui.horizontal(|ui| {
                    settings_changed |= ui
                        .add(egui::Slider::new(&mut self.room_count, 1..=40).show_value(false))
                        .changed();
                    settings_changed |= ui
                        .add(
                            egui::DragValue::new(&mut self.room_count)
                                .speed(1.0)
                                .range(1..=200),
                        )
                        .changed();
                });

                ui.add_space(8.0);
                ui.label("Min Room Size");
                ui.horizontal(|ui| {
                    settings_changed |= ui
                        .add(egui::Slider::new(&mut self.min_room_size, 2..=30).show_value(false))
                        .changed();
                    settings_changed |= ui
                        .add(
                            egui::DragValue::new(&mut self.min_room_size)
                                .speed(1.0)
                                .range(2..=200),
                        )
                        .changed();
                });

                ui.add_space(8.0);
                ui.label("Max Room Size");
                ui.horizontal(|ui| {
                    settings_changed |= ui
                        .add(egui::Slider::new(&mut self.max_room_size, 2..=30).show_value(false))
                        .changed();
                    settings_changed |= ui
                        .add(
                            egui::DragValue::new(&mut self.max_room_size)
                                .speed(1.0)
                                .range(2..=200),
                        )
                        .changed();
                });

                ui.add_space(8.0);
                settings_changed |= ui
                    .checkbox(&mut self.square_rooms_only, "Square Rooms Only")
                    .changed();

                ui.add_space(8.0);
                ui.label("Corridor Randomness (%)");
                ui.horizontal(|ui| {
                    settings_changed |= ui
                        .add(
                            egui::Slider::new(&mut self.corridor_randomness, 0..=100)
                                .show_value(false),
                        )
                        .changed();
                    settings_changed |= ui
                        .add(
                            egui::DragValue::new(&mut self.corridor_randomness)
                                .speed(1.0)
                                .range(0..=100),
                        )
                        .changed();
                });

                ui.add_space(8.0);
                ui.label("Dead-End Rooms (%)");
                ui.horizontal(|ui| {
                    settings_changed |= ui
                        .add(
                            egui::Slider::new(&mut self.dead_end_rooms_percent, 0..=50)
                                .show_value(false),
                        )
                        .changed();
                    settings_changed |= ui
                        .add(
                            egui::DragValue::new(&mut self.dead_end_rooms_percent)
                                .speed(1.0)
                                .range(0..=50),
                        )
                        .changed();
                });

                ui.add_space(10.0);
                if ui.button("Generate New Layout").clicked() {
                    self.seed = self.seed.wrapping_add(1);
                    self.layout = generate_layout(
                        self.cols,
                        self.rows,
                        self.room_count,
                        self.seed,
                        self.min_room_size,
                        self.max_room_size,
                        self.square_rooms_only,
                        self.corridor_randomness,
                        self.dead_end_rooms_percent,
                    );
                }
            });

        if self.min_room_size > self.max_room_size {
            self.max_room_size = self.min_room_size;
        }

        if settings_changed {
            self.layout = generate_layout(
                self.cols,
                self.rows,
                self.room_count,
                self.seed,
                self.min_room_size,
                self.max_room_size,
                self.square_rooms_only,
                self.corridor_randomness,
                self.dead_end_rooms_percent,
            );
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_size_before_wrap();
            let (response, painter) = ui.allocate_painter(available, egui::Sense::hover());
            let canvas = response.rect.shrink(12.0);
            let geometry = draw_grid(&painter, canvas, self.cols, self.rows);
            draw_layout(&painter, &geometry, &self.layout);
        });
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
