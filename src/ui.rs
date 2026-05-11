use crate::seed;
use eframe::egui;
use egui::{Color32, RichText, Stroke};
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
    pub composition_mode: bool,
    pub export_format: ExportFormat,
    pub mask_format: MaskFormat,
    pub export_width: u32,
    pub export_height: u32,
    pub allow_export_aspect_change: bool,
    pub export_show_grid: bool,
    pub last_export_path: Option<String>,
    pub add_tool: AddTool,
    pub add_text_value: String,
    pub add_text_font_size: u16,
    pub add_room_width: usize,
    pub add_room_height: usize,
    pub add_corridor_width: usize,
    pub add_stair_width: usize,
    pub add_stair_height: usize,
    pub add_marker_size: usize,
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
    pub export_level_index: usize,
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
    fn default() -> Self {
        UiSettings {
            seed: 0,
            cols: 32,
            rows: 32,
            room_count: 8,
            min_room_size: 3,
            max_room_size: 8,
            square_rooms_only: true,
            min_corridor_width: 1,
            max_corridor_width: 3,
            corridor_randomness: 50,
            dead_end_rooms_percent: 20,
            pack_rooms_without_corridors: false,
            door_frequency_percent: 50,
            room_hallway_door_percent: 20,
            locked_door_percent: 10,
            secret_door_percent: 5,
            allow_middle_corridor_doors: false,
            windows_enabled: false,
            min_window_width: 2,
            max_window_width: 4,
            window_frequency_percent: 30,
            room_hallway_window_percent: 10,
            allow_internal_windows: false,
            colorblind_mode: false,
            composition_mode: false,
            export_format: ExportFormat::Png,
            mask_format: MaskFormat::Png,
            export_width: 0,
            export_height: 0,
            allow_export_aspect_change: true,
            export_show_grid: false,
            last_export_path: None,
            add_tool: AddTool::None,
            add_text_value: String::from("Text"),
            add_text_font_size: 18,
            add_room_width: 5,
            add_room_height: 5,
            add_corridor_width: 2,
            add_stair_width: 2,
            add_stair_height: 2,
            add_marker_size: 1,
            min_start_marker_size: 1,
            max_start_marker_size: 3,
            min_end_marker_size: 1,
            max_end_marker_size: 3,
            min_start_marker_count: 1,
            max_start_marker_count: 3,
            min_end_marker_count: 1,
            max_end_marker_count: 3,
            min_levels: 1,
            max_levels: 1,
            active_level_index: 0,
            trap_frequency_percent: 30,
            min_traps_per_area: 1,
            max_traps_per_area: 3,
            monster_frequency_percent: 30,
            min_monsters_per_area: 1,
            max_monsters_per_area: 3,
            min_stair_width: 2,
            max_stair_width: 3,
            min_stair_height: 2,
            max_stair_height: 3,
            min_stairs_per_level: 1,
            max_stairs_per_level: 2,
            sync_stairs_across_levels: false,
            export_level_index: 0,
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
    MonsterMarker,
    Staircase,
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
    export_progress: Option<(usize, usize)>,
    num_levels: usize,
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
                        draw_generate_tab(ui, settings, &mut result, export_progress, num_levels)
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

// Draw mode controls and the legend panel.
pub fn draw_legend_panel(ctx: &egui::Context, settings: &mut UiSettings) -> bool {
    let mut composition_mode_changed = false;

    egui::SidePanel::right("legend_panel")
        .resizable(false)
        .min_width(180.0)
        .default_width(200.0)
        .show_separator_line(true)
        .show(ctx, |ui| {
            ui.heading("Modes");
            ui.add_space(6.0);
            ui.checkbox(&mut settings.colorblind_mode, "Colorblind Mode");
            composition_mode_changed = ui
                .checkbox(&mut settings.composition_mode, "Composition Mode")
                .changed();
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(8.0);
            ui.heading("Legend");
            ui.add_space(8.0);
            if settings.colorblind_mode {
                draw_legend_entry_colorblind(ui, "Rooms", LegendStyle::Crosshatch);
                draw_legend_entry_colorblind(ui, "Corridors", LegendStyle::Dots);
                draw_legend_entry_colorblind(ui, "Walls", LegendStyle::SolidLine);
                draw_legend_entry_colorblind(ui, "Doors", LegendStyle::LongDash);
                draw_legend_entry_colorblind(ui, "Locked Doors", LegendStyle::ShortDash);
                draw_legend_entry_colorblind(ui, "Secret Doors", LegendStyle::DashDotDotLine);
                draw_legend_entry_colorblind(ui, "Archways", LegendStyle::DottedLine);
                draw_legend_entry_colorblind(ui, "Windows", LegendStyle::DashDotLine);
                draw_legend_entry_colorblind(ui, "Stairs", LegendStyle::LabelChar('\u{1F5CF}'));
                draw_legend_entry_colorblind(ui, "Start Marker", LegendStyle::Label("S"));
                draw_legend_entry_colorblind(ui, "End Marker", LegendStyle::Label("E"));
                draw_legend_entry_colorblind(ui, "Trap", LegendStyle::Label("T"));
                draw_legend_entry_colorblind(ui, "Monster", LegendStyle::Label("M"));
            } else {
                draw_legend_entry(ui, Color32::from_rgb(70, 120, 160), "Rooms");
                draw_legend_entry(ui, Color32::from_rgb(210, 190, 120), "Corridors");
                draw_legend_entry(ui, Color32::BLACK, "Walls");
                draw_legend_entry(ui, Color32::from_rgb(80, 200, 120), "Doors");
                draw_legend_entry(ui, Color32::from_rgb(220, 70, 70), "Locked Doors");
                draw_legend_entry(ui, Color32::from_rgb(170, 80, 170), "Secret Doors");
                draw_legend_entry(ui, Color32::from_rgb(230, 140, 60), "Archways");
                draw_legend_entry(ui, Color32::from_rgb(70, 130, 220), "Windows");
                draw_legend_entry(ui, Color32::from_rgb(200, 150, 50), "Stairs");
                draw_legend_entry(ui, Color32::from_rgb(60, 220, 200), "Start Marker");
                draw_legend_entry(ui, Color32::from_rgb(240, 90, 90), "End Marker");
                draw_legend_entry(ui, Color32::from_rgb(150, 80, 230), "Trap");
                draw_legend_entry(ui, Color32::from_rgb(200, 50, 50), "Monster");
            }
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(8.0);
            ui.heading("Keybinds");
            ui.add_space(6.0);
            ui.label("• Delete / Backspace: Remove hovered room, corridor, door, text, or marker");
            ui.label("• Ctrl+Z: Undo");
            ui.label("• Ctrl+Y: Redo");
            ui.label("• Left click + drag: Move hovered room");
            ui.label("• Right click: Cancel add tool");
            ui.label("• Right click + drag: Resize hovered room");
        });

    composition_mode_changed
}

// Draw level tabs and return the hovered tab index.
pub fn draw_level_tabs(
    ui: &mut egui::Ui,
    level_count: usize,
    active_level_index: &mut usize,
) -> Option<usize> {
    let mut hovered_level_idx = None;
    ui.horizontal(|ui| {
        for level_idx in 0..level_count {
            let label = format!("Level {}", level_idx + 1);
            let is_active = *active_level_index == level_idx;
            let response = ui.selectable_label(is_active, label);
            if response.hovered() {
                hovered_level_idx = Some(level_idx);
            }
            if response.clicked() {
                *active_level_index = level_idx;
            }
        }
    });
    hovered_level_idx
}

// Render the Generate tab controls.
fn draw_generate_tab(
    ui: &mut egui::Ui,
    settings: &mut UiSettings,
    result: &mut SidePanelResult,
    export_progress: Option<(usize, usize)>,
    num_levels: usize,
) {
    let export_in_progress = export_progress.is_some();
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

    ui.add_space(4.0);
    let level_count = num_levels;
    ui.horizontal(|ui| {
        ui.label("Level");
        let selected_text = if settings.export_level_index == 999 {
            "All Levels".to_string()
        } else {
            format!("Level {}", settings.export_level_index + 1)
        };
        egui::ComboBox::from_id_salt("export_level")
            .selected_text(selected_text)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut settings.export_level_index, 999, "All Levels");
                for i in 0..level_count {
                    ui.selectable_value(
                        &mut settings.export_level_index,
                        i,
                        format!("Level {}", i + 1),
                    );
                }
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
        let (completed, total) = export_progress.unwrap_or((0, 1));
        let total = total.max(1);
        let fraction = (completed as f32 / total as f32).clamp(0.0, 1.0);
        ui.add(
            egui::ProgressBar::new(fraction)
                .show_percentage()
                .text(format!("{completed}/{total} images")),
        );
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
    ui.label(RichText::new("Staircase/Elevator Settings").strong());
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

    ui.label(RichText::new("Structure").strong());
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.selectable_value(&mut settings.add_tool, AddTool::Room, "Room");
        ui.selectable_value(&mut settings.add_tool, AddTool::Corridor, "Corridor");
        ui.selectable_value(&mut settings.add_tool, AddTool::Staircase, "Stairs");
    });
    if ui.button("Add New Level").clicked() {
        result.new_level_clicked = true;
    }

    ui.add_space(8.0);
    ui.label(RichText::new("Passages").strong());
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        ui.selectable_value(&mut settings.add_tool, AddTool::Archway, "Archway");
        ui.selectable_value(&mut settings.add_tool, AddTool::Door, "Door");
        ui.selectable_value(&mut settings.add_tool, AddTool::LockedDoor, "Locked");
        ui.selectable_value(&mut settings.add_tool, AddTool::SecretDoor, "Secret");
    });

    ui.add_space(8.0);
    ui.label(RichText::new("Markers & Annotation").strong());
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        ui.selectable_value(&mut settings.add_tool, AddTool::StartMarker, "Start");
        ui.selectable_value(&mut settings.add_tool, AddTool::EndMarker, "End");
        ui.selectable_value(&mut settings.add_tool, AddTool::TrapMarker, "Trap");
        ui.selectable_value(&mut settings.add_tool, AddTool::MonsterMarker, "Monster");
        ui.selectable_value(&mut settings.add_tool, AddTool::Text, "Text");
    });

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);

    if settings.add_tool != AddTool::None {
        ui.label(RichText::new("Tool Options").strong());
        ui.add_space(4.0);

        match settings.add_tool {
            AddTool::Room => {
                ui.label("Room Width");
                ui.add(egui::Slider::new(&mut settings.add_room_width, 1..=30));
                ui.label("Room Height");
                ui.add(egui::Slider::new(&mut settings.add_room_height, 1..=30));
            }
            AddTool::Corridor => {
                ui.label("Corridor Width");
                ui.add(egui::Slider::new(&mut settings.add_corridor_width, 1..=10));
            }
            AddTool::Staircase => {
                ui.label("Stair Width");
                ui.add(egui::Slider::new(&mut settings.add_stair_width, 1..=10));
                ui.label("Stair Height");
                ui.add(egui::Slider::new(&mut settings.add_stair_height, 1..=10));
            }
            AddTool::StartMarker
            | AddTool::EndMarker
            | AddTool::TrapMarker
            | AddTool::MonsterMarker => {
                ui.label("Marker Size");
                ui.add(egui::Slider::new(&mut settings.add_marker_size, 1..=10));
            }
            AddTool::Text => {
                ui.label("Text Label");
                ui.text_edit_singleline(&mut settings.add_text_value);
                ui.label("Font Size");
                ui.add(egui::Slider::new(&mut settings.add_text_font_size, 8..=96));
            }
            _ => {
                ui.label("No additional options for this tool.");
            }
        }
        ui.add_space(8.0);
        ui.label(RichText::new("Instructions:").italics());
        match settings.add_tool {
            AddTool::Room => {
                ui.label("Click to place a room at the grid cell.");
            }
            AddTool::Corridor => {
                ui.label("Click-drag to place a corridor between two cells.");
            }
            AddTool::Staircase => {
                ui.label(
                    "Click the grid to place stairs (visible on current and adjacent levels).",
                );
            }
            AddTool::Archway => {
                ui.label("Click a valid room or corridor edge to place an archway.");
            }
            AddTool::Door => {
                ui.label("Click a room/corridor edge to place a door.");
            }
            AddTool::LockedDoor => {
                ui.label("Click a room/corridor edge to place a locked door.");
            }
            AddTool::SecretDoor => {
                ui.label("Click a room/corridor edge to place a secret door.");
            }
            AddTool::Text => {
                ui.label("Click a grid cell to place the text label.");
            }
            AddTool::StartMarker => {
                ui.label("Click the grid to place the start marker.");
            }
            AddTool::EndMarker => {
                ui.label("Click the grid to place the end marker.");
            }
            AddTool::TrapMarker => {
                ui.label("Click the grid to place a trap marker.");
            }
            AddTool::MonsterMarker => {
                ui.label("Click the grid to place a monster marker.");
            }
            _ => {}
        }
        ui.add_space(4.0);
        ui.label("Left click to place, Right click/Escape to cancel.");
    } else {
        ui.label("Select a tool above to start building manually.");
    }
}

