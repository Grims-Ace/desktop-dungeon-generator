use crate::app::{DungeonApp, ManualDoorKind};
use crate::layout::{
    self, blocked_room_cells, corridor_cells, rects_overlap, room_index_at_cell, rooms_overlap,
    shortest_path_cells,
};
use crate::ui::AddTool;
use eframe::egui;
use egui::{Color32, Stroke};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct RoomDragState {
    pub room_idx: usize,
    pub offset_x: f32,
    pub offset_y: f32,
}

#[derive(Debug, Clone)]
pub struct TextDragState {
    pub text_idx: usize,
}

#[derive(Debug, Clone)]
pub struct MarkerDragState {
    pub kind: MarkerKind,
    pub marker_idx: usize,
    pub offset_x: f32,
    pub offset_y: f32,
}

#[derive(Debug, Clone)]
pub struct CorridorDragState {
    pub corridor_index: usize,
    pub original_path: Vec<(usize, usize)>,
    pub original_cell: (usize, usize),
}

#[derive(Debug, Clone)]
pub struct AddCorridorDrag {
    pub start_cell: (usize, usize),
    pub current_cell: (usize, usize),
}

#[derive(Debug, Clone, Copy)]
pub enum ResizeCorner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerKind {
    Start,
    End,
    Trap,
    Monster,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoverMarker {
    Start(usize),
    End(usize),
    Trap(usize),
    Monster(usize),
}

#[derive(Debug, Clone)]
pub struct RoomResizeState {
    pub room_idx: usize,
    pub original_room: layout::Room,
    pub resize_corner: ResizeCorner,
}

#[derive(Debug, Clone)]
pub enum DragState {
    Room(RoomDragState),
    Text(TextDragState),
    Marker(MarkerDragState),
    Corridor(CorridorDragState),
    Staircase(StaircaseDragState),
}

#[derive(Debug, Clone)]
pub struct StaircaseDragState {
    pub stair_idx: usize,
    pub offset_x: f32,
    pub offset_y: f32,
    pub original_cell: (usize, usize),
}

#[derive(Debug, Clone)]
pub struct TextResizeState {
    pub text_idx: usize,
    pub origin_cell: (usize, usize),
    pub original_font_size: u16,
}

#[derive(Debug, Clone)]
pub struct StaircaseResizeState {
    pub stair_idx: usize,
    pub original_stair: layout::Staircase,
    pub resize_corner: ResizeCorner,
}

#[derive(Debug, Clone)]
pub enum ResizeState {
    Room(RoomResizeState),
    Text(TextResizeState),
    Staircase(StaircaseResizeState),
}

pub struct GridGeometry {
    pub rect: egui::Rect,
    pub cell_size: f32,
    pub cols: usize,
    pub rows: usize,
}

pub fn draw_grid(
    painter: &egui::Painter,
    area: egui::Rect,
    cols: usize,
    rows: usize,
) -> GridGeometry {
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

pub fn handle_drag(
    app: &mut DungeonApp,
    ctx: &egui::Context,
    response: &egui::Response,
    geometry: &GridGeometry,
) {
    if !ctx.input(|i| i.pointer.primary_down()) && !response.drag_started() {
        return;
    }
    if app.settings.add_tool != AddTool::None {
        return;
    }
    if response.drag_started()
        && let Some(pointer_pos) = response.interact_pointer_pos()
        && let Some(text_idx) = text_at_pointer(app, pointer_pos, geometry)
    {
        app.push_undo_snapshot();
        app.drag_state = Some(DragState::Text(TextDragState { text_idx }));
    } else if response.drag_started()
        && let Some(pointer_pos) = response.interact_pointer_pos()
        && let Some(marker_drag) = marker_drag_at_pointer(app, pointer_pos, geometry)
    {
        app.push_undo_snapshot();
        app.drag_state = Some(DragState::Marker(marker_drag));
    } else if response.drag_started()
        && let Some(pointer_pos) = response.interact_pointer_pos()
        && let Some(stair_drag) = stair_drag_at_pointer(app, pointer_pos, geometry)
    {
        app.push_undo_snapshot();
        app.drag_state = Some(DragState::Staircase(stair_drag));
    } else if response.drag_started()
        && let Some(pointer_pos) = response.interact_pointer_pos()
        && let Some(corridor_drag) = corridor_drag_at_pointer(app, pointer_pos, geometry)
    {
        app.push_undo_snapshot();
        app.drag_state = Some(DragState::Corridor(corridor_drag));
    } else if response.drag_started()
        && let Some(pointer_pos) = response.interact_pointer_pos()
        && let Some((room_idx, offset_x, offset_y)) = room_at_pointer(app, pointer_pos, geometry)
    {
        app.push_undo_snapshot();
        app.drag_state = Some(DragState::Room(RoomDragState {
            room_idx,
            offset_x,
            offset_y,
        }));
    }

    if response.dragged()
        && let Some(pointer_pos) = response.interact_pointer_pos()
        && let Some(drag_state) = app.drag_state.clone()
    {
        match drag_state {
            DragState::Room(drag) => drag_room_to_pointer(app, drag, pointer_pos, geometry),
            DragState::Text(drag) => drag_text_to_pointer(app, drag, pointer_pos, geometry),
            DragState::Marker(drag) => drag_marker_to_pointer(app, drag, pointer_pos, geometry),
            DragState::Corridor(drag) => drag_corridor_to_pointer(app, drag, pointer_pos, geometry),
            DragState::Staircase(drag) => drag_stair_to_pointer(app, drag, pointer_pos, geometry),
        }
    }

    if response.drag_stopped() || !response.ctx.input(|i| i.pointer.primary_down()) {
        app.drag_state = None;
    }
}

pub fn handle_resize(
    app: &mut DungeonApp,
    ctx: &egui::Context,
    response: &egui::Response,
    geometry: &GridGeometry,
) -> bool {
    if app.settings.add_tool != AddTool::None {
        return false;
    }

    if ctx.input(|i| i.pointer.secondary_pressed()) && app.resize_state.is_none() {
        if let Some(pointer_pos) = response.interact_pointer_pos() {
            if let Some(text_idx) = text_at_pointer(app, pointer_pos, geometry) {
                if let Some(state) = start_text_resize(app, text_idx) {
                    app.push_undo_snapshot();
                    app.resize_state = Some(ResizeState::Text(state));
                    return true;
                }
            } else if let Some(stair_idx) = stair_at_pointer(app, pointer_pos, geometry) {
                if let Some(state) = start_staircase_resize(app, stair_idx, pointer_pos, geometry) {
                    app.push_undo_snapshot();
                    app.resize_state = Some(ResizeState::Staircase(state));
                    return true;
                }
            } else if let Some(room_idx) = resize_room_at_pointer(app, pointer_pos, geometry) {
                if let Some(state) = start_resize(app, room_idx, pointer_pos, geometry) {
                    app.push_undo_snapshot();
                    app.resize_state = Some(ResizeState::Room(state));
                    return true;
                }
            }
        }
    }

    if let Some(state) = app.resize_state.clone() {
        if response.dragged_by(egui::PointerButton::Secondary)
            && ctx.input(|i| i.pointer.secondary_down())
            && let Some(pointer_pos) = response.interact_pointer_pos()
        {
            if let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry) {
                let cell = (grid_x.floor() as usize, grid_y.floor() as usize);
                match state {
                    ResizeState::Room(state) => resize_room(app, &state, cell),
                    ResizeState::Text(state) => resize_text(app, &state, cell),
                    ResizeState::Staircase(state) => resize_staircase(app, &state, cell),
                }
            }
        }

        if ctx.input(|i| i.pointer.button_released(egui::PointerButton::Secondary)) {
            app.resize_state = None;
        }
        return true;
    }

    false
}

pub fn handle_add_tool(app: &mut DungeonApp, response: &egui::Response, geometry: &GridGeometry) {
    if app.settings.add_tool != AddTool::None && response.secondary_clicked() {
        app.settings.add_tool = AddTool::None;
        app.add_corridor_drag = None;
        return;
    }

    match app.settings.add_tool {
        AddTool::Room => {
            app.add_corridor_drag = None;
            if response.clicked()
                && let Some(pointer_pos) = response.interact_pointer_pos()
                && let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry)
            {
                let cell = (grid_x.floor() as usize, grid_y.floor() as usize);
                add_room_at_cell(app, cell);
            }
        }
        AddTool::Corridor => {
            if response.drag_started()
                && let Some(pointer_pos) = response.interact_pointer_pos()
                && let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry)
            {
                let cell = (grid_x.floor() as usize, grid_y.floor() as usize);
                app.add_corridor_drag = Some(AddCorridorDrag {
                    start_cell: cell,
                    current_cell: cell,
                });
            }

            if response.dragged()
                && let Some(pointer_pos) = response.interact_pointer_pos()
                && let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry)
                && let Some(drag) = &mut app.add_corridor_drag
            {
                drag.current_cell = (grid_x.floor() as usize, grid_y.floor() as usize);
            }

