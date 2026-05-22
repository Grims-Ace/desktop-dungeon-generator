/*
 * UI settings and option types.
 * Defines serializable configuration used by the app and UI controls.
 */

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tab {
    Generate,
    Layout,
    StartAndEnd,
    MonstersAndTraps,
    Add,
    ExportSave,
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
            ExportFormat::Png => "PNG Image",
            ExportFormat::Jpeg => "JPEG Image",
            ExportFormat::Webp => "WebP Image",
            ExportFormat::Svg => "SVG Vector Graphic",
            ExportFormat::Folder => "Folder",
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
            MaskFormat::Png => "PNG Image",
            MaskFormat::Jpeg => "JPEG Image",
            MaskFormat::Webp => "WebP Image",
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
    pub overlay_level_index: Option<usize>,
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
            overlay_level_index: None,
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
    Window,
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
