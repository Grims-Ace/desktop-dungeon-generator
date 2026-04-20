mod exporter;
mod layout;
mod saveandload;
mod seed;
mod settings;
mod startend;
mod ui;

use std::collections::HashSet;
use std::sync::mpsc;
use std::thread;

use eframe::egui;
use egui::{Color32, Stroke};
use layout::{
    DoorSettings, DungeonLayout, WindowSettings, blocked_room_cells, corridor_cells,
    shortest_path_cells,
};
use startend::{
    manual_marker_size, manual_monster_marker_size, manual_trap_marker_size,
    populate_random_markers,
};
use ui::{AddTool, UiSettings, draw_side_panel};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ManualDoorKind {
    Archway,
    Regular,
    Locked,
    Secret,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AppSnapshot {
    settings: UiSettings,
    layout: DungeonLayout,
    suppressed_auto_door_edges: HashSet<((usize, usize), (usize, usize))>,
}

// Boot the native app and start the egui event loop.
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();

    eframe::run_native(
        "Desktop Dungeon Generator",
        options,
        Box::new(|_cc| Ok(Box::new(DungeonApp::default()))),
    )
}

struct DungeonApp {
    settings: UiSettings,
    layout: DungeonLayout,
    suppressed_auto_door_edges: HashSet<((usize, usize), (usize, usize))>,
    undo_stack: Vec<AppSnapshot>,
    redo_stack: Vec<AppSnapshot>,
    drag_state: Option<DragState>,
    export_rx: Option<mpsc::Receiver<Result<std::path::PathBuf, String>>>,
    pending_delete: bool,
    add_corridor_drag: Option<AddCorridorDrag>,
    resize_state: Option<ResizeState>,
    hover_room_idx: Option<usize>,
    hover_corridor_idx: Option<usize>,
    hover_door_idx: Option<usize>,
    hover_text_idx: Option<usize>,
    hover_marker: Option<HoverMarker>,
}

impl Default for DungeonApp {
    // Build the app with persisted settings and a generated layout.
    fn default() -> Self {
        let settings = settings::load_settings().unwrap_or_default();
        let layout = layout::generate_layout(
            settings.cols,
            settings.rows,
            settings.room_count,
            settings.seed,
            settings.min_room_size,
            settings.max_room_size,
            settings.square_rooms_only,
            settings.min_corridor_width,
            settings.max_corridor_width,
            settings.corridor_randomness,
            settings.dead_end_rooms_percent,
            settings.pack_rooms_without_corridors,
            door_settings_from_ui(&settings),
            window_settings_from_ui(&settings),
        );
        let layout = populate_random_markers(layout, &settings);
        Self {
            settings,
            layout,
            suppressed_auto_door_edges: HashSet::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            drag_state: None,
            export_rx: None,
            pending_delete: false,
            add_corridor_drag: None,
            resize_state: None,
            hover_room_idx: None,
            hover_corridor_idx: None,
            hover_door_idx: None,
            hover_text_idx: None,
            hover_marker: None,
        }
    }
}

impl Drop for DungeonApp {
    // Persist settings when the app is being dropped.
    fn drop(&mut self) {
        if let Err(err) = settings::save_settings(&self.settings) {
            eprintln!("Failed to save settings: {err}");
        }
    }
}

impl eframe::App for DungeonApp {
    // Render UI, handle input, and update app state each frame.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let panel_result = draw_side_panel(ctx, &mut self.settings, self.export_rx.is_some());
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
        } else if panel_result.reset_clicked || panel_result.settings_changed {
            self.push_undo_snapshot();
            self.regenerate_layout();
        } else if panel_result.save_clicked {
            let svg = exporter::build_svg(&self.layout, &self.settings);
            if let Err(err) = saveandload::save_dungeon(&self.settings, &self.layout, svg) {
                eprintln!("{err}");
            }
        } else if panel_result.load_clicked {
            match saveandload::load_dungeon() {
                Ok(data) => {
                    self.push_undo_snapshot();
                    self.settings = data.settings;
                    self.layout = data.layout;
                    // Don't regenerate_layout() here to preserve the exact loaded state.
                }
                Err(err) => {
                    eprintln!("{err}");
                }
            }
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
            let layout = self.layout.clone();
            let settings = self.settings.clone();
            let (tx, rx) = mpsc::channel();
            thread::spawn(move || {
                let result = exporter::export_to_target(&layout, &settings, target);
                let _ = tx.send(result);
            });
            self.export_rx = Some(rx);
        }

        if let Some(rx) = &self.export_rx {
            match rx.try_recv() {
                Ok(Ok(_)) => {
                    self.export_rx = None;
                }
                Ok(Err(err)) => {
                    eprintln!("{err}");
                    self.export_rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.export_rx = None;
                }
            }
        }

        egui::SidePanel::right("legend_panel")
            .resizable(false)
            .min_width(180.0)
            .default_width(200.0)
            .show_separator_line(true)
            .show(ctx, |ui| {
                ui.heading("Accessibility");
                ui.add_space(6.0);
                ui.checkbox(&mut self.settings.colorblind_mode, "Colorblind Mode");
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);
                ui.heading("Legend");
                ui.add_space(8.0);
                if self.settings.colorblind_mode {
                    draw_legend_entry_colorblind(ui, "Rooms", LegendStyle::Crosshatch);
                    draw_legend_entry_colorblind(ui, "Corridors", LegendStyle::Dots);
                    draw_legend_entry_colorblind(ui, "Walls", LegendStyle::SolidLine);
                    draw_legend_entry_colorblind(ui, "Doors", LegendStyle::LongDash);
                    draw_legend_entry_colorblind(ui, "Locked Doors", LegendStyle::ShortDash);
                    draw_legend_entry_colorblind(ui, "Secret Doors", LegendStyle::DashDotDotLine);
                    draw_legend_entry_colorblind(ui, "Archways", LegendStyle::DottedLine);
                    draw_legend_entry_colorblind(ui, "Windows", LegendStyle::DashDotLine);
                    draw_legend_entry_colorblind(ui, "Start Marker", LegendStyle::SolidLine);
                    draw_legend_entry_colorblind(ui, "End Marker", LegendStyle::SolidLine);
                    draw_legend_entry_colorblind(ui, "Trap", LegendStyle::SolidLine);
                    draw_legend_entry_colorblind(ui, "Monster", LegendStyle::SolidLine);
                } else {
                    draw_legend_entry(ui, Color32::from_rgb(70, 120, 160), "Rooms");
                    draw_legend_entry(ui, Color32::from_rgb(210, 190, 120), "Corridors");
                    draw_legend_entry(ui, Color32::BLACK, "Walls");
                    draw_legend_entry(ui, Color32::from_rgb(80, 200, 120), "Doors");
                    draw_legend_entry(ui, Color32::from_rgb(220, 70, 70), "Locked Doors");
                    draw_legend_entry(ui, Color32::from_rgb(170, 80, 170), "Secret Doors");
                    draw_legend_entry(ui, Color32::from_rgb(230, 140, 60), "Archways");
                    draw_legend_entry(ui, Color32::from_rgb(70, 130, 220), "Windows");
                    draw_legend_entry(ui, Color32::from_rgb(60, 220, 200), "Start Marker");
                    draw_legend_entry(ui, Color32::from_rgb(240, 90, 90), "End Marker");
                    draw_legend_entry(ui, Color32::from_rgb(150, 80, 230), "Trap");
                    draw_legend_entry(ui, Color32::from_rgb(200, 50, 50), "Monster");
                }
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);
                ui.heading("Keybinds");
                ui.add_space(6.0);
                ui.label(
                    "• Delete / Backspace: Remove hovered room, corridor, door, text, or marker",
                );
                ui.label("• Ctrl+Z: Undo");
                ui.label("• Ctrl+Y: Redo");
                ui.label("• Left click + drag: Move hovered room");
                ui.label("• Right click: Cancel add tool");
                ui.label("• Right click + drag: Resize hovered room");
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_size_before_wrap();
            let (response, painter) = ui.allocate_painter(available, egui::Sense::click_and_drag());
            let canvas = response.rect.shrink(12.0);
            let geometry = draw_grid(&painter, canvas, self.settings.cols, self.settings.rows);
            self.update_hover_targets(ctx, &geometry);
            let resizing = self.handle_resize(ctx, &response, &geometry);
            if !resizing {
                self.handle_drag(ctx, &response, &geometry);
            }
            self.handle_add_tool(&response, &geometry);
            if self.pending_delete {
                if let Some(pointer_pos) = ctx.pointer_interact_pos() {
                    self.delete_at_pointer(pointer_pos, &geometry);
                }
                self.pending_delete = false;
            }
            draw_layout(
                &painter,
                &geometry,
                &self.layout,
                self.settings.colorblind_mode,
                self.hover_text_idx,
                self.hover_marker,
            );
            self.draw_add_overlay(&painter, &geometry);
            self.draw_resize_overlay(&painter, &geometry);
            self.draw_corridor_hover_overlay(&painter, &geometry);
            self.draw_door_hover_overlay(&painter, &geometry);
        });
    }
}

fn clamp_dependent_settings(settings: &mut UiSettings) {
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
}

impl DungeonApp {
    fn clear_hover_targets(&mut self) {
        self.hover_room_idx = None;
        self.hover_corridor_idx = None;
        self.hover_door_idx = None;
        self.hover_text_idx = None;
        self.hover_marker = None;
    }

    fn reset_transient_state(&mut self) {
        self.drag_state = None;
        self.add_corridor_drag = None;
        self.resize_state = None;
        self.clear_hover_targets();
        self.pending_delete = false;
    }

    fn snapshot(&self) -> AppSnapshot {
        AppSnapshot {
            settings: self.settings.clone(),
            layout: self.layout.clone(),
            suppressed_auto_door_edges: self.suppressed_auto_door_edges.clone(),
        }
    }

