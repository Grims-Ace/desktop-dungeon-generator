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
    AddCorridorDrag, AddDoorDrag, DragState, HoverMarker, MarkerKind, MultiDragState,
    MultiResizeState, ResizeState, SelectedItem, SelectionBoxDrag, draw_grid,
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

#[derive(Debug, Clone)]
pub enum ClipboardItem {
    Room(crate::layout::Room),
    Text(crate::layout::TextLabel),
    Staircase(crate::layout::Staircase),
    Marker(crate::interact::MarkerKind, crate::layout::AreaMarker),
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
    pub rename_level_state: Option<(usize, String)>,
    pub rename_text_state: Option<(usize, String)>,
    pub selection: Vec<SelectedItem>,
    pub selection_box_drag: Option<SelectionBoxDrag>,
    pub multi_drag_state: Option<MultiDragState>,
    pub multi_resize_state: Option<MultiResizeState>,
    pub clipboard_items: Vec<ClipboardItem>,
    pub debug_mode: bool,
}

impl DungeonApp {
    pub fn new(debug_mode: bool) -> Self {
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
            rename_level_state: None,
            rename_text_state: None,
            selection: Vec::new(),
            selection_box_drag: None,
            multi_drag_state: None,
            multi_resize_state: None,
            clipboard_items: Vec::new(),
            debug_mode,
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

impl Default for DungeonApp {
    // Provides default settings for this type.
    fn default() -> Self {
        Self::new(false)
    }
}

impl Drop for DungeonApp {
    // Cleans up background resources when the app is dropped.
    fn drop(&mut self) {
        self.settings.overlay_level_index = None;
        if let Err(err) = settings::save_settings(&self.settings) {
            eprintln!("Failed to save settings: {err}");
        }
    }
}

impl eframe::App for DungeonApp {
    // Runs one UI frame and handles app interaction.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Reset transient states that depend on pointer being down if pointer is released
        if !ctx.input(|i| i.pointer.primary_down()) {
            self.drag_state = None;
            self.multi_drag_state = None;
            self.selection_box_drag = None;
            self.add_corridor_drag = None;
            self.add_door_drag = None;
        }
        if !ctx.input(|i| i.pointer.secondary_down()) {
            self.resize_state = None;
            self.multi_resize_state = None;
        }

        let panel_result = draw_side_panel(
            ctx,
            &mut self.settings,
            self.export_progress
                .map(|progress| (progress.completed, progress.total)),
            self.levels.len(),
        );
        let mut copy_requested = false;
        let mut paste_requested = false;
        let mut undo_requested = false;
        let mut redo_requested = false;

        ctx.input(|i| {
            for event in &i.events {
                match event {
                    egui::Event::Copy => {
                        if self.debug_mode {
                            println!("DEBUG: Received Event::Copy");
                        }
                        copy_requested = true;
                    }
                    egui::Event::Paste(_) => {
                        if self.debug_mode {
                            println!("DEBUG: Received Event::Paste");
                        }
                        paste_requested = true;
                    }
                    egui::Event::Key {
                        key: egui::Key::C,
                        pressed: true,
                        modifiers,
                        ..
                    } if modifiers.command || modifiers.ctrl => {
                        if self.debug_mode {
                            println!("DEBUG: Received Key::C with command/ctrl");
                        }
                        copy_requested = true;
                    }
                    egui::Event::Key {
                        key: egui::Key::V,
                        pressed: true,
                        modifiers,
                        ..
                    } if modifiers.command || modifiers.ctrl => {
                        if self.debug_mode {
                            println!("DEBUG: Received Key::V with command/ctrl");
                        }
                        paste_requested = true;
                    }
                    egui::Event::Key {
                        key: egui::Key::Z,
                        pressed: true,
                        modifiers,
                        ..
                    } if modifiers.command || modifiers.ctrl => {
                        if modifiers.shift {
                            redo_requested = true;
                        } else {
                            undo_requested = true;
                        }
                    }
                    egui::Event::Key {
                        key: egui::Key::Y,
                        pressed: true,
                        modifiers,
                        ..
                    } if modifiers.command || modifiers.ctrl => {
                        redo_requested = true;
                    }
                    _ => {}
                }
            }
        });

        let escape_pressed = ctx.input(|i| i.key_pressed(egui::Key::Escape));

        if escape_pressed || panel_result.settings_changed {
            self.settings.add_tool = AddTool::None;
            if !self.clipboard_items.is_empty() {
                if self.debug_mode {
                    println!("DEBUG: Clipboard cleared (Escape or settings changed)");
                }
            }
            self.clipboard_items.clear();
            if self.debug_mode {
                println!("DEBUG: UI reset (Escape pressed or settings changed)");
            }
        }

        clamp_dependent_settings(&mut self.settings);

        if undo_requested {
            if self.debug_mode {
                println!("DEBUG: Executing Undo");
            }
            self.undo();
        } else if redo_requested {
            if self.debug_mode {
                println!("DEBUG: Executing Redo");
            }
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
                let mut layout = DungeonLayout::empty(self.settings.pack_rooms_without_corridors);
                layout.name = format!("Level {}", self.levels.len() + 1);
                self.levels.push(layout);
            } else {
                self.levels
                    .push(generate_level(&self.settings, self.levels.len()));
                // Stairs first so markers can be placed away from level transitions.
                let stair_results = layout::populate_stairs(self.levels.clone(), &self.settings);
                self.levels = stair_results.into_iter().map(|(l, _)| l).collect();
                layout::assign_start_end_markers(&mut self.levels, &self.settings);
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
            self.hover_level_idx = draw_level_tabs(
                ui,
                &self.levels,
                &mut self.settings.active_level_index,
                &mut self.rename_level_state,
            );
            ui.separator();

            if let Some((level_idx, mut name)) = self.rename_level_state.take() {
                let mut open = true;
                egui::Window::new("Rename Level")
                    .collapsible(false)
                    .resizable(false)
                    .open(&mut open)
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Name:");
                            let res = ui.text_edit_singleline(&mut name);
                            if ui.button("Rename").clicked()
                                || (res.lost_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                            {
                                if level_idx < self.levels.len() {
                                    if self.levels[level_idx].name != name {
                                        self.push_undo_snapshot();
                                        self.levels[level_idx].name = name.clone();
                                    }
                                    self.rename_level_state = None;
                                }
                            }
                        });
                    });

                if open {
                    self.rename_level_state = Some((level_idx, name));
                }
            }

