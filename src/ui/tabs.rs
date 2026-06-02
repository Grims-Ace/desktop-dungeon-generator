/*
 * Individual tab rendering logic for the side panel.
 * Contains functions for drawing the "Generate", "Layout", "Add",
 * "Start & End", and "Monsters & Traps" control panels.
 */

use super::{AddTool, ExportFormat, MaskFormat, SidePanelResult, UiSettings};

use crate::seed;
use eframe::egui;
use egui::RichText;

// Render the Generate tab controls.
pub fn draw_generate_tab(
    ui: &mut egui::Ui,
    settings: &mut UiSettings,
    result: &mut SidePanelResult,
    _export_progress: Option<(usize, usize)>,
    _num_levels: usize,
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
}

// Render the Export/Save tab controls.
pub fn draw_export_save_tab(
    ui: &mut egui::Ui,
    settings: &mut UiSettings,
    result: &mut SidePanelResult,
    export_progress: Option<(usize, usize)>,
    num_levels: usize,
) {
    let export_in_progress = export_progress.is_some();

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
pub fn draw_layout_tab(ui: &mut egui::Ui, settings: &mut UiSettings, result: &mut SidePanelResult) {
    ui.label(RichText::new("Levels").strong());
    ui.add_space(8.0);

    draw_min_max_slider_rows(
        ui,
        result,
        "Min Levels",
        "Max Levels",
        &mut settings.min_levels,
        &mut settings.max_levels,
        1..=20,
        1..=100,
    );

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
    draw_min_max_slider_rows(
        ui,
        result,
        "Min Room Size",
        "Max Room Size",
        &mut settings.min_room_size,
        &mut settings.max_room_size,
        2..=30,
        2..=200,
    );

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
    draw_min_max_slider_rows(
        ui,
        result,
        "Min Corridor Width",
        "Max Corridor Width",
        &mut settings.min_corridor_width,
        &mut settings.max_corridor_width,
        1..=10,
        1..=50,
    );

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
        draw_min_max_slider_rows(
            ui,
            result,
            "Min Window Width",
            "Max Window Width",
            &mut settings.min_window_width,
            &mut settings.max_window_width,
            1..=5,
            1..=5,
        );

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
    draw_min_max_slider_rows(
        ui,
        result,
        "Min Stair Width",
        "Max Stair Width",
        &mut settings.min_stair_width,
        &mut settings.max_stair_width,
        1..=5,
        1..=10,
    );

    ui.add_space(8.0);
    draw_min_max_slider_rows(
        ui,
        result,
        "Min Stair Height",
        "Max Stair Height",
        &mut settings.min_stair_height,
        &mut settings.max_stair_height,
        1..=5,
        1..=10,
    );

    ui.add_space(8.0);
    draw_min_max_slider_rows(
        ui,
        result,
        "Min Stairs Per Level",
        "Max Stairs Per Level",
        &mut settings.min_stairs_per_level,
        &mut settings.max_stairs_per_level,
        0..=10,
        0..=20,
    );
}

// Render the Add tab controls.
pub fn draw_add_tab(ui: &mut egui::Ui, settings: &mut UiSettings, result: &mut SidePanelResult) {
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
        ui.selectable_value(&mut settings.add_tool, AddTool::Window, "Window");
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
        ui.add_space(12.0);
        ui.label(RichText::new("Controls").strong());
        ui.add_space(4.0);
        match settings.add_tool {
            AddTool::Room => {
                ui.label("Left-click to place a room of the specified size.");
            }
            AddTool::Corridor => {
                ui.label("Left-click and drag from one room to another to create a corridor.");
            }
            AddTool::Staircase => {
                ui.label("Left-click to place stairs connecting the current and adjacent levels.");
            }
            AddTool::Archway => {
                ui.label("Left-click or drag along room/corridor edges to place archways.");
            }
            AddTool::Door => {
                ui.label("Left-click or drag along room/corridor edges to place doors.");
            }
            AddTool::LockedDoor => {
                ui.label("Left-click or drag along room/corridor edges to place locked doors.");
            }
            AddTool::SecretDoor => {
                ui.label("Left-click or drag along room/corridor edges to place secret doors.");
            }
            AddTool::Window => {
                ui.label("Left-click or drag along room edges to place windows.");
            }
            AddTool::Text => {
                ui.label("Left-click to place the text label (edit text in options above).");
            }
            AddTool::StartMarker => {
                ui.label("Left-click to place a start marker of the specified size.");
            }
            AddTool::EndMarker => {
                ui.label("Left-click to place an end marker of the specified size.");
            }
            AddTool::TrapMarker => {
                ui.label("Left-click to place a trap marker of the specified size.");
            }
            AddTool::MonsterMarker => {
                ui.label("Left-click to place a monster marker of the specified size.");
            }
            _ => {}
        }
        ui.add_space(4.0);
        ui.label(RichText::new("Right-click to cancel tool.").small());
    } else {
        ui.label("Select a tool above to start building manually.");
    }
}