    fn push_undo_snapshot(&mut self) {
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

    fn restore_snapshot(&mut self, snapshot: AppSnapshot) {
        self.settings = snapshot.settings;
        self.layout = snapshot.layout;
        self.suppressed_auto_door_edges = snapshot.suppressed_auto_door_edges;
        self.reset_transient_state();
    }

    fn undo(&mut self) {
        let Some(snapshot) = self.undo_stack.pop() else {
            return;
        };
        self.redo_stack.push(self.snapshot());
        self.restore_snapshot(snapshot);
    }

    fn redo(&mut self) {
        let Some(snapshot) = self.redo_stack.pop() else {
            return;
        };
        self.undo_stack.push(self.snapshot());
        self.restore_snapshot(snapshot);
    }

    // Clear the current layout and reset transient interaction state.
    fn clear_layout(&mut self) {
        self.layout = DungeonLayout {
            packed_rooms: self.settings.pack_rooms_without_corridors,
            ..DungeonLayout::default()
        };
        self.suppressed_auto_door_edges.clear();
        self.reset_transient_state();
    }

    // Regenerate the dungeon layout from current settings and reset drag state.
    fn regenerate_layout(&mut self) {
        self.drag_state = None;
        self.suppressed_auto_door_edges.clear();
        let layout = layout::generate_layout(
            self.settings.cols,
            self.settings.rows,
            self.settings.room_count,
            self.settings.seed,
            self.settings.min_room_size,
            self.settings.max_room_size,
            self.settings.square_rooms_only,
            self.settings.min_corridor_width,
            self.settings.max_corridor_width,
            self.settings.corridor_randomness,
            self.settings.dead_end_rooms_percent,
            self.settings.pack_rooms_without_corridors,
            door_settings_from_ui(&self.settings),
            window_settings_from_ui(&self.settings),
        );
        self.layout = populate_random_markers(layout, &self.settings);
    }

    // Handle dragging rooms or corridors with the primary mouse button.
    fn handle_drag(
        &mut self,
        ctx: &egui::Context,
        response: &egui::Response,
        geometry: &GridGeometry,
    ) {
        if !ctx.input(|i| i.pointer.primary_down()) && !response.drag_started() {
            return;
        }
        if self.settings.add_tool != AddTool::None {
            return;
        }
        if response.drag_started()
            && let Some(pointer_pos) = response.interact_pointer_pos()
            && let Some(text_idx) = self.text_at_pointer(pointer_pos, geometry)
        {
            self.push_undo_snapshot();
            self.drag_state = Some(DragState::Text(TextDragState { text_idx }));
        } else if response.drag_started()
            && let Some(pointer_pos) = response.interact_pointer_pos()
            && let Some(marker_drag) = self.marker_drag_at_pointer(pointer_pos, geometry)
        {
            self.push_undo_snapshot();
            self.drag_state = Some(DragState::Marker(marker_drag));
        } else if response.drag_started()
            && let Some(pointer_pos) = response.interact_pointer_pos()
            && let Some((room_idx, offset_x, offset_y)) =
                self.room_at_pointer(pointer_pos, geometry)
        {
            self.push_undo_snapshot();
            self.drag_state = Some(DragState::Room(RoomDragState {
                room_idx,
                offset_x,
                offset_y,
            }));
        } else if response.drag_started()
            && let Some(pointer_pos) = response.interact_pointer_pos()
            && let Some(corridor_drag) = self.corridor_drag_at_pointer(pointer_pos, geometry)
        {
            self.push_undo_snapshot();
            self.drag_state = Some(DragState::Corridor(corridor_drag));
        }

        if response.dragged()
            && let Some(pointer_pos) = response.interact_pointer_pos()
            && let Some(drag_state) = self.drag_state.clone()
        {
            match drag_state {
                DragState::Room(drag) => self.drag_room_to_pointer(drag, pointer_pos, geometry),
                DragState::Text(drag) => self.drag_text_to_pointer(drag, pointer_pos, geometry),
                DragState::Marker(drag) => self.drag_marker_to_pointer(drag, pointer_pos, geometry),
                DragState::Corridor(drag) => {
                    self.drag_corridor_to_pointer(drag, pointer_pos, geometry)
                }
            }
        }

        if response.drag_stopped() || !response.ctx.input(|i| i.pointer.primary_down()) {
            self.drag_state = None;
        }
    }

    // Resolve the top-most room under the pointer and return its index and offset.
    fn room_at_pointer(
        &self,
        pointer_pos: egui::Pos2,
        geometry: &GridGeometry,
    ) -> Option<(usize, f32, f32)> {
        let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;

        for (room_idx, room) in self.layout.rooms.iter().enumerate().rev() {
            let room_x = room.x as f32;
            let room_y = room.y as f32;
            let room_right = room_x + room.width as f32;
            let room_bottom = room_y + room.height as f32;

            if grid_x >= room_x && grid_x < room_right && grid_y >= room_y && grid_y < room_bottom {
                return Some((room_idx, grid_x - room_x, grid_y - room_y));
            }
        }

        None
    }

    // Move a room to follow the pointer and reroute its corridors.
    fn drag_room_to_pointer(
        &mut self,
        drag: RoomDragState,
        pointer_pos: egui::Pos2,
        geometry: &GridGeometry,
    ) {
        let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry) else {
            return;
        };

        if drag.room_idx >= self.layout.rooms.len() {
            self.drag_state = None;
            return;
        }

        let (width, height, old_x, old_y) = {
            let room = &self.layout.rooms[drag.room_idx];
            (room.width, room.height, room.x, room.y)
        };
        let max_x = self.settings.cols.saturating_sub(width);
        let max_y = self.settings.rows.saturating_sub(height);

        let target_x = (grid_x - drag.offset_x).floor().max(0.0) as usize;
        let target_y = (grid_y - drag.offset_y).floor().max(0.0) as usize;
        let new_x = target_x.min(max_x);
        let new_y = target_y.min(max_y);

        if new_x == old_x && new_y == old_y {
            return;
        }

        self.layout.rooms[drag.room_idx].x = new_x;
        self.layout.rooms[drag.room_idx].y = new_y;
        self.reroute_corridors_for_room(drag.room_idx);
        self.refresh_doors();
    }

    fn drag_text_to_pointer(
        &mut self,
        drag: TextDragState,
        pointer_pos: egui::Pos2,
        geometry: &GridGeometry,
    ) {
        let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry) else {
            return;
        };

        if drag.text_idx >= self.layout.text_labels.len() {
            self.drag_state = None;
            return;
        }

        let target = (
            (grid_x.floor().max(0.0) as usize).min(self.settings.cols.saturating_sub(1)),
            (grid_y.floor().max(0.0) as usize).min(self.settings.rows.saturating_sub(1)),
        );