// Draw a colored legend entry.
fn draw_legend_entry(ui: &mut egui::Ui, color: Color32, label: &str) {
    ui.horizontal(|ui| {
        let (rect, _resp) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 0.0, color);
        ui.painter().rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0, Color32::WHITE),
            egui::StrokeKind::Middle,
        );
        ui.add_space(6.0);
        ui.label(label);
    });
    ui.add_space(4.0);
}

#[derive(Debug, Clone, Copy)]
enum LegendStyle {
    Crosshatch,
    Dots,
    SolidLine,
    LongDash,
    ShortDash,
    DottedLine,
    DashDotLine,
    DashDotDotLine,
    Label(&'static str),
    LabelChar(char),
}

// Draw a patterned legend entry.
fn draw_legend_entry_colorblind(ui: &mut egui::Ui, label: &str, style: LegendStyle) {
    ui.horizontal(|ui| {
        let (rect, _resp) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 0.0, Color32::from_gray(220));
        ui.painter().rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0, Color32::BLACK),
            egui::StrokeKind::Middle,
        );

        match style {
            LegendStyle::Crosshatch => {
                ui.painter().line_segment(
                    [rect.left_top(), rect.right_bottom()],
                    Stroke::new(1.2, Color32::BLACK),
                );
                ui.painter().line_segment(
                    [rect.right_top(), rect.left_bottom()],
                    Stroke::new(1.2, Color32::BLACK),
                );
            }
            LegendStyle::Dots => {
                ui.painter()
                    .circle_filled(rect.center(), 2.0, Color32::BLACK);
                ui.painter().circle_filled(
                    egui::pos2(rect.center().x - 4.0, rect.center().y),
                    1.4,
                    Color32::BLACK,
                );
                ui.painter().circle_filled(
                    egui::pos2(rect.center().x + 4.0, rect.center().y),
                    1.4,
                    Color32::BLACK,
                );
            }
            LegendStyle::SolidLine => {
                ui.painter().line_segment(
                    [
                        egui::pos2(rect.left() + 1.0, rect.center().y),
                        egui::pos2(rect.right() - 1.0, rect.center().y),
                    ],
                    Stroke::new(2.0, Color32::BLACK),
                );
            }
            LegendStyle::Label(prefix) => {
                ui.painter()
                    .circle_filled(rect.center(), 6.0, Color32::from_gray(180));
                ui.painter()
                    .circle_stroke(rect.center(), 6.0, Stroke::new(1.0, Color32::BLACK));
                let font_size = 10.0;
                let font_id = egui::FontId::monospace(font_size);
                let text_width = prefix.len() as f32 * font_size * 0.6;
                let text_height = font_size * 0.8;
                let text_pos = egui::pos2(
                    rect.center().x - text_width / 2.0,
                    rect.center().y - text_height / 2.0 + font_size / 5.0,
                );
                ui.painter().text(
                    text_pos,
                    egui::Align2::LEFT_TOP,
                    prefix,
                    font_id,
                    Color32::BLACK,
                );
            }
            LegendStyle::LabelChar(character) => {
                ui.painter()
                    .circle_filled(rect.center(), 6.0, Color32::from_gray(180));
                ui.painter()
                    .circle_stroke(rect.center(), 6.0, Stroke::new(1.0, Color32::BLACK));
                let font_size = 10.0;
                let font_id = egui::FontId::monospace(font_size);
                let text_width = font_size * 0.6;
                let text_height = font_size * 0.8;
                let text_pos = egui::pos2(
                    rect.center().x - text_width / 2.0,
                    rect.center().y - text_height / 2.0 + font_size / 5.0,
                );
                ui.painter().text(
                    text_pos,
                    egui::Align2::LEFT_TOP,
                    character,
                    font_id,
                    Color32::BLACK,
                );
            }
            LegendStyle::LongDash => draw_dashed_line(
                ui.painter(),
                egui::pos2(rect.left() + 1.0, rect.center().y),
                egui::pos2(rect.right() - 1.0, rect.center().y),
                Stroke::new(2.0, Color32::BLACK),
                6.0,
                3.0,
            ),
            LegendStyle::ShortDash => draw_dashed_line(
                ui.painter(),
                egui::pos2(rect.left() + 1.0, rect.center().y),
                egui::pos2(rect.right() - 1.0, rect.center().y),
                Stroke::new(2.0, Color32::BLACK),
                3.0,
                2.0,
            ),
            LegendStyle::DottedLine => draw_dotted_line(
                ui.painter(),
                egui::pos2(rect.left() + 1.0, rect.center().y),
                egui::pos2(rect.right() - 1.0, rect.center().y),
                Stroke::new(2.0, Color32::BLACK),
            ),
            LegendStyle::DashDotLine => draw_dash_dot_line(
                ui.painter(),
                egui::pos2(rect.left() + 1.0, rect.center().y),
                egui::pos2(rect.right() - 1.0, rect.center().y),
                Stroke::new(2.0, Color32::BLACK),
            ),
            LegendStyle::DashDotDotLine => draw_dash_dot_dot_line(
                ui.painter(),
                egui::pos2(rect.left() + 1.0, rect.center().y),
                egui::pos2(rect.right() - 1.0, rect.center().y),
                Stroke::new(2.0, Color32::BLACK),
            ),
        }

