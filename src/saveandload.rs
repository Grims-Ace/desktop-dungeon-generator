/*
 * Persistence logic for dungeon state.
 * Handles saving and loading the complete dungeon state
 * (settings and layout data) to and from disk using JSON.
 */

use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::layout::DungeonLayout;
use crate::ui::UiSettings;

#[derive(Serialize, Deserialize)]
pub struct DungeonSave {
    pub settings: UiSettings,
    pub layouts: Vec<DungeonLayout>,
    pub svgs: Vec<String>,
}

pub fn save_dungeon(
    settings: &UiSettings,
    layouts: Vec<DungeonLayout>,
    svgs: Vec<String>,
) -> Result<PathBuf, String> {
    let path = FileDialog::new()
        .set_title("Save Dungeon State")
        .add_filter("Dungeon File", &["dungeon"])
        .set_file_name("my_dungeon.dungeon")
        .save_file()
        .ok_or_else(|| "Save canceled".to_string())?;

    let save_data = DungeonSave {
        settings: settings.clone(),
        layouts,
        svgs,
    };

    let json = serde_json::to_string_pretty(&save_data)
        .map_err(|e| format!("Failed to serialize dungeon: {e}"))?;

    fs::write(&path, json).map_err(|e| format!("Failed to write file: {e}"))?;

    Ok(path)
}

pub fn load_dungeon() -> Result<DungeonSave, String> {
    let path = FileDialog::new()
        .set_title("Load Dungeon State")
        .add_filter("Dungeon File", &["dungeon"])
        .pick_file()
        .ok_or_else(|| "Load canceled".to_string())?;

    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {e}"))?;

    let save_data: DungeonSave =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse dungeon file: {e}"))?;

    Ok(save_data)
}