            if let Some((text_idx, mut text)) = self.rename_text_state.take() {
                let mut open = true;
                egui::Window::new("Rename Text")
                    .collapsible(false)
                    .resizable(false)
                    .open(&mut open)
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Text:");
                            let res = ui.text_edit_singleline(&mut text);
                            if ui.button("Rename").clicked()
                                || (res.lost_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                            {
                                let changed = self
                                    .levels
                                    .get(self.settings.active_level_index)
                                    .and_then(|l| l.text_labels.get(text_idx))
                                    .map(|l| l.text != text)
                                    .unwrap_or(false);

                                if changed {
                                    self.push_undo_snapshot();
                                    if let Some(layout) =
                                        self.levels.get_mut(self.settings.active_level_index)
                                    {
                                        if text_idx < layout.text_labels.len() {
                                            layout.text_labels[text_idx].text = text.clone();
                                        }
                                    }
                                }
                                self.rename_text_state = None;
                            }
                        });
                    });

                if open {
                    self.rename_text_state = Some((text_idx, text));
                }
            }

            let available = ui.available_size_before_wrap();
            let (response, painter) = ui.allocate_painter(available, egui::Sense::click_and_drag());
            let canvas = response.rect.shrink(12.0);
            let geometry = draw_grid(&painter, canvas, self.settings.cols, self.settings.rows);
            crate::interact::update_hover_targets(self, ctx, &geometry);

            if copy_requested {
                if !self.selection.is_empty() {
                    if self.debug_mode {
                        println!("DEBUG: Copying {} selected items", self.selection.len());
                    }
                    if let Some(layout) = self.levels.get(self.settings.active_level_index) {
                        let mut items = Vec::new();
                        for selected in &self.selection {
                            match selected {
                                SelectedItem::Room(idx) => {
                                    if let Some(room) = layout.rooms.get(*idx) {
                                        items.push(ClipboardItem::Room(room.clone()));
                                    }
                                }
                                SelectedItem::Text(idx) => {
                                    if let Some(text) = layout.text_labels.get(*idx) {
                                        items.push(ClipboardItem::Text(text.clone()));
                                    }
                                }
                                SelectedItem::Staircase(idx) => {
                                    if let Some(stair) = layout.stairs.get(*idx) {
                                        items.push(ClipboardItem::Staircase(stair.clone()));
                                    }
                                }
                                SelectedItem::Marker(kind, idx) => {
                                    let marker = match kind {
                                        MarkerKind::Start => layout.start_markers.get(*idx),
                                        MarkerKind::End => layout.end_markers.get(*idx),
                                        MarkerKind::Trap => layout.trap_markers.get(*idx),
                                        MarkerKind::Monster => layout.monster_markers.get(*idx),
                                    };
                                    if let Some(m) = marker {
                                        items.push(ClipboardItem::Marker(*kind, m.clone()));
                                    }
                                }
                            }
                        }
                        if self.debug_mode {
                            println!("Copied {} items to clipboard", items.len());
                        }
                        self.clipboard_items = items;
                    }
                } else if let Some(room_idx) = self.hover_room_idx {
                    if let Some(layout) = self.levels.get(self.settings.active_level_index) {
                        if let Some(room) = layout.rooms.get(room_idx) {
                            if self.debug_mode {
                                println!(
                                    "Copied room {}: {}x{} at ({}, {})",
                                    room_idx, room.width, room.height, room.x, room.y
                                );
                            }
                            self.clipboard_items = vec![ClipboardItem::Room(room.clone())];
                        }
                    }
                } else {
                    if self.debug_mode {
                        println!("Copy requested but nothing is selected or hovered.");
                    }
                }
            }

            if paste_requested
                && self.drag_state.is_none()
                && self.multi_drag_state.is_none()
                && self.resize_state.is_none()
                && self.multi_resize_state.is_none()
                && self.add_corridor_drag.is_none()
                && self.add_door_drag.is_none()
                && self.selection_box_drag.is_none()
            {
                if let Some(pointer_pos) = ctx.pointer_hover_pos() {
                    if let Some(grid_pos) = crate::interact::pointer_to_grid(pointer_pos, &geometry)
                    {
                        if !self.clipboard_items.is_empty() {
                            if self.debug_mode {
                                println!(
                                    "Pasting {} items at grid pos: {:?}",
                                    self.clipboard_items.len(),
                                    grid_pos
                                );
                            }
                            self.paste_items(grid_pos);
                        } else {
                            if self.debug_mode {
                                println!("Paste requested but clipboard is empty.");
                            }
                        }
                    }
                }
            } else if paste_requested {
                if self.debug_mode {
                    println!("Paste requested but ignored due to active interaction state:");
                    if self.drag_state.is_some() {
                        println!("  - drag_state");
                    }
                    if self.multi_drag_state.is_some() {
                        println!("  - multi_drag_state");
                    }
                    if self.resize_state.is_some() {
                        println!("  - resize_state");
                    }
                    if self.multi_resize_state.is_some() {
                        println!("  - multi_resize_state");
                    }
                    if self.add_corridor_drag.is_some() {
                        println!("  - add_corridor_drag");
                    }
                    if self.add_door_drag.is_some() {
                        println!("  - add_door_drag");
                    }
                    if self.selection_box_drag.is_some() {
                        println!("  - selection_box_drag");
                    }
                }
            }

            if response.double_clicked() {
                if let Some(text_idx) = self.hover_text_idx {
                    if let Some(layout) = self.levels.get(self.settings.active_level_index) {
                        if let Some(label) = layout.text_labels.get(text_idx) {
                            self.rename_text_state = Some((text_idx, label.text.clone()));
                        }
                    }
                }
            }

            let level_delete_requested = ctx.input(|i| {
                (i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace))
                    && self.hover_level_idx.is_some()
            });
            if level_delete_requested && let Some(level_idx) = self.hover_level_idx {
                self.delete_level(level_idx);
            }

            // --- Paste Mode Mouse Interaction ---
            let in_paste_mode = !self.clipboard_items.is_empty();
            if in_paste_mode {
                if response.secondary_clicked() {
                    if self.debug_mode {
                        println!("DEBUG: Right-click dismissed paste mode.");
                    }
                    self.clipboard_items.clear();
                } else if response.clicked() {
                    if let Some(pointer_pos) = response.interact_pointer_pos() {
                        if let Some(grid_pos) =
                            crate::interact::pointer_to_grid(pointer_pos, &geometry)
                        {
                            if self.debug_mode {
                                println!(
                                    "Left-click pasting {} items at grid pos: {:?}",
                                    self.clipboard_items.len(),
                                    grid_pos
                                );
                            }
                            self.paste_items(grid_pos);
                        }
                    }
                }
            }

            let multi_consumed = if in_paste_mode {
                true
            } else {
                crate::interact::handle_multi_select(self, ctx, &response, &geometry)
            };
            let resizing = if !multi_consumed {
                crate::interact::handle_resize(self, ctx, &response, &geometry)
            } else {
                false
            };
            if !resizing && !multi_consumed {
                crate::interact::handle_drag(self, ctx, &response, &geometry);
            }
            if !in_paste_mode {
                crate::interact::handle_add_tool(self, &response, &geometry);
            }
            if self.pending_delete {
                if let Some(pointer_pos) = ctx.pointer_interact_pos() {
                    crate::interact::delete_at_pointer(self, pointer_pos, &geometry);
                }
                self.pending_delete = false;
                self.selection.clear();
                self.selection_box_drag = None;
                self.multi_drag_state = None;
                self.multi_resize_state = None;
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
                    1.0,
                );
            }

            if let Some(overlay_idx) = self.settings.overlay_level_index {
                if overlay_idx < self.levels.len()
                    && overlay_idx != self.settings.active_level_index
                {
                    draw_layout(
                        &painter,
                        &geometry,
                        &self.levels[overlay_idx],
                        self.settings.colorblind_mode,
                        None,
                        None,
                        None,
                        0.4,
                    );
                }
            }

            let overlay_btn_rect = egui::Rect::from_min_size(
                egui::pos2(canvas.right() - 84.0, canvas.top() + 4.0),
                egui::vec2(80.0, 24.0),
            );
            ui.put(overlay_btn_rect, |ui: &mut egui::Ui| {
                let is_overlay =
                    self.settings.overlay_level_index == Some(self.settings.active_level_index);
                let response = ui.selectable_label(is_overlay, "Overlay");
                if response.clicked() {
                    if is_overlay {
                        self.settings.overlay_level_index = None;
                    } else {
                        self.settings.overlay_level_index = Some(self.settings.active_level_index);
                    }
                }
                response
            });

            crate::interact::draw_add_overlay(self, &painter, &geometry);
            crate::interact::draw_add_tool_ghost(self, &painter, &geometry);
            crate::interact::draw_paste_ghost(self, &painter, &geometry);
            crate::interact::draw_resize_overlay(self, &painter, &geometry);
            crate::interact::draw_corridor_hover_overlay(self, &painter, &geometry);
            crate::interact::draw_door_hover_overlay(self, &painter, &geometry);
            crate::interact::draw_selection_highlights(self, &painter, &geometry);
            crate::interact::draw_selection_box(self, &painter, &geometry);
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
    // Populate stairs first so assign_start_end_markers can place start/end
    // markers as far from the stairs (level transitions) as possible.
    let stair_results = layout::populate_stairs(levels, settings);
    let mut levels: Vec<_> = stair_results.into_iter().map(|(l, _)| l).collect();
    layout::assign_start_end_markers(&mut levels, settings);
    levels
}