            if response.drag_stopped() {
                if let Some(drag) = app.add_corridor_drag.take() {
                    add_corridor_between(app, drag.start_cell, drag.current_cell);
                }
            }
        }
        AddTool::Staircase => {
            app.add_corridor_drag = None;
            if response.clicked()
                && let Some(pointer_pos) = response.interact_pointer_pos()
                && let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry)
            {
                let cell = (grid_x.floor() as usize, grid_y.floor() as usize);
                add_staircase_at_cell(app, cell);
            }
        }
        AddTool::Archway | AddTool::Door | AddTool::LockedDoor | AddTool::SecretDoor => {
            app.add_corridor_drag = None;
            if response.clicked()
                && let Some(pointer_pos) = response.interact_pointer_pos()
            {
                let kind = match app.settings.add_tool {
                    AddTool::Archway => ManualDoorKind::Archway,
                    AddTool::Door => ManualDoorKind::Regular,
                    AddTool::LockedDoor => ManualDoorKind::Locked,
                    AddTool::SecretDoor => ManualDoorKind::Secret,
                    _ => unreachable!(),
                };
                add_door_at_pointer(app, pointer_pos, geometry, kind);
            }
        }
        AddTool::Text => {
            app.add_corridor_drag = None;
            if response.clicked()
                && let Some(pointer_pos) = response.interact_pointer_pos()
                && let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry)
            {
                let cell = (grid_x.floor() as usize, grid_y.floor() as usize);
                add_text_at_cell(app, cell);
            }
        }
        AddTool::StartMarker
        | AddTool::EndMarker
        | AddTool::TrapMarker
        | AddTool::MonsterMarker => {
            app.add_corridor_drag = None;
            if response.clicked()
                && let Some(pointer_pos) = response.interact_pointer_pos()
                && let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry)
            {
                let cell = (grid_x.floor() as usize, grid_y.floor() as usize);
                let kind = match app.settings.add_tool {
                    AddTool::StartMarker => MarkerKind::Start,
                    AddTool::EndMarker => MarkerKind::End,
                    AddTool::TrapMarker => MarkerKind::Trap,
                    AddTool::MonsterMarker => MarkerKind::Monster,
                    _ => unreachable!(),
                };
                add_marker_at_cell(app, cell, kind);
            }
        }
        AddTool::None => {
            app.add_corridor_drag = None;
        }
    }
}

pub fn update_hover_targets(app: &mut DungeonApp, ctx: &egui::Context, geometry: &GridGeometry) {
    if app.drag_state.is_some() || app.resize_state.is_some() || app.add_corridor_drag.is_some() {
        return;
    }

    let Some(pointer_pos) = ctx.pointer_hover_pos() else {
        app.clear_hover_targets();
        return;
    };
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let layout = &app.levels[app.settings.active_level_index];

    if let Some(edge) = door_edge_at_pointer(app, pointer_pos, geometry) {
        let target = normalized_edge(edge.0, edge.1);
        app.hover_door_idx = layout.doors.iter().position(|d| {
            door_edges_for(d, app.settings.cols, app.settings.rows)
                .iter()
                .any(|e| normalized_edge(e.0, e.1) == target)
        });
        if app.hover_door_idx.is_some() {
            app.hover_room_idx = None;
            app.hover_corridor_idx = None;
            app.hover_text_idx = None;
            app.hover_marker = None;
            return;
        }
    }

    app.hover_text_idx = text_at_pointer(app, pointer_pos, geometry);
    if app.hover_text_idx.is_some() {
        app.hover_room_idx = None;
        app.hover_corridor_idx = None;
        app.hover_door_idx = None;
        app.hover_marker = None;
        return;
    }

    app.hover_marker = marker_at_pointer(app, pointer_pos, geometry);
    if app.hover_marker.is_some() {
        app.hover_room_idx = None;
        app.hover_corridor_idx = None;
        app.hover_door_idx = None;
        app.hover_stair_idx = None;
        return;
    }

    app.hover_stair_idx = stair_at_pointer(app, pointer_pos, geometry);
    if app.hover_stair_idx.is_some() {
        app.hover_room_idx = None;
        app.hover_corridor_idx = None;
        app.hover_door_idx = None;
        app.hover_marker = None;
        return;
    }

    if let Some((room_idx, _, _)) = room_at_pointer(app, pointer_pos, geometry) {
        app.hover_room_idx = Some(room_idx);
        app.hover_corridor_idx = None;
        app.hover_door_idx = None;
        app.hover_marker = None;
    } else {
        app.hover_room_idx = None;
        if let Some(corridor_drag) = corridor_drag_at_pointer(app, pointer_pos, geometry) {
            app.hover_corridor_idx = Some(corridor_drag.corridor_index);
            app.hover_door_idx = None;
            app.hover_marker = None;
        } else {
            app.hover_corridor_idx = None;
            app.hover_door_idx = None;
            app.hover_marker = None;
        }
    }
}

