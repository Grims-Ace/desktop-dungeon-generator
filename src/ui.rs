use crate::seed;
use eframe::egui;
use egui::RichText;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum Tab {
    Generate,
    Layout,
    StartAndEnd,
    MonstersAndTraps,
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
    pub last_export_path: Option<String>,
    pub add_tool: AddTool,
    pub add_text_value: String,
    pub min_start_marker_size: usize,
    pub max_start_marker_size: usize,
    pub min_end_marker_size: usize,
    pub max_end_marker_size: usize,
    pub min_start_marker_count: usize,
    pub max_start_marker_count: usize,
    pub min_end_marker_count: usize,
    pub max_end_marker_count: usize,
    pub min_levels: usize,
    pub max_levels: usize,
    pub active_level_index: usize,
    pub trap_frequency_percent: usize,
    pub min_traps_per_area: usize,
    pub max_traps_per_area: usize,
    pub monster_frequency_percent: usize,
    pub min_monsters_per_area: usize,
    pub max_monsters_per_area: usize,
    pub min_stair_width: usize,
    pub max_stair_width: usize,
    pub min_stair_height: usize,
    pub max_stair_height: usize,
    pub min_stairs_per_level: usize,
    pub max_stairs_per_level: usize,
    pub sync_stairs_across_levels: bool,
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
            last_export_path: None,
            add_tool: AddTool::None,
            add_text_value: "Text".to_string(),
            min_start_marker_size: 1,
            max_start_marker_size: 1,
            min_end_marker_size: 1,
            max_end_marker_size: 1,
            min_start_marker_count: 1,
            max_start_marker_count: 1,
            min_end_marker_count: 1,
            max_end_marker_count: 1,
            min_levels: 1,
            max_levels: 1,
            active_level_index: 0,
            trap_frequency_percent: 20,
            min_traps_per_area: 1,
            max_traps_per_area: 3,
            monster_frequency_percent: 15,
            min_monsters_per_area: 1,
            max_monsters_per_area: 2,
            min_stair_width: 1,
            max_stair_width: 2,
            min_stair_height: 2,
            max_stair_height: 3,
            min_stairs_per_level: 1,
            max_stairs_per_level: 1,
            sync_stairs_across_levels: true,
            active_tab: Tab::Generate,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AddTool {
    None,
    Room,
    Corridor,
    Archway,
    Door,
    LockedDoor,
    SecretDoor,
    Text,
    StartMarker,
    EndMarker,
    TrapMarker,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SidePanelResult {
    pub settings_changed: bool,
    pub reset_clicked: bool,
    pub clear_clicked: bool,
    pub export_clicked: bool,
    pub save_clicked: bool,
    pub load_clicked: bool,
    pub new_level_clicked: bool,
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
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal(|ui| {
                    let is_generate = settings.active_tab == Tab::Generate;
                    let is_layout = settings.active_tab == Tab::Layout;

                    if ui.selectable_label(is_generate, "Generate").clicked() {
                        settings.active_tab = Tab::Generate;
                    }
                    if ui.selectable_label(is_layout, "Layout").clicked() {
                        settings.active_tab = Tab::Layout;
                    }
                    if ui
                        .selectable_label(settings.active_tab == Tab::StartAndEnd, "Start & End")
                        .clicked()
                    {
                        settings.active_tab = Tab::StartAndEnd;
                    }
                    if ui
                        .selectable_label(
                            settings.active_tab == Tab::MonstersAndTraps,
                            "Monsters/Traps",
                        )
                        .clicked()
                    {
                        settings.active_tab = Tab::MonstersAndTraps;
                    }
                    if ui
                        .selectable_label(settings.active_tab == Tab::Add, "Add")
                        .clicked()
                    {
                        settings.active_tab = Tab::Add;
                    }
                });
                ui.separator();

                match settings.active_tab {
                    Tab::Generate => {
                        draw_generate_tab(ui, settings, &mut result, export_in_progress)
                    }
                    Tab::Layout => draw_layout_tab(ui, settings, &mut result),
                    Tab::StartAndEnd => draw_start_and_end_tab(ui, settings, &mut result),
                    Tab::MonstersAndTraps => draw_monsters_and_traps_tab(ui, settings, &mut result),
                    Tab::Add => draw_add_tab(ui, settings, &mut result),
                }
            });
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
    ui.label(RichText::new("Dungeon Files").strong());
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        if ui.button("Save State").clicked() {
            result.save_clicked = true;
        }
        if ui.button("Load State").clicked() {
            result.load_clicked = true;
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
    let export_button = ui.add_enabled(!export_in_progress, egui::Button::new("Export Image"));
    if export_button.clicked() {
        result.export_clicked = true;
    }

    if export_in_progress {
        ui.add_space(6.0);
        ui.add(egui::ProgressBar::new(0.0).animate(true));
    }
}

// Render the Layout tab controls.
fn draw_layout_tab(ui: &mut egui::Ui, settings: &mut UiSettings, result: &mut SidePanelResult) {
    ui.label(RichText::new("Levels").strong());
    ui.add_space(8.0);

    ui.label("Min Levels");
    ui.horizontal(|ui| {
        result.settings_changed |= ui
            .add(egui::Slider::new(&mut settings.min_levels, 1..=20).show_value(false))
            .changed();
        result.settings_changed |= ui
            .add(
                egui::DragValue::new(&mut settings.min_levels)
                    .speed(1.0)
                    .range(1..=100),
            )
            .changed();
    });

    ui.add_space(8.0);
    ui.label("Max Levels");
    ui.horizontal(|ui| {
        result.settings_changed |= ui
            .add(egui::Slider::new(&mut settings.max_levels, 1..=20).show_value(false))
            .changed();
        result.settings_changed |= ui
            .add(
                egui::DragValue::new(&mut settings.max_levels)
                    .speed(1.0)
                    .range(1..=100),
            )
            .changed();
    });

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
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

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label(RichText::new("Staircase Settings").strong());
    ui.add_space(8.0);

    result.settings_changed |= ui
        .checkbox(
            &mut settings.sync_stairs_across_levels,
            "Sync Stairs Across All Levels",
        )
        .changed();

    ui.add_space(8.0);
    ui.label("Min Stair Width");
    ui.horizontal(|ui| {
        result.settings_changed |= ui
            .add(egui::Slider::new(&mut settings.min_stair_width, 1..=5).show_value(false))
            .changed();
        result.settings_changed |= ui
            .add(
                egui::DragValue::new(&mut settings.min_stair_width)
                    .speed(1.0)
                    .range(1..=10),
            )
            .changed();
    });

    ui.add_space(8.0);
    ui.label("Max Stair Width");
    ui.horizontal(|ui| {
        result.settings_changed |= ui
            .add(egui::Slider::new(&mut settings.max_stair_width, 1..=5).show_value(false))
            .changed();
        result.settings_changed |= ui
            .add(
                egui::DragValue::new(&mut settings.max_stair_width)
                    .speed(1.0)
                    .range(1..=10),
            )
            .changed();
    });

    ui.add_space(8.0);
    ui.label("Min Stair Height");
    ui.horizontal(|ui| {
        result.settings_changed |= ui
            .add(egui::Slider::new(&mut settings.min_stair_height, 1..=5).show_value(false))
            .changed();
        result.settings_changed |= ui
            .add(
                egui::DragValue::new(&mut settings.min_stair_height)
                    .speed(1.0)
                    .range(1..=10),
            )
            .changed();
    });

    ui.add_space(8.0);
    ui.label("Max Stair Height");
    ui.horizontal(|ui| {
        result.settings_changed |= ui
            .add(egui::Slider::new(&mut settings.max_stair_height, 1..=5).show_value(false))
            .changed();
        result.settings_changed |= ui
            .add(
                egui::DragValue::new(&mut settings.max_stair_height)
                    .speed(1.0)
                    .range(1..=10),
            )
            .changed();
    });

    ui.add_space(8.0);
    ui.label("Min Stairs Per Level");
    ui.horizontal(|ui| {
        result.settings_changed |= ui
            .add(egui::Slider::new(&mut settings.min_stairs_per_level, 0..=10).show_value(false))
            .changed();
        result.settings_changed |= ui
            .add(
                egui::DragValue::new(&mut settings.min_stairs_per_level)
                    .speed(1.0)
                    .range(0..=20),
            )
            .changed();
    });

    ui.add_space(8.0);
    ui.label("Max Stairs Per Level");
    ui.horizontal(|ui| {
        result.settings_changed |= ui
            .add(egui::Slider::new(&mut settings.max_stairs_per_level, 0..=10).show_value(false))
            .changed();
        result.settings_changed |= ui
            .add(
                egui::DragValue::new(&mut settings.max_stairs_per_level)
                    .speed(1.0)
                    .range(0..=20),
            )
            .changed();
    });
}

// Render the Add tab controls.
fn draw_add_tab(ui: &mut egui::Ui, settings: &mut UiSettings, result: &mut SidePanelResult) {
    ui.label(RichText::new("Add Tools").strong());
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        ui.selectable_value(&mut settings.add_tool, AddTool::Room, "Add Room");
        ui.selectable_value(&mut settings.add_tool, AddTool::Corridor, "Add Corridor");
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.selectable_value(&mut settings.add_tool, AddTool::Archway, "Add Archway");
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
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.selectable_value(&mut settings.add_tool, AddTool::Text, "Add Text");
        ui.selectable_value(&mut settings.add_tool, AddTool::StartMarker, "Place Start");
        ui.selectable_value(&mut settings.add_tool, AddTool::EndMarker, "Place End");
        ui.selectable_value(&mut settings.add_tool, AddTool::TrapMarker, "Place Trap");
    });
    ui.add_space(4.0);
    ui.label("Left click to place, Right click/Escape to cancel");

    ui.add_space(8.0);
    if ui.button("Add New Level").clicked() {
        result.new_level_clicked = true;
    }

    ui.add_space(6.0);
    match settings.add_tool {
        AddTool::Room => ui.label("Click to place a room at the grid cell."),
        AddTool::Corridor => ui.label("Click-drag to place a corridor between two cells."),
        AddTool::Archway => ui.label("Click a valid room or corridor edge to place an archway."),
        AddTool::Door => ui.label("Click a room/corridor edge to place a door."),
        AddTool::LockedDoor => ui.label("Click a room/corridor edge to place a locked door."),
        AddTool::SecretDoor => ui.label("Click a room/corridor edge to place a secret door."),
        AddTool::Text => {
            ui.label("Enter the label text, then click a grid cell to place it.");
            ui.add_space(6.0);
            ui.label("Text");
            let response = ui.text_edit_singleline(&mut settings.add_text_value);
            ui.add_space(4.0);
            ui.label("Note: Left click on the canvas to place the text.");
            response
        }
        AddTool::StartMarker => ui.label("Click the grid to place the start marker."),
        AddTool::EndMarker => ui.label("Click the grid to place the end marker."),
        AddTool::TrapMarker => ui.label("Click the grid to place a trap marker."),
        AddTool::None => ui.label("Select a tool to add rooms or corridors."),
    };
}

