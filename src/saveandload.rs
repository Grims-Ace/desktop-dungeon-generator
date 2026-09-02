/*
 * Persistence logic for dungeon state.
 * Handles saving and loading the complete dungeon state
 * (settings, layout data, and SVGs) to and from compressed XML archives.
 */

use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::layout::DungeonLayout;
use crate::ui::UiSettings;

#[derive(Serialize, Deserialize)]
pub struct DungeonSave {
    pub settings: UiSettings,
    pub layouts: Vec<DungeonLayout>,
    pub svgs: Vec<String>,
}

const XML_ENTRY_NAME: &str = "dungeon.xml";

#[derive(Serialize, Deserialize)]
struct DungeonXml {
    #[serde(rename = "$text")]
    payload: String,
}

fn serialize_dungeon(save_data: &DungeonSave) -> Result<Vec<u8>, String> {
    let payload = serde_json::to_string(save_data)
        .map_err(|e| format!("Failed to serialize dungeon: {e}"))?;
    quick_xml::se::to_string(&DungeonXml { payload })
        .map(|xml| {
            let mut document = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
            document.push_str(&xml);
            document.into_bytes()
        })
        .map_err(|e| format!("Failed to serialize dungeon: {e}"))
}

fn deserialize_dungeon(xml: &[u8]) -> Result<DungeonSave, String> {
    let document: DungeonXml = quick_xml::de::from_reader(Cursor::new(xml))
        .map_err(|e| format!("Failed to parse dungeon XML: {e}"))?;
    serde_json::from_str(&document.payload)
        .map_err(|e| format!("Failed to parse dungeon XML payload: {e}"))
}

fn create_archive(save_data: &DungeonSave) -> Result<Vec<u8>, String> {
    let xml = serialize_dungeon(save_data)?;
    let mut archive = Cursor::new(Vec::new());
    {
        let mut writer = ZipWriter::new(&mut archive);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        writer
            .start_file(XML_ENTRY_NAME, options)
            .map_err(|e| format!("Failed to create dungeon archive: {e}"))?;
        writer
            .write_all(&xml)
            .map_err(|e| format!("Failed to write dungeon XML: {e}"))?;
        writer
            .finish()
            .map_err(|e| format!("Failed to finish dungeon archive: {e}"))?;
    }
    Ok(archive.into_inner())
}

fn read_archive(bytes: &[u8]) -> Result<DungeonSave, String> {
    let mut archive =
        ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("Invalid dungeon archive: {e}"))?;
    let mut xml_file = archive
        .by_name(XML_ENTRY_NAME)
        .map_err(|e| format!("Dungeon archive is missing {XML_ENTRY_NAME}: {e}"))?;
    let mut xml = Vec::new();
    xml_file
        .read_to_end(&mut xml)
        .map_err(|e| format!("Failed to read dungeon XML: {e}"))?;
    deserialize_dungeon(&xml)
}

// Saves dungeon state to a compressed XML archive.
pub fn save_dungeon(
    settings: &UiSettings,
    layouts: Vec<DungeonLayout>,
    svgs: Vec<String>,
) -> Result<PathBuf, String> {
    let dialog = FileDialog::new()
        .set_title("Save Dungeon State")
        .add_filter("Dungeon File", &["dun"])
        .set_file_name("my_dungeon.dun");

    #[cfg(target_os = "windows")]
    let path = std::thread::spawn(move || dialog.save_file())
        .join()
        .map_err(|_| "Dialog thread panicked".to_string())?;
    #[cfg(not(target_os = "windows"))]
    let path = dialog.save_file();

    let mut path = path.ok_or_else(|| "Save canceled".to_string())?;
    if path.extension().is_none() {
        path.set_extension("dun");
    }

    let save_data = DungeonSave {
        settings: settings.clone(),
        layouts,
        svgs,
    };

    let archive = create_archive(&save_data)?;
    fs::write(&path, archive).map_err(|e| format!("Failed to write file: {e}"))?;

    Ok(path)
}

// Loads dungeon state from a compressed XML archive.
pub fn load_dungeon() -> Result<DungeonSave, String> {
    let dialog = FileDialog::new()
        .set_title("Load Dungeon State")
        .add_filter("Dungeon File", &["dun"]);

    #[cfg(target_os = "windows")]
    let path = std::thread::spawn(move || dialog.pick_file())
        .join()
        .map_err(|_| "Dialog thread panicked".to_string())?;
    #[cfg(not(target_os = "windows"))]
    let path = dialog.pick_file();

    let path = path.ok_or_else(|| "Load canceled".to_string())?;

    let content = fs::read(path).map_err(|e| format!("Failed to read file: {e}"))?;
    read_archive(&content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_round_trip_preserves_saved_data() {
        let save = DungeonSave {
            settings: UiSettings::default(),
            layouts: vec![DungeonLayout {
                name: "Level 1".to_string(),
                rooms: vec![crate::layout::types::Room {
                    x: 2,
                    y: 3,
                    width: 5,
                    height: 4,
                }],
                text_labels: vec![crate::layout::types::TextLabel {
                    cell: (4, 5),
                    text: "Keep & escape".to_string(),
                    font_size: 18,
                }],
                ..DungeonLayout::default()
            }],
            svgs: vec!["<svg><text>test &amp;</text></svg>".to_string()],
        };

        let archive = create_archive(&save).expect("archive should serialize");
        let restored = read_archive(&archive).expect("archive should deserialize");

        assert_eq!(restored.settings, save.settings);
        assert_eq!(restored.layouts, save.layouts);
        assert_eq!(restored.svgs, save.svgs);
    }

    #[test]
    fn archive_uses_deflate_compression() {
        let save = DungeonSave {
            settings: UiSettings::default(),
            layouts: Vec::new(),
            svgs: vec!["<svg>".repeat(100)],
        };
        let archive = create_archive(&save).expect("archive should serialize");
        let mut zip = ZipArchive::new(Cursor::new(archive)).expect("archive should be valid");
        let entry = zip.by_name(XML_ENTRY_NAME).expect("XML entry should exist");

        assert_eq!(entry.compression(), CompressionMethod::Deflated);
    }
}
