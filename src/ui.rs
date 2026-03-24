use crate::seed;
use eframe::egui;
use egui::RichText;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum Tab {
    Generate,
    Layout,
    Add,
}

impl Default for Tab {
    // Start with the Generate tab selected.
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
    // Default export format is PNG.
    fn default() -> Self {
        Self::Png
    }
}

impl ExportFormat {
    // Return the file extension for this export format.
    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Png => "png",
            ExportFormat::Jpeg => "jpeg",
            ExportFormat::Webp => "webp",
            ExportFormat::Svg => "svg",
            ExportFormat::Folder => "folder",
        }
    }

    // Return the UI label for this export format.
    pub fn label(self) -> &'static str {
        match self {
            ExportFormat::Png => ".png",
            ExportFormat::Jpeg => ".jpeg",
            ExportFormat::Webp => ".webp",
            ExportFormat::Svg => ".svg",
            ExportFormat::Folder => "Folder (Composite)",
        }
    }

    // Report whether this export format supports a raster resolution.
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
    // Default mask format is PNG.
    fn default() -> Self {
        Self::Png
    }
}

impl MaskFormat {
    // Return the file extension for this mask format.
    pub fn extension(self) -> &'static str {
        match self {
            MaskFormat::Png => "png",
            MaskFormat::Jpeg => "jpeg",
            MaskFormat::Webp => "webp",
        }
    }

    // Return the UI label for this mask format.
    pub fn label(self) -> &'static str {
        match self {
            MaskFormat::Png => ".png",
            MaskFormat::Jpeg => ".jpeg",
            MaskFormat::Webp => ".webp",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    pub pack_rooms_without_corridors: bool,
    pub door_frequency_percent: usize,
    pub room_hallway_door_percent: usize,
    pub locked_door_percent: usize,
    pub secret_door_percent: usize,
    pub allow_middle_corridor_doors: bool,
    pub windows_enabled: bool,
    pub min_window_width: usize,
    pub max_window_width: usize,
    pub window_frequency_percent: usize,
    pub room_hallway_window_percent: usize,
    pub allow_internal_windows: bool,
    pub colorblind_mode: bool,
    pub export_format: ExportFormat,
    pub mask_format: MaskFormat,
    pub export_width: u32,
    pub export_height: u32,
    pub allow_export_aspect_change: bool,
    pub export_show_grid: bool,
    pub add_tool: AddTool,
    active_tab: Tab,
}

impl Default for UiSettings {
    // Provide initial UI settings for a fresh session.
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
            pack_rooms_without_corridors: false,
            door_frequency_percent: 30,
            room_hallway_door_percent: 70,
            locked_door_percent: 25,
            secret_door_percent: 10,
            allow_middle_corridor_doors: false,
            windows_enabled: false,
            min_window_width: 1,
            max_window_width: 2,
            window_frequency_percent: 30,
            room_hallway_window_percent: 40,
            allow_internal_windows: false,
            colorblind_mode: false,
            export_format: ExportFormat::Png,
            mask_format: MaskFormat::Png,
            export_width: 1920,
            export_height: 1080,
            allow_export_aspect_change: false,
            export_show_grid: true,
            add_tool: AddTool::None,
            active_tab: Tab::Generate,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AddTool {
    None,
    Room,
    Corridor,
    Door,
    LockedDoor,
    SecretDoor,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SidePanelResult {
    pub settings_changed: bool,
    pub reset_clicked: bool,
    pub clear_clicked: bool,
    pub export_clicked: bool,
}

pub fn draw_side_panel(
    ctx: &egui::Context,
    settings: &mut UiSettings,
    export_in_progress: bool,
) -> SidePanelResult {
    // Render the left side panel and return user interaction results.
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
                let is_add = settings.active_tab == Tab::Add;

                if ui.selectable_label(is_generate, "Generate").clicked() {
                    settings.active_tab = Tab::Generate;
                }
                if ui.selectable_label(is_layout, "Layout").clicked() {
                    settings.active_tab = Tab::Layout;
                }
                if ui.selectable_label(is_add, "Add").clicked() {
                    settings.active_tab = Tab::Add;
                }
            });
            ui.separator();

            match settings.active_tab {
                Tab::Generate => draw_generate_tab(ui, settings, &mut result, export_in_progress),
                Tab::Layout => draw_layout_tab(ui, settings, &mut result),
                Tab::Add => draw_add_tab(ui, settings),
            }
        });

    result
}