pub fn delete_at_pointer(app: &mut DungeonApp, pointer_pos: egui::Pos2, geometry: &GridGeometry) {
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }

    if let Some(idx) = text_at_pointer(app, pointer_pos, geometry) {
        app.push_undo_snapshot();
        let layout = &mut app.levels[app.settings.active_level_index];
        if idx < layout.text_labels.len() {
            layout.text_labels.remove(idx);
        }
        return;
    }

    if let Some(marker) = marker_at_pointer(app, pointer_pos, geometry) {
        app.push_undo_snapshot();
        let layout = &mut app.levels[app.settings.active_level_index];
        match marker {
            HoverMarker::Start(idx) => {
                if idx < layout.start_markers.len() {
                    layout.start_markers.remove(idx);
                }
            }
            HoverMarker::End(idx) => {
                if idx < layout.end_markers.len() {
                    layout.end_markers.remove(idx);
                }
            }
            HoverMarker::Trap(idx) => {
                if idx < layout.trap_markers.len() {
                    layout.trap_markers.remove(idx);
                }
            }
            HoverMarker::Monster(idx) => {
                if idx < layout.monster_markers.len() {
                    layout.monster_markers.remove(idx);
                }
            }
        }
        return;
    }

    if let Some(edge) = door_edge_at_pointer(app, pointer_pos, geometry) {
        let target = normalized_edge(edge.0, edge.1);
        let active_index = app.settings.active_level_index;
        if let Some(idx) = app.levels[active_index].doors.iter().position(|d| {
            door_edges_for(d, app.settings.cols, app.settings.rows)
                .iter()
                .any(|e| normalized_edge(e.0, e.1) == target)
        }) {
            app.push_undo_snapshot();
            let layout = &mut app.levels[app.settings.active_level_index];
            if !layout.doors[idx].manual {
                for edge in door_edges_for(&layout.doors[idx], app.settings.cols, app.settings.rows)
                {
                    app.suppressed_auto_door_edges.insert(edge);
                }
                layout.doors.remove(idx);
                return;
            }

            layout.doors.remove(idx);
            return;
        }
    }

    if let Some(stair_idx) = stair_at_pointer(app, pointer_pos, geometry) {
        let original_cell = {
            let layout = &app.levels[app.settings.active_level_index];
            layout.stairs[stair_idx].cell
        };

        app.push_undo_snapshot();
        if app.settings.sync_stairs_across_levels {
            for layout in &mut app.levels {
                layout.stairs.retain(|s| s.cell != original_cell);
            }
        } else {
            let current_idx = app.settings.active_level_index;
            let level_indices = vec![
                Some(current_idx),
                current_idx.checked_sub(1),
                if current_idx + 1 < app.levels.len() {
                    Some(current_idx + 1)
                } else {
                    None
                },
            ];

            for idx in level_indices.into_iter().flatten() {
                app.levels[idx].stairs.retain(|s| s.cell != original_cell);
            }
        }
        return;
    }

    if let Some((room_idx, _, _)) = room_at_pointer(app, pointer_pos, geometry) {
        app.push_undo_snapshot();
        delete_room(app, room_idx);
        return;
    }

    if let Some(corridor_drag) = corridor_drag_at_pointer(app, pointer_pos, geometry) {
        let active_index = app.settings.active_level_index;
        if corridor_drag.corridor_index < app.levels[active_index].corridors.len() {
            app.push_undo_snapshot();
            let layout = &mut app.levels[app.settings.active_level_index];
            layout.corridors.remove(corridor_drag.corridor_index);
            app.drag_state = None;
            app.refresh_doors();
        }
    }
}

pub fn add_room_at_cell(app: &mut DungeonApp, cell: (usize, usize)) {
    app.push_undo_snapshot();
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let layout = &mut app.levels[app.settings.active_level_index];

    let width = app.settings.add_room_width.max(1);
    let height = app.settings.add_room_height.max(1);
    if cell.0 >= app.settings.cols || cell.1 >= app.settings.rows {
        return;
    }
    let max_w = app.settings.cols.saturating_sub(cell.0);
    let max_h = app.settings.rows.saturating_sub(cell.1);
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

    if layout
        .rooms
        .iter()
        .any(|room| rooms_overlap(&new_room, room))
    {
        return;
    }

    layout.rooms.push(new_room);
    app.refresh_doors();
}

pub fn add_corridor_between(app: &mut DungeonApp, start: (usize, usize), end: (usize, usize)) {
    app.push_undo_snapshot();
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let layout = &mut app.levels[app.settings.active_level_index];

    if start == end {
        return;
    }

    let start_room = room_index_at_cell(&layout.rooms, start);
    let end_room = room_index_at_cell(&layout.rooms, end);
    let (Some(start_room), Some(end_room)) = (start_room, end_room) else {
        return;
    };

    let blocked = blocked_room_cells(&layout.rooms, &[start_room, end_room]);

    let Some(path) =
        shortest_path_cells(start, end, app.settings.cols, app.settings.rows, &blocked)
    else {
        return;
    };

    let next_id = layout
        .corridors
        .iter()
        .map(|c| c.id)
        .max()
        .unwrap_or(0)
        .wrapping_add(1);
    let width = app.settings.add_corridor_width.max(1);

    layout.corridors.push(layout::Corridor {
        id: next_id,
        start_room_id: start_room,
        end_room_id: end_room,
        path,
        width,
    });
    app.refresh_doors();
}

pub fn add_text_at_cell(app: &mut DungeonApp, cell: (usize, usize)) {
    app.push_undo_snapshot();
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let layout = &mut app.levels[app.settings.active_level_index];

    if cell.0 >= app.settings.cols || cell.1 >= app.settings.rows {
        return;
    }

    let text = app.settings.add_text_value.trim().to_string();
    if text.is_empty() {
        return;
    }

    if let Some(existing) = layout
        .text_labels
        .iter_mut()
        .find(|label| label.cell == cell)
    {
        existing.text = text;
        return;
    }

    layout.text_labels.push(layout::TextLabel {
        cell,
        text,
        font_size: app.settings.add_text_font_size,
    });
}

pub fn add_marker_at_cell(app: &mut DungeonApp, cell: (usize, usize), kind: MarkerKind) {
    app.push_undo_snapshot();
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let layout = &mut app.levels[app.settings.active_level_index];

    if cell.0 >= app.settings.cols || cell.1 >= app.settings.rows {
        return;
    }

    let size = app.settings.add_marker_size.max(1);
    let origin = centered_marker_origin(cell, size, app.settings.cols, app.settings.rows);
    let marker = layout::AreaMarker { cell: origin, size };

    match kind {
        MarkerKind::Start => layout.start_markers.push(marker),
        MarkerKind::End => layout.end_markers.push(marker),
        MarkerKind::Trap => layout.trap_markers.push(marker),
        MarkerKind::Monster => layout.monster_markers.push(marker),
    }
}

pub fn add_staircase_at_cell(app: &mut DungeonApp, cell: (usize, usize)) {
    app.push_undo_snapshot();
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }

    if cell.0 >= app.settings.cols || cell.1 >= app.settings.rows {
        return;
    }

    let width = app.settings.add_stair_width.max(1);
    let height = app.settings.add_stair_height.max(1);
    let max_x = app.settings.cols.saturating_sub(width);
    let max_y = app.settings.rows.saturating_sub(height);
    let origin_x = cell.0.min(max_x);
    let origin_y = cell.1.min(max_y);

    let stair = layout::Staircase {
        cell: (origin_x, origin_y),
        width,
        height,
    };

    if app.settings.sync_stairs_across_levels {
        for layout in &mut app.levels {
            layout.stairs.push(stair.clone());
        }
    } else {
        let active_idx = app.settings.active_level_index;
        let layout = &mut app.levels[active_idx];
        layout.stairs.push(stair.clone());

        if active_idx + 1 < app.levels.len() {
            let above_layout = &mut app.levels[active_idx + 1];
            above_layout.stairs.push(stair);
        } else if active_idx > 0 {
            let below_layout = &mut app.levels[active_idx - 1];
            below_layout.stairs.push(stair);
        }
    }
}

pub fn delete_room(app: &mut DungeonApp, room_idx: usize) {
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let layout = &mut app.levels[app.settings.active_level_index];

    if room_idx >= layout.rooms.len() {
        return;
    }
    layout.rooms.remove(room_idx);
    layout
        .corridors
        .retain(|c| c.start_room_id != room_idx && c.end_room_id != room_idx);
    for corridor in &mut layout.corridors {
        if corridor.start_room_id > room_idx {
            corridor.start_room_id -= 1;
        }
        if corridor.end_room_id > room_idx {
            corridor.end_room_id -= 1;
        }
    }
    app.drag_state = None;
    app.refresh_doors();
}

