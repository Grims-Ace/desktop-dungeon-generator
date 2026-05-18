mod app;
mod exporter;
mod interact;
mod layout;
mod rendering;
mod saveandload;
mod seed;
mod settings;
mod ui;

use app::DungeonApp;

// Boot the native app and start the egui event loop.
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();

    eframe::run_native(
        "Desktop Dungeon Generator",
        options,
        Box::new(|_cc| Ok(Box::new(DungeonApp::default()))),
    )
}