// Render the Generate tab controls.
fn draw_generate_tab(
    ui: &mut egui::Ui,
    settings: &mut UiSettings,
    result: &mut SidePanelResult,
    export_in_progress: bool,
) {
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
    ui.horizontal(|ui| {
        if ui.button("Reset").clicked() {
            result.reset_clicked = true;
        }
        if ui.button("Clear").clicked() {
            result.clear_clicked = true;
        }
    });

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

    ui.add_space(6.0);
    result.settings_changed |= ui
        .checkbox(&mut settings.export_show_grid, "Include Grid In Export")
        .changed();

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

    if export_in_progress {
        ui.add_space(6.0);
        ui.add(egui::ProgressBar::new(0.0).animate(true));
    }
}

// Render the Layout tab controls.
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
    result.settings_changed |= ui
        .checkbox(
            &mut settings.pack_rooms_without_corridors,
            "Pack Rooms Together (No Corridors)",
        )
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
    ui.label("Secret Door Chance (%)");
    ui.horizontal(|ui| {
        result.settings_changed |= ui
            .add(egui::Slider::new(&mut settings.secret_door_percent, 0..=100).show_value(false))
            .changed();
        result.settings_changed |= ui
            .add(
                egui::DragValue::new(&mut settings.secret_door_percent)
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

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label(RichText::new("Window Settings").strong());
    ui.add_space(8.0);

    result.settings_changed |= ui
        .checkbox(&mut settings.windows_enabled, "Enable Windows")
        .changed();

    if settings.windows_enabled {
        ui.add_space(8.0);
        ui.label("Min Window Width");
        ui.horizontal(|ui| {
            result.settings_changed |= ui
                .add(egui::Slider::new(&mut settings.min_window_width, 1..=5).show_value(false))
                .changed();
            result.settings_changed |= ui
                .add(
                    egui::DragValue::new(&mut settings.min_window_width)
                        .speed(1.0)
                        .range(1..=5),
                )
                .changed();
        });

        ui.add_space(8.0);
        ui.label("Max Window Width");
        ui.horizontal(|ui| {
            result.settings_changed |= ui
                .add(egui::Slider::new(&mut settings.max_window_width, 1..=5).show_value(false))
                .changed();
            result.settings_changed |= ui
                .add(
                    egui::DragValue::new(&mut settings.max_window_width)
                        .speed(1.0)
                        .range(1..=5),
                )
                .changed();
        });

        ui.add_space(8.0);
        ui.label("Window Frequency (%)");
        ui.horizontal(|ui| {
            result.settings_changed |= ui
                .add(
                    egui::Slider::new(&mut settings.window_frequency_percent, 0..=100)
                        .show_value(false),
                )
                .changed();
            result.settings_changed |= ui
                .add(
                    egui::DragValue::new(&mut settings.window_frequency_percent)
                        .speed(1.0)
                        .range(0..=100),
                )
                .changed();
        });

        ui.add_space(8.0);
        ui.label("Internal Window Chance (%)");
        ui.horizontal(|ui| {
            result.settings_changed |= ui
                .add(
                    egui::Slider::new(&mut settings.room_hallway_window_percent, 0..=100)
                        .show_value(false),
                )
                .changed();
            result.settings_changed |= ui
                .add(
                    egui::DragValue::new(&mut settings.room_hallway_window_percent)
                        .speed(1.0)
                        .range(0..=100),
                )
                .changed();
        });

        ui.add_space(8.0);
        result.settings_changed |= ui
            .checkbox(
                &mut settings.allow_internal_windows,
                "Allow Internal Windows",
            )
            .changed();
    }
}

// Render the Add tab controls.
fn draw_add_tab(ui: &mut egui::Ui, settings: &mut UiSettings) {
    ui.label(RichText::new("Add Tools").strong());
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        ui.selectable_value(&mut settings.add_tool, AddTool::Room, "Add Room");
        ui.selectable_value(&mut settings.add_tool, AddTool::Corridor, "Add Corridor");
        ui.selectable_value(&mut settings.add_tool, AddTool::Door, "Add Door");
        ui.selectable_value(
            &mut settings.add_tool,
            AddTool::LockedDoor,
            "Add Locked Door",
        );
        ui.selectable_value(
            &mut settings.add_tool,
            AddTool::SecretDoor,
            "Add Secret Door",
        );
    });

    ui.add_space(6.0);
    match settings.add_tool {
        AddTool::Room => ui.label("Click to place a room at the grid cell."),
        AddTool::Corridor => ui.label("Click-drag to place a corridor between two cells."),
        AddTool::Door => ui.label("Click a room/corridor edge to place a door."),
        AddTool::LockedDoor => ui.label("Click a room/corridor edge to place a locked door."),
        AddTool::SecretDoor => ui.label("Click a room/corridor edge to place a secret door."),
        AddTool::None => ui.label("Select a tool to add rooms or corridors."),
    };
}