pub fn add_door_at_pointer(
    app: &mut DungeonApp,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
    kind: ManualDoorKind,
) {
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }

    let Some(edge) = door_edge_at_pointer(app, pointer_pos, geometry) else {
        return;
    };

    app.push_undo_snapshot();
    let layout = &mut app.levels[app.settings.active_level_index];

    let target = normalized_edge(edge.0, edge.1);
    if let Some(idx) = layout.doors.iter().position(|d| {
        door_edges_for(d, app.settings.cols, app.settings.rows)
            .iter()
            .any(|e| normalized_edge(e.0, e.1) == target)
    }) {
        let edges = door_edges_for(&layout.doors[idx], app.settings.cols, app.settings.rows);
        let mut spawned = Vec::new();
        for edge in edges {
            if normalized_edge(edge.0, edge.1) == target {
                continue;
            }
            if layout
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
        let door = &mut layout.doors[idx];
        door.locked = kind == ManualDoorKind::Locked;
        door.archway = kind == ManualDoorKind::Archway;
        door.secret = kind == ManualDoorKind::Secret;
        door.width = 1;
        door.span_width = false;
        door.manual = true;
        for edge in door_edges_for(door, app.settings.cols, app.settings.rows) {
            app.suppressed_auto_door_edges.remove(&edge);
        }
        layout.doors.extend(spawned);
        return;
    }

    let corridor_cells = corridor_cells(layout, app.settings.cols, app.settings.rows);
    let a_room = room_index_at_cell(&layout.rooms, edge.0);
    let b_room = room_index_at_cell(&layout.rooms, edge.1);
    let a_in_corridor = corridor_cells.contains(&edge.0);
    let b_in_corridor = corridor_cells.contains(&edge.1);

    if !manual_door_edge_allowed(a_room, b_room, a_in_corridor, b_in_corridor) {
        return;
    }

    layout.doors.push(layout::Door {
        from: edge.0,
        to: edge.1,
        width: 1,
        span_width: false,
        locked: kind == ManualDoorKind::Locked,
        archway: kind == ManualDoorKind::Archway,
        secret: kind == ManualDoorKind::Secret,
        manual: true,
    });
    app.suppressed_auto_door_edges.remove(&target);
}

pub fn door_edge_at_pointer(
    app: &DungeonApp,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) -> Option<((usize, usize), (usize, usize))> {
    let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;
    let cell_x = grid_x.floor() as isize;
    let cell_y = grid_y.floor() as isize;
    if cell_x < 0
        || cell_y < 0
        || cell_x >= app.settings.cols as isize
        || cell_y >= app.settings.rows as isize
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
        || ax >= app.settings.cols as isize
        || bx >= app.settings.cols as isize
        || ay >= app.settings.rows as isize
        || by >= app.settings.rows as isize
    {
        return None;
    }

    Some(((ax as usize, ay as usize), (bx as usize, by as usize)))
}

pub fn room_at_pointer(
    app: &DungeonApp,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) -> Option<(usize, f32, f32)> {
    let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;

    if app.settings.active_level_index >= app.levels.len() {
        return None;
    }
    let layout = &app.levels[app.settings.active_level_index];

    for (room_idx, room) in layout.rooms.iter().enumerate().rev() {
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

pub fn drag_room_to_pointer(
    app: &mut DungeonApp,
    drag: RoomDragState,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) {
    let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry) else {
        return;
    };

    if app.settings.active_level_index >= app.levels.len() {
        return;
    }

    if drag.room_idx >= app.levels[app.settings.active_level_index].rooms.len() {
        app.drag_state = None;
        return;
    }

    let (width, height, old_x, old_y) = {
        let room = &app.levels[app.settings.active_level_index].rooms[drag.room_idx];
        (room.width, room.height, room.x, room.y)
    };
    let max_x = app.settings.cols.saturating_sub(width);
    let max_y = app.settings.rows.saturating_sub(height);

    let target_x = (grid_x - drag.offset_x).floor().max(0.0) as usize;
    let target_y = (grid_y - drag.offset_y).floor().max(0.0) as usize;
    let new_x = target_x.min(max_x);
    let new_y = target_y.min(max_y);

    if new_x == old_x && new_y == old_y {
        return;
    }

    app.levels[app.settings.active_level_index].rooms[drag.room_idx].x = new_x;
    app.levels[app.settings.active_level_index].rooms[drag.room_idx].y = new_y;
    reroute_corridors_for_room(app, drag.room_idx);
    app.refresh_doors();
}

pub fn drag_text_to_pointer(
    app: &mut DungeonApp,
    drag: TextDragState,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) {
    let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry) else {
        return;
    };

    if app.settings.active_level_index >= app.levels.len() {
        return;
    }

    if drag.text_idx
        >= app.levels[app.settings.active_level_index]
            .text_labels
            .len()
    {
        app.drag_state = None;
        return;
    }

    let target = (
        (grid_x.floor().max(0.0) as usize).min(app.settings.cols.saturating_sub(1)),
        (grid_y.floor().max(0.0) as usize).min(app.settings.rows.saturating_sub(1)),
    );

    app.levels[app.settings.active_level_index].text_labels[drag.text_idx].cell = target;
}

pub fn reroute_corridors_for_room(app: &mut DungeonApp, room_idx: usize) {
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let layout = &mut app.levels[app.settings.active_level_index];

    if layout.packed_rooms {
        return;
    }

    for corridor in &mut layout.corridors {
        if corridor.start_room_id != room_idx && corridor.end_room_id != room_idx {
            continue;
        }

        let start = layout.rooms[corridor.start_room_id].center_cell();
        let end = layout.rooms[corridor.end_room_id].center_cell();
        let blocked = blocked_room_cells(
            &layout.rooms,
            &[corridor.start_room_id, corridor.end_room_id],
        );
        if let Some(path) =
            shortest_path_cells(start, end, app.settings.cols, app.settings.rows, &blocked)
        {
            corridor.path = path;
        }
    }
}

pub fn corridor_drag_at_pointer(
    app: &DungeonApp,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) -> Option<CorridorDragState> {
    let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;
    let clicked = (grid_x.floor() as usize, grid_y.floor() as usize);
    if app.settings.active_level_index >= app.levels.len() {
        return None;
    }
    let layout = &app.levels[app.settings.active_level_index];

    let corridor_index = layout
        .corridors
        .iter()
        .position(|corridor| corridor.path.contains(&clicked))?;

    Some(CorridorDragState {
        corridor_index,
        original_path: layout.corridors[corridor_index].path.clone(),
        original_cell: clicked,
    })
}

pub fn drag_corridor_to_pointer(
    app: &mut DungeonApp,
    drag: CorridorDragState,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) {
    let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry) else {
        return;
    };

    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let layout = &mut app.levels[app.settings.active_level_index];

    if drag.corridor_index >= layout.corridors.len() {
        app.drag_state = None;
        return;
    }
    let corridor = &layout.corridors[drag.corridor_index];

    let target = (
        (grid_x.floor().max(0.0) as usize).min(app.settings.cols.saturating_sub(1)),
        (grid_y.floor().max(0.0) as usize).min(app.settings.rows.saturating_sub(1)),
    );

    let Some(new_path) = reroute_path_through_cell(
        &drag.original_path,
        drag.original_cell,
        target,
        app.settings.cols,
        app.settings.rows,
        &blocked_room_cells(
            &layout.rooms,
            &[corridor.start_room_id, corridor.end_room_id],
        ),
    ) else {
        return;
    };

    layout.corridors[drag.corridor_index].path = new_path;
    app.refresh_doors();
}

