/*
 * Contains the core DungeonApp struct and its eframe::App implementation.
 * This file manages the application lifecycle, main state, undo/redo stacks,
 * and high-level level management (generation, deletion, and refreshing).
 */

use std::collections::HashSet;
use std::sync::mpsc;
use std::thread;

use crate::exporter;
use crate::interact::{
    AddCorridorDrag, AddDoorDrag, DragState, HoverMarker, ResizeState, draw_grid,
};
use crate::layout::{
    self, DoorSettings, DungeonLayout, WindowSettings, populate_random_markers, populate_stairs,
};
use crate::rendering::draw_layout;
use crate::saveandload;
use crate::settings;
use crate::ui::{AddTool, UiSettings, draw_legend_panel, draw_level_tabs, draw_side_panel};
use eframe::egui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManualDoorKind {
    Archway,
    Regular,
    Locked,
    Secret,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppSnapshot {
    pub settings: UiSettings,
    pub levels: Vec<DungeonLayout>,
    pub suppressed_auto_door_edges: HashSet<((usize, usize), (usize, usize))>,
}

pub struct DungeonApp {
    pub settings: UiSettings,
    pub levels: Vec<DungeonLayout>,
    pub suppressed_auto_door_edges: HashSet<((usize, usize), (usize, usize))>,
    pub undo_stack: Vec<AppSnapshot>,
    pub redo_stack: Vec<AppSnapshot>,
    pub drag_state: Option<DragState>,
    pub export_rx: Option<mpsc::Receiver<exporter::ExportEvent>>,
    pub export_progress: Option<exporter::ExportProgress>,
    pub pending_delete: bool,
    pub add_corridor_drag: Option<AddCorridorDrag>,
    pub add_door_drag: Option<AddDoorDrag>,
    pub resize_state: Option<ResizeState>,
    pub hover_room_idx: Option<usize>,
    pub hover_corridor_idx: Option<usize>,
    pub hover_door_idx: Option<usize>,
    pub hover_text_idx: Option<usize>,
    pub hover_marker: Option<HoverMarker>,
    pub hover_level_idx: Option<usize>,
    pub hover_stair_idx: Option<usize>,
}

impl Default for DungeonApp {
    // Provides default settings for this type.
    fn default() -> Self {
        let settings = settings::load_settings().unwrap_or_default();
        let mut app = Self {
            settings,
            levels: Vec::new(),
            suppressed_auto_door_edges: HashSet::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            drag_state: None,
            export_rx: None,
            export_progress: None,
            pending_delete: false,
            add_corridor_drag: None,
            add_door_drag: None,
            resize_state: None,
            hover_room_idx: None,
            hover_corridor_idx: None,
            hover_door_idx: None,
            hover_text_idx: None,
            hover_marker: None,
            hover_level_idx: None,
            hover_stair_idx: None,
        };
        if app.settings.composition_mode {
            app.enter_composition_layout();
        } else {
            app.levels = generate_all_levels(&app.settings);
            app.refresh_stairs();
        }
        app
    }
}

impl Drop for DungeonApp {
    // Cleans up background resources when the app is dropped.
    fn drop(&mut self) {
        if let Err(err) = settings::save_settings(&self.settings) {
            eprintln!("Failed to save settings: {err}");
        }
    }
}

impl eframe::App for DungeonApp {
    // Runs one UI frame and handles app interaction.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let panel_result = draw_side_panel(
            ctx,
            &mut self.settings,
            self.export_progress
                .map(|progress| (progress.completed, progress.total)),
            self.levels.len(),
        );
        let undo_requested = ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Z));
        let redo_requested = ctx.input(|i| {
            (i.modifiers.command && i.key_pressed(egui::Key::Y))
                || (i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::Z))
        });
        let escape_pressed = ctx.input(|i| i.key_pressed(egui::Key::Escape));

        if escape_pressed || panel_result.settings_changed {
            self.settings.add_tool = AddTool::None;
        }

        clamp_dependent_settings(&mut self.settings);

        if undo_requested {
            self.undo();
        } else if redo_requested {
            self.redo();
        }

        if panel_result.clear_clicked {
            self.push_undo_snapshot();
            self.clear_layout();
        } else if panel_result.reset_clicked {
            self.push_undo_snapshot();
            if self.settings.composition_mode {
                self.enter_composition_layout();
            } else {
                self.regenerate_layout();
            }
        } else if panel_result.settings_changed && !self.settings.composition_mode {
            self.push_undo_snapshot();
            self.regenerate_layout();
        } else if panel_result.save_clicked {
            let mut layouts = Vec::new();
            let mut svgs = Vec::new();
            for layout in &self.levels {
                layouts.push(layout.clone());
                svgs.push(exporter::build_svg(layout, &self.settings));
            }
            if let Err(err) = saveandload::save_dungeon(&self.settings, layouts, svgs) {
                eprintln!("{err}");
            }
        } else if panel_result.load_clicked {
            match saveandload::load_dungeon() {
                Ok(data) => {
                    self.push_undo_snapshot();
                    self.settings = data.settings;
                    self.levels = data.layouts;
                    self.settings.active_level_index = 0;
                }
                Err(err) => {
                    eprintln!("{err}");
                }
            }
        } else if panel_result.new_level_clicked {
            self.push_undo_snapshot();
            if self.settings.composition_mode {
                self.levels.push(DungeonLayout::empty(
                    self.settings.pack_rooms_without_corridors,
                ));
            } else {
                self.levels
                    .push(generate_level(&self.settings, self.levels.len()));
                self.refresh_stairs();
            }
            self.settings.active_level_index = self.levels.len() - 1;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)) {
            self.pending_delete = true;
        }
        if panel_result.export_clicked && self.export_rx.is_none() {
            let target = match exporter::select_export_target(&self.settings) {
                Ok(target) => target,
                Err(err) => {
                    eprintln!("{err}");
                    return;
                }
            };

            match &target {
                exporter::ExportTarget::File(path) => {
                    if let Some(parent) = path.parent() {
                        self.settings.last_export_path = Some(parent.to_string_lossy().to_string());
                    }
                }
                exporter::ExportTarget::Folder(path) => {
                    self.settings.last_export_path = Some(path.to_string_lossy().to_string());
                }
            }

            let layouts = self.levels.clone();
            let settings = self.settings.clone();
            let (tx, rx) = mpsc::channel();
            thread::spawn(move || {
                let result = exporter::export_to_target(&layouts, &settings, target, &tx);
                let _ = tx.send(exporter::ExportEvent::Finished(result));
            });
            self.export_progress = Some(exporter::ExportProgress {
                completed: 0,
                total: 1,
            });
            self.export_rx = Some(rx);
        }

        let mut export_finished = false;
        if let Some(rx) = &self.export_rx {
            loop {
                match rx.try_recv() {
                    Ok(exporter::ExportEvent::Progress(progress)) => {
                        self.export_progress = Some(progress);
                        ctx.request_repaint();
                    }
                    Ok(exporter::ExportEvent::Finished(Ok(_))) => {
                        export_finished = true;
                        break;
                    }
                    Ok(exporter::ExportEvent::Finished(Err(err))) => {
                        eprintln!("{err}");
                        export_finished = true;
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        export_finished = true;
                        break;
                    }
                }
            }
        }
        if export_finished {
            self.export_rx = None;
            self.export_progress = None;
        }

        if draw_legend_panel(ctx, &mut self.settings) {
            self.push_undo_snapshot();
            if self.settings.composition_mode {
                self.enter_composition_layout();
            } else {
                self.regenerate_layout();
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            self.hover_level_idx =
                draw_level_tabs(ui, self.levels.len(), &mut self.settings.active_level_index);
            ui.separator();

            let available = ui.available_size_before_wrap();
            let (response, painter) = ui.allocate_painter(available, egui::Sense::click_and_drag());
            let canvas = response.rect.shrink(12.0);
            let geometry = draw_grid(&painter, canvas, self.settings.cols, self.settings.rows);
            crate::interact::update_hover_targets(self, ctx, &geometry);

            let level_delete_requested = ctx.input(|i| {
                (i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace))
                    && self.hover_level_idx.is_some()
            });
            if level_delete_requested && let Some(level_idx) = self.hover_level_idx {
                self.delete_level(level_idx);
            }

            let resizing = crate::interact::handle_resize(self, ctx, &response, &geometry);
            if !resizing {
                crate::interact::handle_drag(self, ctx, &response, &geometry);
            }
            crate::interact::handle_add_tool(self, &response, &geometry);
            if self.pending_delete {
                if let Some(pointer_pos) = ctx.pointer_interact_pos() {
                    crate::interact::delete_at_pointer(self, pointer_pos, &geometry);
                }
                self.pending_delete = false;
            }

            if self.settings.active_level_index < self.levels.len() {
                draw_layout(
                    &painter,
                    &geometry,
                    &self.levels[self.settings.active_level_index],
                    self.settings.colorblind_mode,
                    self.hover_text_idx,
                    self.hover_marker,
                    self.hover_stair_idx,
                );
            }
            crate::interact::draw_add_overlay(self, &painter, &geometry);
            crate::interact::draw_add_tool_ghost(self, &painter, &geometry);
            crate::interact::draw_resize_overlay(self, &painter, &geometry);
            crate::interact::draw_corridor_hover_overlay(self, &painter, &geometry);
            crate::interact::draw_door_hover_overlay(self, &painter, &geometry);
        });
    }
}