fn draw_start_and_end_tab(
    ui: &mut egui::Ui,
    settings: &mut UiSettings,
    result: &mut SidePanelResult,
) {
    ui.label(RichText::new("Start Markers").strong());
    ui.add_space(8.0);
    draw_usize_slider_row(
        ui,
        result,
        "Min Start Marker Size",
        &mut settings.min_start_marker_size,
        1..=10,
        1..=10,
    );
    draw_usize_slider_row(
        ui,
        result,
        "Max Start Marker Size",
        &mut settings.max_start_marker_size,
        1..=10,
        1..=10,
    );
    draw_usize_slider_row(
        ui,
        result,
        "Min Start Marker Count",
        &mut settings.min_start_marker_count,
        1..=20,
        1..=100,
    );
    draw_usize_slider_row(
        ui,
        result,
        "Max Start Marker Count",
        &mut settings.max_start_marker_count,
        1..=20,
        1..=100,
    );

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label(RichText::new("End Markers").strong());
    ui.add_space(8.0);
    draw_usize_slider_row(
        ui,
        result,
        "Min End Marker Size",
        &mut settings.min_end_marker_size,
        1..=10,
        1..=10,
    );
    draw_usize_slider_row(
        ui,
        result,
        "Max End Marker Size",
        &mut settings.max_end_marker_size,
        1..=10,
        1..=10,
    );
    draw_usize_slider_row(
        ui,
        result,
        "Min End Marker Count",
        &mut settings.min_end_marker_count,
        1..=20,
        1..=100,
    );
    draw_usize_slider_row(
        ui,
        result,
        "Max End Marker Count",
        &mut settings.max_end_marker_count,
        1..=20,
        1..=100,
    );

    ui.add_space(8.0);
    ui.label("Markers are generated inside rooms. Matching start/end indices are paired as far apart as possible.");
}