pub fn drag_marker_to_pointer(
    app: &mut DungeonApp,
    drag: MarkerDragState,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) {
    let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry) else {
        return;
    };

    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let layout = &mut app.levels[app.settings.active_level_index];

    let markers = match drag.kind {
        MarkerKind::Start => &mut layout.start_markers,
        MarkerKind::End => &mut layout.end_markers,
        MarkerKind::Trap => &mut layout.trap_markers,
        MarkerKind::Monster => &mut layout.monster_markers,
    };
    let Some(marker) = markers.get_mut(drag.marker_idx) else {
        app.drag_state = None;
        return;
    };

    let max_x = app.settings.cols.saturating_sub(marker.size);
    let max_y = app.settings.rows.saturating_sub(marker.size);
    let target_x = (grid_x - drag.offset_x).floor().max(0.0) as usize;
    let target_y = (grid_y - drag.offset_y).floor().max(0.0) as usize;
    marker.cell = (target_x.min(max_x), target_y.min(max_y));
}

pub fn drag_stair_to_pointer(
    app: &mut DungeonApp,
    drag: StaircaseDragState,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) {
    let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry) else {
        return;
    };

    if app.settings.active_level_index >= app.levels.len() {
        return;
    }

    let (stair_width, stair_height) = {
        let layout = &app.levels[app.settings.active_level_index];
        let Some(stair) = layout.stairs.get(drag.stair_idx) else {
            return;
        };
        (stair.width, stair.height)
    };

    let max_x = app.settings.cols.saturating_sub(stair_width);
    let max_y = app.settings.rows.saturating_sub(stair_height);
    let target_x = (grid_x - drag.offset_x).floor().max(0.0) as usize;
    let target_y = (grid_y - drag.offset_y).floor().max(0.0) as usize;
    let new_cell = (target_x.min(max_x), target_y.min(max_y));

    if app.settings.sync_stairs_across_levels {
        for layout in &mut app.levels {
            for stair in &mut layout.stairs {
                if stair.cell == drag.original_cell {
                    stair.cell = new_cell;
                }
            }
        }
    } else {
        let current_idx = app.settings.active_level_index;
        let level_indices = vec![
            Some(current_idx),
            current_idx.checked_sub(1),
            if current_idx + 1 < app.levels.len() {
                Some(current_idx + 1)
            } else {
                None
            },
        ];

        for idx in level_indices.into_iter().flatten() {
            let layout = &mut app.levels[idx];
            for stair in &mut layout.stairs {
                if stair.cell == drag.original_cell {
                    stair.cell = new_cell;
                }
            }
        }
    }

    if let Some(DragState::Staircase(ref mut d)) = app.drag_state {
        d.original_cell = new_cell;
    }
}

pub fn text_at_pointer(
    app: &DungeonApp,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) -> Option<usize> {
    let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;
    let clicked = (grid_x.floor() as usize, grid_y.floor() as usize);
    if app.settings.active_level_index >= app.levels.len() {
        return None;
    }
    let layout = &app.levels[app.settings.active_level_index];

    layout
        .text_labels
        .iter()
        .position(|label| label.cell == clicked)
}

pub fn marker_at_pointer(
    app: &DungeonApp,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) -> Option<HoverMarker> {
    let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;
    let clicked = (grid_x.floor() as usize, grid_y.floor() as usize);
    if app.settings.active_level_index >= app.levels.len() {
        return None;
    }
    let layout = &app.levels[app.settings.active_level_index];

    layout
        .start_markers
        .iter()
        .enumerate()
        .find(|(_, marker)| marker_contains_cell(Some(marker), clicked))
        .map(|(idx, _)| HoverMarker::Start(idx))
        .or_else(|| {
            layout
                .end_markers
                .iter()
                .enumerate()
                .find(|(_, marker)| marker_contains_cell(Some(marker), clicked))
                .map(|(idx, _)| HoverMarker::End(idx))
        })
        .or_else(|| {
            layout
                .trap_markers
                .iter()
                .enumerate()
                .find(|(_, marker)| marker_contains_cell(Some(marker), clicked))
                .map(|(idx, _)| HoverMarker::Trap(idx))
        })
        .or_else(|| {
            layout
                .monster_markers
                .iter()
                .enumerate()
                .find(|(_, marker)| marker_contains_cell(Some(marker), clicked))
                .map(|(idx, _)| HoverMarker::Monster(idx))
        })
}

pub fn marker_drag_at_pointer(
    app: &DungeonApp,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) -> Option<MarkerDragState> {
    let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;
    if app.settings.active_level_index >= app.levels.len() {
        return None;
    }
    let layout = &app.levels[app.settings.active_level_index];

    match marker_at_pointer(app, pointer_pos, geometry)? {
        HoverMarker::Start(marker_idx) => {
            let marker = layout.start_markers.get(marker_idx)?;
            Some(MarkerDragState {
                kind: MarkerKind::Start,
                marker_idx,
                offset_x: grid_x - marker.cell.0 as f32,
                offset_y: grid_y - marker.cell.1 as f32,
            })
        }
        HoverMarker::End(marker_idx) => {
            let marker = layout.end_markers.get(marker_idx)?;
            Some(MarkerDragState {
                kind: MarkerKind::End,
                marker_idx,
                offset_x: grid_x - marker.cell.0 as f32,
                offset_y: grid_y - marker.cell.1 as f32,
            })
        }
        HoverMarker::Trap(marker_idx) => {
            let marker = layout.trap_markers.get(marker_idx)?;
            Some(MarkerDragState {
                kind: MarkerKind::Trap,
                marker_idx,
                offset_x: grid_x - marker.cell.0 as f32,
                offset_y: grid_y - marker.cell.1 as f32,
            })
        }
        HoverMarker::Monster(marker_idx) => {
            let marker = layout.monster_markers.get(marker_idx)?;
            Some(MarkerDragState {
                kind: MarkerKind::Monster,
                marker_idx,
                offset_x: grid_x - marker.cell.0 as f32,
                offset_y: grid_y - marker.cell.1 as f32,
            })
        }
    }
}

pub fn stair_at_pointer(
    app: &DungeonApp,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) -> Option<usize> {
    let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;
    let clicked = (grid_x.floor() as usize, grid_y.floor() as usize);
    if app.settings.active_level_index >= app.levels.len() {
        return None;
    }
    let layout = &app.levels[app.settings.active_level_index];

    layout.stairs.iter().position(|stair| {
        clicked.0 >= stair.cell.0
            && clicked.0 < stair.cell.0 + stair.width
            && clicked.1 >= stair.cell.1
            && clicked.1 < stair.cell.1 + stair.height
    })
}

pub fn stair_drag_at_pointer(
    app: &DungeonApp,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) -> Option<StaircaseDragState> {
    let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;
    if app.settings.active_level_index >= app.levels.len() {
        return None;
    }
    let stair_idx = stair_at_pointer(app, pointer_pos, geometry)?;
    let layout = &app.levels[app.settings.active_level_index];
    let stair = layout.stairs.get(stair_idx)?;

    Some(StaircaseDragState {
        stair_idx,
        offset_x: grid_x - stair.cell.0 as f32,
        offset_y: grid_y - stair.cell.1 as f32,
        original_cell: stair.cell,
    })
}

pub fn start_text_resize(app: &DungeonApp, text_idx: usize) -> Option<TextResizeState> {
    if app.settings.active_level_index >= app.levels.len() {
        return None;
    }
    let layout = &app.levels[app.settings.active_level_index];

    let label = layout.text_labels.get(text_idx)?;
    Some(TextResizeState {
        text_idx,
        origin_cell: label.cell,
        original_font_size: label.font_size,
    })
}