// Draws start and end tab.
pub fn draw_start_and_end_tab(
    ui: &mut egui::Ui,
    settings: &mut UiSettings,
    result: &mut SidePanelResult,
) {
    ui.label(RichText::new("Start Markers").strong());
    ui.add_space(8.0);
    draw_min_max_slider_rows(
        ui,
        result,
        "Min Start Marker Size",
        "Max Start Marker Size",
        &mut settings.min_start_marker_size,
        &mut settings.max_start_marker_size,
        1..=10,
        1..=10,
    );
    draw_min_max_slider_rows(
        ui,
        result,
        "Min Start Marker Count",
        "Max Start Marker Count",
        &mut settings.min_start_marker_count,
        &mut settings.max_start_marker_count,
        1..=20,
        1..=100,
    );

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label(RichText::new("End Markers").strong());
    ui.add_space(8.0);
    draw_min_max_slider_rows(
        ui,
        result,
        "Min End Marker Size",
        "Max End Marker Size",
        &mut settings.min_end_marker_size,
        &mut settings.max_end_marker_size,
        1..=10,
        1..=10,
    );
    draw_min_max_slider_rows(
        ui,
        result,
        "Min End Marker Count",
        "Max End Marker Count",
        &mut settings.min_end_marker_count,
        &mut settings.max_end_marker_count,
        1..=20,
        1..=100,
    );

    ui.add_space(8.0);
    ui.label("Markers are generated inside rooms. Matching start/end indices are paired as far apart as possible.");
}

// Draws monsters and traps tab.
pub fn draw_monsters_and_traps_tab(
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
    draw_min_max_slider_rows(
        ui,
        result,
        "Min Traps Per Room/Corridor",
        "Max Traps Per Room/Corridor",
        &mut settings.min_traps_per_area,
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
    draw_min_max_slider_rows(
        ui,
        result,
        "Min Monsters Per Room/Corridor",
        "Max Monsters Per Room/Corridor",
        &mut settings.min_monsters_per_area,
        &mut settings.max_monsters_per_area,
        0..=20,
        0..=100,
    );
}
// Draws usize slider row.
pub fn draw_usize_slider_row(
    ui: &mut egui::Ui,
    result: &mut SidePanelResult,
    label: &str,
    value: &mut usize,
    slider_range: std::ops::RangeInclusive<usize>,
    drag_range: std::ops::RangeInclusive<usize>,
) -> bool {
    let mut changed = false;
    ui.label(label);
    ui.horizontal(|ui| {
        if ui
            .add(egui::Slider::new(value, slider_range).show_value(false))
            .changed()
        {
            changed = true;
        }
        if ui
            .add(egui::DragValue::new(value).speed(1.0).range(drag_range))
            .changed()
        {
            changed = true;
        }
    });
    result.settings_changed |= changed;
    changed
}

// Draws min/max usize slider rows.
pub fn draw_min_max_slider_rows(
    ui: &mut egui::Ui,
    result: &mut SidePanelResult,
    label_min: &str,
    label_max: &str,
    min_value: &mut usize,
    max_value: &mut usize,
    slider_range: std::ops::RangeInclusive<usize>,
    drag_range: std::ops::RangeInclusive<usize>,
) {
    if draw_usize_slider_row(
        ui,
        result,
        label_min,
        min_value,
        slider_range.clone(),
        drag_range.clone(),
    ) {
        if *min_value > *max_value {
            *max_value = *min_value;
        }
    }

    ui.add_space(8.0);

    if draw_usize_slider_row(ui, result, label_max, max_value, slider_range, drag_range) {
        if *max_value < *min_value {
            *min_value = *max_value;
        }
    }
}