        self.layout.text_labels[drag.text_idx].cell = target;
    }

    // Recalculate corridor paths connected to a moved room.
    fn reroute_corridors_for_room(&mut self, room_idx: usize) {
        if self.layout.packed_rooms {
            return;
        }

        for corridor in &mut self.layout.corridors {
            if corridor.start_room_id != room_idx && corridor.end_room_id != room_idx {
                continue;
            }

            let start = self.layout.rooms[corridor.start_room_id].center_cell();
            let end = self.layout.rooms[corridor.end_room_id].center_cell();
            let blocked = blocked_room_cells(
                &self.layout.rooms,
                &[corridor.start_room_id, corridor.end_room_id],
            );
            if let Some(path) =
                shortest_path_cells(start, end, self.settings.cols, self.settings.rows, &blocked)
            {
                corridor.path = path;
            }
        }
    }

    // Capture corridor drag state when the pointer is over a corridor cell.
    fn corridor_drag_at_pointer(
        &self,
        pointer_pos: egui::Pos2,
        geometry: &GridGeometry,
    ) -> Option<CorridorDragState> {
        let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;
        let clicked = (grid_x.floor() as usize, grid_y.floor() as usize);
        let corridor_index = self
            .layout
            .corridors
            .iter()
            .position(|corridor| corridor.path.contains(&clicked))?;

        Some(CorridorDragState {
            corridor_index,
            original_path: self.layout.corridors[corridor_index].path.clone(),
            original_cell: clicked,
        })
    }

    // Reroute a corridor path through the current pointer cell.
    fn drag_corridor_to_pointer(
        &mut self,
        drag: CorridorDragState,
        pointer_pos: egui::Pos2,
        geometry: &GridGeometry,
    ) {
        let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry) else {
            return;
        };

        if drag.corridor_index >= self.layout.corridors.len() {
            self.drag_state = None;
            return;
        }
        let corridor = &self.layout.corridors[drag.corridor_index];

        let target = (
            (grid_x.floor().max(0.0) as usize).min(self.settings.cols.saturating_sub(1)),
            (grid_y.floor().max(0.0) as usize).min(self.settings.rows.saturating_sub(1)),
        );

        let Some(new_path) = reroute_path_through_cell(
            &drag.original_path,
            drag.original_cell,
            target,
            self.settings.cols,
            self.settings.rows,
            &blocked_room_cells(
                &self.layout.rooms,
                &[corridor.start_room_id, corridor.end_room_id],
            ),
        ) else {
            return;
        };

        self.layout.corridors[drag.corridor_index].path = new_path;
        self.refresh_doors();
    }

    // Rebuild doors based on the current layout and settings.
    fn refresh_doors(&mut self) {
        layout::apply_doors(
            &mut self.layout,
            self.settings.seed,
            door_settings_from_ui(&self.settings),
            self.settings.cols,
            self.settings.rows,
        );
        self.layout.doors.retain(|door| {
            !door_edges_for(door, self.settings.cols, self.settings.rows)
                .iter()
                .any(|edge| self.suppressed_auto_door_edges.contains(edge))
        });
        layout::apply_windows(
            &mut self.layout,
            self.settings.seed,
            window_settings_from_ui(&self.settings),
            self.settings.cols,
            self.settings.rows,
        );
    }

    // Handle add-tool interactions for rooms, corridors, and doors.
    fn handle_add_tool(&mut self, response: &egui::Response, geometry: &GridGeometry) {
        if self.settings.add_tool != AddTool::None && response.secondary_clicked() {
            self.settings.add_tool = AddTool::None;
            self.add_corridor_drag = None;
            return;
        }

        match self.settings.add_tool {
            AddTool::Room => {
                self.add_corridor_drag = None;
                if response.clicked()
                    && let Some(pointer_pos) = response.interact_pointer_pos()
                    && let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry)
                {
                    let cell = (grid_x.floor() as usize, grid_y.floor() as usize);
                    self.add_room_at_cell(cell);
                }
            }
            AddTool::Corridor => {
                if response.drag_started()
                    && let Some(pointer_pos) = response.interact_pointer_pos()
                    && let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry)
                {
                    let cell = (grid_x.floor() as usize, grid_y.floor() as usize);
                    self.add_corridor_drag = Some(AddCorridorDrag {
                        start_cell: cell,
                        current_cell: cell,
                    });
                }

                if response.dragged()
                    && let Some(pointer_pos) = response.interact_pointer_pos()
                    && let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry)
                    && let Some(drag) = &mut self.add_corridor_drag
                {
                    drag.current_cell = (grid_x.floor() as usize, grid_y.floor() as usize);
                }

                if response.drag_stopped() {
                    if let Some(drag) = self.add_corridor_drag.take() {
                        self.add_corridor_between(drag.start_cell, drag.current_cell);
                    }
                }
            }
            AddTool::Archway | AddTool::Door | AddTool::LockedDoor | AddTool::SecretDoor => {
                self.add_corridor_drag = None;
                if response.clicked()
                    && let Some(pointer_pos) = response.interact_pointer_pos()
                {
                    let kind = match self.settings.add_tool {
                        AddTool::Archway => ManualDoorKind::Archway,
                        AddTool::Door => ManualDoorKind::Regular,
                        AddTool::LockedDoor => ManualDoorKind::Locked,
                        AddTool::SecretDoor => ManualDoorKind::Secret,
                        _ => unreachable!(),
                    };
                    self.add_door_at_pointer(pointer_pos, geometry, kind);
                }
            }
            AddTool::Text => {
                self.add_corridor_drag = None;
                if response.clicked()
                    && let Some(pointer_pos) = response.interact_pointer_pos()
                    && let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry)
                {
                    let cell = (grid_x.floor() as usize, grid_y.floor() as usize);
                    self.add_text_at_cell(cell);
                }
            }
            AddTool::StartMarker | AddTool::EndMarker | AddTool::TrapMarker => {
                self.add_corridor_drag = None;
                if response.clicked()
                    && let Some(pointer_pos) = response.interact_pointer_pos()
                    && let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry)
                {
                    let cell = (grid_x.floor() as usize, grid_y.floor() as usize);
                    let kind = match self.settings.add_tool {
                        AddTool::StartMarker => MarkerKind::Start,
                        AddTool::EndMarker => MarkerKind::End,
                        AddTool::TrapMarker => MarkerKind::Trap,
                        _ => unreachable!(),
                    };
                    self.add_marker_at_cell(cell, kind);
                }
            }
            AddTool::None => {
                self.add_corridor_drag = None;
            }
        }
    }

    // Draw the overlay showing the corridor add drag preview.
    fn draw_add_overlay(&self, painter: &egui::Painter, geometry: &GridGeometry) {
        if self.settings.add_tool != AddTool::Corridor {
            return;
        }
        let Some(drag) = &self.add_corridor_drag else {
            return;
        };

        let start = cell_center(geometry, drag.start_cell.0, drag.start_cell.1);
        let end = cell_center(geometry, drag.current_cell.0, drag.current_cell.1);
        let stroke = Stroke::new(2.0, Color32::WHITE);
        painter.circle_filled(start, 3.0, Color32::WHITE);
        draw_dashed_line(painter, start, end, stroke, 8.0, 6.0);
    }

    // Add a room anchored to the given grid cell if space allows.
    fn add_room_at_cell(&mut self, cell: (usize, usize)) {
        self.push_undo_snapshot();
        let width = self.settings.min_room_size.max(1);
        let height = self.settings.min_room_size.max(1);
        if cell.0 >= self.settings.cols || cell.1 >= self.settings.rows {
            return;
        }
        let max_w = self.settings.cols.saturating_sub(cell.0);
        let max_h = self.settings.rows.saturating_sub(cell.1);
        let width = width.min(max_w);
        let height = height.min(max_h);
        if width == 0 || height == 0 {
            return;
        }

        let new_room = layout::Room {
            x: cell.0,
            y: cell.1,
            width,
            height,
        };

        if self
            .layout
            .rooms
            .iter()
            .any(|room| rooms_overlap(&new_room, room))
        {
            return;
        }

        self.layout.rooms.push(new_room);
        self.refresh_doors();
    }

    // Add a corridor between two room cells using a shortest path.
    fn add_corridor_between(&mut self, start: (usize, usize), end: (usize, usize)) {
        self.push_undo_snapshot();
        if start == end {
            return;
        }

        let start_room = room_index_at_cell(&self.layout.rooms, start);
        let end_room = room_index_at_cell(&self.layout.rooms, end);
        let (Some(start_room), Some(end_room)) = (start_room, end_room) else {
            return;
        };

        let blocked = blocked_room_cells(&self.layout.rooms, &[start_room, end_room]);

        let Some(path) =
            shortest_path_cells(start, end, self.settings.cols, self.settings.rows, &blocked)
        else {
            return;
        };

        let next_id = self
            .layout
            .corridors
            .iter()
            .map(|c| c.id)
            .max()
            .unwrap_or(0)
            .wrapping_add(1);
        let width = self.settings.min_corridor_width.max(1);

        self.layout.corridors.push(layout::Corridor {
            id: next_id,
            start_room_id: start_room,
            end_room_id: end_room,
            path,
            width,
        });
        self.refresh_doors();
    }

    fn add_text_at_cell(&mut self, cell: (usize, usize)) {
        if cell.0 >= self.settings.cols || cell.1 >= self.settings.rows {
            return;
        }

        let text = self.settings.add_text_value.trim().to_string();
        if text.is_empty() {
            return;
        }

        self.push_undo_snapshot();
        if let Some(existing) = self
            .layout
            .text_labels
            .iter_mut()
            .find(|label| label.cell == cell)
        {
            existing.text = text;
            return;
        }

        self.layout.text_labels.push(layout::TextLabel {
            cell,
            text,
            font_size: 18,
        });
    }

    fn add_marker_at_cell(&mut self, cell: (usize, usize), kind: MarkerKind) {
        if cell.0 >= self.settings.cols || cell.1 >= self.settings.rows {
            return;
        }

        let size = match kind {
            MarkerKind::Start => manual_marker_size(&self.settings, true),
            MarkerKind::End => manual_marker_size(&self.settings, false),
            MarkerKind::Trap => manual_trap_marker_size(&self.settings),
            MarkerKind::Monster => manual_monster_marker_size(&self.settings),
        };
        let origin = centered_marker_origin(cell, size, self.settings.cols, self.settings.rows);
        let marker = layout::AreaMarker { cell: origin, size };

        self.push_undo_snapshot();
        match kind {
            MarkerKind::Start => self.layout.start_markers.push(marker),
            MarkerKind::End => self.layout.end_markers.push(marker),
            MarkerKind::Trap => self.layout.trap_markers.push(marker),
            MarkerKind::Monster => self.layout.monster_markers.push(marker),
        }
    }

    // Update which room/corridor/door is currently hovered.
    fn update_hover_targets(&mut self, ctx: &egui::Context, geometry: &GridGeometry) {
        let Some(pointer_pos) = ctx.pointer_hover_pos() else {
            self.clear_hover_targets();
            return;
        };
        if let Some(edge) = self.door_edge_at_pointer(pointer_pos, geometry) {
            let target = normalized_edge(edge.0, edge.1);
            self.hover_door_idx = self.layout.doors.iter().position(|d| {
                door_edges_for(d, self.settings.cols, self.settings.rows)
                    .iter()
                    .any(|e| normalized_edge(e.0, e.1) == target)
            });
            if self.hover_door_idx.is_some() {
                self.hover_room_idx = None;
                self.hover_corridor_idx = None;
                self.hover_text_idx = None;
                self.hover_marker = None;
                return;
            }
        }

        self.hover_text_idx = self.text_at_pointer(pointer_pos, geometry);
        if self.hover_text_idx.is_some() {
            self.hover_room_idx = None;
            self.hover_corridor_idx = None;
            self.hover_door_idx = None;
            self.hover_marker = None;
            return;
        }

        self.hover_marker = self.marker_at_pointer(pointer_pos, geometry);
        if self.hover_marker.is_some() {
            self.hover_room_idx = None;
            self.hover_corridor_idx = None;
            self.hover_door_idx = None;
            return;
        }

        if let Some((room_idx, _, _)) = self.room_at_pointer(pointer_pos, geometry) {
            self.hover_room_idx = Some(room_idx);
            self.hover_corridor_idx = None;
            self.hover_door_idx = None;
            self.hover_marker = None;
        } else {
            self.hover_room_idx = None;
            if let Some(corridor_drag) = self.corridor_drag_at_pointer(pointer_pos, geometry) {
                self.hover_corridor_idx = Some(corridor_drag.corridor_index);
                self.hover_door_idx = None;
                self.hover_marker = None;
            } else {
                self.hover_corridor_idx = None;
                self.hover_door_idx = None;
                self.hover_marker = None;
            }
        }
    }

    fn text_at_pointer(&self, pointer_pos: egui::Pos2, geometry: &GridGeometry) -> Option<usize> {
        let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;
        let clicked = (grid_x.floor() as usize, grid_y.floor() as usize);
        self.layout
            .text_labels
            .iter()
            .position(|label| label.cell == clicked)
    }

    fn marker_at_pointer(
        &self,
        pointer_pos: egui::Pos2,
        geometry: &GridGeometry,
    ) -> Option<HoverMarker> {
        let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;
        let clicked = (grid_x.floor() as usize, grid_y.floor() as usize);
        self.layout
            .start_markers
            .iter()
            .enumerate()
            .find(|(_, marker)| marker_contains_cell(Some(marker), clicked))
            .map(|(idx, _)| HoverMarker::Start(idx))
            .or_else(|| {
                self.layout
                    .end_markers
                    .iter()
                    .enumerate()
                    .find(|(_, marker)| marker_contains_cell(Some(marker), clicked))
                    .map(|(idx, _)| HoverMarker::End(idx))
            })
            .or_else(|| {
                self.layout
                    .trap_markers
                    .iter()
                    .enumerate()
                    .find(|(_, marker)| marker_contains_cell(Some(marker), clicked))
                    .map(|(idx, _)| HoverMarker::Trap(idx))
            })
            .or_else(|| {
                self.layout
                    .monster_markers
                    .iter()
                    .enumerate()
                    .find(|(_, marker)| marker_contains_cell(Some(marker), clicked))
                    .map(|(idx, _)| HoverMarker::Monster(idx))
            })
    }

    fn marker_drag_at_pointer(
        &self,
        pointer_pos: egui::Pos2,
        geometry: &GridGeometry,
    ) -> Option<MarkerDragState> {
        let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;
        match self.marker_at_pointer(pointer_pos, geometry)? {
            HoverMarker::Start(marker_idx) => {
                let marker = self.layout.start_markers.get(marker_idx)?;
                Some(MarkerDragState {
                    kind: MarkerKind::Start,
                    marker_idx,
                    offset_x: grid_x - marker.cell.0 as f32,
                    offset_y: grid_y - marker.cell.1 as f32,
                })
            }
            HoverMarker::End(marker_idx) => {
                let marker = self.layout.end_markers.get(marker_idx)?;
                Some(MarkerDragState {
                    kind: MarkerKind::End,
                    marker_idx,
                    offset_x: grid_x - marker.cell.0 as f32,
                    offset_y: grid_y - marker.cell.1 as f32,
                })
            }
            HoverMarker::Trap(marker_idx) => {
                let marker = self.layout.trap_markers.get(marker_idx)?;
                Some(MarkerDragState {
                    kind: MarkerKind::Trap,
                    marker_idx,
                    offset_x: grid_x - marker.cell.0 as f32,
                    offset_y: grid_y - marker.cell.1 as f32,
                })
            }
            HoverMarker::Monster(marker_idx) => {
                let marker = self.layout.monster_markers.get(marker_idx)?;
                Some(MarkerDragState {
                    kind: MarkerKind::Monster,
                    marker_idx,
                    offset_x: grid_x - marker.cell.0 as f32,
                    offset_y: grid_y - marker.cell.1 as f32,
                })
            }
        }
    }

    fn drag_marker_to_pointer(
        &mut self,
        drag: MarkerDragState,
        pointer_pos: egui::Pos2,
        geometry: &GridGeometry,
    ) {
        let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry) else {
            return;
        };

        let markers = match drag.kind {
            MarkerKind::Start => &mut self.layout.start_markers,
            MarkerKind::End => &mut self.layout.end_markers,
            MarkerKind::Trap => &mut self.layout.trap_markers,
            MarkerKind::Monster => &mut self.layout.monster_markers,
        };
        let Some(marker) = markers.get_mut(drag.marker_idx) else {
            self.drag_state = None;
            return;
        };

        let max_x = self.settings.cols.saturating_sub(marker.size);
        let max_y = self.settings.rows.saturating_sub(marker.size);
        let target_x = (grid_x - drag.offset_x).floor().max(0.0) as usize;
        let target_y = (grid_y - drag.offset_y).floor().max(0.0) as usize;
        marker.cell = (target_x.min(max_x), target_y.min(max_y));
    }

    // Handle right-click resizing of rooms and return whether resizing is active.
    fn handle_resize(
        &mut self,
        ctx: &egui::Context,
        response: &egui::Response,
        geometry: &GridGeometry,
    ) -> bool {
        if self.settings.add_tool != AddTool::None {
            return false;
        }

        if ctx.input(|i| i.pointer.secondary_pressed()) && self.resize_state.is_none() {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                if let Some(text_idx) = self.text_at_pointer(pointer_pos, geometry) {
                    if let Some(state) = self.start_text_resize(text_idx) {
                        self.push_undo_snapshot();
                        self.resize_state = Some(ResizeState::Text(state));
                        return true;
                    }
                } else if let Some(room_idx) = self.resize_room_at_pointer(pointer_pos, geometry) {
                    if let Some(state) = self.start_resize(room_idx, pointer_pos, geometry) {
                        self.push_undo_snapshot();
                        self.resize_state = Some(ResizeState::Room(state));
                        return true;
                    }
                }
            }
        }

        if let Some(state) = self.resize_state.clone() {
            if response.dragged_by(egui::PointerButton::Secondary)
                && ctx.input(|i| i.pointer.secondary_down())
                && let Some(pointer_pos) = response.interact_pointer_pos()
            {
                if let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry) {
                    let cell = (grid_x.floor() as usize, grid_y.floor() as usize);
                    match state {
                        ResizeState::Room(state) => self.resize_room(&state, cell),
                        ResizeState::Text(state) => self.resize_text(&state, cell),
                    }
                }
            }

            if ctx.input(|i| i.pointer.button_released(egui::PointerButton::Secondary)) {
                self.resize_state = None;
            }
            return true;
        }

        false
    }

    fn start_text_resize(&self, text_idx: usize) -> Option<TextResizeState> {
        let label = self.layout.text_labels.get(text_idx)?;
        Some(TextResizeState {
            text_idx,
            origin_cell: label.cell,
            original_font_size: label.font_size,
        })
    }

    // Resolve the room that should respond to a resize click, including outer corner targets.
    fn resize_room_at_pointer(
        &self,
        pointer_pos: egui::Pos2,
        geometry: &GridGeometry,
    ) -> Option<usize> {
        if let Some((room_idx, _, _)) = self.room_at_pointer(pointer_pos, geometry) {
            return Some(room_idx);
        }

        let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;
        let clicked = (
            grid_x.floor().max(0.0) as usize,
            grid_y.floor().max(0.0) as usize,
        );

        for (room_idx, room) in self.layout.rooms.iter().enumerate().rev() {
            if resize_hit_cells(
                room,
                room_idx,
                &self.layout.rooms,
                geometry.cols,
                geometry.rows,
            )
            .contains(&clicked)
            {
                return Some(room_idx);
            }
        }

        None
    }

    // Pick the resize corner from the initial click quadrant and keep the opposite corner fixed.
    fn start_resize(
        &self,
        room_idx: usize,
        pointer_pos: egui::Pos2,
        geometry: &GridGeometry,
    ) -> Option<RoomResizeState> {
        let original_room = self.layout.rooms.get(room_idx)?.clone();
        let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;
        let center_x = original_room.x as f32 + original_room.width as f32 * 0.5;
        let center_y = original_room.y as f32 + original_room.height as f32 * 0.5;
        let pointer_x = grid_x;
        let pointer_y = grid_y;

        let resize_corner = match (pointer_x >= center_x, pointer_y >= center_y) {
            (false, false) => ResizeCorner::TopLeft,
            (true, false) => ResizeCorner::TopRight,
            (false, true) => ResizeCorner::BottomLeft,
            (true, true) => ResizeCorner::BottomRight,
        };

        Some(RoomResizeState {
            room_idx,
            original_room,
            resize_corner,
        })
    }

    // Resize only the corner chosen at drag start while keeping the opposite corner fixed.
    fn resize_room(&mut self, state: &RoomResizeState, target_cell: (usize, usize)) {
        let cols = self.settings.cols.max(1);
        let rows = self.settings.rows.max(1);

        let tx = target_cell.0.min(cols - 1) as isize;
        let ty = target_cell.1.min(rows - 1) as isize;
        let original = &state.original_room;

        let left = original.x as isize;
        let top = original.y as isize;
        let right = (original.x + original.width - 1) as isize;
        let bottom = (original.y + original.height - 1) as isize;
        let (x0, x1, y0, y1) = match state.resize_corner {
            ResizeCorner::TopLeft => (tx.min(right), right, ty.min(bottom), bottom),
            ResizeCorner::TopRight => (left, tx.max(left), ty.min(bottom), bottom),
            ResizeCorner::BottomLeft => (tx.min(right), right, top, ty.max(top)),
            ResizeCorner::BottomRight => (left, tx.max(left), top, ty.max(top)),
        };

        let new_room = layout::Room {
            x: x0 as usize,
            y: y0 as usize,
            width: (x1 - x0 + 1) as usize,
            height: (y1 - y0 + 1) as usize,
        };

        if new_room.width == 0 || new_room.height == 0 {
            return;
        }

        if new_room.x + new_room.width > cols || new_room.y + new_room.height > rows {
            return;
        }

        if self
            .layout
            .rooms
            .iter()
            .enumerate()
            .any(|(idx, room)| idx != state.room_idx && rooms_overlap(&new_room, room))
        {
            return;
        }

        if let Some(room) = self.layout.rooms.get_mut(state.room_idx) {
            room.x = new_room.x;
            room.y = new_room.y;
            room.width = new_room.width;
            room.height = new_room.height;
            self.reroute_corridors_for_room(state.room_idx);
            self.refresh_doors();
        }
    }

    fn resize_text(&mut self, state: &TextResizeState, target_cell: (usize, usize)) {
        let Some(label) = self.layout.text_labels.get_mut(state.text_idx) else {
            self.resize_state = None;
            return;
        };

        let dx = target_cell.0 as isize - state.origin_cell.0 as isize;
        let dy = target_cell.1 as isize - state.origin_cell.1 as isize;
        let delta_cells = if dx.abs() >= dy.abs() { dx } else { -dy };
        let new_size = (state.original_font_size as isize + (delta_cells * 4)).clamp(8, 96);
        label.font_size = new_size as u16;
    }

    // Draw dashed overlays for room resizing interactions.
    fn draw_resize_overlay(&self, painter: &egui::Painter, geometry: &GridGeometry) {
        if let Some(text_idx) = self
            .resize_state
            .as_ref()
            .and_then(|s| match s {
                ResizeState::Text(state) => Some(state.text_idx),
                ResizeState::Room(_) => None,
            })
            .or(self.hover_text_idx)
        {
            if let Some(label) = self.layout.text_labels.get(text_idx) {
                let rect = text_label_rect(geometry, label).expand(3.0);
                painter.rect_stroke(
                    rect,
                    0.0,
                    Stroke::new(2.0, Color32::from_gray(180)),
                    egui::StrokeKind::Middle,
                );
                return;
            }
        }

        let room_idx = self
            .resize_state
            .as_ref()
            .and_then(|s| match s {
                ResizeState::Room(state) => Some(state.room_idx),
                ResizeState::Text(_) => None,
            })
            .or(self.hover_room_idx);
        let Some(room_idx) = room_idx else {
            return;
        };
        let Some(room) = self.layout.rooms.get(room_idx) else {
            return;
        };
        let color = Color32::from_gray(170);
        let stroke = Stroke::new(1.5, color);

        let left = geometry.rect.left() + room.x as f32 * geometry.cell_size;
        let top = geometry.rect.top() + room.y as f32 * geometry.cell_size;
        let right = left + room.width as f32 * geometry.cell_size;
        let bottom = top + room.height as f32 * geometry.cell_size;

        let tl = egui::pos2(left, top);
        let tr = egui::pos2(right, top);
        let br = egui::pos2(right, bottom);
        let bl = egui::pos2(left, bottom);

        draw_dashed_line(painter, tl, tr, stroke, 6.0, 6.0);
        draw_dashed_line(painter, tr, br, stroke, 6.0, 6.0);
        draw_dashed_line(painter, br, bl, stroke, 6.0, 6.0);
        draw_dashed_line(painter, bl, tl, stroke, 6.0, 6.0);

        let font = egui::FontId::proportional(12.0);
        let center_top = egui::pos2((left + right) * 0.5, top - 10.0);
        let center_left = egui::pos2(left - 12.0, (top + bottom) * 0.5);
        painter.text(
            center_top,
            egui::Align2::CENTER_CENTER,
            "<->",
            font.clone(),
            color,
        );
        painter.text(center_left, egui::Align2::CENTER_CENTER, "^v", font, color);

        if matches!(self.resize_state, Some(ResizeState::Room(_))) {
            draw_resize_handle(painter, tl, ResizeCorner::TopLeft, color);
            draw_resize_handle(painter, tr, ResizeCorner::TopRight, color);
            draw_resize_handle(painter, br, ResizeCorner::BottomRight, color);
            draw_resize_handle(painter, bl, ResizeCorner::BottomLeft, color);
        }
    }

    // Draw a dashed overlay along the hovered corridor path.
    fn draw_corridor_hover_overlay(&self, painter: &egui::Painter, geometry: &GridGeometry) {
        let Some(idx) = self.hover_corridor_idx else {
            return;
        };
        let Some(corridor) = self.layout.corridors.get(idx) else {
            return;
        };

        let color = Color32::from_gray(170);
        let stroke = Stroke::new(2.0, color);
        for pair in corridor.path.windows(2) {
            let a = cell_center(geometry, pair[0].0, pair[0].1);
            let b = cell_center(geometry, pair[1].0, pair[1].1);
            draw_dashed_line(painter, a, b, stroke, 6.0, 6.0);
        }
    }

    // Draw a dashed overlay along the hovered door.
    fn draw_door_hover_overlay(&self, painter: &egui::Painter, geometry: &GridGeometry) {
        let Some(idx) = self.hover_door_idx else {
            return;
        };
        let Some(door) = self.layout.doors.get(idx) else {
            return;
        };
        let color = Color32::from_gray(200);
        let stroke = Stroke::new(2.0, color);
        draw_door_line(
            painter,
            geometry,
            door.from,
            door.to,
            door_render_width(door),
            stroke,
            DoorLineStyle::LongDash,
        );
    }

    // Delete the door, room, or corridor at the pointer position.
    fn delete_at_pointer(&mut self, pointer_pos: egui::Pos2, geometry: &GridGeometry) {
        if let Some(idx) = self.text_at_pointer(pointer_pos, geometry) {
            self.push_undo_snapshot();
            if idx < self.layout.text_labels.len() {
                self.layout.text_labels.remove(idx);
            }
            return;
        }

        if let Some(marker) = self.marker_at_pointer(pointer_pos, geometry) {
            self.push_undo_snapshot();
            match marker {
                HoverMarker::Start(idx) => {
                    if idx < self.layout.start_markers.len() {
                        self.layout.start_markers.remove(idx);
                    }
                }
                HoverMarker::End(idx) => {
                    if idx < self.layout.end_markers.len() {
                        self.layout.end_markers.remove(idx);
                    }
                }
                HoverMarker::Trap(idx) => {
                    if idx < self.layout.trap_markers.len() {
                        self.layout.trap_markers.remove(idx);
                    }
                }
                HoverMarker::Monster(idx) => {
                    if idx < self.layout.monster_markers.len() {
                        self.layout.monster_markers.remove(idx);
                    }
                }
            }
            return;
        }

        if let Some(edge) = self.door_edge_at_pointer(pointer_pos, geometry) {
            let target = normalized_edge(edge.0, edge.1);
            if let Some(idx) = self.layout.doors.iter().position(|d| {
                door_edges_for(d, self.settings.cols, self.settings.rows)
                    .iter()
                    .any(|e| normalized_edge(e.0, e.1) == target)
            }) {
                self.push_undo_snapshot();
                if !self.layout.doors[idx].manual {
                    for edge in door_edges_for(
                        &self.layout.doors[idx],
                        self.settings.cols,
                        self.settings.rows,
                    ) {
                        self.suppressed_auto_door_edges.insert(edge);
                    }
                    self.layout.doors.remove(idx);
                    return;
                }

                self.layout.doors.remove(idx);
                return;
            }
        }

        if let Some((room_idx, _, _)) = self.room_at_pointer(pointer_pos, geometry) {
            self.push_undo_snapshot();
            self.delete_room(room_idx);
            return;
        }

        if let Some(corridor_drag) = self.corridor_drag_at_pointer(pointer_pos, geometry) {
            if corridor_drag.corridor_index < self.layout.corridors.len() {
                self.push_undo_snapshot();
                self.layout.corridors.remove(corridor_drag.corridor_index);
                self.drag_state = None;
                self.refresh_doors();
            }
        }
    }

    // Remove a room and update corridors/doors to match.
    fn delete_room(&mut self, room_idx: usize) {
        if room_idx >= self.layout.rooms.len() {
            return;
        }
        self.layout.rooms.remove(room_idx);
        self.layout
            .corridors
            .retain(|c| c.start_room_id != room_idx && c.end_room_id != room_idx);
        for corridor in &mut self.layout.corridors {
            if corridor.start_room_id > room_idx {
                corridor.start_room_id -= 1;
            }
            if corridor.end_room_id > room_idx {
                corridor.end_room_id -= 1;
            }
        }
        self.drag_state = None;
        self.refresh_doors();
    }

    // Add or update a door at the pointer edge.
    fn add_door_at_pointer(
        &mut self,
        pointer_pos: egui::Pos2,
        geometry: &GridGeometry,
        kind: ManualDoorKind,
    ) {
        self.push_undo_snapshot();
        let Some(edge) = self.door_edge_at_pointer(pointer_pos, geometry) else {
            return;
        };
        let target = normalized_edge(edge.0, edge.1);
        if let Some(idx) = self.layout.doors.iter().position(|d| {
            door_edges_for(d, self.settings.cols, self.settings.rows)
                .iter()
                .any(|e| normalized_edge(e.0, e.1) == target)
        }) {
            let edges = door_edges_for(
                &self.layout.doors[idx],
                self.settings.cols,
                self.settings.rows,
            );
            let mut spawned = Vec::new();
            for edge in edges {
                if normalized_edge(edge.0, edge.1) == target {
                    continue;
                }
                if self
                    .layout
                    .doors
                    .iter()
                    .any(|d| normalized_edge(d.from, d.to) == normalized_edge(edge.0, edge.1))
                {
                    continue;
                }
                spawned.push(layout::Door {
                    from: edge.0,
                    to: edge.1,
                    width: 1,
                    span_width: false,
                    locked: false,
                    archway: true,
                    secret: false,
                    manual: true,
                });
            }
            let door = &mut self.layout.doors[idx];
            door.locked = kind == ManualDoorKind::Locked;
            door.archway = kind == ManualDoorKind::Archway;
            door.secret = kind == ManualDoorKind::Secret;
            door.width = 1;
            door.span_width = false;
            door.manual = true;
            for edge in door_edges_for(door, self.settings.cols, self.settings.rows) {
                self.suppressed_auto_door_edges.remove(&edge);
            }
            self.layout.doors.extend(spawned);
            return;
        }

        let corridor_cells = corridor_cells(&self.layout, self.settings.cols, self.settings.rows);
        let a_room = room_index_at_cell(&self.layout.rooms, edge.0);
        let b_room = room_index_at_cell(&self.layout.rooms, edge.1);
        let a_in_corridor = corridor_cells.contains(&edge.0);
        let b_in_corridor = corridor_cells.contains(&edge.1);

        if !manual_door_edge_allowed(a_room, b_room, a_in_corridor, b_in_corridor) {
            return;
        }

        self.layout.doors.push(layout::Door {
            from: edge.0,
            to: edge.1,
            width: 1,
            span_width: false,
            locked: kind == ManualDoorKind::Locked,
            archway: kind == ManualDoorKind::Archway,
            secret: kind == ManualDoorKind::Secret,
            manual: true,
        });
        self.suppressed_auto_door_edges.remove(&target);
    }

    // Find the nearest cell edge under the pointer for door placement.
    fn door_edge_at_pointer(
        &self,
        pointer_pos: egui::Pos2,
        geometry: &GridGeometry,
    ) -> Option<((usize, usize), (usize, usize))> {
        let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;
        let cell_x = grid_x.floor() as isize;
        let cell_y = grid_y.floor() as isize;
        if cell_x < 0
            || cell_y < 0
            || cell_x >= self.settings.cols as isize
            || cell_y >= self.settings.rows as isize
        {
            return None;
        }

        let fx = grid_x - grid_x.floor();
        let fy = grid_y - grid_y.floor();
        let left = fx;
        let right = 1.0 - fx;
        let top = fy;
        let bottom = 1.0 - fy;

        let mut candidates = [
            (left, (cell_x - 1, cell_y), (cell_x, cell_y)),
            (right, (cell_x + 1, cell_y), (cell_x, cell_y)),
            (top, (cell_x, cell_y - 1), (cell_x, cell_y)),
            (bottom, (cell_x, cell_y + 1), (cell_x, cell_y)),
        ];
        candidates.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let (dist, a, b) = candidates[0];
        if dist > 0.2 {
            return None;
        }

        let (ax, ay) = a;
        let (bx, by) = b;
        if ax < 0
            || ay < 0
            || bx < 0
            || by < 0
            || ax >= self.settings.cols as isize
            || bx >= self.settings.cols as isize
            || ay >= self.settings.rows as isize
            || by >= self.settings.rows as isize
        {
            return None;
        }

        Some(((ax as usize, ay as usize), (bx as usize, by as usize)))
    }
}

