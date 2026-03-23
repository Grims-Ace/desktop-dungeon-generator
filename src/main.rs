mod exporter;
mod layout;
mod seed;
mod settings;
mod ui;

use std::collections::HashSet;
use std::sync::mpsc;
use std::thread;

use eframe::egui;
use egui::{Color32, Stroke};
use layout::{
    DoorSettings, DungeonLayout, blocked_room_cells, corridor_cells, shortest_path_cells,
};
use ui::{AddTool, UiSettings, draw_side_panel};

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
    drag_state: Option<DragState>,
    export_rx: Option<mpsc::Receiver<Result<std::path::PathBuf, String>>>,
    pending_delete: bool,
    add_corridor_drag: Option<AddCorridorDrag>,
    resize_state: Option<RoomResizeState>,
    hover_room_idx: Option<usize>,
    hover_corridor_idx: Option<usize>,
    hover_door_idx: Option<usize>,
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
        );
        Self {
            settings,
            layout,
            drag_state: None,
            export_rx: None,
            pending_delete: false,
            add_corridor_drag: None,
            resize_state: None,
            hover_room_idx: None,
            hover_corridor_idx: None,
            hover_door_idx: None,
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

        if self.settings.min_room_size > self.settings.max_room_size {
            self.settings.max_room_size = self.settings.min_room_size;
        }
        if self.settings.min_corridor_width == 0 {
            self.settings.min_corridor_width = 1;
        }
        if self.settings.min_corridor_width > self.settings.max_corridor_width {
            self.settings.max_corridor_width = self.settings.min_corridor_width;
        }

        if panel_result.reset_clicked || panel_result.settings_changed {
            self.regenerate_layout();
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
                    draw_legend_entry_colorblind(ui, "Archways", LegendStyle::DottedLine);
                } else {
                    draw_legend_entry(ui, Color32::from_rgb(70, 120, 160), "Rooms");
                    draw_legend_entry(ui, Color32::from_rgb(210, 190, 120), "Corridors");
                    draw_legend_entry(ui, Color32::BLACK, "Walls");
                    draw_legend_entry(ui, Color32::from_rgb(80, 200, 120), "Doors");
                    draw_legend_entry(ui, Color32::from_rgb(220, 70, 70), "Locked Doors");
                    draw_legend_entry(ui, Color32::from_rgb(70, 130, 220), "Archways");
                }
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);
                ui.heading("Keybinds");
                ui.add_space(6.0);
                ui.label("Delete / Backspace: Remove hovered room or corridor");
                ui.label("Right click: Cancel add tool");
                ui.label("Right click + drag: Resize hovered room");
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
            );
            self.draw_add_overlay(&painter, &geometry);
            self.draw_resize_overlay(&painter, &geometry);
            self.draw_corridor_hover_overlay(&painter, &geometry);
            self.draw_door_hover_overlay(&painter, &geometry);
        });
    }
}

