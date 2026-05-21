/*
 * UI panel composition.
 * Draws the side panel, legend panel, and level tab strip.
 */

use super::{SidePanelResult, Tab, UiSettings, tabs, widgets};
use eframe::egui;

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
                    if ui
                        .selectable_label(settings.active_tab == Tab::ExportSave, "Export/Save")
                        .clicked()
                    {
                        settings.active_tab = Tab::ExportSave;
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
                    Tab::ExportSave => tabs::draw_export_save_tab(
                        ui,
                        settings,
                        &mut result,
                        export_progress,
                        num_levels,
                    ),
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

use crate::layout::DungeonLayout;

// Draws level tabs.
pub fn draw_level_tabs(
    ui: &mut egui::Ui,
    levels: &[DungeonLayout],
    active_level_index: &mut usize,
    rename_level_state: &mut Option<(usize, String)>,
) -> Option<usize> {
    let mut hovered_level_idx = None;
    ui.horizontal(|ui| {
        for (level_idx, level) in levels.iter().enumerate() {
            let label = if level.name.is_empty() {
                format!("Level {}", level_idx + 1)
            } else {
                level.name.clone()
            };
            let is_active = *active_level_index == level_idx;
            let response = ui.selectable_label(is_active, label);
            if response.hovered() {
                hovered_level_idx = Some(level_idx);
            }
            if response.clicked() {
                *active_level_index = level_idx;
            }
            if response.double_clicked() {
                *rename_level_state = Some((level_idx, level.name.clone()));
            }
        }
    });
    hovered_level_idx
}