#[derive(Debug, Clone)]
struct RoomDragState {
    room_idx: usize,
    offset_x: f32,
    offset_y: f32,
}

#[derive(Debug, Clone)]
struct TextDragState {
    text_idx: usize,
}

#[derive(Debug, Clone)]
struct MarkerDragState {
    kind: MarkerKind,
    marker_idx: usize,
    offset_x: f32,
    offset_y: f32,
}

#[derive(Debug, Clone)]
struct CorridorDragState {
    corridor_index: usize,
    original_path: Vec<(usize, usize)>,
    original_cell: (usize, usize),
}

#[derive(Debug, Clone)]
struct AddCorridorDrag {
    start_cell: (usize, usize),
    current_cell: (usize, usize),
}

#[derive(Debug, Clone, Copy)]
enum ResizeCorner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MarkerKind {
    Start,
    End,
    Trap,
    Monster,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HoverMarker {
    Start(usize),
    End(usize),
    Trap(usize),
    Monster(usize),
}

#[derive(Debug, Clone)]
struct RoomResizeState {
    room_idx: usize,
    original_room: layout::Room,
    resize_corner: ResizeCorner,
}

#[derive(Debug, Clone)]
enum DragState {
    Room(RoomDragState),
    Text(TextDragState),
    Marker(MarkerDragState),
    Corridor(CorridorDragState),
}

#[derive(Debug, Clone)]
struct TextResizeState {
    text_idx: usize,
    origin_cell: (usize, usize),
    original_font_size: u16,
}

#[derive(Debug, Clone)]
enum ResizeState {
    Room(RoomResizeState),
    Text(TextResizeState),
}

struct GridGeometry {
    rect: egui::Rect,
    cell_size: f32,
    cols: usize,
    rows: usize,
}

// Draw the grid and return geometry metrics for hit testing.
fn draw_grid(painter: &egui::Painter, area: egui::Rect, cols: usize, rows: usize) -> GridGeometry {
    let cols = cols.max(1);
    let rows = rows.max(1);

    let cell_size = (area.width() / cols as f32).min(area.height() / rows as f32);
    let grid_width = cell_size * cols as f32;
    let grid_height = cell_size * rows as f32;

    let grid_rect = egui::Rect::from_min_size(
        egui::pos2(
            area.center().x - (grid_width * 0.5),
            area.center().y - (grid_height * 0.5),
        ),
        egui::vec2(grid_width, grid_height),
    );

    let stroke = Stroke::new(1.0, Color32::from_gray(160));
    painter.rect_stroke(grid_rect, 0.0, stroke, egui::StrokeKind::Middle);

    for col in 1..cols {
        let x = grid_rect.left() + (col as f32 * cell_size);
        painter.line_segment(
            [
                egui::pos2(x, grid_rect.top()),
                egui::pos2(x, grid_rect.bottom()),
            ],
            stroke,
        );
    }

    for row in 1..rows {
        let y = grid_rect.top() + (row as f32 * cell_size);
        painter.line_segment(
            [
                egui::pos2(grid_rect.left(), y),
                egui::pos2(grid_rect.right(), y),
            ],
            stroke,
        );
    }

    GridGeometry {
        rect: grid_rect,
        cell_size,
        cols,
        rows,
    }
}

// Render rooms, corridors, walls, and doors in the main canvas.
fn draw_layout(
    painter: &egui::Painter,
    geometry: &GridGeometry,
    layout: &DungeonLayout,
    colorblind_mode: bool,
    hover_text_idx: Option<usize>,
    hover_marker: Option<HoverMarker>,
) {
    let wall_width_px = (geometry.cell_size / 5.0).max(1.0);
    let door_width_px = (geometry.cell_size / 10.0).max(1.0);

    let room_fill = Color32::from_rgb(70, 120, 160);
    let corridor_fill = Color32::from_rgb(210, 190, 120);
    let wall_color = Color32::BLACK;
    let corridor_cells_set = corridor_cells(layout, geometry.cols, geometry.rows);
    let corridor_edges_set = corridor_edges_from_cells(&corridor_cells_set);
    let mut room_cells_set = HashSet::new();
    let room_edges_set = room_edges_from_rooms(&layout.rooms);
    let mut door_edges_set = HashSet::new();

    for door in &layout.doors {
        for edge in door_edges_for(door, geometry.cols, geometry.rows) {
            door_edges_set.insert(edge);
        }
    }

    for &(col, row) in &corridor_cells_set {
        painter.rect_filled(cell_rect(geometry, col, row), 0.0, corridor_fill);
        if colorblind_mode {
            painter.circle_filled(
                cell_center(geometry, col, row),
                geometry.cell_size * 0.14,
                Color32::BLACK,
            );
        }
    }

    for &(col, row) in &corridor_cells_set {
        let rect = cell_rect(geometry, col, row);
        let right = (col + 1, row);
        let bottom = (col, row + 1);
        let left = col.checked_sub(1).map(|x| (x, row));
        let top = row.checked_sub(1).map(|y| (col, y));

        if col == 0
            || (!corridor_cells_set.contains(&(col - 1, row))
                && left
                    .map(|n| !door_edges_set.contains(&normalized_edge((col, row), n)))
                    .unwrap_or(true))
        {
            draw_wall_segment(
                painter,
                egui::pos2(rect.left(), rect.top()),
                egui::pos2(rect.left(), rect.bottom()),
                wall_width_px,
                wall_color,
            );
        }
        if (!corridor_cells_set.contains(&right)
            || !corridor_edges_set.contains(&normalized_edge((col, row), right)))
            && !door_edges_set.contains(&normalized_edge((col, row), right))
        {
            draw_wall_segment(
                painter,
                egui::pos2(rect.right(), rect.top()),
                egui::pos2(rect.right(), rect.bottom()),
                wall_width_px,
                wall_color,
            );
        }
        if row == 0
            || (!corridor_cells_set.contains(&(col, row - 1))
                && top
                    .map(|n| !door_edges_set.contains(&normalized_edge((col, row), n)))
                    .unwrap_or(true))
        {
            draw_wall_segment(
                painter,
                egui::pos2(rect.left(), rect.top()),
                egui::pos2(rect.right(), rect.top()),
                wall_width_px,
                wall_color,
            );
        }
        if (!corridor_cells_set.contains(&bottom)
            || !corridor_edges_set.contains(&normalized_edge((col, row), bottom)))
            && !door_edges_set.contains(&normalized_edge((col, row), bottom))
        {
            draw_wall_segment(
                painter,
                egui::pos2(rect.left(), rect.bottom()),
                egui::pos2(rect.right(), rect.bottom()),
                wall_width_px,
                wall_color,
            );
        }
    }

    for room in &layout.rooms {
        for x in room.x..(room.x + room.width) {
            for y in room.y..(room.y + room.height) {
                room_cells_set.insert((x, y));
            }
        }

        let left = geometry.rect.left() + room.x as f32 * geometry.cell_size;
        let top = geometry.rect.top() + room.y as f32 * geometry.cell_size;
        let right = left + room.width as f32 * geometry.cell_size;
        let bottom = top + room.height as f32 * geometry.cell_size;
        let room_rect = egui::Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, bottom));

