/*
 * Multi-selection interaction logic.
 *
 * Rubber-band selection: left-click drag on empty space draws a selection box.
 * Multi-move:  left-click drag when items are already selected (and pointer is not
 *              on an empty space) moves all selected items together.
 * Multi-resize: right-click drag when items are selected resizes all of them together.
 */

use super::{
    GridGeometry, MarkerKind, MultiDragState, MultiResizeState, SelectedItem, SelectionBoxDrag,
    corridor_drag_at_pointer, marker_at_pointer, pointer_to_grid, room_at_pointer,
    stair_at_pointer, text_at_pointer,
};
use crate::app::DungeonApp;
use crate::interact::reroute_corridors_for_room;
use crate::ui::AddTool;
use eframe::egui;
use egui::{Color32, Pos2, Rect, Stroke, pos2};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns true when the pointer is over any interactive element.
pub fn pointer_on_item(app: &DungeonApp, pos: Pos2, geometry: &GridGeometry) -> bool {
    text_at_pointer(app, pos, geometry).is_some()
        || marker_at_pointer(app, pos, geometry).is_some()
        || stair_at_pointer(app, pos, geometry).is_some()
        || room_at_pointer(app, pos, geometry).is_some()
        || corridor_drag_at_pointer(app, pos, geometry).is_some()
}

