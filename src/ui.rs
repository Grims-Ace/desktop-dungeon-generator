use crate::seed;
use eframe::egui;
use egui::RichText;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum Tab {
    Generate,
    Layout,
}

impl Default for Tab {
    fn default() -> Self {
        Self::Generate
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    Png,
    Jpeg,
    Webp,
    Svg,
    Folder,
}

impl Default for ExportFormat {
    fn default() -> Self {
        Self::Png
    }
}

impl ExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Png => "png",
            ExportFormat::Jpeg => "jpeg",
            ExportFormat::Webp => "webp",
            ExportFormat::Svg => "svg",
            ExportFormat::Folder => "folder",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ExportFormat::Png => ".png",
            ExportFormat::Jpeg => ".jpeg",
            ExportFormat::Webp => ".webp",
            ExportFormat::Svg => ".svg",
            ExportFormat::Folder => "Folder (Composite)",
        }
    }

    pub fn supports_resolution(self) -> bool {
        !matches!(self, ExportFormat::Svg)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaskFormat {
    Png,
    Jpeg,
    Webp,
}

impl Default for MaskFormat {
    fn default() -> Self {
        Self::Png
    }
}

impl MaskFormat {
    pub fn extension(self) -> &'static str {
        match self {
            MaskFormat::Png => "png",
            MaskFormat::Jpeg => "jpeg",
            MaskFormat::Webp => "webp",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            MaskFormat::Png => ".png",
            MaskFormat::Jpeg => ".jpeg",
            MaskFormat::Webp => ".webp",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiSettings {
    pub seed: u64,
    pub cols: usize,
    pub rows: usize,
    pub room_count: usize,
    pub min_room_size: usize,
    pub max_room_size: usize,
    pub square_rooms_only: bool,
    pub min_corridor_width: usize,
    pub max_corridor_width: usize,
    pub corridor_randomness: usize,
    pub dead_end_rooms_percent: usize,
    pub door_frequency_percent: usize,
    pub room_hallway_door_percent: usize,
    pub locked_door_percent: usize,
    pub allow_middle_corridor_doors: bool,
    pub colorblind_mode: bool,
    pub export_format: ExportFormat,
    pub mask_format: MaskFormat,
    pub export_width: u32,
    pub export_height: u32,
    pub allow_export_aspect_change: bool,
    active_tab: Tab,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            seed: 1,
            cols: 24,
            rows: 18,
            room_count: 8,
            min_room_size: 2,
            max_room_size: 6,
            square_rooms_only: false,
            min_corridor_width: 1,
            max_corridor_width: 2,
            corridor_randomness: 0,
            dead_end_rooms_percent: 0,
            door_frequency_percent: 30,
            room_hallway_door_percent: 70,
            locked_door_percent: 25,
            allow_middle_corridor_doors: false,
            colorblind_mode: false,
            export_format: ExportFormat::Png,
            mask_format: MaskFormat::Png,
            export_width: 1920,
            export_height: 1080,
            allow_export_aspect_change: false,
            active_tab: Tab::Generate,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SidePanelResult {
    pub settings_changed: bool,
    pub reset_clicked: bool,
    pub export_clicked: bool,
}

pub fn draw_side_panel(ctx: &egui::Context, settings: &mut UiSettings) -> SidePanelResult {
    let mut result = SidePanelResult::default();

    egui::SidePanel::left("options_panel")
        .resizable(true)
        .min_width(180.0)
        .default_width(280.0)
        .show_separator_line(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let is_generate = settings.active_tab == Tab::Generate;
                let is_layout = settings.active_tab == Tab::Layout;

                if ui.selectable_label(is_generate, "Generate").clicked() {
                    settings.active_tab = Tab::Generate;
                }
                if ui.selectable_label(is_layout, "Layout").clicked() {
                    settings.active_tab = Tab::Layout;
                }
            });
            ui.separator();

            match settings.active_tab {
                Tab::Generate => draw_generate_tab(ui, settings, &mut result),
                Tab::Layout => draw_layout_tab(ui, settings, &mut result),
            }
        });

    result
}

fn draw_generate_tab(ui: &mut egui::Ui, settings: &mut UiSettings, result: &mut SidePanelResult) {
    ui.label(RichText::new("Grid Settings").strong());
    ui.add_space(8.0);

    ui.label("Columns");
    ui.horizontal(|ui| {
        result.settings_changed |= ui
            .add(egui::Slider::new(&mut settings.cols, 6..=100).show_value(false))
            .changed();
        result.settings_changed |= ui
            .add(
                egui::DragValue::new(&mut settings.cols)
                    .speed(1.0)
                    .range(2..=500),
            )
            .changed();
    });

    ui.add_space(8.0);
    ui.label("Rows");
    ui.horizontal(|ui| {
        result.settings_changed |= ui
            .add(egui::Slider::new(&mut settings.rows, 6..=100).show_value(false))
            .changed();
        result.settings_changed |= ui
            .add(
                egui::DragValue::new(&mut settings.rows)
                    .speed(1.0)
                    .range(2..=500),
            )
            .changed();
    });

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label(RichText::new("Seed Settings").strong());
    ui.add_space(8.0);

    ui.label("Master Seed");
    ui.horizontal(|ui| {
        result.settings_changed |= ui
            .add(egui::DragValue::new(&mut settings.seed).speed(1.0))
            .changed();
        if ui.button("Random").clicked() {
            settings.seed = seed::random_seed();
            result.settings_changed = true;
        }
    });

    ui.add_space(8.0);
    if ui.button("Reset").clicked() {
        result.reset_clicked = true;
    }

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label(RichText::new("Export").strong());
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        ui.label("Format");
        egui::ComboBox::from_id_salt("export_format")
            .selected_text(settings.export_format.label())
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut settings.export_format, ExportFormat::Png, ".png");
                ui.selectable_value(&mut settings.export_format, ExportFormat::Jpeg, ".jpeg");
                ui.selectable_value(&mut settings.export_format, ExportFormat::Webp, ".webp");
                ui.selectable_value(&mut settings.export_format, ExportFormat::Svg, ".svg");
                ui.selectable_value(
                    &mut settings.export_format,
                    ExportFormat::Folder,
                    "Folder (Composite)",
                );
            });
    });

    if settings.export_format == ExportFormat::Folder {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label("Mask Format");
            egui::ComboBox::from_id_salt("mask_format")
                .selected_text(settings.mask_format.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut settings.mask_format, MaskFormat::Png, ".png");
                    ui.selectable_value(&mut settings.mask_format, MaskFormat::Jpeg, ".jpeg");
                    ui.selectable_value(&mut settings.mask_format, MaskFormat::Webp, ".webp");
                });
        });
    }

    if settings.export_format.supports_resolution() {
        ui.add_space(8.0);
        ui.checkbox(
            &mut settings.allow_export_aspect_change,
            "Allow aspect ratio change",
        );

        let mut width_changed = false;
        let mut height_changed = false;

        ui.add_space(6.0);
        ui.label("Resolution Width");
        ui.horizontal(|ui| {
            width_changed |= ui
                .add(egui::Slider::new(&mut settings.export_width, 1..=10_000).show_value(false))
                .changed();
            width_changed |= ui
                .add(
                    egui::DragValue::new(&mut settings.export_width)
                        .speed(1.0)
                        .range(1..=10_000),
                )
                .changed();
        });

        ui.add_space(6.0);
        ui.label("Resolution Height");
        ui.horizontal(|ui| {
            height_changed |= ui
                .add(egui::Slider::new(&mut settings.export_height, 1..=10_000).show_value(false))
                .changed();
            height_changed |= ui
                .add(
                    egui::DragValue::new(&mut settings.export_height)
                        .speed(1.0)
                        .range(1..=10_000),
                )
                .changed();
        });

        if !settings.allow_export_aspect_change {
            let cols = settings.cols.max(1) as f32;
            let rows = settings.rows.max(1) as f32;
            let ratio = cols / rows;

            if width_changed {
                let h = ((settings.export_width as f32) / ratio).round() as u32;
                settings.export_height = h.clamp(1, 10_000);
            } else if height_changed {
                let w = ((settings.export_height as f32) * ratio).round() as u32;
                settings.export_width = w.clamp(1, 10_000);
            }
        }
    }

    ui.add_space(6.0);
    if ui.button("Export Image").clicked() {
        result.export_clicked = true;
    }
}