        painter.rect_filled(room_rect, 0.0, room_fill);
        if colorblind_mode {
            draw_room_crosshatch(painter, geometry, room, Stroke::new(1.5, Color32::BLACK));
        }
    }

    for &(col, row) in &room_cells_set {
        let rect = cell_rect(geometry, col, row);
        let right = (col + 1, row);
        let bottom = (col, row + 1);
        let left = col.checked_sub(1).map(|x| (x, row));
        let top = row.checked_sub(1).map(|y| (col, y));

        let left_edge = col == 0
            || (!room_cells_set.contains(&(col - 1, row))
                || left
                    .map(|n| !room_edges_set.contains(&normalized_edge((col, row), n)))
                    .unwrap_or(true))
                && left
                    .map(|n| !door_edges_set.contains(&normalized_edge((col, row), n)))
                    .unwrap_or(true);
        if left_edge {
            draw_wall_segment(
                painter,
                egui::pos2(rect.left(), rect.top()),
                egui::pos2(rect.left(), rect.bottom()),
                wall_width_px,
                wall_color,
            );
        }
        if (!room_cells_set.contains(&right)
            || !room_edges_set.contains(&normalized_edge((col, row), right)))
            && !door_edges_set.contains(&normalized_edge((col, row), right))
        {
            draw_wall_segment(
                painter,
                egui::pos2(rect.right(), rect.top()),
                egui::pos2(rect.right(), rect.bottom()),
                wall_width_px,
                wall_color,
            );
        }
        let top_edge = row == 0
            || (!room_cells_set.contains(&(col, row - 1))
                || top
                    .map(|n| !room_edges_set.contains(&normalized_edge((col, row), n)))
                    .unwrap_or(true))
                && top
                    .map(|n| !door_edges_set.contains(&normalized_edge((col, row), n)))
                    .unwrap_or(true);
        if top_edge {
            draw_wall_segment(
                painter,
                egui::pos2(rect.left(), rect.top()),
                egui::pos2(rect.right(), rect.top()),
                wall_width_px,
                wall_color,
            );
        }
        if (!room_cells_set.contains(&bottom)
            || !room_edges_set.contains(&normalized_edge((col, row), bottom)))
            && !door_edges_set.contains(&normalized_edge((col, row), bottom))
        {
            draw_wall_segment(
                painter,
                egui::pos2(rect.left(), rect.bottom()),
                egui::pos2(rect.right(), rect.bottom()),
                wall_width_px,
                wall_color,
            );
        }
    }

    for door in &layout.doors {
        let color = if colorblind_mode {
            Color32::BLACK
        } else if door.secret {
            Color32::from_rgb(170, 80, 170)
        } else if door.archway {
            Color32::from_rgb(230, 140, 60)
        } else if door.locked {
            Color32::from_rgb(220, 70, 70)
        } else {
            Color32::from_rgb(80, 200, 120)
        };
        let style = if colorblind_mode {
            if door.secret {
                DoorLineStyle::DashDotDot
            } else if door.archway {
                DoorLineStyle::Dotted
            } else if door.locked {
                DoorLineStyle::ShortDash
            } else {
                DoorLineStyle::LongDash
            }
        } else {
            DoorLineStyle::Solid
        };
        draw_door_line(
            painter,
            geometry,
            door.from,
            door.to,
            door_render_width(door),
            Stroke::new(door_width_px, color),
            style,
        );
    }

    for window in &layout.windows {
        let style = if colorblind_mode {
            DoorLineStyle::DashDot
        } else {
            DoorLineStyle::Solid
        };
        draw_window_line(
            painter,
            geometry,
            window,
            Stroke::new(door_width_px, Color32::from_rgb(70, 130, 220)),
            style,
        );
    }

    for (idx, label) in layout.text_labels.iter().enumerate() {
        if hover_text_idx == Some(idx) {
            painter.rect_stroke(
                text_label_rect(geometry, label).expand(2.0),
                0.0,
                Stroke::new(2.0, Color32::from_rgb(255, 220, 120)),
                egui::StrokeKind::Middle,
            );
        }
        let pos = cell_center(geometry, label.cell.0, label.cell.1);
        let color = if colorblind_mode {
            Color32::BLACK
        } else {
            Color32::WHITE
        };
        painter.text(
            pos,
            egui::Align2::CENTER_CENTER,
            &label.text,
            egui::FontId::proportional(label.font_size as f32),
            color,
        );
    }

    draw_area_marker_group(
        painter,
        geometry,
        &layout.start_markers,
        "S",
        Color32::from_rgb(60, 220, 200),
        colorblind_mode,
        hover_marker,
        MarkerKind::Start,
    );
    draw_area_marker_group(
        painter,
        geometry,
        &layout.end_markers,
        "E",
        Color32::from_rgb(240, 90, 90),
        colorblind_mode,
        hover_marker,
        MarkerKind::End,
    );
    draw_area_marker_group(
        painter,
        geometry,
        &layout.trap_markers,
        "T",
        Color32::from_rgb(150, 80, 230), // Purple
        colorblind_mode,
        hover_marker,
        MarkerKind::Trap,
    );
    draw_area_marker_group(
        painter,
        geometry,
        &layout.monster_markers,
        "M",
        Color32::from_rgb(200, 50, 50), // Red
        colorblind_mode,
        hover_marker,
        MarkerKind::Monster,
    );
}