/// Returns true when the pointer is over one of the currently selected items.
pub fn pointer_on_selected_item(app: &DungeonApp, pos: Pos2, geometry: &GridGeometry) -> bool {
    if app.selection.is_empty() {
        return false;
    }
    if app.settings.active_level_index >= app.levels.len() {
        return false;
    }
    let Some((grid_x, grid_y)) = pointer_to_grid(pos, geometry) else {
        return false;
    };
    let cell = (grid_x.floor() as usize, grid_y.floor() as usize);
    let layout = &app.levels[app.settings.active_level_index];

    for item in &app.selection {
        match item {
            SelectedItem::Room(idx) => {
                if let Some(r) = layout.rooms.get(*idx) {
                    if cell.0 >= r.x
                        && cell.0 < r.x + r.width
                        && cell.1 >= r.y
                        && cell.1 < r.y + r.height
                    {
                        return true;
                    }
                }
            }
            SelectedItem::Text(idx) => {
                if let Some(l) = layout.text_labels.get(*idx) {
                    if l.cell == cell {
                        return true;
                    }
                }
            }
            SelectedItem::Staircase(idx) => {
                if let Some(s) = layout.stairs.get(*idx) {
                    if cell.0 >= s.cell.0
                        && cell.0 < s.cell.0 + s.width
                        && cell.1 >= s.cell.1
                        && cell.1 < s.cell.1 + s.height
                    {
                        return true;
                    }
                }
            }
            SelectedItem::Marker(kind, idx) => {
                let markers = match kind {
                    MarkerKind::Start => &layout.start_markers,
                    MarkerKind::End => &layout.end_markers,
                    MarkerKind::Trap => &layout.trap_markers,
                    MarkerKind::Monster => &layout.monster_markers,
                };
                if let Some(m) = markers.get(*idx) {
                    if cell.0 >= m.cell.0
                        && cell.0 < m.cell.0 + m.size
                        && cell.1 >= m.cell.1
                        && cell.1 < m.cell.1 + m.size
                    {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Returns the top-left grid cell of a selected item.
fn item_origin(app: &DungeonApp, item: &SelectedItem) -> Option<(isize, isize)> {
    if app.settings.active_level_index >= app.levels.len() {
        return None;
    }
    let layout = &app.levels[app.settings.active_level_index];
    match item {
        SelectedItem::Room(idx) => layout
            .rooms
            .get(*idx)
            .map(|r| (r.x as isize, r.y as isize)),
        SelectedItem::Text(idx) => layout
            .text_labels
            .get(*idx)
            .map(|l| (l.cell.0 as isize, l.cell.1 as isize)),
        SelectedItem::Staircase(idx) => layout
            .stairs
            .get(*idx)
            .map(|s| (s.cell.0 as isize, s.cell.1 as isize)),
        SelectedItem::Marker(kind, idx) => {
            let markers = match kind {
                MarkerKind::Start => &layout.start_markers,
                MarkerKind::End => &layout.end_markers,
                MarkerKind::Trap => &layout.trap_markers,
                MarkerKind::Monster => &layout.monster_markers,
            };
            markers.get(*idx).map(|m| (m.cell.0 as isize, m.cell.1 as isize))
        }
    }
}

/// Returns (x, y, width, height) for resize origins. For text, width = font_size as isize, height = 0.
fn item_size_origin(app: &DungeonApp, item: &SelectedItem) -> Option<(isize, isize, isize, isize)> {
    if app.settings.active_level_index >= app.levels.len() {
        return None;
    }
    let layout = &app.levels[app.settings.active_level_index];
    match item {
        SelectedItem::Room(idx) => layout.rooms.get(*idx).map(|r| {
            (r.x as isize, r.y as isize, r.width as isize, r.height as isize)
        }),
        SelectedItem::Staircase(idx) => layout.stairs.get(*idx).map(|s| {
            (s.cell.0 as isize, s.cell.1 as isize, s.width as isize, s.height as isize)
        }),
        SelectedItem::Text(idx) => layout.text_labels.get(*idx).map(|l| {
            (l.cell.0 as isize, l.cell.1 as isize, l.font_size as isize, 0)
        }),
        SelectedItem::Marker(kind, idx) => {
            let markers = match kind {
                MarkerKind::Start => &layout.start_markers,
                MarkerKind::End => &layout.end_markers,
                MarkerKind::Trap => &layout.trap_markers,
                MarkerKind::Monster => &layout.monster_markers,
            };
            markers.get(*idx).map(|m| {
                (m.cell.0 as isize, m.cell.1 as isize, m.size as isize, m.size as isize)
            })
        }
    }
}

// ---------------------------------------------------------------------------
// Selection box
// ---------------------------------------------------------------------------

/// Start a rubber-band selection drag.
pub fn start_selection_box(app: &mut DungeonApp, pos: Pos2, geometry: &GridGeometry) {
    let Some((gx, gy)) = pointer_to_grid(pos, geometry) else { return };
    app.selection.clear();
    app.selection_box_drag = Some(SelectionBoxDrag {
        start_cell: (gx, gy),
        current_cell: (gx, gy),
    });
}

/// Update the rubber-band selection rectangle and compute the new selection set.
pub fn update_selection_box(app: &mut DungeonApp, pos: Pos2, geometry: &GridGeometry) {
    let Some((gx, gy)) = pointer_to_grid(pos, geometry) else { return };
    if let Some(ref mut drag) = app.selection_box_drag {
        drag.current_cell = (gx, gy);
    } else {
        return;
    }

    if app.settings.active_level_index >= app.levels.len() {
        return;
    }

    let (sx, sy, cx, cy) = {
        let drag = app.selection_box_drag.as_ref().unwrap();
        (drag.start_cell.0, drag.start_cell.1, drag.current_cell.0, drag.current_cell.1)
    };

    let min_x = sx.min(cx).floor() as usize;
    let min_y = sy.min(cy).floor() as usize;
    let max_x = sx.max(cx).ceil() as usize;
    let max_y = sy.max(cy).ceil() as usize;

    let layout = &app.levels[app.settings.active_level_index];
    let mut selection = Vec::new();

    for (idx, room) in layout.rooms.iter().enumerate() {
        // Room overlaps selection box if they intersect.
        if room.x < max_x
            && room.x + room.width > min_x
            && room.y < max_y
            && room.y + room.height > min_y
        {
            selection.push(SelectedItem::Room(idx));
        }
    }

    for (idx, label) in layout.text_labels.iter().enumerate() {
        if label.cell.0 >= min_x
            && label.cell.0 < max_x
            && label.cell.1 >= min_y
            && label.cell.1 < max_y
        {
            selection.push(SelectedItem::Text(idx));
        }
    }

    for (idx, stair) in layout.stairs.iter().enumerate() {
        if stair.cell.0 < max_x
            && stair.cell.0 + stair.width > min_x
            && stair.cell.1 < max_y
            && stair.cell.1 + stair.height > min_y
        {
            selection.push(SelectedItem::Staircase(idx));
        }
    }

    let add_markers = |markers: &Vec<crate::layout::AreaMarker>,
                       kind: MarkerKind,
                       selection: &mut Vec<SelectedItem>| {
        for (idx, m) in markers.iter().enumerate() {
            if m.cell.0 < max_x
                && m.cell.0 + m.size > min_x
                && m.cell.1 < max_y
                && m.cell.1 + m.size > min_y
            {
                selection.push(SelectedItem::Marker(kind, idx));
            }
        }
    };

    add_markers(&layout.start_markers.clone(), MarkerKind::Start, &mut selection);
    add_markers(&layout.end_markers.clone(), MarkerKind::End, &mut selection);
    add_markers(&layout.trap_markers.clone(), MarkerKind::Trap, &mut selection);
    add_markers(&layout.monster_markers.clone(), MarkerKind::Monster, &mut selection);

    app.selection = selection;
}

/// Finish the rubber-band drag (keep current selection).
pub fn finish_selection_box(app: &mut DungeonApp) {
    app.selection_box_drag = None;
    // If nothing ended up selected, clear to stay clean.
    // (keep the selection so the user can immediately use it)
}

// ---------------------------------------------------------------------------
// Multi-drag (left button move)
// ---------------------------------------------------------------------------

/// Start a multi-item drag.
pub fn start_multi_drag(app: &mut DungeonApp, pos: Pos2, geometry: &GridGeometry) {
    let Some((gx, gy)) = pointer_to_grid(pos, geometry) else { return };

    let origins: Vec<(SelectedItem, (isize, isize))> = app
        .selection
        .iter()
        .filter_map(|item| {
            let origin = item_origin(app, item)?;
            Some((item.clone(), origin))
        })
        .collect();

    if origins.is_empty() {
        return;
    }

    app.push_undo_snapshot();
    app.multi_drag_state = Some(MultiDragState {
        start_grid: (gx, gy),
        origins,
    });
}

/// Apply multi-item movement based on current pointer position.
pub fn update_multi_drag(app: &mut DungeonApp, pos: Pos2, geometry: &GridGeometry) {
    let Some((gx, gy)) = pointer_to_grid(pos, geometry) else { return };
    let Some(state) = app.multi_drag_state.clone() else { return };

    let dx = (gx - state.start_grid.0).round() as isize;
    let dy = (gy - state.start_grid.1).round() as isize;

    if app.settings.active_level_index >= app.levels.len() {
        return;
    }

    let cols = app.settings.cols as isize;
    let rows = app.settings.rows as isize;

    for (item, (ox, oy)) in &state.origins {
        let new_x = (ox + dx).max(0);
        let new_y = (oy + dy).max(0);

        match item {
            SelectedItem::Room(idx) => {
                let layout = &mut app.levels[app.settings.active_level_index];
                if let Some(room) = layout.rooms.get_mut(*idx) {
                    let max_x = (cols - room.width as isize).max(0);
                    let max_y = (rows - room.height as isize).max(0);
                    room.x = new_x.min(max_x) as usize;
                    room.y = new_y.min(max_y) as usize;
                }
            }
            SelectedItem::Text(idx) => {
                let layout = &mut app.levels[app.settings.active_level_index];
                if let Some(label) = layout.text_labels.get_mut(*idx) {
                    label.cell = (
                        new_x.min(cols - 1).max(0) as usize,
                        new_y.min(rows - 1).max(0) as usize,
                    );
                }
            }
            SelectedItem::Staircase(idx) => {
                let layout = &mut app.levels[app.settings.active_level_index];
                if let Some(stair) = layout.stairs.get_mut(*idx) {
                    let max_x = (cols - stair.width as isize).max(0);
                    let max_y = (rows - stair.height as isize).max(0);
                    stair.cell = (new_x.min(max_x) as usize, new_y.min(max_y) as usize);
                }
            }
            SelectedItem::Marker(kind, idx) => {
                let layout = &mut app.levels[app.settings.active_level_index];
                let markers = match kind {
                    MarkerKind::Start => &mut layout.start_markers,
                    MarkerKind::End => &mut layout.end_markers,
                    MarkerKind::Trap => &mut layout.trap_markers,
                    MarkerKind::Monster => &mut layout.monster_markers,
                };
                if let Some(m) = markers.get_mut(*idx) {
                    let max_x = (cols - m.size as isize).max(0);
                    let max_y = (rows - m.size as isize).max(0);
                    m.cell = (new_x.min(max_x) as usize, new_y.min(max_y) as usize);
                }
            }
        }
    }

    // Reroute corridors for all moved rooms.
    let moved_rooms: Vec<usize> = state
        .origins
        .iter()
        .filter_map(|(item, _)| {
            if let SelectedItem::Room(idx) = item { Some(*idx) } else { None }
        })
        .collect();
    for room_idx in moved_rooms {
        reroute_corridors_for_room(app, room_idx);
    }
    app.refresh_doors();
}

/// Finish multi-drag.
pub fn finish_multi_drag(app: &mut DungeonApp) {
    app.multi_drag_state = None;
}

// ---------------------------------------------------------------------------
// Multi-resize (right button)
// ---------------------------------------------------------------------------

/// Start a multi-item resize.
pub fn start_multi_resize(app: &mut DungeonApp, pos: Pos2, geometry: &GridGeometry) {
    let Some((gx, gy)) = pointer_to_grid(pos, geometry) else { return };

    let origins: Vec<(SelectedItem, (isize, isize, isize, isize))> = app
        .selection
        .iter()
        .filter_map(|item| {
            let origin = item_size_origin(app, item)?;
            Some((item.clone(), origin))
        })
        .collect();

    if origins.is_empty() {
        return;
    }

    app.push_undo_snapshot();
    app.multi_resize_state = Some(MultiResizeState {
        start_grid: (gx, gy),
        origins,
    });
}

/// Apply multi-item resize based on current pointer position.
/// Scale factor is driven by the mouse delta: moving right/down grows items,
/// moving left/up shrinks them. The delta is applied as an integer cell offset
/// to width and height (rooms/stairs grow from bottom-right corner; text scales font).
pub fn update_multi_resize(app: &mut DungeonApp, pos: Pos2, geometry: &GridGeometry) {
    let Some((gx, gy)) = pointer_to_grid(pos, geometry) else { return };
    let Some(state) = app.multi_resize_state.clone() else { return };

    let dx = (gx - state.start_grid.0).round() as isize;
    let dy = (gy - state.start_grid.1).round() as isize;
    // Use the larger of dx/dy as a single resize delta so all items scale uniformly.
    let delta = if dx.abs() >= dy.abs() { dx } else { dy };

    if app.settings.active_level_index >= app.levels.len() {
        return;
    }

    let cols = app.settings.cols as isize;
    let rows = app.settings.rows as isize;

    for (item, (ox, oy, ow, oh)) in &state.origins {
        match item {
            SelectedItem::Room(idx) => {
                let new_w = (*ow + delta).max(1);
                let new_h = (*oh + delta).max(1);
                let layout = &mut app.levels[app.settings.active_level_index];
                if let Some(room) = layout.rooms.get_mut(*idx) {
                    if ox + new_w <= cols && oy + new_h <= rows {
                        room.width = new_w as usize;
                        room.height = new_h as usize;
                    }
                }
            }
            SelectedItem::Staircase(idx) => {
                let new_w = (*ow + delta).max(1);
                let new_h = (*oh + delta).max(1);
                let layout = &mut app.levels[app.settings.active_level_index];
                if let Some(stair) = layout.stairs.get_mut(*idx) {
                    if ox + new_w <= cols && oy + new_h <= rows {
                        stair.width = new_w as usize;
                        stair.height = new_h as usize;
                    }
                }
            }
            SelectedItem::Text(idx) => {
                // ow stores the original font_size for text labels.
                let new_size = (*ow + delta * 4).clamp(8, 96) as u16;
                let layout = &mut app.levels[app.settings.active_level_index];
                if let Some(label) = layout.text_labels.get_mut(*idx) {
                    label.font_size = new_size;
                }
            }
            SelectedItem::Marker(kind, idx) => {
                let new_size = (*ow + delta).max(1) as usize;
                let layout = &mut app.levels[app.settings.active_level_index];
                let markers = match kind {
                    MarkerKind::Start => &mut layout.start_markers,
                    MarkerKind::End => &mut layout.end_markers,
                    MarkerKind::Trap => &mut layout.trap_markers,
                    MarkerKind::Monster => &mut layout.monster_markers,
                };
                if let Some(m) = markers.get_mut(*idx) {
                    let max_x = (cols - new_size as isize).max(0);
                    let max_y = (rows - new_size as isize).max(0);
                    if *ox <= max_x && *oy <= max_y {
                        m.size = new_size;
                    }
                }
            }
        }
    }

    // Reroute corridors for resized rooms.
    let resized_rooms: Vec<usize> = state
        .origins
        .iter()
        .filter_map(|(item, _)| {
            if let SelectedItem::Room(idx) = item { Some(*idx) } else { None }
        })
        .collect();
    for room_idx in resized_rooms {
        reroute_corridors_for_room(app, room_idx);
    }
    app.refresh_doors();
}

/// Finish multi-resize.
pub fn finish_multi_resize(app: &mut DungeonApp) {
    app.multi_resize_state = None;
}

// ---------------------------------------------------------------------------
// Master handler (called from app.rs update loop)
// ---------------------------------------------------------------------------

/// Central handler for all multi-selection interactions.
/// Returns true if it consumed the input so the single-item handlers are skipped.
pub fn handle_multi_select(
    app: &mut DungeonApp,
    ctx: &egui::Context,
    response: &egui::Response,
    geometry: &GridGeometry,
) -> bool {
    if app.settings.add_tool != AddTool::None {
        return false;
    }

    // --- Right click: multi-resize ---
    if ctx.input(|i| i.pointer.secondary_pressed())
        && !app.selection.is_empty()
        && app.multi_resize_state.is_none()
    {
        if let Some(pos) = response.interact_pointer_pos() {
            start_multi_resize(app, pos, geometry);
            return true;
        }
    }

    if app.multi_resize_state.is_some() {
        if response.dragged_by(egui::PointerButton::Secondary)
            && ctx.input(|i| i.pointer.secondary_down())
        {
            if let Some(pos) = response.interact_pointer_pos() {
                update_multi_resize(app, pos, geometry);
            }
        }
        if ctx.input(|i| i.pointer.button_released(egui::PointerButton::Secondary)) {
            finish_multi_resize(app);
        }
        return true;
    }

    // --- Left click on empty space: rubber-band select ---
    if response.drag_started()
        && ctx.input(|i| i.pointer.primary_down())
        && app.multi_drag_state.is_none()
        && app.selection_box_drag.is_none()
    {
        if let Some(pos) = response.interact_pointer_pos() {
            let on_item = pointer_on_item(app, pos, geometry);
            let on_selected = pointer_on_selected_item(app, pos, geometry);

            if on_selected && !app.selection.is_empty() {
                // Start moving the selection.
                start_multi_drag(app, pos, geometry);
                return true;
            } else if !on_item {
                // Start rubber-band selection.
                start_selection_box(app, pos, geometry);
                return true;
            }
            // on an unselected item — fall through to single-item handlers.
        }
    }

    if app.selection_box_drag.is_some() {
        if response.dragged() {
            if let Some(pos) = response.interact_pointer_pos() {
                update_selection_box(app, pos, geometry);
            }
        }
        if response.drag_stopped() || !ctx.input(|i| i.pointer.primary_down()) {
            finish_selection_box(app);
        }
        return true;
    }

    if app.multi_drag_state.is_some() {
        if response.dragged() && ctx.input(|i| i.pointer.primary_down()) {
            if let Some(pos) = response.interact_pointer_pos() {
                update_multi_drag(app, pos, geometry);
            }
        }
        if response.drag_stopped() || !ctx.input(|i| i.pointer.primary_down()) {
            finish_multi_drag(app);
        }
        return true;
    }

    // Clear selection on plain click on empty space.
    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            if !pointer_on_item(app, pos, geometry) {
                app.selection.clear();
            }
        }
    }

    false
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

/// Draws the rubber-band selection box while dragging.
pub fn draw_selection_box(app: &DungeonApp, painter: &egui::Painter, geometry: &GridGeometry) {
    let Some(drag) = &app.selection_box_drag else { return };

    let to_screen = |gx: f32, gy: f32| -> Pos2 {
        pos2(
            geometry.rect.left() + gx * geometry.cell_size,
            geometry.rect.top() + gy * geometry.cell_size,
        )
    };

    let tl = to_screen(
        drag.start_cell.0.min(drag.current_cell.0),
        drag.start_cell.1.min(drag.current_cell.1),
    );
    let br = to_screen(
        drag.start_cell.0.max(drag.current_cell.0),
        drag.start_cell.1.max(drag.current_cell.1),
    );

    let rect = Rect::from_min_max(tl, br);
    painter.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(100, 160, 255, 30));
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(1.5, Color32::from_rgba_unmultiplied(100, 160, 255, 200)),
        egui::StrokeKind::Middle,
    );
}

/// Draws highlight outlines around all selected items.
pub fn draw_selection_highlights(
    app: &DungeonApp,
    painter: &egui::Painter,
    geometry: &GridGeometry,
) {
    if app.selection.is_empty() {
        return;
    }
    if app.settings.active_level_index >= app.levels.len() {
        return;
    }
    let layout = &app.levels[app.settings.active_level_index];
    let color = Color32::from_rgba_unmultiplied(100, 180, 255, 220);
    let stroke = Stroke::new(2.0, color);

    for item in &app.selection {
        match item {
            SelectedItem::Room(idx) => {
                if let Some(r) = layout.rooms.get(*idx) {
                    let left = geometry.rect.left() + r.x as f32 * geometry.cell_size;
                    let top = geometry.rect.top() + r.y as f32 * geometry.cell_size;
                    let rect = Rect::from_min_size(
                        pos2(left, top),
                        egui::vec2(
                            r.width as f32 * geometry.cell_size,
                            r.height as f32 * geometry.cell_size,
                        ),
                    );
                    painter.rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Middle);
                }
            }
            SelectedItem::Staircase(idx) => {
                if let Some(s) = layout.stairs.get(*idx) {
                    let left = geometry.rect.left() + s.cell.0 as f32 * geometry.cell_size;
                    let top = geometry.rect.top() + s.cell.1 as f32 * geometry.cell_size;
                    let rect = Rect::from_min_size(
                        pos2(left, top),
                        egui::vec2(
                            s.width as f32 * geometry.cell_size,
                            s.height as f32 * geometry.cell_size,
                        ),
                    );
                    painter.rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Middle);
                }
            }
            SelectedItem::Text(idx) => {
                if let Some(l) = layout.text_labels.get(*idx) {
                    let center = pos2(
                        geometry.rect.left() + (l.cell.0 as f32 + 0.5) * geometry.cell_size,
                        geometry.rect.top() + (l.cell.1 as f32 + 0.5) * geometry.cell_size,
                    );
                    let half = geometry.cell_size * 0.5;
                    let rect = Rect::from_center_size(center, egui::vec2(half * 2.0, half * 2.0));
                    painter.rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Middle);
                }
            }
            SelectedItem::Marker(kind, idx) => {
                let markers = match kind {
                    MarkerKind::Start => &layout.start_markers,
                    MarkerKind::End => &layout.end_markers,
                    MarkerKind::Trap => &layout.trap_markers,
                    MarkerKind::Monster => &layout.monster_markers,
                };
                if let Some(m) = markers.get(*idx) {
                    let left = geometry.rect.left() + m.cell.0 as f32 * geometry.cell_size;
                    let top = geometry.rect.top() + m.cell.1 as f32 * geometry.cell_size;
                    let rect = Rect::from_min_size(
                        pos2(left, top),
                        egui::vec2(
                            m.size as f32 * geometry.cell_size,
                            m.size as f32 * geometry.cell_size,
                        ),
                    );
                    painter.rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Middle);
                }
            }
        }
    }
}