// Generates all levels.
pub fn generate_all_levels(settings: &UiSettings) -> Vec<DungeonLayout> {
    let level_count = if settings.min_levels == settings.max_levels {
        settings.min_levels
    } else {
        let range_seed = crate::seed::derive_seed(settings.seed, 0x0001_E7E1_u64);
        (range_seed as usize % (settings.max_levels - settings.min_levels + 1))
            + settings.min_levels
    };

    let mut levels = Vec::with_capacity(level_count);
    for i in 0..level_count {
        levels.push(generate_level(settings, i));
    }
    levels
}

// Generates a single dungeon level from the UI settings.
pub fn generate_level(settings: &UiSettings, level_index: usize) -> DungeonLayout {
    let level_seed = crate::seed::derive_seed(settings.seed, level_index as u64);
    let layout = layout::generate_layout(
        settings.cols,
        settings.rows,
        settings.room_count,
        level_seed,
        settings.min_room_size,
        settings.max_room_size,
        settings.square_rooms_only,
        settings.min_corridor_width,
        settings.max_corridor_width,
        settings.corridor_randomness,
        settings.dead_end_rooms_percent,
        settings.pack_rooms_without_corridors,
        door_settings_from_ui(settings),
        window_settings_from_ui(settings),
    );

    populate_random_markers(layout, settings)
}