// Compute the rectangle for a given grid cell.
fn cell_rect(geometry: &GridGeometry, col: usize, row: usize) -> egui::Rect {
    let left = geometry.rect.left() + col as f32 * geometry.cell_size;
    let top = geometry.rect.top() + row as f32 * geometry.cell_size;
    egui::Rect::from_min_size(
        egui::pos2(left, top),
        egui::vec2(geometry.cell_size, geometry.cell_size),
    )
}

fn marker_rect(marker: &layout::AreaMarker, geometry: &GridGeometry) -> egui::Rect {
    let left = geometry.rect.left() + marker.cell.0 as f32 * geometry.cell_size;
    let top = geometry.rect.top() + marker.cell.1 as f32 * geometry.cell_size;
    egui::Rect::from_min_size(
        egui::pos2(left, top),
        egui::vec2(
            marker.size as f32 * geometry.cell_size,
            marker.size as f32 * geometry.cell_size,
        ),
    )
}

fn marker_contains_cell(marker: Option<&layout::AreaMarker>, cell: (usize, usize)) -> bool {
    let Some(marker) = marker else {
        return false;
    };
    cell.0 >= marker.cell.0
        && cell.0 < marker.cell.0 + marker.size
        && cell.1 >= marker.cell.1
        && cell.1 < marker.cell.1 + marker.size
}

fn centered_marker_origin(
    cell: (usize, usize),
    size: usize,
    cols: usize,
    rows: usize,
) -> (usize, usize) {
    let size = size.clamp(1, 10).min(cols.max(1)).min(rows.max(1));
    let half = size / 2;
    let max_x = cols.saturating_sub(size);
    let max_y = rows.saturating_sub(size);
    (
        cell.0.saturating_sub(half).min(max_x),
        cell.1.saturating_sub(half).min(max_y),
    )
}

fn draw_area_marker(
    painter: &egui::Painter,
    geometry: &GridGeometry,
    marker: &layout::AreaMarker,
    label: &str,
    base_color: Color32,
    colorblind_mode: bool,
    hovered: bool,
) {
    let rect = marker_rect(marker, geometry);
    let fill = base_color.gamma_multiply(if colorblind_mode { 0.22 } else { 0.35 });
    let stroke_color = if hovered {
        Color32::from_rgb(255, 220, 120)
    } else if colorblind_mode {
        Color32::BLACK
    } else {
        base_color
    };
    let text_color = if colorblind_mode {
        Color32::BLACK
    } else {
        Color32::WHITE
    };
    let font_size = (geometry.cell_size * marker.size as f32 * 0.5).clamp(14.0, 72.0);

    painter.rect_filled(rect, 4.0, fill);
    painter.rect_stroke(
        rect,
        4.0,
        Stroke::new(if hovered { 3.0 } else { 2.0 }, stroke_color),
        egui::StrokeKind::Middle,
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(font_size),
        text_color,
    );
}

fn draw_area_marker_group(
    painter: &egui::Painter,
    geometry: &GridGeometry,
    markers: &[layout::AreaMarker],
    label: &str,
    base_color: Color32,
    colorblind_mode: bool,
    hover_marker: Option<HoverMarker>,
    kind: MarkerKind,
) {
    for (idx, marker) in markers.iter().enumerate() {
        let hovered = match (kind, hover_marker) {
            (MarkerKind::Start, Some(HoverMarker::Start(hover_idx))) => hover_idx == idx,
            (MarkerKind::End, Some(HoverMarker::End(hover_idx))) => hover_idx == idx,
            (MarkerKind::Trap, Some(HoverMarker::Trap(hover_idx))) => hover_idx == idx,
            (MarkerKind::Monster, Some(HoverMarker::Monster(hover_idx))) => hover_idx == idx,
            _ => false,
        };
        draw_area_marker(
            painter,
            geometry,
            marker,
            label,
            base_color,
            colorblind_mode,
            hovered,
        );
    }
}

// Draw a rectangular wall segment between two points.
fn draw_wall_segment(
    painter: &egui::Painter,
    from: egui::Pos2,
    to: egui::Pos2,
    width: f32,
    color: Color32,
) {
    let half = width * 0.5;
    if (from.x - to.x).abs() <= f32::EPSILON {
        let x = from.x.min(to.x);
        let y0 = from.y.min(to.y);
        let y1 = from.y.max(to.y);
        let rect = egui::Rect::from_min_max(
            egui::pos2(x - half, y0 - half),
            egui::pos2(x + half, y1 + half),
        );
        painter.rect_filled(rect, 0.0, color);
    } else {
        let y = from.y.min(to.y);
        let x0 = from.x.min(to.x);
        let x1 = from.x.max(to.x);
        let rect = egui::Rect::from_min_max(
            egui::pos2(x0 - half, y - half),
            egui::pos2(x1 + half, y + half),
        );
        painter.rect_filled(rect, 0.0, color);
    }
}

// Normalize an edge tuple to a stable ordering.
fn normalized_edge(a: (usize, usize), b: (usize, usize)) -> ((usize, usize), (usize, usize)) {
    if a <= b { (a, b) } else { (b, a) }
}

// Build a set of corridor-adjacent edges from corridor cells.
fn corridor_edges_from_cells(
    corridor_cells: &HashSet<(usize, usize)>,
) -> HashSet<((usize, usize), (usize, usize))> {
    let mut edges = HashSet::new();
    for &(col, row) in corridor_cells {
        let right = (col + 1, row);
        let bottom = (col, row + 1);
        if corridor_cells.contains(&right) {
            edges.insert(normalized_edge((col, row), right));
        }
        if corridor_cells.contains(&bottom) {
            edges.insert(normalized_edge((col, row), bottom));
        }
    }
    edges
}

// Build a set of internal room edges from room rectangles.
fn room_edges_from_rooms(rooms: &[layout::Room]) -> HashSet<((usize, usize), (usize, usize))> {
    let mut edges = HashSet::new();
    for room in rooms {
        let x_end = room.x + room.width;
        let y_end = room.y + room.height;
        for x in room.x..x_end {
            for y in room.y..y_end {
                if x + 1 < x_end {
                    edges.insert(normalized_edge((x, y), (x + 1, y)));
                }
                if y + 1 < y_end {
                    edges.insert(normalized_edge((x, y), (x, y + 1)));
                }
            }
        }
    }
    edges
}

// Find the index of the room containing a specific cell.
fn room_index_at_cell(rooms: &[layout::Room], cell: (usize, usize)) -> Option<usize> {
    rooms.iter().position(|room| {
        cell.0 >= room.x
            && cell.0 < room.x + room.width
            && cell.1 >= room.y
            && cell.1 < room.y + room.height
    })
}