fn draw_layout_tab(ui: &mut egui::Ui, settings: &mut UiSettings, result: &mut SidePanelResult) {
    ui.label(RichText::new("Room Settings").strong());
    ui.add_space(8.0);

    ui.label("Room Count");
    ui.horizontal(|ui| {
        result.settings_changed |= ui
            .add(egui::Slider::new(&mut settings.room_count, 1..=40).show_value(false))
            .changed();
        result.settings_changed |= ui
            .add(
                egui::DragValue::new(&mut settings.room_count)
                    .speed(1.0)
                    .range(1..=200),
            )
            .changed();
    });

    ui.add_space(8.0);
    ui.label("Min Room Size");
    ui.horizontal(|ui| {
        result.settings_changed |= ui
            .add(egui::Slider::new(&mut settings.min_room_size, 2..=30).show_value(false))
            .changed();
        result.settings_changed |= ui
            .add(
                egui::DragValue::new(&mut settings.min_room_size)
                    .speed(1.0)
                    .range(2..=200),
            )
            .changed();
    });

    ui.add_space(8.0);
    ui.label("Max Room Size");
    ui.horizontal(|ui| {
        result.settings_changed |= ui
            .add(egui::Slider::new(&mut settings.max_room_size, 2..=30).show_value(false))
            .changed();
        result.settings_changed |= ui
            .add(
                egui::DragValue::new(&mut settings.max_room_size)
                    .speed(1.0)
                    .range(2..=200),
            )
            .changed();
    });

    ui.add_space(8.0);
    result.settings_changed |= ui
        .checkbox(&mut settings.square_rooms_only, "Square Rooms Only")
        .changed();

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label(RichText::new("Corridor Settings").strong());
    ui.add_space(8.0);

    ui.label("Corridor Randomness (%)");
    ui.horizontal(|ui| {
        result.settings_changed |= ui
            .add(egui::Slider::new(&mut settings.corridor_randomness, 0..=100).show_value(false))
            .changed();
        result.settings_changed |= ui
            .add(
                egui::DragValue::new(&mut settings.corridor_randomness)
                    .speed(1.0)
                    .range(0..=100),
            )
            .changed();
    });

    ui.add_space(8.0);
    ui.label("Min Corridor Width");
    ui.horizontal(|ui| {
        result.settings_changed |= ui
            .add(egui::Slider::new(&mut settings.min_corridor_width, 1..=10).show_value(false))
            .changed();
        result.settings_changed |= ui
            .add(
                egui::DragValue::new(&mut settings.min_corridor_width)
                    .speed(1.0)
                    .range(1..=50),
            )
            .changed();
    });

    ui.add_space(8.0);
    ui.label("Max Corridor Width");
    ui.horizontal(|ui| {
        result.settings_changed |= ui
            .add(egui::Slider::new(&mut settings.max_corridor_width, 1..=10).show_value(false))
            .changed();
        result.settings_changed |= ui
            .add(
                egui::DragValue::new(&mut settings.max_corridor_width)
                    .speed(1.0)
                    .range(1..=50),
            )
            .changed();
    });

    ui.add_space(8.0);
    ui.label("Dead-End Rooms (%)");
    ui.horizontal(|ui| {
        result.settings_changed |= ui
            .add(egui::Slider::new(&mut settings.dead_end_rooms_percent, 0..=50).show_value(false))
            .changed();
        result.settings_changed |= ui
            .add(
                egui::DragValue::new(&mut settings.dead_end_rooms_percent)
                    .speed(1.0)
                    .range(0..=50),
            )
            .changed();
    });

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label(RichText::new("Door Settings").strong());
    ui.add_space(8.0);

    ui.label("Door Frequency (%)");
    ui.horizontal(|ui| {
        result.settings_changed |= ui
            .add(egui::Slider::new(&mut settings.door_frequency_percent, 0..=100).show_value(false))
            .changed();
        result.settings_changed |= ui
            .add(
                egui::DragValue::new(&mut settings.door_frequency_percent)
                    .speed(1.0)
                    .range(0..=100),
            )
            .changed();
    });

    ui.add_space(8.0);
    ui.label("Room/Hallway Door Chance (%)");
    ui.horizontal(|ui| {
        result.settings_changed |= ui
            .add(
                egui::Slider::new(&mut settings.room_hallway_door_percent, 0..=100)
                    .show_value(false),
            )
            .changed();
        result.settings_changed |= ui
            .add(
                egui::DragValue::new(&mut settings.room_hallway_door_percent)
                    .speed(1.0)
                    .range(0..=100),
            )
            .changed();
    });

    ui.add_space(8.0);
    ui.label("Locked Door Chance (%)");
    ui.horizontal(|ui| {
        result.settings_changed |= ui
            .add(egui::Slider::new(&mut settings.locked_door_percent, 0..=100).show_value(false))
            .changed();
        result.settings_changed |= ui
            .add(
                egui::DragValue::new(&mut settings.locked_door_percent)
                    .speed(1.0)
                    .range(0..=100),
            )
            .changed();
    });

    ui.add_space(8.0);
    result.settings_changed |= ui
        .checkbox(
            &mut settings.allow_middle_corridor_doors,
            "Allow Middle Corridor Doors",
        )
        .changed();
}