impl DungeonApp {
    // Regenerate the dungeon layout from current settings and reset drag state.
    fn regenerate_layout(&mut self) {
        self.drag_state = None;
        self.layout = layout::generate_layout(
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
        );
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
            && let Some((room_idx, offset_x, offset_y)) =
                self.room_at_pointer(pointer_pos, geometry)
        {
            self.drag_state = Some(DragState::Room(RoomDragState {
                room_idx,
                offset_x,
                offset_y,
            }));
        } else if response.drag_started()
            && let Some(pointer_pos) = response.interact_pointer_pos()
            && let Some(corridor_drag) = self.corridor_drag_at_pointer(pointer_pos, geometry)
        {
            self.drag_state = Some(DragState::Corridor(corridor_drag));
        }

        if response.dragged()
            && let Some(pointer_pos) = response.interact_pointer_pos()
            && let Some(drag_state) = self.drag_state.clone()
        {
            match drag_state {
                DragState::Room(drag) => self.drag_room_to_pointer(drag, pointer_pos, geometry),
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
                    self.settings.add_tool = AddTool::None;
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
                    self.settings.add_tool = AddTool::None;
                }
            }
            AddTool::Door | AddTool::LockedDoor => {
                self.add_corridor_drag = None;
                if response.clicked()
                    && let Some(pointer_pos) = response.interact_pointer_pos()
                {
                    let locked = self.settings.add_tool == AddTool::LockedDoor;
                    self.add_door_at_pointer(pointer_pos, geometry, locked);
                    self.settings.add_tool = AddTool::None;
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

    // Update which room/corridor/door is currently hovered.
    fn update_hover_targets(&mut self, ctx: &egui::Context, geometry: &GridGeometry) {
        let Some(pointer_pos) = ctx.pointer_hover_pos() else {
            self.hover_room_idx = None;
            self.hover_corridor_idx = None;
            self.hover_door_idx = None;
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
                return;
            }
        }

        if let Some((room_idx, _, _)) = self.room_at_pointer(pointer_pos, geometry) {
            self.hover_room_idx = Some(room_idx);
            self.hover_corridor_idx = None;
            self.hover_door_idx = None;
        } else {
            self.hover_room_idx = None;
            if let Some(corridor_drag) = self.corridor_drag_at_pointer(pointer_pos, geometry) {
                self.hover_corridor_idx = Some(corridor_drag.corridor_index);
                self.hover_door_idx = None;
            } else {
                self.hover_corridor_idx = None;
                self.hover_door_idx = None;
            }
        }
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

        if response.secondary_clicked() && self.resize_state.is_none() {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                if let Some((room_idx, _, _)) = self.room_at_pointer(pointer_pos, geometry) {
                    if let Some(state) = self.start_resize(room_idx, pointer_pos, geometry) {
                        self.resize_state = Some(state);
                        return true;
                    }
                }
            }
        }

        if let Some(state) = self.resize_state.clone() {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                if let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry) {
                    let cell = (grid_x.floor() as usize, grid_y.floor() as usize);
                    self.resize_room(&state, cell);
                }
            }

            if ctx.input(|i| i.pointer.button_released(egui::PointerButton::Secondary)) {
                self.resize_state = None;
            }
            return true;
        }