// Clamps dependent settings.
pub fn clamp_dependent_settings(settings: &mut UiSettings) {
    if settings.min_room_size > settings.max_room_size {
        settings.max_room_size = settings.min_room_size;
    }
    if settings.min_corridor_width == 0 {
        settings.min_corridor_width = 1;
    }
    if settings.min_corridor_width > settings.max_corridor_width {
        settings.max_corridor_width = settings.min_corridor_width;
    }
    if settings.min_window_width > settings.max_window_width {
        settings.max_window_width = settings.min_window_width;
    }
    if settings.min_start_marker_size > settings.max_start_marker_size {
        settings.max_start_marker_size = settings.min_start_marker_size;
    }
    if settings.min_end_marker_size > settings.max_end_marker_size {
        settings.max_end_marker_size = settings.min_end_marker_size;
    }
    if settings.min_start_marker_count > settings.max_start_marker_count {
        settings.max_start_marker_count = settings.min_start_marker_count;
    }
    if settings.min_end_marker_count > settings.max_end_marker_count {
        settings.max_end_marker_count = settings.min_end_marker_count;
    }
    if settings.min_traps_per_area > settings.max_traps_per_area {
        settings.max_traps_per_area = settings.min_traps_per_area;
    }
    if settings.min_monsters_per_area > settings.max_monsters_per_area {
        settings.max_monsters_per_area = settings.min_monsters_per_area;
    }
    if settings.min_levels == 0 {
        settings.min_levels = 1;
    }
    if settings.min_levels > settings.max_levels {
        settings.max_levels = settings.min_levels;
    }
}