        ui.add_space(6.0);
        ui.label(label);
    });
    ui.add_space(4.0);
}

// Draw a dashed line.
fn draw_dashed_line(
    painter: &egui::Painter,
    from: egui::Pos2,
    to: egui::Pos2,
    stroke: Stroke,
    dash_len: f32,
    gap_len: f32,
) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 0.0 {
        return;
    }
    let ux = dx / len;
    let uy = dy / len;

    let mut dist = 0.0_f32;
    while dist < len {
        let seg_start = dist;
        let seg_end = (dist + dash_len).min(len);
        let p0 = egui::pos2(from.x + ux * seg_start, from.y + uy * seg_start);
        let p1 = egui::pos2(from.x + ux * seg_end, from.y + uy * seg_end);
        painter.line_segment([p0, p1], stroke);
        dist += dash_len + gap_len;
    }
}

// Draw a dotted line.
fn draw_dotted_line(painter: &egui::Painter, from: egui::Pos2, to: egui::Pos2, stroke: Stroke) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 0.0 {
        return;
    }
    let ux = dx / len;
    let uy = dy / len;
    let step = 5.0_f32;
    let radius = (stroke.width * 0.35).max(1.0);

    let mut dist = 0.0_f32;
    while dist <= len {
        let point = egui::pos2(from.x + ux * dist, from.y + uy * dist);
        painter.circle_filled(point, radius, stroke.color);
        dist += step;
    }
}