fn draw_monsters_and_traps_tab(
    ui: &mut egui::Ui,
    settings: &mut UiSettings,
    result: &mut SidePanelResult,
) {
    ui.label(RichText::new("Trap Settings").strong());
    ui.add_space(8.0);
    draw_usize_slider_row(
        ui,
        result,
        "Trap Frequency (%)",
        &mut settings.trap_frequency_percent,
        0..=100,
        0..=100,
    );
    draw_usize_slider_row(
        ui,
        result,
        "Min Traps Per Room/Corridor",
        &mut settings.min_traps_per_area,
        0..=20,
        0..=100,
    );
    draw_usize_slider_row(
        ui,
        result,
        "Max Traps Per Room/Corridor",
        &mut settings.max_traps_per_area,
        0..=20,
        0..=100,
    );

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label(RichText::new("Monster Settings").strong());
    ui.add_space(8.0);
    draw_usize_slider_row(
        ui,
        result,
        "Monster Frequency (%)",
        &mut settings.monster_frequency_percent,
        0..=100,
        0..=100,
    );
    draw_usize_slider_row(
        ui,
        result,
        "Min Monsters Per Room/Corridor",
        &mut settings.min_monsters_per_area,
        0..=20,
        0..=100,
    );
    draw_usize_slider_row(
        ui,
        result,
        "Max Monsters Per Room/Corridor",
        &mut settings.max_monsters_per_area,
        0..=20,
        0..=100,
    );
}

fn draw_usize_slider_row(
    ui: &mut egui::Ui,
    result: &mut SidePanelResult,
    label: &str,
    value: &mut usize,
    slider_range: std::ops::RangeInclusive<usize>,
    drag_range: std::ops::RangeInclusive<usize>,
) {
    ui.label(label);
    ui.horizontal(|ui| {
        result.settings_changed |= ui
            .add(egui::Slider::new(value, slider_range).show_value(false))
            .changed();
        result.settings_changed |= ui
            .add(egui::DragValue::new(value).speed(1.0).range(drag_range))
            .changed();
    });
    ui.add_space(6.0);
}