// Generates a single dungeon level from the UI settings.
pub fn generate_level(settings: &UiSettings, level_index: usize) -> DungeonLayout {
    let level_seed = crate::seed::derive_seed(settings.seed, level_index as u64);
    let mut layout = layout::generate_layout(
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
    layout.name = format!("Level {}", level_index + 1);

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
    if settings.min_stair_width > settings.max_stair_width {
        settings.max_stair_width = settings.min_stair_width;
    }
    if settings.min_stair_height > settings.max_stair_height {
        settings.max_stair_height = settings.min_stair_height;
    }
    if settings.min_stairs_per_level > settings.max_stairs_per_level {
        settings.max_stairs_per_level = settings.min_stairs_per_level;
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
        self.selection.clear();
        self.selection_box_drag = None;
        self.multi_drag_state = None;
        self.multi_resize_state = None;
        self.rename_text_state = None;
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
        if let Some(overlay_idx) = self.settings.overlay_level_index {
            if overlay_idx == level_idx {
                self.settings.overlay_level_index = None;
            } else if overlay_idx > level_idx {
                self.settings.overlay_level_index = Some(overlay_idx - 1);
            }
        }
        if !self.settings.composition_mode {
            // Stairs first so markers can be placed away from level transitions.
            let stair_results = layout::populate_stairs(self.levels.clone(), &self.settings);
            self.levels = stair_results.into_iter().map(|(l, _)| l).collect();
            layout::assign_start_end_markers(&mut self.levels, &self.settings);
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

    pub fn paste_items(&mut self, grid_pos: (f32, f32)) {
        if self.clipboard_items.is_empty() {
            return;
        }
        self.push_undo_snapshot();
        if self.settings.active_level_index >= self.levels.len() {
            return;
        }

        // Calculate bounding box of clipboard items to find their center
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for item in &self.clipboard_items {
            let (ix, iy, iw, ih) = match item {
                ClipboardItem::Room(r) => (r.x as f32, r.y as f32, r.width as f32, r.height as f32),
                ClipboardItem::Text(t) => (t.cell.0 as f32, t.cell.1 as f32, 1.0, 1.0),
                ClipboardItem::Staircase(s) => (
                    s.cell.0 as f32,
                    s.cell.1 as f32,
                    s.width as f32,
                    s.height as f32,
                ),
                ClipboardItem::Marker(_, m) => (
                    m.cell.0 as f32,
                    m.cell.1 as f32,
                    m.size as f32,
                    m.size as f32,
                ),
            };
            min_x = min_x.min(ix);
            min_y = min_y.min(iy);
            max_x = max_x.max(ix + iw);
            max_y = max_y.max(iy + ih);
        }

        let center_x = (min_x + max_x) / 2.0;
        let center_y = (min_y + max_y) / 2.0;

        let delta_x = grid_pos.0 - center_x;
        let delta_y = grid_pos.1 - center_y;

        // Offset all items and clamp them to the grid
        let active_idx = self.settings.active_level_index;
        let mut rooms_to_add = Vec::new();
        let mut text_to_add = Vec::new();
        let mut stairs_to_add = Vec::new();
        let mut markers_to_add = Vec::new();

        for item in &self.clipboard_items {
            match item {
                ClipboardItem::Room(r) => {
                    let mut new_room = r.clone();
                    let nx = (r.x as f32 + delta_x).round() as isize;
                    let ny = (r.y as f32 + delta_y).round() as isize;
                    new_room.x = nx
                        .clamp(0, (self.settings.cols as isize - r.width as isize).max(0))
                        as usize;
                    new_room.y = ny
                        .clamp(0, (self.settings.rows as isize - r.height as isize).max(0))
                        as usize;
                    rooms_to_add.push(new_room);
                }
                ClipboardItem::Text(t) => {
                    let mut new_text = t.clone();
                    let nx = (t.cell.0 as f32 + delta_x).round() as isize;
                    let ny = (t.cell.1 as f32 + delta_y).round() as isize;
                    new_text.cell.0 = nx.clamp(0, self.settings.cols as isize - 1) as usize;
                    new_text.cell.1 = ny.clamp(0, self.settings.rows as isize - 1) as usize;
                    text_to_add.push(new_text);
                }
                ClipboardItem::Staircase(s) => {
                    let mut new_stair = s.clone();
                    let nx = (s.cell.0 as f32 + delta_x).round() as isize;
                    let ny = (s.cell.1 as f32 + delta_y).round() as isize;
                    new_stair.cell.0 = nx
                        .clamp(0, (self.settings.cols as isize - s.width as isize).max(0))
                        as usize;
                    new_stair.cell.1 = ny
                        .clamp(0, (self.settings.rows as isize - s.height as isize).max(0))
                        as usize;
                    stairs_to_add.push(new_stair);
                }
                ClipboardItem::Marker(kind, m) => {
                    let mut new_marker = m.clone();
                    let nx = (m.cell.0 as f32 + delta_x).round() as isize;
                    let ny = (m.cell.1 as f32 + delta_y).round() as isize;
                    new_marker.cell.0 = nx
                        .clamp(0, (self.settings.cols as isize - m.size as isize).max(0))
                        as usize;
                    new_marker.cell.1 = ny
                        .clamp(0, (self.settings.rows as isize - m.size as isize).max(0))
                        as usize;
                    markers_to_add.push((*kind, new_marker));
                }
            }
        }

        let layout = &mut self.levels[active_idx];
        layout.rooms.extend(rooms_to_add);
        layout.text_labels.extend(text_to_add);
        layout.stairs.extend(stairs_to_add);
        for (kind, marker) in markers_to_add {
            match kind {
                MarkerKind::Start => layout.start_markers.push(marker),
                MarkerKind::End => layout.end_markers.push(marker),
                MarkerKind::Trap => layout.trap_markers.push(marker),
                MarkerKind::Monster => layout.monster_markers.push(marker),
            }
        }

        self.refresh_doors();
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