// Check whether two rooms overlap by area.
fn rooms_overlap(a: &layout::Room, b: &layout::Room) -> bool {
    let a_right = a.x + a.width;
    let a_bottom = a.y + a.height;
    let b_right = b.x + b.width;
    let b_bottom = b.y + b.height;
    a.x < b_right && a_right > b.x && a.y < b_bottom && a_bottom > b.y
}

fn manual_door_edge_allowed(
    a_room: Option<usize>,
    b_room: Option<usize>,
    a_in_corridor: bool,
    b_in_corridor: bool,
) -> bool {
    if matches!((a_room, b_room), (Some(a), Some(b)) if a == b) {
        return false;
    }

    let a_is_feature = a_room.is_some() || a_in_corridor;
    let b_is_feature = b_room.is_some() || b_in_corridor;
    a_is_feature || b_is_feature
}

fn resize_hit_cells(
    room: &layout::Room,
    room_idx: usize,
    rooms: &[layout::Room],
    cols: usize,
    rows: usize,
) -> HashSet<(usize, usize)> {
    let mut cells = HashSet::new();
    let corners = [
        (room.x as isize, room.y as isize, -1isize, -1isize),
        (
            (room.x + room.width - 1) as isize,
            room.y as isize,
            1isize,
            -1isize,
        ),
        (
            room.x as isize,
            (room.y + room.height - 1) as isize,
            -1isize,
            1isize,
        ),
        (
            (room.x + room.width - 1) as isize,
            (room.y + room.height - 1) as isize,
            1isize,
            1isize,
        ),
    ];

    for (cx, cy, ox, oy) in corners {
        let candidates = [
            (cx, cy),
            (cx - ox, cy),
            (cx, cy - oy),
            (cx - ox, cy - oy),
            (cx + ox, cy),
            (cx, cy + oy),
            (cx + ox, cy + oy),
        ];

        for (x, y) in candidates {
            if x < 0 || y < 0 || x >= cols as isize || y >= rows as isize {
                continue;
            }

            let cell = (x as usize, y as usize);
            let in_other_room = rooms.iter().enumerate().any(|(idx, other_room)| {
                idx != room_idx
                    && cell.0 >= other_room.x
                    && cell.0 < other_room.x + other_room.width
                    && cell.1 >= other_room.y
                    && cell.1 < other_room.y + other_room.height
            });
            if !in_other_room {
                cells.insert(cell);
            }
        }
    }

    cells
}

// Draw a single resize handle glyph at a corner.
fn draw_resize_handle(
    painter: &egui::Painter,
    corner: egui::Pos2,
    which: ResizeCorner,
    color: Color32,
) {
    let size = 10.0;
    let half = size * 0.5;
    let rect = egui::Rect::from_center_size(corner, egui::vec2(size, size));
    painter.rect_filled(rect, 2.0, Color32::from_gray(30));
    painter.rect_stroke(rect, 2.0, Stroke::new(1.0, color), egui::StrokeKind::Middle);

    let center = corner;
    let stroke = Stroke::new(1.0, color);
    match which {
        ResizeCorner::TopLeft => {
            painter.line_segment(
                [center, egui::pos2(center.x - half + 2.0, center.y)],
                stroke,
            );
            painter.line_segment(
                [center, egui::pos2(center.x, center.y - half + 2.0)],
                stroke,
            );
        }
        ResizeCorner::TopRight => {
            painter.line_segment(
                [center, egui::pos2(center.x + half - 2.0, center.y)],
                stroke,
            );
            painter.line_segment(
                [center, egui::pos2(center.x, center.y - half + 2.0)],
                stroke,
            );
        }
        ResizeCorner::BottomLeft => {
            painter.line_segment(
                [center, egui::pos2(center.x - half + 2.0, center.y)],
                stroke,
            );
            painter.line_segment(
                [center, egui::pos2(center.x, center.y + half - 2.0)],
                stroke,
            );
        }
        ResizeCorner::BottomRight => {
            painter.line_segment(
                [center, egui::pos2(center.x + half - 2.0, center.y)],
                stroke,
            );
            painter.line_segment(
                [center, egui::pos2(center.x, center.y + half - 2.0)],
                stroke,
            );
        }
    }
}

// Expand a door into all grid edges it spans based on width.
fn door_edges_for(
    door: &layout::Door,
    cols: usize,
    rows: usize,
) -> Vec<((usize, usize), (usize, usize))> {
    let mut edges = Vec::new();
    if door.from.0.abs_diff(door.to.0) + door.from.1.abs_diff(door.to.1) != 1 {
        return edges;
    }

    let width = door_render_width(door).max(1) as isize;
    let min_offset = -((width - 1) / 2);
    let max_offset = width / 2;

    if door.from.0 != door.to.0 {
        let y = door.from.1 as isize;
        for dy in min_offset..=max_offset {
            let ny = y + dy;
            if ny < 0 || ny >= rows as isize {
                continue;
            }
            let a = (door.from.0, ny as usize);
            let b = (door.to.0, ny as usize);
            edges.push(normalized_edge(a, b));
        }
    } else {
        let x = door.from.0 as isize;
        for dx in min_offset..=max_offset {
            let nx = x + dx;
            if nx < 0 || nx >= cols as isize {
                continue;
            }
            let a = (nx as usize, door.from.1);
            let b = (nx as usize, door.to.1);
            edges.push(normalized_edge(a, b));
        }
    }

    edges
}

// Determine the rendered door width in cells.
fn door_render_width(door: &layout::Door) -> usize {
    if door.span_width {
        door.width.max(1)
    } else {
        1
    }
}

fn window_render_width(window: &layout::Window) -> usize {
    if window.span_width {
        window.width.max(1)
    } else {
        1
    }
}

// Convert a pointer position to grid coordinates if inside the grid.
fn pointer_to_grid(pointer_pos: egui::Pos2, geometry: &GridGeometry) -> Option<(f32, f32)> {
    if !geometry.rect.contains(pointer_pos) {
        return None;
    }

    let x = (pointer_pos.x - geometry.rect.left()) / geometry.cell_size;
    let y = (pointer_pos.y - geometry.rect.top()) / geometry.cell_size;
    Some((x, y))
}

// Recompute a path that must pass through a target cell.
fn reroute_path_through_cell(
    original_path: &[(usize, usize)],
    original_cell: (usize, usize),
    target_cell: (usize, usize),
    cols: usize,
    rows: usize,
    blocked_room_cells_set: &HashSet<(usize, usize)>,
) -> Option<Vec<(usize, usize)>> {
    let (&start, &end) = (original_path.first()?, original_path.last()?);
    if start == end {
        return None;
    }

    let mut blocked = blocked_room_cells_set.clone();
    blocked.insert(original_cell);
    blocked.remove(&start);
    blocked.remove(&end);
    blocked.remove(&target_cell);

    let first_leg = shortest_path_cells(start, target_cell, cols, rows, &blocked)?;

    let mut blocked_second = blocked.clone();
    for &cell in first_leg
        .iter()
        .skip(1)
        .take(first_leg.len().saturating_sub(2))
    {
        if cell != target_cell && cell != end {
            blocked_second.insert(cell);
        }
    }

    let second_leg = shortest_path_cells(target_cell, end, cols, rows, &blocked_second)
        .or_else(|| shortest_path_cells(target_cell, end, cols, rows, &blocked))?;

    let mut full_path = first_leg;
    for cell in second_leg.into_iter().skip(1) {
        if full_path.last().copied() != Some(cell) {
            full_path.push(cell);
        }
    }

    simplify_path_loops(&mut full_path);

    if full_path.len() < 2 || !full_path.contains(&target_cell) {
        return None;
    }

    if original_cell != start && original_cell != end && full_path.contains(&original_cell) {
        return None;
    }

    Some(full_path)
}

// Remove loops in a path by truncating back to repeated cells.
fn simplify_path_loops(path: &mut Vec<(usize, usize)>) {
    let mut out = Vec::new();
    for &cell in path.iter() {
        if let Some(pos) = out.iter().position(|&c| c == cell) {
            out.truncate(pos + 1);
        } else {
            out.push(cell);
        }
    }
    *path = out;
}

// Convert UI door settings into layout door settings.
fn door_settings_from_ui(settings: &UiSettings) -> DoorSettings {
    DoorSettings {
        frequency_percent: settings.door_frequency_percent,
        room_hallway_percent: settings.room_hallway_door_percent,
        locked_percent: settings.locked_door_percent,
        secret_percent: settings.secret_door_percent,
        allow_middle_corridor_doors: settings.allow_middle_corridor_doors,
    }
}

fn window_settings_from_ui(settings: &UiSettings) -> WindowSettings {
    WindowSettings {
        enabled: settings.windows_enabled,
        min_width: settings.min_window_width,
        max_width: settings.max_window_width,
        frequency_percent: settings.window_frequency_percent,
        room_hallway_percent: settings.room_hallway_window_percent,
        allow_internal_windows: settings.allow_internal_windows,
    }
}

// Draw a door line segment with a selected style.
fn draw_door_line(
    painter: &egui::Painter,
    geometry: &GridGeometry,
    a: (usize, usize),
    b: (usize, usize),
    width_cells: usize,
    stroke: Stroke,
    style: DoorLineStyle,
) {
    if a.0.abs_diff(b.0) + a.1.abs_diff(b.1) != 1 {
        return;
    }

    let width_cells = width_cells.max(1);
    let min_offset = -((width_cells as isize - 1) / 2);
    let max_offset = width_cells as isize / 2;

    if a.0 != b.0 {
        let x = geometry.rect.left() + (a.0.max(b.0) as f32) * geometry.cell_size;
        let row = a.1 as isize;
        let y0_cell = (row + min_offset).clamp(0, geometry.rows.saturating_sub(1) as isize);
        let y1_cell = (row + max_offset).clamp(0, geometry.rows.saturating_sub(1) as isize);
        let y0 = geometry.rect.top() + y0_cell as f32 * geometry.cell_size;
        let y1 = geometry.rect.top() + (y1_cell as f32 + 1.0) * geometry.cell_size;
        draw_styled_line(painter, egui::pos2(x, y0), egui::pos2(x, y1), stroke, style);
    } else {
        let y = geometry.rect.top() + (a.1.max(b.1) as f32) * geometry.cell_size;
        let col = a.0 as isize;
        let x0_cell = (col + min_offset).clamp(0, geometry.cols.saturating_sub(1) as isize);
        let x1_cell = (col + max_offset).clamp(0, geometry.cols.saturating_sub(1) as isize);
        let x0 = geometry.rect.left() + x0_cell as f32 * geometry.cell_size;
        let x1 = geometry.rect.left() + (x1_cell as f32 + 1.0) * geometry.cell_size;
        draw_styled_line(painter, egui::pos2(x0, y), egui::pos2(x1, y), stroke, style);
    }
}