fn draw_dash_dot_line(painter: &egui::Painter, from: egui::Pos2, to: egui::Pos2, stroke: Stroke) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 0.0 {
        return;
    }

    let ux = dx / len;
    let uy = dy / len;
    let mut dist = 0.0_f32;
    while dist < len {
        let dash_end = (dist + 7.0).min(len);
        let p0 = egui::pos2(from.x + ux * dist, from.y + uy * dist);
        let p1 = egui::pos2(from.x + ux * dash_end, from.y + uy * dash_end);
        painter.line_segment([p0, p1], stroke);
        dist = dash_end + 3.0;
        if dist >= len {
            break;
        }
        let dot = egui::pos2(from.x + ux * dist, from.y + uy * dist);
        painter.circle_filled(dot, (stroke.width * 0.35).max(1.0), stroke.color);
        dist += 4.0;
    }
}

fn draw_dash_dot_dot_line(
    painter: &egui::Painter,
    from: egui::Pos2,
    to: egui::Pos2,
    stroke: Stroke,
) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 0.0 {
        return;
    }

    let ux = dx / len;
    let uy = dy / len;
    let radius = (stroke.width * 0.35).max(1.0);
    let mut dist = 0.0_f32;
    while dist < len {
        let dash_end = (dist + 7.0).min(len);
        let p0 = egui::pos2(from.x + ux * dist, from.y + uy * dist);
        let p1 = egui::pos2(from.x + ux * dash_end, from.y + uy * dash_end);
        painter.line_segment([p0, p1], stroke);
        dist = dash_end + 3.0;
        if dist >= len {
            break;
        }
        let dot1 = egui::pos2(from.x + ux * dist, from.y + uy * dist);
        painter.circle_filled(dot1, radius, stroke.color);
        dist += 3.0;
        if dist >= len {
            break;
        }
        let dot2 = egui::pos2(from.x + ux * dist, from.y + uy * dist);
        painter.circle_filled(dot2, radius, stroke.color);
        dist += 4.0;
    }
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
