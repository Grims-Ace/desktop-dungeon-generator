mod layout;
mod seed;
mod settings;
mod ui;

use std::collections::HashSet;

use eframe::egui;
use egui::{Color32, Stroke};
use layout::{DoorSettings, DungeonLayout, shortest_path_cells};
use ui::{UiSettings, draw_side_panel};

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
}

impl Default for DungeonApp {
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
            settings.corridor_randomness,
            settings.dead_end_rooms_percent,
            door_settings_from_ui(&settings),
        );
        Self {
            settings,
            layout,
            drag_state: None,
        }
    }
}

impl Drop for DungeonApp {
    fn drop(&mut self) {
        if let Err(err) = settings::save_settings(&self.settings) {
            eprintln!("Failed to save settings: {err}");
        }
    }
}

impl eframe::App for DungeonApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let panel_result = draw_side_panel(ctx, &mut self.settings);

        if self.settings.min_room_size > self.settings.max_room_size {
            self.settings.max_room_size = self.settings.min_room_size;
        }

        if panel_result.reset_clicked || panel_result.settings_changed {
            self.regenerate_layout();
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
                    draw_legend_entry(ui, Color32::from_rgb(220, 70, 70), "Doors");
                    draw_legend_entry(ui, Color32::from_rgb(80, 200, 120), "Locked Doors");
                    draw_legend_entry(ui, Color32::from_rgb(70, 130, 220), "Archways");
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_size_before_wrap();
            let (response, painter) = ui.allocate_painter(available, egui::Sense::click_and_drag());
            let canvas = response.rect.shrink(12.0);
            let geometry = draw_grid(&painter, canvas, self.settings.cols, self.settings.rows);
            self.handle_drag(&response, &geometry);
            draw_layout(
                &painter,
                &geometry,
                &self.layout,
                self.settings.colorblind_mode,
            );
        });
    }
}