fn draw_window_line(
    painter: &egui::Painter,
    geometry: &GridGeometry,
    window: &layout::Window,
    stroke: Stroke,
    style: DoorLineStyle,
) {
    let width_cells = window_render_width(window).max(1) as isize;
    let min_offset = -((width_cells - 1) / 2);
    let max_offset = width_cells / 2;

    match window.side {
        layout::WindowSide::Left => {
            let x = geometry.rect.left() + window.cell.0 as f32 * geometry.cell_size;
            let row = window.cell.1 as isize;
            let y0_cell = (row + min_offset).clamp(0, geometry.rows.saturating_sub(1) as isize);
            let y1_cell = (row + max_offset).clamp(0, geometry.rows.saturating_sub(1) as isize);
            let y0 = geometry.rect.top() + y0_cell as f32 * geometry.cell_size;
            let y1 = geometry.rect.top() + (y1_cell as f32 + 1.0) * geometry.cell_size;
            draw_styled_line(painter, egui::pos2(x, y0), egui::pos2(x, y1), stroke, style);
        }
        layout::WindowSide::Right => {
            let x = geometry.rect.left() + (window.cell.0 as f32 + 1.0) * geometry.cell_size;
            let row = window.cell.1 as isize;
            let y0_cell = (row + min_offset).clamp(0, geometry.rows.saturating_sub(1) as isize);
            let y1_cell = (row + max_offset).clamp(0, geometry.rows.saturating_sub(1) as isize);
            let y0 = geometry.rect.top() + y0_cell as f32 * geometry.cell_size;
            let y1 = geometry.rect.top() + (y1_cell as f32 + 1.0) * geometry.cell_size;
            draw_styled_line(painter, egui::pos2(x, y0), egui::pos2(x, y1), stroke, style);
        }
        layout::WindowSide::Top => {
            let y = geometry.rect.top() + window.cell.1 as f32 * geometry.cell_size;
            let col = window.cell.0 as isize;
            let x0_cell = (col + min_offset).clamp(0, geometry.cols.saturating_sub(1) as isize);
            let x1_cell = (col + max_offset).clamp(0, geometry.cols.saturating_sub(1) as isize);
            let x0 = geometry.rect.left() + x0_cell as f32 * geometry.cell_size;
            let x1 = geometry.rect.left() + (x1_cell as f32 + 1.0) * geometry.cell_size;
            draw_styled_line(painter, egui::pos2(x0, y), egui::pos2(x1, y), stroke, style);
        }
        layout::WindowSide::Bottom => {
            let y = geometry.rect.top() + (window.cell.1 as f32 + 1.0) * geometry.cell_size;
            let col = window.cell.0 as isize;
            let x0_cell = (col + min_offset).clamp(0, geometry.cols.saturating_sub(1) as isize);
            let x1_cell = (col + max_offset).clamp(0, geometry.cols.saturating_sub(1) as isize);
            let x0 = geometry.rect.left() + x0_cell as f32 * geometry.cell_size;
            let x1 = geometry.rect.left() + (x1_cell as f32 + 1.0) * geometry.cell_size;
            draw_styled_line(painter, egui::pos2(x0, y), egui::pos2(x1, y), stroke, style);
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum DoorLineStyle {
    Solid,
    LongDash,
    ShortDash,
    Dotted,
    DashDot,
    DashDotDot,
}

// Draw a line with solid, dashed, or dotted styling.
fn draw_styled_line(
    painter: &egui::Painter,
    from: egui::Pos2,
    to: egui::Pos2,
    stroke: Stroke,
    style: DoorLineStyle,
) {
    match style {
        DoorLineStyle::Solid => {
            painter.line_segment([from, to], stroke);
        }
        DoorLineStyle::LongDash => draw_dashed_line(painter, from, to, stroke, 14.0, 7.0),
        DoorLineStyle::ShortDash => draw_dashed_line(painter, from, to, stroke, 6.0, 4.0),
        DoorLineStyle::Dotted => draw_dotted_line(painter, from, to, stroke),
        DoorLineStyle::DashDot => draw_dash_dot_line(painter, from, to, stroke),
        DoorLineStyle::DashDotDot => draw_dash_dot_dot_line(painter, from, to, stroke),
    }
}

// Draw a dashed line between two points.
fn draw_dashed_line(
    painter: &egui::Painter,
    from: egui::Pos2,
    to: egui::Pos2,
    stroke: Stroke,
    dash_len: f32,
    gap_len: f32,
) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 0.0 {
        return;
    }
    let ux = dx / len;
    let uy = dy / len;

    let mut dist = 0.0_f32;
    while dist < len {
        let seg_start = dist;
        let seg_end = (dist + dash_len).min(len);
        let p0 = egui::pos2(from.x + ux * seg_start, from.y + uy * seg_start);
        let p1 = egui::pos2(from.x + ux * seg_end, from.y + uy * seg_end);
        painter.line_segment([p0, p1], stroke);
        dist += dash_len + gap_len;
    }
}

// Draw a dotted line between two points.
fn draw_dotted_line(painter: &egui::Painter, from: egui::Pos2, to: egui::Pos2, stroke: Stroke) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 0.0 {
        return;
    }
    let ux = dx / len;
    let uy = dy / len;
    let step = 5.0_f32;
    let radius = (stroke.width * 0.35).max(1.0);

    let mut dist = 0.0_f32;
    while dist <= len {
        let p = egui::pos2(from.x + ux * dist, from.y + uy * dist);
        painter.circle_filled(p, radius, stroke.color);
        dist += step;
    }
}

fn draw_dash_dot_line(painter: &egui::Painter, from: egui::Pos2, to: egui::Pos2, stroke: Stroke) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 0.0 {
        return;
    }

    let ux = dx / len;
    let uy = dy / len;
    let mut dist = 0.0_f32;
    while dist < len {
        let dash_end = (dist + 7.0).min(len);
        let p0 = egui::pos2(from.x + ux * dist, from.y + uy * dist);
        let p1 = egui::pos2(from.x + ux * dash_end, from.y + uy * dash_end);
        painter.line_segment([p0, p1], stroke);
        dist = dash_end + 3.0;
        if dist >= len {
            break;
        }
        let dot = egui::pos2(from.x + ux * dist, from.y + uy * dist);
        painter.circle_filled(dot, (stroke.width * 0.35).max(1.0), stroke.color);
        dist += 4.0;
    }
}

fn draw_dash_dot_dot_line(
    painter: &egui::Painter,
    from: egui::Pos2,
    to: egui::Pos2,
    stroke: Stroke,
) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 0.0 {
        return;
    }

    let ux = dx / len;
    let uy = dy / len;
    let radius = (stroke.width * 0.35).max(1.0);
    let mut dist = 0.0_f32;
    while dist < len {
        let dash_end = (dist + 7.0).min(len);
        let p0 = egui::pos2(from.x + ux * dist, from.y + uy * dist);
        let p1 = egui::pos2(from.x + ux * dash_end, from.y + uy * dash_end);
        painter.line_segment([p0, p1], stroke);
        dist = dash_end + 3.0;
        if dist >= len {
            break;
        }
        let dot1 = egui::pos2(from.x + ux * dist, from.y + uy * dist);
        painter.circle_filled(dot1, radius, stroke.color);
        dist += 3.0;
        if dist >= len {
            break;
        }
        let dot2 = egui::pos2(from.x + ux * dist, from.y + uy * dist);
        painter.circle_filled(dot2, radius, stroke.color);
        dist += 4.0;
    }
}

// Compute the center position of a grid cell.
fn cell_center(geometry: &GridGeometry, col: usize, row: usize) -> egui::Pos2 {
    egui::pos2(
        geometry.rect.left() + (col as f32 + 0.5) * geometry.cell_size,
        geometry.rect.top() + (row as f32 + 0.5) * geometry.cell_size,
    )
}

fn text_label_rect(geometry: &GridGeometry, label: &layout::TextLabel) -> egui::Rect {
    let center = cell_center(geometry, label.cell.0, label.cell.1);
    let font_size = label.font_size as f32;
    let width =
        (label.text.chars().count().max(1) as f32 * font_size * 0.6).max(geometry.cell_size * 0.5);
    let height = font_size.max(geometry.cell_size * 0.35);
    egui::Rect::from_center_size(center, egui::vec2(width, height))
}

// Draw a crosshatch pattern inside a room for colorblind mode.
fn draw_room_crosshatch(
    painter: &egui::Painter,
    geometry: &GridGeometry,
    room: &layout::Room,
    stroke: Stroke,
) {
    for x in room.x..(room.x + room.width) {
        for y in room.y..(room.y + room.height) {
            let rect = cell_rect(geometry, x, y).shrink(2.0);
            painter.line_segment([rect.left_top(), rect.right_bottom()], stroke);
            painter.line_segment([rect.right_top(), rect.left_bottom()], stroke);
        }
    }
}

// Draw a colored legend entry in the sidebar.
fn draw_legend_entry(ui: &mut egui::Ui, color: Color32, label: &str) {
    ui.horizontal(|ui| {
        let (rect, _resp) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 0.0, color);
        ui.painter().rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0, Color32::WHITE),
            egui::StrokeKind::Middle,
        );
        ui.add_space(6.0);
        ui.label(label);
    });
    ui.add_space(4.0);
}

#[derive(Debug, Clone, Copy)]
enum LegendStyle {
    Crosshatch,
    Dots,
    SolidLine,
    LongDash,
    ShortDash,
    DottedLine,
    DashDotLine,
    DashDotDotLine,
}

// Draw a patterned legend entry for colorblind mode.
fn draw_legend_entry_colorblind(ui: &mut egui::Ui, label: &str, style: LegendStyle) {
    ui.horizontal(|ui| {
        let (rect, _resp) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 0.0, Color32::from_gray(220));
        ui.painter().rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0, Color32::BLACK),
            egui::StrokeKind::Middle,
        );

        match style {
            LegendStyle::Crosshatch => {
                ui.painter().line_segment(
                    [rect.left_top(), rect.right_bottom()],
                    Stroke::new(1.2, Color32::BLACK),
                );
                ui.painter().line_segment(
                    [rect.right_top(), rect.left_bottom()],
                    Stroke::new(1.2, Color32::BLACK),
                );
            }
            LegendStyle::Dots => {
                ui.painter()
                    .circle_filled(rect.center(), 2.0, Color32::BLACK);
                ui.painter().circle_filled(
                    egui::pos2(rect.center().x - 4.0, rect.center().y),
                    1.4,
                    Color32::BLACK,
                );
                ui.painter().circle_filled(
                    egui::pos2(rect.center().x + 4.0, rect.center().y),
                    1.4,
                    Color32::BLACK,
                );
            }
            LegendStyle::SolidLine => {
                ui.painter().line_segment(
                    [
                        egui::pos2(rect.left() + 1.0, rect.center().y),
                        egui::pos2(rect.right() - 1.0, rect.center().y),
                    ],
                    Stroke::new(2.0, Color32::BLACK),
                );
            }
            LegendStyle::LongDash => draw_dashed_line(
                ui.painter(),
                egui::pos2(rect.left() + 1.0, rect.center().y),
                egui::pos2(rect.right() - 1.0, rect.center().y),
                Stroke::new(2.0, Color32::BLACK),
                6.0,
                3.0,
            ),
            LegendStyle::ShortDash => draw_dashed_line(
                ui.painter(),
                egui::pos2(rect.left() + 1.0, rect.center().y),
                egui::pos2(rect.right() - 1.0, rect.center().y),
                Stroke::new(2.0, Color32::BLACK),
                3.0,
                2.0,
            ),
            LegendStyle::DottedLine => draw_dotted_line(
                ui.painter(),
                egui::pos2(rect.left() + 1.0, rect.center().y),
                egui::pos2(rect.right() - 1.0, rect.center().y),
                Stroke::new(2.0, Color32::BLACK),
            ),
            LegendStyle::DashDotLine => draw_dash_dot_line(
                ui.painter(),
                egui::pos2(rect.left() + 1.0, rect.center().y),
                egui::pos2(rect.right() - 1.0, rect.center().y),
                Stroke::new(2.0, Color32::BLACK),
            ),
            LegendStyle::DashDotDotLine => draw_dash_dot_dot_line(
                ui.painter(),
                egui::pos2(rect.left() + 1.0, rect.center().y),
                egui::pos2(rect.right() - 1.0, rect.center().y),
                Stroke::new(2.0, Color32::BLACK),
            ),
        }

        ui.add_space(6.0);
        ui.label(label);
    });
    ui.add_space(4.0);
}
