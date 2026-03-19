use std::fs;
use std::io;
use std::path::PathBuf;

use crate::ui::UiSettings;

const APP_DIR_NAME: &str = "desktop_dungeon_generator";
const SETTINGS_FILE_NAME: &str = "settings.json";

// Load settings JSON from the user data directory.
pub fn load_settings() -> Option<UiSettings> {
    let path = settings_path()?;
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str::<UiSettings>(&content).ok()
}

// Save settings JSON to the user data directory.
pub fn save_settings(settings: &UiSettings) -> io::Result<()> {
    let path = settings_path().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "No OS application data directory found",
        )
    })?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(settings)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    fs::write(path, json)
}

// Build the full path for the settings file.
fn settings_path() -> Option<PathBuf> {
    let base = dirs::data_local_dir().or_else(dirs::data_dir)?;
    Some(base.join(APP_DIR_NAME).join(SETTINGS_FILE_NAME))
}