pub fn resize_room_at_pointer(
    app: &DungeonApp,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) -> Option<usize> {
    if let Some((room_idx, _, _)) = room_at_pointer(app, pointer_pos, geometry) {
        return Some(room_idx);
    }

    let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;
    let clicked = (
        grid_x.floor().max(0.0) as usize,
        grid_y.floor().max(0.0) as usize,
    );

    if app.settings.active_level_index >= app.levels.len() {
        return None;
    }
    let layout = &app.levels[app.settings.active_level_index];

    for (room_idx, room) in layout.rooms.iter().enumerate().rev() {
        if resize_hit_cells(room, room_idx, &layout.rooms, geometry.cols, geometry.rows)
            .contains(&clicked)
        {
            return Some(room_idx);
        }
    }

    None
}

pub fn start_resize(
    app: &DungeonApp,
    room_idx: usize,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) -> Option<RoomResizeState> {
    if app.settings.active_level_index >= app.levels.len() {
        return None;
    }
    let layout = &app.levels[app.settings.active_level_index];

    let original_room = layout.rooms.get(room_idx)?.clone();
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

pub fn start_staircase_resize(
    app: &DungeonApp,
    stair_idx: usize,
    pointer_pos: egui::Pos2,
    geometry: &GridGeometry,
) -> Option<StaircaseResizeState> {
    if app.settings.active_level_index >= app.levels.len() {
        return None;
    }
    let layout = &app.levels[app.settings.active_level_index];

    let original_stair = layout.stairs.get(stair_idx)?.clone();
    let (grid_x, grid_y) = pointer_to_grid(pointer_pos, geometry)?;
    let center_x = original_stair.cell.0 as f32 + original_stair.width as f32 * 0.5;
    let center_y = original_stair.cell.1 as f32 + original_stair.height as f32 * 0.5;
    let pointer_x = grid_x;
    let pointer_y = grid_y;

    let resize_corner = match (pointer_x >= center_x, pointer_y >= center_y) {
        (false, false) => ResizeCorner::TopLeft,
        (true, false) => ResizeCorner::TopRight,
        (false, true) => ResizeCorner::BottomLeft,
        (true, true) => ResizeCorner::BottomRight,
    };

    Some(StaircaseResizeState {
        stair_idx,
        original_stair,
        resize_corner,
    })
}

pub fn resize_room(app: &mut DungeonApp, state: &RoomResizeState, target_cell: (usize, usize)) {
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let layout = &mut app.levels[app.settings.active_level_index];

    let cols = app.settings.cols.max(1);
    let rows = app.settings.rows.max(1);

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

    if layout
        .rooms
        .iter()
        .enumerate()
        .any(|(idx, room)| idx != state.room_idx && rooms_overlap(&new_room, room))
    {
        return;
    }

    if let Some(room) = layout.rooms.get_mut(state.room_idx) {
        room.x = new_room.x;
        room.y = new_room.y;
        room.width = new_room.width;
        room.height = new_room.height;
        reroute_corridors_for_room(app, state.room_idx);
        app.refresh_doors();
    }
}

pub fn resize_text(app: &mut DungeonApp, state: &TextResizeState, target_cell: (usize, usize)) {
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let layout = &mut app.levels[app.settings.active_level_index];

    let Some(label) = layout.text_labels.get_mut(state.text_idx) else {
        app.resize_state = None;
        return;
    };

    let dx = target_cell.0 as isize - state.origin_cell.0 as isize;
    let dy = target_cell.1 as isize - state.origin_cell.1 as isize;
    let delta_cells = if dx.abs() >= dy.abs() { dx } else { -dy };
    let new_size = (state.original_font_size as isize + (delta_cells * 4)).clamp(8, 96);
    label.font_size = new_size as u16;
}

pub fn resize_staircase(
    app: &mut DungeonApp,
    state: &StaircaseResizeState,
    target_cell: (usize, usize),
) {
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }

    let cols = app.settings.cols.max(1);
    let rows = app.settings.rows.max(1);

    let tx = target_cell.0.min(cols - 1) as isize;
    let ty = target_cell.1.min(rows - 1) as isize;
    let original = &state.original_stair;

    let left = original.cell.0 as isize;
    let top = original.cell.1 as isize;
    let right = (original.cell.0 + original.width - 1) as isize;
    let bottom = (original.cell.1 + original.height - 1) as isize;

    let (x0, x1, y0, y1) = match state.resize_corner {
        ResizeCorner::TopLeft => (tx.min(right), right, ty.min(bottom), bottom),
        ResizeCorner::TopRight => (left, tx.max(left), ty.min(bottom), bottom),
        ResizeCorner::BottomLeft => (tx.min(right), right, top, ty.max(top)),
        ResizeCorner::BottomRight => (left, tx.max(left), top, ty.max(top)),
    };

    let new_stair_cell = (x0 as usize, y0 as usize);
    let new_stair_width = (x1 - x0 + 1) as usize;
    let new_stair_height = (y1 - y0 + 1) as usize;

    if new_stair_width == 0 || new_stair_height == 0 {
        return;
    }

    let active_layout = &app.levels[app.settings.active_level_index];
    if active_layout.stairs.iter().enumerate().any(|(idx, stair)| {
        idx != state.stair_idx
            && rects_overlap(
                new_stair_cell.0,
                new_stair_cell.1,
                new_stair_width,
                new_stair_height,
                stair.cell.0,
                stair.cell.1,
                stair.width,
                stair.height,
            )
    }) {
        return;
    }

    if app.settings.sync_stairs_across_levels {
        for layout in &mut app.levels {
            for stair in &mut layout.stairs {
                if stair.cell == state.original_stair.cell {
                    stair.cell = new_stair_cell;
                    stair.width = new_stair_width;
                    stair.height = new_stair_height;
                }
            }
        }
    } else {
        let current_idx = app.settings.active_level_index;
        let level_indices = vec![
            Some(current_idx),
            current_idx.checked_sub(1),
            if current_idx + 1 < app.levels.len() {
                Some(current_idx + 1)
            } else {
                None
            },
        ];

        for idx in level_indices.into_iter().flatten() {
            let layout = &mut app.levels[idx];
            for stair in &mut layout.stairs {
                if stair.cell == state.original_stair.cell {
                    stair.cell = new_stair_cell;
                    stair.width = new_stair_width;
                    stair.height = new_stair_height;
                }
            }
        }
    }

    if let Some(ResizeState::Staircase(ref mut s)) = app.resize_state {
        s.original_stair.cell = new_stair_cell;
        s.original_stair.width = new_stair_width;
        s.original_stair.height = new_stair_height;
    }
}

pub fn draw_add_overlay(app: &DungeonApp, painter: &egui::Painter, geometry: &GridGeometry) {
    if app.settings.add_tool != AddTool::Corridor {
        return;
    }
    let Some(drag) = &app.add_corridor_drag else {
        return;
    };

    let start = cell_center(geometry, drag.start_cell.0, drag.start_cell.1);
    let end = cell_center(geometry, drag.current_cell.0, drag.current_cell.1);
    let stroke = Stroke::new(2.0, Color32::WHITE);
    painter.circle_filled(start, 3.0, Color32::WHITE);
    crate::rendering::draw_dashed_line(painter, start, end, stroke, 8.0, 6.0);
}