impl DungeonApp {
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
            self.settings.corridor_randomness,
            self.settings.dead_end_rooms_percent,
            door_settings_from_ui(&self.settings),
        );
    }

    fn handle_drag(&mut self, response: &egui::Response, geometry: &GridGeometry) {
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

    fn reroute_corridors_for_room(&mut self, room_idx: usize) {
        let empty = HashSet::new();
        for corridor in &mut self.layout.corridors {
            if corridor.start_room_id != room_idx && corridor.end_room_id != room_idx {
                continue;
            }

            let start = self.layout.rooms[corridor.start_room_id].center_cell();
            let end = self.layout.rooms[corridor.end_room_id].center_cell();
            corridor.path =
                shortest_path_cells(start, end, self.settings.cols, self.settings.rows, &empty)
                    .unwrap_or_else(|| vec![start, end]);
        }
    }

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
        ) else {
            return;
        };

        self.layout.corridors[drag.corridor_index].path = new_path;
        self.refresh_doors();
    }

    fn refresh_doors(&mut self) {
        layout::apply_doors(
            &mut self.layout,
            self.settings.seed,
            door_settings_from_ui(&self.settings),
        );
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
enum DragState {
    Room(RoomDragState),
    Corridor(CorridorDragState),
}

struct GridGeometry {
    rect: egui::Rect,
    cell_size: f32,
}

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
    }
}

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
    let outline_stroke = Stroke::new(wall_width_px, Color32::BLACK);
    let mut corridor_cells_set = HashSet::new();
    let mut corridor_edges_set = HashSet::new();
    let mut room_cells_set = HashSet::new();
    let mut door_edges_set = HashSet::new();

    for door in &layout.doors {
        door_edges_set.insert(normalized_edge(door.from, door.to));
    }

    for corridor in &layout.corridors {
        for &cell in &corridor.path {
            corridor_cells_set.insert(cell);
        }
        for pair in corridor.path.windows(2) {
            corridor_edges_set.insert(normalized_edge(pair[0], pair[1]));
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
            painter.line_segment(
                [
                    egui::pos2(rect.left(), rect.top()),
                    egui::pos2(rect.left(), rect.bottom()),
                ],
                outline_stroke,
            );
        }
        if (!corridor_cells_set.contains(&right)
            || !corridor_edges_set.contains(&normalized_edge((col, row), right)))
            && !door_edges_set.contains(&normalized_edge((col, row), right))
        {
            painter.line_segment(
                [
                    egui::pos2(rect.right(), rect.top()),
                    egui::pos2(rect.right(), rect.bottom()),
                ],
                outline_stroke,
            );
        }
        if row == 0
            || (!corridor_cells_set.contains(&(col, row - 1))
                && top
                    .map(|n| !door_edges_set.contains(&normalized_edge((col, row), n)))
                    .unwrap_or(true))
        {
            painter.line_segment(
                [
                    egui::pos2(rect.left(), rect.top()),
                    egui::pos2(rect.right(), rect.top()),
                ],
                outline_stroke,
            );
        }
        if (!corridor_cells_set.contains(&bottom)
            || !corridor_edges_set.contains(&normalized_edge((col, row), bottom)))
            && !door_edges_set.contains(&normalized_edge((col, row), bottom))
        {
            painter.line_segment(
                [
                    egui::pos2(rect.left(), rect.bottom()),
                    egui::pos2(rect.right(), rect.bottom()),
                ],
                outline_stroke,
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

        if col == 0
            || (!room_cells_set.contains(&(col - 1, row))
                && left
                    .map(|n| !door_edges_set.contains(&normalized_edge((col, row), n)))
                    .unwrap_or(true))
        {
            painter.line_segment(
                [
                    egui::pos2(rect.left(), rect.top()),
                    egui::pos2(rect.left(), rect.bottom()),
                ],
                outline_stroke,
            );
        }
        if !room_cells_set.contains(&right)
            && !door_edges_set.contains(&normalized_edge((col, row), right))
        {
            painter.line_segment(
                [
                    egui::pos2(rect.right(), rect.top()),
                    egui::pos2(rect.right(), rect.bottom()),
                ],
                outline_stroke,
            );
        }
        if row == 0
            || (!room_cells_set.contains(&(col, row - 1))
                && top
                    .map(|n| !door_edges_set.contains(&normalized_edge((col, row), n)))
                    .unwrap_or(true))
        {
            painter.line_segment(
                [
                    egui::pos2(rect.left(), rect.top()),
                    egui::pos2(rect.right(), rect.top()),
                ],
                outline_stroke,
            );
        }
        if !room_cells_set.contains(&bottom)
            && !door_edges_set.contains(&normalized_edge((col, row), bottom))
        {
            painter.line_segment(
                [
                    egui::pos2(rect.left(), rect.bottom()),
                    egui::pos2(rect.right(), rect.bottom()),
                ],
                outline_stroke,
            );
        }
    }

    for door in &layout.doors {
        let color = if colorblind_mode {
            Color32::BLACK
        } else if door.archway {
            Color32::from_rgb(70, 130, 220)
        } else if door.locked {
            Color32::from_rgb(80, 200, 120)
        } else {
            Color32::from_rgb(220, 70, 70)
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
            Stroke::new(door_width_px, color),
            style,
        );
    }
}

fn cell_rect(geometry: &GridGeometry, col: usize, row: usize) -> egui::Rect {
    let left = geometry.rect.left() + col as f32 * geometry.cell_size;
    let top = geometry.rect.top() + row as f32 * geometry.cell_size;
    egui::Rect::from_min_size(
        egui::pos2(left, top),
        egui::vec2(geometry.cell_size, geometry.cell_size),
    )
}

fn normalized_edge(a: (usize, usize), b: (usize, usize)) -> ((usize, usize), (usize, usize)) {
    if a <= b { (a, b) } else { (b, a) }
}

fn pointer_to_grid(pointer_pos: egui::Pos2, geometry: &GridGeometry) -> Option<(f32, f32)> {
    if !geometry.rect.contains(pointer_pos) {
        return None;
    }

    let x = (pointer_pos.x - geometry.rect.left()) / geometry.cell_size;
    let y = (pointer_pos.y - geometry.rect.top()) / geometry.cell_size;
    Some((x, y))
}

fn reroute_path_through_cell(
    original_path: &[(usize, usize)],
    original_cell: (usize, usize),
    target_cell: (usize, usize),
    cols: usize,
    rows: usize,
) -> Option<Vec<(usize, usize)>> {
    let (&start, &end) = (original_path.first()?, original_path.last()?);
    if start == end {
        return None;
    }

    let mut blocked = HashSet::new();
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

fn door_settings_from_ui(settings: &UiSettings) -> DoorSettings {
    DoorSettings {
        frequency_percent: settings.door_frequency_percent,
        room_hallway_percent: settings.room_hallway_door_percent,
        locked_percent: settings.locked_door_percent,
        allow_middle_corridor_doors: settings.allow_middle_corridor_doors,
    }
}

fn draw_door_line(
    painter: &egui::Painter,
    geometry: &GridGeometry,
    a: (usize, usize),
    b: (usize, usize),
    stroke: Stroke,
    style: DoorLineStyle,
) {
    if a.0.abs_diff(b.0) + a.1.abs_diff(b.1) != 1 {
        return;
    }

    if a.0 != b.0 {
        let x = geometry.rect.left() + (a.0.max(b.0) as f32) * geometry.cell_size;
        let row = a.1;
        let y0 = geometry.rect.top() + row as f32 * geometry.cell_size;
        let y1 = y0 + geometry.cell_size;
        draw_styled_line(painter, egui::pos2(x, y0), egui::pos2(x, y1), stroke, style);
    } else {
        let y = geometry.rect.top() + (a.1.max(b.1) as f32) * geometry.cell_size;
        let col = a.0;
        let x0 = geometry.rect.left() + col as f32 * geometry.cell_size;
        let x1 = x0 + geometry.cell_size;
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

fn cell_center(geometry: &GridGeometry, col: usize, row: usize) -> egui::Pos2 {
    egui::pos2(
        geometry.rect.left() + (col as f32 + 0.5) * geometry.cell_size,
        geometry.rect.top() + (row as f32 + 0.5) * geometry.cell_size,
    )
}

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