        false
    }

    // Choose the nearest room corner and initialize resize state.
    fn start_resize(
        &self,
        room_idx: usize,
        pointer_pos: egui::Pos2,
        geometry: &GridGeometry,
    ) -> Option<RoomResizeState> {
        let room = self.layout.rooms.get(room_idx)?;
        let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;
        let px = grid_x.floor().max(0.0) as isize;
        let py = grid_y.floor().max(0.0) as isize;

        let corners = [
            (ResizeCorner::TopLeft, (room.x as isize, room.y as isize)),
            (
                ResizeCorner::TopRight,
                ((room.x + room.width - 1) as isize, room.y as isize),
            ),
            (
                ResizeCorner::BottomLeft,
                (room.x as isize, (room.y + room.height - 1) as isize),
            ),
            (
                ResizeCorner::BottomRight,
                (
                    (room.x + room.width - 1) as isize,
                    (room.y + room.height - 1) as isize,
                ),
            ),
        ];

        let mut best = corners[0];
        let mut best_dist = isize::MAX;
        for corner in corners {
            let dx = corner.1.0 - px;
            let dy = corner.1.1 - py;
            let dist = dx * dx + dy * dy;
            if dist < best_dist {
                best = corner;
                best_dist = dist;
            }
        }

        let anchor_cell = match best.0 {
            ResizeCorner::TopLeft => (room.x + room.width - 1, room.y + room.height - 1),
            ResizeCorner::TopRight => (room.x, room.y + room.height - 1),
            ResizeCorner::BottomLeft => (room.x + room.width - 1, room.y),
            ResizeCorner::BottomRight => (room.x, room.y),
        };

        Some(RoomResizeState {
            room_idx,
            anchor_cell,
        })
    }

    // Resize a room toward a target cell while honoring constraints.
    fn resize_room(&mut self, state: &RoomResizeState, target_cell: (usize, usize)) {
        let cols = self.settings.cols.max(1);
        let rows = self.settings.rows.max(1);
        let min_size = self.settings.min_room_size.max(1);

        let ax = state.anchor_cell.0 as isize;
        let ay = state.anchor_cell.1 as isize;
        let tx = target_cell.0.min(cols - 1) as isize;
        let ty = target_cell.1.min(rows - 1) as isize;

        let x0 = ax.min(tx);
        let x1 = ax.max(tx);
        let y0 = ay.min(ty);
        let y1 = ay.max(ty);

        let new_room = layout::Room {
            x: x0 as usize,
            y: y0 as usize,
            width: (x1 - x0 + 1) as usize,
            height: (y1 - y0 + 1) as usize,
        };

        if new_room.width < min_size || new_room.height < min_size {
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

    // Draw dashed overlays for room resizing interactions.
    fn draw_resize_overlay(&self, painter: &egui::Painter, geometry: &GridGeometry) {
        let room_idx = self
            .resize_state
            .as_ref()
            .map(|s| s.room_idx)
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

        if self.resize_state.is_some() {
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
        if let Some(edge) = self.door_edge_at_pointer(pointer_pos, geometry) {
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
                    });
                }
                let door = &mut self.layout.doors[idx];
                door.locked = false;
                door.archway = true;
                door.width = 1;
                door.span_width = false;
                self.layout.doors.extend(spawned);
                return;
            }
        }

        if let Some((room_idx, _, _)) = self.room_at_pointer(pointer_pos, geometry) {
            self.delete_room(room_idx);
            return;
        }

        if let Some(corridor_drag) = self.corridor_drag_at_pointer(pointer_pos, geometry) {
            if corridor_drag.corridor_index < self.layout.corridors.len() {
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
        locked: bool,
    ) {
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
                });
            }
            let door = &mut self.layout.doors[idx];
            door.locked = locked;
            door.archway = false;
            door.width = 1;
            door.span_width = false;
            self.layout.doors.extend(spawned);
            return;
        }

        let corridor_cells = corridor_cells(&self.layout, self.settings.cols, self.settings.rows);
        let a_in_room = room_index_at_cell(&self.layout.rooms, edge.0).is_some();
        let b_in_room = room_index_at_cell(&self.layout.rooms, edge.1).is_some();
        let a_in_corridor = corridor_cells.contains(&edge.0);
        let b_in_corridor = corridor_cells.contains(&edge.1);

        if !(a_in_room ^ b_in_room) {
            return;
        }
        if !(a_in_corridor ^ b_in_corridor) {
            return;
        }

        self.layout.doors.push(layout::Door {
            from: edge.0,
            to: edge.1,
            width: 1,
            span_width: false,
            locked,
            archway: false,
        });
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

#[derive(Debug, Clone)]
struct RoomResizeState {
    room_idx: usize,
    anchor_cell: (usize, usize),
}

#[derive(Debug, Clone)]
enum DragState {
    Room(RoomDragState),
    Corridor(CorridorDragState),
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
        } else if door.archway {
            Color32::from_rgb(70, 130, 220)
        } else if door.locked {
            Color32::from_rgb(220, 70, 70)
        } else {
            Color32::from_rgb(80, 200, 120)
        };
        let style = if colorblind_mode {
            if door.archway {
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
        allow_middle_corridor_doors: settings.allow_middle_corridor_doors,
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

#[derive(Debug, Clone, Copy)]
enum DoorLineStyle {
    Solid,
    LongDash,
    ShortDash,
    Dotted,
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

// Compute the center position of a grid cell.
fn cell_center(geometry: &GridGeometry, col: usize, row: usize) -> egui::Pos2 {
    egui::pos2(
        geometry.rect.left() + (col as f32 + 0.5) * geometry.cell_size,
        geometry.rect.top() + (row as f32 + 0.5) * geometry.cell_size,
    )
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
        }

        ui.add_space(6.0);
        ui.label(label);
    });
    ui.add_space(4.0);
}