pub fn draw_add_tool_ghost(app: &DungeonApp, painter: &egui::Painter, geometry: &GridGeometry) {
    let tool = app.settings.add_tool;
    if tool == AddTool::None || tool == AddTool::Corridor {
        return;
    }

    let Some(pointer_pos) = painter.ctx().pointer_hover_pos() else {
        return;
    };
    let Some((grid_x, grid_y)) = pointer_to_grid(pointer_pos, geometry) else {
        return;
    };
    let cell = (grid_x.floor() as usize, grid_y.floor() as usize);

    let ghost_alpha = 0.35;

    match tool {
        AddTool::Room => {
            let width = app.settings.add_room_width.max(1);
            let height = app.settings.add_room_height.max(1);
            let max_x = app.settings.cols.saturating_sub(cell.0);
            let max_y = app.settings.rows.saturating_sub(cell.1);
            let w = width.min(max_x).max(1);
            let h = height.min(max_y).max(1);

            let left = geometry.rect.left() + cell.0 as f32 * geometry.cell_size;
            let top = geometry.rect.top() + cell.1 as f32 * geometry.cell_size;
            let right = left + w as f32 * geometry.cell_size;
            let bottom = top + h as f32 * geometry.cell_size;
            let rect = egui::Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, bottom));

            let fill = Color32::from_rgb(70, 120, 160).gamma_multiply(ghost_alpha);
            painter.rect_filled(rect, 4.0, fill);
            painter.rect_stroke(
                rect,
                4.0,
                Stroke::new(
                    1.5,
                    Color32::from_rgb(150, 200, 240).gamma_multiply(ghost_alpha + 0.3),
                ),
                egui::StrokeKind::Middle,
            );
        }
        AddTool::Text => {
            let text = app.settings.add_text_value.trim();
            if text.is_empty() {
                return;
            }
            let font_size = app.settings.add_text_font_size.max(8);
            let rect = crate::rendering::text_label_rect(
                geometry,
                &layout::TextLabel {
                    cell,
                    text: text.to_string(),
                    font_size,
                },
            );

            let fill = Color32::from_rgb(50, 50, 70).gamma_multiply(ghost_alpha);
            painter.rect_filled(rect.expand(4.0), 4.0, fill);
            painter.rect_stroke(
                rect.expand(4.0),
                4.0,
                Stroke::new(
                    1.0,
                    Color32::from_rgb(180, 180, 220).gamma_multiply(ghost_alpha + 0.2),
                ),
                egui::StrokeKind::Middle,
            );

            let color = Color32::WHITE.gamma_multiply(ghost_alpha + 0.1);
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::proportional(font_size as f32),
                color,
            );
        }
        AddTool::StartMarker
        | AddTool::EndMarker
        | AddTool::TrapMarker
        | AddTool::MonsterMarker => {
            let size = app.settings.add_marker_size.max(1);
            let origin = centered_marker_origin(cell, size, app.settings.cols, app.settings.rows);

            let rect = marker_rect(&layout::AreaMarker { cell: origin, size }, geometry);

            let base_color = match tool {
                AddTool::StartMarker => Color32::from_rgb(60, 220, 200),
                AddTool::EndMarker => Color32::from_rgb(240, 90, 90),
                AddTool::TrapMarker => Color32::from_rgb(150, 80, 230),
                AddTool::MonsterMarker => Color32::from_rgb(200, 50, 50),
                _ => Color32::WHITE,
            };

            let fill = base_color.gamma_multiply(ghost_alpha);
            painter.rect_filled(rect, 4.0, fill);
            painter.rect_stroke(
                rect,
                4.0,
                Stroke::new(
                    1.5,
                    Color32::from_rgb(255, 255, 255).gamma_multiply(ghost_alpha + 0.2),
                ),
                egui::StrokeKind::Middle,
            );

            let label = match tool {
                AddTool::StartMarker => "S",
                AddTool::EndMarker => "E",
                AddTool::TrapMarker => "T",
                AddTool::MonsterMarker => "M",
                _ => "?",
            };
            let text_color = Color32::WHITE.gamma_multiply(ghost_alpha + 0.15);
            let font_size = (geometry.cell_size * size as f32 * 0.5).clamp(14.0, 72.0);
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(font_size),
                text_color,
            );
        }
        AddTool::Staircase => {
            let width = app.settings.add_stair_width.max(1);
            let height = app.settings.add_stair_height.max(1);
            let max_x = app.settings.cols.saturating_sub(cell.0);
            let max_y = app.settings.rows.saturating_sub(cell.1);
            let w = width.min(max_x).max(1);
            let h = height.min(max_y).max(1);

            let left = geometry.rect.left() + cell.0 as f32 * geometry.cell_size;
            let top = geometry.rect.top() + cell.1 as f32 * geometry.cell_size;
            let right = left + w as f32 * geometry.cell_size;
            let bottom = top + h as f32 * geometry.cell_size;
            let rect = egui::Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, bottom));

            let fill = Color32::from_rgb(200, 150, 50).gamma_multiply(ghost_alpha);
            painter.rect_filled(rect, 4.0, fill);
            painter.rect_stroke(
                rect,
                4.0,
                Stroke::new(
                    1.5,
                    Color32::from_rgb(220, 180, 80).gamma_multiply(ghost_alpha + 0.2),
                ),
                egui::StrokeKind::Middle,
            );

            let steps = (h as f32 * 2.0).round() as usize;
            let step_height = (rect.height() / (steps as f32)).max(1.0);
            let line_color = Color32::from_rgb(150, 100, 30).gamma_multiply(ghost_alpha + 0.15);
            for i in 0..steps {
                let y = rect.top() + (i as f32 * step_height) + step_height * 0.5;
                painter.line_segment(
                    [
                        egui::pos2(rect.left() + 2.0, y),
                        egui::pos2(rect.right() - 2.0, y),
                    ],
                    Stroke::new(1.5, line_color),
                );
            }
        }
        AddTool::Archway | AddTool::Door | AddTool::LockedDoor | AddTool::SecretDoor => {
            if let Some((edge_a, edge_b)) = door_edge_at_pointer(app, pointer_pos, geometry) {
                let door_width_cells = 1;
                let style = match tool {
                    AddTool::Archway => crate::rendering::DoorLineStyle::Dotted,
                    AddTool::Door => crate::rendering::DoorLineStyle::LongDash,
                    AddTool::LockedDoor => crate::rendering::DoorLineStyle::ShortDash,
                    AddTool::SecretDoor => crate::rendering::DoorLineStyle::DashDotDot,
                    _ => crate::rendering::DoorLineStyle::Solid,
                };
                let color = match tool {
                    AddTool::Archway => Color32::from_rgb(230, 140, 60),
                    AddTool::Door => Color32::from_rgb(80, 200, 120),
                    AddTool::LockedDoor => Color32::from_rgb(220, 70, 70),
                    AddTool::SecretDoor => Color32::from_rgb(170, 80, 170),
                    _ => Color32::WHITE,
                };
                let ghost_color = color.gamma_multiply(ghost_alpha);
                crate::rendering::draw_door_line(
                    painter,
                    geometry,
                    edge_a,
                    edge_b,
                    door_width_cells,
                    Stroke::new((geometry.cell_size / 5.0).max(1.0), ghost_color),
                    style,
                );
            }
        }
        AddTool::None | AddTool::Corridor => {}
    }
}

