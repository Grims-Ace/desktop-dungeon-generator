/*
 * The primary UI module. Defines the UiSettings structure and
 * manages the side panel layout, legend panel, and level selection tabs.
 * Serves as the entry point for all UI-related rendering.
 */
use eframe::egui;
use serde::{Deserialize, Serialize};

pub mod tabs;
pub mod widgets;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tab {
    Generate,
    Layout,
    StartAndEnd,
    MonstersAndTraps,
    Add,
}

impl Default for Tab {
    // Provides default settings for this type.
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
    // Provides default settings for this type.
    fn default() -> Self {
        Self::Png
    }
}

impl ExportFormat {
    // Returns the file extension for this format.
    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Png => "png",
            ExportFormat::Jpeg => "jpeg",
            ExportFormat::Webp => "webp",
            ExportFormat::Svg => "svg",
            ExportFormat::Folder => "folder",
        }
    }

    // Returns the user-facing label for this option.
    pub fn label(self) -> &'static str {
        match self {
            ExportFormat::Png => ".png",
            ExportFormat::Jpeg => ".jpeg",
            ExportFormat::Webp => ".webp",
            ExportFormat::Svg => ".svg",
            ExportFormat::Folder => "Folder (Composite)",
        }
    }

    // Reports whether this format supports resolution scaling.
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
    // Provides default settings for this type.
    fn default() -> Self {
        Self::Png
    }
}

impl MaskFormat {
    // Returns the file extension for this format.
    pub fn extension(self) -> &'static str {
        match self {
            MaskFormat::Png => "png",
            MaskFormat::Jpeg => "jpeg",
            MaskFormat::Webp => "webp",
        }
    }

    // Returns the user-facing label for this option.
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
    pub active_tab: Tab,
}

impl Default for UiSettings {
    // Provides default settings for this type.
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

// Draws side panel.
pub fn draw_side_panel(
    ctx: &egui::Context,
    settings: &mut UiSettings,
    export_progress: Option<(usize, usize)>,
    num_levels: usize,
) -> SidePanelResult {
    let mut result = SidePanelResult::default();

    egui::SidePanel::left("options_panel")
        .resizable(true)
        .min_width(180.0)
        .default_width(280.0)
        .show_separator_line(true)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(settings.active_tab == Tab::Generate, "Generate")
                        .clicked()
                    {
                        settings.active_tab = Tab::Generate;
                    }
                    if ui
                        .selectable_label(settings.active_tab == Tab::Layout, "Layout")
                        .clicked()
                    {
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
                    Tab::Generate => tabs::draw_generate_tab(
                        ui,
                        settings,
                        &mut result,
                        export_progress,
                        num_levels,
                    ),
                    Tab::Layout => tabs::draw_layout_tab(ui, settings, &mut result),
                    Tab::StartAndEnd => tabs::draw_start_and_end_tab(ui, settings, &mut result),
                    Tab::MonstersAndTraps => {
                        tabs::draw_monsters_and_traps_tab(ui, settings, &mut result)
                    }
                    Tab::Add => tabs::draw_add_tab(ui, settings, &mut result),
                }
            });
        });

    result
}

// Draws legend panel.
pub fn draw_legend_panel(ctx: &egui::Context, settings: &mut UiSettings) -> bool {
    let mut composition_mode_changed = false;
    use egui::Color32;
    use widgets::LegendStyle;

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
                widgets::draw_legend_entry_colorblind(ui, "Rooms", LegendStyle::Crosshatch);
                widgets::draw_legend_entry_colorblind(ui, "Corridors", LegendStyle::Dots);
                widgets::draw_legend_entry_colorblind(ui, "Walls", LegendStyle::SolidLine);
                widgets::draw_legend_entry_colorblind(ui, "Doors", LegendStyle::LongDash);
                widgets::draw_legend_entry_colorblind(ui, "Locked Doors", LegendStyle::ShortDash);
                widgets::draw_legend_entry_colorblind(
                    ui,
                    "Secret Doors",
                    LegendStyle::DashDotDotLine,
                );
                widgets::draw_legend_entry_colorblind(ui, "Archways", LegendStyle::DottedLine);
                widgets::draw_legend_entry_colorblind(ui, "Windows", LegendStyle::DashDotLine);
                widgets::draw_legend_entry_colorblind(
                    ui,
                    "Stairs",
                    LegendStyle::LabelChar('\u{1F5CF}'),
                );
                widgets::draw_legend_entry_colorblind(ui, "Start Marker", LegendStyle::Label("S"));
                widgets::draw_legend_entry_colorblind(ui, "End Marker", LegendStyle::Label("E"));
                widgets::draw_legend_entry_colorblind(ui, "Trap", LegendStyle::Label("T"));
                widgets::draw_legend_entry_colorblind(ui, "Monster", LegendStyle::Label("M"));
            } else {
                widgets::draw_legend_entry(ui, Color32::from_rgb(70, 120, 160), "Rooms");
                widgets::draw_legend_entry(ui, Color32::from_rgb(210, 190, 120), "Corridors");
                widgets::draw_legend_entry(ui, Color32::BLACK, "Walls");
                widgets::draw_legend_entry(ui, Color32::from_rgb(80, 200, 120), "Doors");
                widgets::draw_legend_entry(ui, Color32::from_rgb(220, 70, 70), "Locked Doors");
                widgets::draw_legend_entry(ui, Color32::from_rgb(170, 80, 170), "Secret Doors");
                widgets::draw_legend_entry(ui, Color32::from_rgb(230, 140, 60), "Archways");
                widgets::draw_legend_entry(ui, Color32::from_rgb(70, 130, 220), "Windows");
                widgets::draw_legend_entry(ui, Color32::from_rgb(200, 150, 50), "Stairs");
                widgets::draw_legend_entry(ui, Color32::from_rgb(60, 220, 200), "Start Marker");
                widgets::draw_legend_entry(ui, Color32::from_rgb(240, 90, 90), "End Marker");
                widgets::draw_legend_entry(ui, Color32::from_rgb(150, 80, 230), "Trap");
                widgets::draw_legend_entry(ui, Color32::from_rgb(200, 50, 50), "Monster");
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

// Draws level tabs.
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
