use eframe::egui;
use egui::{Color32, Stroke};

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
}

impl Default for DungeonApp {
    fn default() -> Self {
        Self { cols: 12, rows: 8 }
    }
}

impl eframe::App for DungeonApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
                    ui.add(egui::Slider::new(&mut self.cols, 2..=100).show_value(false));
                    ui.add(
                        egui::DragValue::new(&mut self.cols)
                            .speed(1.0)
                            .range(2..=500),
                    );
                });

                ui.add_space(8.0);
                ui.label("Rows");
                ui.horizontal(|ui| {
                    ui.add(egui::Slider::new(&mut self.rows, 2..=100).show_value(false));
                    ui.add(
                        egui::DragValue::new(&mut self.rows)
                            .speed(1.0)
                            .range(2..=500),
                    );
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_size_before_wrap();
            let (response, painter) = ui.allocate_painter(available, egui::Sense::hover());
            let canvas = response.rect.shrink(12.0);
            draw_grid(&painter, canvas, self.cols, self.rows);
        });
    }
}

fn draw_grid(painter: &egui::Painter, area: egui::Rect, cols: usize, rows: usize) {
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
}