pub fn draw_resize_overlay(app: &DungeonApp, painter: &egui::Painter, geometry: &GridGeometry) {
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let layout = &app.levels[app.settings.active_level_index];

    if let Some(state) = &app.resize_state {
        match state {
            ResizeState::Text(state) => {
                if let Some(label) = layout.text_labels.get(state.text_idx) {
                    let rect = crate::rendering::text_label_rect(geometry, label).expand(3.0);
                    painter.rect_stroke(
                        rect,
                        0.0,
                        Stroke::new(2.0, Color32::from_gray(180)),
                        egui::StrokeKind::Middle,
                    );
                }
            }
            ResizeState::Room(state) => {
                if let Some(room) = layout.rooms.get(state.room_idx) {
                    draw_room_resize_visuals(painter, geometry, room, true);
                }
            }
            ResizeState::Staircase(state) => {
                if let Some(stair) = layout.stairs.get(state.stair_idx) {
                    draw_staircase_resize_visuals(painter, geometry, stair, true);
                }
            }
        }
        return;
    }

    if let Some(text_idx) = app.hover_text_idx {
        if let Some(label) = layout.text_labels.get(text_idx) {
            let rect = crate::rendering::text_label_rect(geometry, label).expand(3.0);
            painter.rect_stroke(
                rect,
                0.0,
                Stroke::new(2.0, Color32::from_gray(180)),
                egui::StrokeKind::Middle,
            );
        }
    } else if let Some(room_idx) = app.hover_room_idx {
        if let Some(room) = layout.rooms.get(room_idx) {
            draw_room_resize_visuals(painter, geometry, room, false);
        }
    } else if let Some(stair_idx) = app.hover_stair_idx {
        if let Some(stair) = layout.stairs.get(stair_idx) {
            draw_staircase_resize_visuals(painter, geometry, stair, false);
        }
    }
}

pub fn draw_room_resize_visuals(
    painter: &egui::Painter,
    geometry: &GridGeometry,
    room: &layout::Room,
    show_handles: bool,
) {
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

    crate::rendering::draw_dashed_line(painter, tl, tr, stroke, 6.0, 6.0);
    crate::rendering::draw_dashed_line(painter, tr, br, stroke, 6.0, 6.0);
    crate::rendering::draw_dashed_line(painter, br, bl, stroke, 6.0, 6.0);
    crate::rendering::draw_dashed_line(painter, bl, tl, stroke, 6.0, 6.0);

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

    if show_handles {
        draw_resize_handle(painter, tl, ResizeCorner::TopLeft, color);
        draw_resize_handle(painter, tr, ResizeCorner::TopRight, color);
        draw_resize_handle(painter, br, ResizeCorner::BottomRight, color);
        draw_resize_handle(painter, bl, ResizeCorner::BottomLeft, color);
    }
}

pub fn draw_staircase_resize_visuals(
    painter: &egui::Painter,
    geometry: &GridGeometry,
    stair: &layout::Staircase,
    show_handles: bool,
) {
    let color = Color32::from_gray(170);
    let stroke = Stroke::new(1.5, color);

    let left = geometry.rect.left() + stair.cell.0 as f32 * geometry.cell_size;
    let top = geometry.rect.top() + stair.cell.1 as f32 * geometry.cell_size;
    let right = left + stair.width as f32 * geometry.cell_size;
    let bottom = top + stair.height as f32 * geometry.cell_size;

    let tl = egui::pos2(left, top);
    let tr = egui::pos2(right, top);
    let br = egui::pos2(right, bottom);
    let bl = egui::pos2(left, bottom);

    crate::rendering::draw_dashed_line(painter, tl, tr, stroke, 6.0, 6.0);
    crate::rendering::draw_dashed_line(painter, tr, br, stroke, 6.0, 6.0);
    crate::rendering::draw_dashed_line(painter, br, bl, stroke, 6.0, 6.0);
    crate::rendering::draw_dashed_line(painter, bl, tl, stroke, 6.0, 6.0);

    if show_handles {
        draw_resize_handle(painter, tl, ResizeCorner::TopLeft, color);
        draw_resize_handle(painter, tr, ResizeCorner::TopRight, color);
        draw_resize_handle(painter, br, ResizeCorner::BottomRight, color);
        draw_resize_handle(painter, bl, ResizeCorner::BottomLeft, color);
    }
}

pub fn draw_corridor_hover_overlay(
    app: &DungeonApp,
    painter: &egui::Painter,
    geometry: &GridGeometry,
) {
    let Some(idx) = app.hover_corridor_idx else {
        return;
    };
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let layout = &app.levels[app.settings.active_level_index];

    let Some(corridor) = layout.corridors.get(idx) else {
        return;
    };

    let color = Color32::from_gray(170);
    let stroke = Stroke::new(2.0, color);
    for pair in corridor.path.windows(2) {
        let a = cell_center(geometry, pair[0].0, pair[0].1);
        let b = cell_center(geometry, pair[1].0, pair[1].1);
        crate::rendering::draw_dashed_line(painter, a, b, stroke, 6.0, 6.0);
    }
}

pub fn draw_door_hover_overlay(app: &DungeonApp, painter: &egui::Painter, geometry: &GridGeometry) {
    let Some(idx) = app.hover_door_idx else {
        return;
    };
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let layout = &app.levels[app.settings.active_level_index];

    let Some(door) = layout.doors.get(idx) else {
        return;
    };
    let color = Color32::from_gray(200);
    let stroke = Stroke::new(2.0, color);
    crate::rendering::draw_door_line(
        painter,
        geometry,
        door.from,
        door.to,
        door_render_width(door),
        stroke,
        crate::rendering::DoorLineStyle::LongDash,
    );
}

pub fn pointer_to_grid(pointer_pos: egui::Pos2, geometry: &GridGeometry) -> Option<(f32, f32)> {
    if !geometry.rect.contains(pointer_pos) {
        return None;
    }

    let x = (pointer_pos.x - geometry.rect.left()) / geometry.cell_size;
    let y = (pointer_pos.y - geometry.rect.top()) / geometry.cell_size;
    Some((x, y))
}

pub fn reroute_path_through_cell(
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

pub fn simplify_path_loops(path: &mut Vec<(usize, usize)>) {
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

pub fn manual_door_edge_allowed(
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

pub fn resize_hit_cells(
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

pub fn draw_resize_handle(
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

pub fn door_edges_for(
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

pub fn door_render_width(door: &layout::Door) -> usize {
    if door.span_width {
        door.width.max(1)
    } else {
        1
    }
}

pub fn window_render_width(window: &layout::Window) -> usize {
    if window.span_width {
        window.width.max(1)
    } else {
        1
    }
}

pub fn normalized_edge(a: (usize, usize), b: (usize, usize)) -> ((usize, usize), (usize, usize)) {
    if a <= b { (a, b) } else { (b, a) }
}

pub fn cell_center(geometry: &GridGeometry, col: usize, row: usize) -> egui::Pos2 {
    egui::pos2(
        geometry.rect.left() + (col as f32 + 0.5) * geometry.cell_size,
        geometry.rect.top() + (row as f32 + 0.5) * geometry.cell_size,
    )
}

pub fn cell_rect(geometry: &GridGeometry, col: usize, row: usize) -> egui::Rect {
    let left = geometry.rect.left() + col as f32 * geometry.cell_size;
    let top = geometry.rect.top() + row as f32 * geometry.cell_size;
    egui::Rect::from_min_size(
        egui::pos2(left, top),
        egui::vec2(geometry.cell_size, geometry.cell_size),
    )
}

pub fn marker_rect(marker: &layout::AreaMarker, geometry: &GridGeometry) -> egui::Rect {
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

pub fn marker_contains_cell(marker: Option<&layout::AreaMarker>, cell: (usize, usize)) -> bool {
    let Some(marker) = marker else {
        return false;
    };
    cell.0 >= marker.cell.0
        && cell.0 < marker.cell.0 + marker.size
        && cell.1 >= marker.cell.1
        && cell.1 < marker.cell.1 + marker.size
}

pub fn centered_marker_origin(
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