impl DungeonApp {
    // Clears hover targets.
    pub fn clear_hover_targets(&mut self) {
        self.hover_room_idx = None;
        self.hover_corridor_idx = None;
        self.hover_door_idx = None;
        self.hover_text_idx = None;
        self.hover_marker = None;
        self.hover_level_idx = None;
        self.hover_stair_idx = None;
    }

    // Resets transient state.
    pub fn reset_transient_state(&mut self) {
        self.drag_state = None;
        self.add_corridor_drag = None;
        self.add_door_drag = None;
        self.resize_state = None;
        self.clear_hover_targets();
        self.pending_delete = false;
    }

    // Captures the current app state for undo history.
    pub fn snapshot(&self) -> AppSnapshot {
        AppSnapshot {
            settings: self.settings.clone(),
            levels: self.levels.clone(),
            suppressed_auto_door_edges: self.suppressed_auto_door_edges.clone(),
        }
    }

    // Pushes undo snapshot.
    pub fn push_undo_snapshot(&mut self) {
        let snapshot = self.snapshot();
        if self.undo_stack.last() == Some(&snapshot) {
            return;
        }
        self.undo_stack.push(snapshot);
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    // Restores a saved app state snapshot.
    pub fn restore_snapshot(&mut self, snapshot: AppSnapshot) {
        self.settings = snapshot.settings;
        self.levels = snapshot.levels;
        self.suppressed_auto_door_edges = snapshot.suppressed_auto_door_edges;
        self.reset_transient_state();
    }

    // Restores the previous undo snapshot. undo.
    pub fn undo(&mut self) {
        let Some(snapshot) = self.undo_stack.pop() else {
            return;
        };
        self.redo_stack.push(self.snapshot());
        self.restore_snapshot(snapshot);
    }

    // Restores the next redo snapshot. redo.
    pub fn redo(&mut self) {
        let Some(snapshot) = self.redo_stack.pop() else {
            return;
        };
        self.undo_stack.push(self.snapshot());
        self.restore_snapshot(snapshot);
    }

    // Clears all generated layout content and resets editing state.
    pub fn clear_layout(&mut self) {
        if self.settings.active_level_index >= self.levels.len() {
            return;
        }
        self.levels[self.settings.active_level_index] =
            DungeonLayout::empty(self.settings.pack_rooms_without_corridors);
        self.suppressed_auto_door_edges.clear();
        self.reset_transient_state();
    }

    // Switches to composition layout.
    pub fn enter_composition_layout(&mut self) {
        self.levels = vec![DungeonLayout::empty(
            self.settings.pack_rooms_without_corridors,
        )];
        self.settings.active_level_index = 0;
        self.settings.export_level_index = 0;
        self.suppressed_auto_door_edges.clear();
        self.reset_transient_state();
    }

    // Regenerates every level from the current settings.
    pub fn regenerate_layout(&mut self) {
        self.drag_state = None;
        self.suppressed_auto_door_edges.clear();
        self.levels = generate_all_levels(&self.settings);
        self.refresh_stairs();
        if self.settings.active_level_index >= self.levels.len() {
            self.settings.active_level_index = 0;
        }
    }

    // Deletes a level and keeps the active level index valid.
    pub fn delete_level(&mut self, level_idx: usize) {
        if self.levels.len() <= 1 {
            return;
        }
        if level_idx >= self.levels.len() {
            return;
        }
        self.push_undo_snapshot();
        self.levels.remove(level_idx);
        if self.settings.active_level_index >= self.levels.len() {
            self.settings.active_level_index = self.levels.len() - 1;
        }
        self.reset_transient_state();
    }

    // Regenerates one level while preserving manual annotations where possible.
    pub fn refresh_level(&mut self, level_idx: usize) {
        if level_idx >= self.levels.len() {
            return;
        }

        let layout = &mut self.levels[level_idx];
        if !layout.packed_rooms {
            let rooms = layout.rooms.clone();
            for corridor in &mut layout.corridors {
                let start = rooms[corridor.start_room_id].center_cell();
                let end = rooms[corridor.end_room_id].center_cell();
                let blocked = layout::blocked_room_cells(
                    &rooms,
                    &[corridor.start_room_id, corridor.end_room_id],
                );
                if let Some(path) = layout::shortest_path_cells(
                    start,
                    end,
                    self.settings.cols,
                    self.settings.rows,
                    &blocked,
                ) {
                    corridor.path = path;
                }
            }
        }

        let door_settings = door_settings_from_ui(&self.settings);
        layout::apply_doors(
            layout,
            self.settings.seed,
            door_settings,
            self.settings.cols,
            self.settings.rows,
        );
        layout.doors.retain(|door| {
            !crate::interact::door_edges_for(door, self.settings.cols, self.settings.rows)
                .iter()
                .any(|edge| self.suppressed_auto_door_edges.contains(edge))
        });

        let window_settings = window_settings_from_ui(&self.settings);
        layout::apply_windows(
            layout,
            self.settings.seed,
            window_settings,
            self.settings.cols,
            self.settings.rows,
        );
    }

    // Recomputes staircase links across the current levels.
    pub fn refresh_stairs(&mut self) {
        let results = populate_stairs(self.levels.clone(), &self.settings);
        self.levels = results.iter().map(|(layout, _)| layout.clone()).collect();

        for (level_idx, (_, modified)) in results.into_iter().enumerate() {
            if modified {
                self.refresh_level(level_idx);
            }
        }
    }

    // Recomputes generated doors while keeping manual doors.
    pub fn refresh_doors(&mut self) {
        if self.settings.active_level_index >= self.levels.len() {
            return;
        }
        let layout = &mut self.levels[self.settings.active_level_index];

        layout::apply_doors(
            layout,
            self.settings.seed,
            door_settings_from_ui(&self.settings),
            self.settings.cols,
            self.settings.rows,
        );
        layout.doors.retain(|door| {
            !crate::interact::door_edges_for(door, self.settings.cols, self.settings.rows)
                .iter()
                .any(|edge| self.suppressed_auto_door_edges.contains(edge))
        });
        layout::apply_windows(
            layout,
            self.settings.seed,
            window_settings_from_ui(&self.settings),
            self.settings.cols,
            self.settings.rows,
        );
    }
}

// Computes settings from ui.
pub fn door_settings_from_ui(settings: &UiSettings) -> DoorSettings {
    DoorSettings {
        frequency_percent: settings.door_frequency_percent,
        room_hallway_percent: settings.room_hallway_door_percent,
        locked_percent: settings.locked_door_percent,
        secret_percent: settings.secret_door_percent,
        allow_middle_corridor_doors: settings.allow_middle_corridor_doors,
    }
}

// Computes settings from ui.
pub fn window_settings_from_ui(settings: &UiSettings) -> WindowSettings {
    WindowSettings {
        enabled: settings.windows_enabled,
        min_width: settings.min_window_width,
        max_width: settings.max_window_width,
        frequency_percent: settings.window_frequency_percent,
        room_hallway_percent: settings.room_hallway_window_percent,
        allow_internal_windows: settings.allow_internal_windows,
    }
}
