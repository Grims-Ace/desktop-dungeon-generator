use std::collections::HashSet;
use std::fmt::Write as _;
use std::path::PathBuf;

use image::{DynamicImage, ImageFormat, RgbaImage, imageops::FilterType};
use rayon::prelude::*;
use rfd::FileDialog;
use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Rect, Stroke, StrokeDash, Transform};

use crate::layout::{
    AreaMarker, Door, DungeonLayout, Room, Staircase, Window, WindowSide, corridor_cells,
};
use crate::ui::{ExportFormat, MaskFormat, UiSettings};

const BG: (u8, u8, u8, u8) = (24, 24, 26, 255);
const GRID: (u8, u8, u8, u8) = (130, 130, 130, 255);
const ROOM: (u8, u8, u8, u8) = (70, 120, 160, 255);
const CORRIDOR: (u8, u8, u8, u8) = (210, 190, 120, 255);
const BLACK: (u8, u8, u8, u8) = (0, 0, 0, 255);
const OPEN_DOOR: (u8, u8, u8, u8) = (80, 200, 120, 255);
const LOCKED_DOOR: (u8, u8, u8, u8) = (220, 70, 70, 255);
const SECRET_DOOR: (u8, u8, u8, u8) = (170, 80, 170, 255);
const ARCHWAY: (u8, u8, u8, u8) = (230, 140, 60, 255);
const WINDOW: (u8, u8, u8, u8) = (70, 130, 220, 255);
const WHITE: (u8, u8, u8, u8) = (255, 255, 255, 255);
const START_MARKER: (u8, u8, u8, u8) = (60, 220, 200, 255);
const END_MARKER: (u8, u8, u8, u8) = (240, 90, 90, 255);
const TRAP_MARKER: (u8, u8, u8, u8) = (150, 80, 230, 255);
const MONSTER_MARKER: (u8, u8, u8, u8) = (200, 50, 50, 255);
const STAIRS: (u8, u8, u8, u8) = (200, 150, 50, 255);

pub enum ExportTarget {
    File(PathBuf),
    Folder(PathBuf),
}

#[derive(Debug, Clone, Copy)]
enum DoorStyle {
    Solid,
    LongDash,
    ShortDash,
    Dotted,
    DashDot,
    DashDotDot,
}

#[derive(Debug, Clone, Copy)]
struct ExportGeometry {
    cols: usize,
    rows: usize,
    cell: f32,
    pad: f32,
    width: u32,
    height: u32,
}

impl ExportGeometry {
    // Build export geometry with a fixed cell size and padding.
    fn new(cols: usize, rows: usize) -> Self {
        let cell = 64.0;
        let pad = 0.0;
        let width = (cols as f32 * cell).round() as u32;
        let height = (rows as f32 * cell).round() as u32;
        Self {
            cols,
            rows,
            cell,
            pad,
            width,
            height,
        }
    }

    // Return the left padding offset.
    fn left(&self) -> f32 {
        self.pad
    }

    // Return the top padding offset.
    fn top(&self) -> f32 {
        self.pad
    }

    // Compute the rectangle for a cell in export space.
    fn cell_rect(&self, col: usize, row: usize) -> Option<Rect> {
        Rect::from_xywh(
            self.left() + col as f32 * self.cell,
            self.top() + row as f32 * self.cell,
            self.cell,
            self.cell,
        )
    }

    // Compute the center point of a cell in export space.
    fn cell_center(&self, col: usize, row: usize) -> (f32, f32) {
        (
            self.left() + (col as f32 + 0.5) * self.cell,
            self.top() + (row as f32 + 0.5) * self.cell,
        )
    }
}

#[derive(Default)]
struct SceneData {
    corridor_cells: HashSet<(usize, usize)>,
    corridor_edges: HashSet<((usize, usize), (usize, usize))>,
    room_cells: HashSet<(usize, usize)>,
    room_edges: HashSet<((usize, usize), (usize, usize))>,
    door_edges: HashSet<((usize, usize), (usize, usize))>,
}

// Collect precomputed cell and edge sets needed for rendering.
fn collect_scene_data(layout: &DungeonLayout, settings: &UiSettings) -> SceneData {
    let mut scene = SceneData::default();

    for door in &layout.doors {
        for edge in door_edges_for(door, settings.cols, settings.rows) {
            scene.door_edges.insert(edge);
        }
    }

    scene.corridor_cells = corridor_cells(layout, settings.cols, settings.rows);
    scene.corridor_edges = corridor_edges_from_cells(&scene.corridor_cells);

    for room in &layout.rooms {
        for x in room.x..(room.x + room.width) {
            for y in room.y..(room.y + room.height) {
                scene.room_cells.insert((x, y));
            }
        }
    }
    scene.room_edges = room_edges_from_rooms(&layout.rooms);

    scene
}

// Open a dialog to select an export destination.
pub fn select_export_target(settings: &UiSettings) -> Result<ExportTarget, String> {
    if settings.export_format == ExportFormat::Folder || settings.export_level_index == 999 {
        let title = if settings.export_level_index == 999 {
            "Select export folder for all levels"
        } else {
            "Export composite masks folder"
        };
        let mut dialog = FileDialog::new().set_title(title);
        if let Some(ref last_path) = settings.last_export_path {
            dialog = dialog.set_directory(last_path);
        }
        let folder = dialog
            .pick_folder()
            .ok_or_else(|| "Export canceled".to_string())?;
        return Ok(ExportTarget::Folder(folder));
    }

    let ext = settings.export_format.extension();
    let default_name = format!("dungeon_export.{ext}");

    let mut dialog = FileDialog::new();
    dialog = dialog
        .set_title("Export image")
        .set_file_name(&default_name)
        .add_filter(settings.export_format.label(), &[ext]);

    if let Some(ref last_path) = settings.last_export_path {
        dialog = dialog.set_directory(last_path);
    }

    let mut path = dialog
        .save_file()
        .ok_or_else(|| "Export canceled".to_string())?;

    if path.extension().is_none() {
        path.set_extension(ext);
    }

    Ok(ExportTarget::File(path))
}

// Export the current layout to the chosen file or folder target.
pub fn export_to_target(
    layouts: &[DungeonLayout],
    settings: &UiSettings,
    target: ExportTarget,
) -> Result<PathBuf, String> {
    let export_indices: Vec<usize> = if settings.export_level_index == 999 {
        (0..layouts.len()).collect()
    } else {
        let idx = settings
            .export_level_index
            .min(layouts.len().saturating_sub(1));
        vec![idx]
    };

    match target {
        ExportTarget::Folder(folder) => {
            for &idx in &export_indices {
                let layout = &layouts[idx];
                if settings.export_format == ExportFormat::Folder {
                    let subfolder = if settings.export_level_index == 999 {
                        folder.join(format!("level {}", idx + 1))
                    } else {
                        folder.clone()
                    };
                    export_composite_masks(layout, settings, &subfolder)?;
                } else {
                    let ext = settings.export_format.extension();
                    let filename = format!("level{}.{ext}", idx + 1);
                    let path = folder.join(filename);
                    save_layout_to_file(layout, settings, &path)?;
                }
            }
            Ok(folder)
        }
        ExportTarget::File(path) => {
            let idx = export_indices[0];
            save_layout_to_file(&layouts[idx], settings, &path)?;
            Ok(path)
        }
    }
}

fn save_layout_to_file(
    layout: &DungeonLayout,
    settings: &UiSettings,
    path: &std::path::Path,
) -> Result<(), String> {
    match settings.export_format {
        ExportFormat::Svg => {
            let svg = build_svg(layout, settings);
            std::fs::write(path, svg).map_err(|e| format!("Failed writing SVG: {e}"))?;
        }
        ExportFormat::Png | ExportFormat::Jpeg | ExportFormat::Webp => {
            let pixmap = render_pixmap(layout, settings)?;
            let img = raster_image_from_pixmap(&pixmap, settings)?;
            let format = match settings.export_format {
                ExportFormat::Png => ImageFormat::Png,
                ExportFormat::Jpeg => ImageFormat::Jpeg,
                ExportFormat::Webp => ImageFormat::WebP,
                ExportFormat::Svg | ExportFormat::Folder => unreachable!(),
            };
            img.save_with_format(path, format)
                .map_err(|e| format!("Failed writing image: {e}"))?;
        }
        ExportFormat::Folder => unreachable!(),
    }
    Ok(())
}

// Render the dungeon layout to a pixmap.
fn render_pixmap(layout: &DungeonLayout, settings: &UiSettings) -> Result<Pixmap, String> {
    let g = ExportGeometry::new(settings.cols, settings.rows);
    let scene = collect_scene_data(layout, settings);
    let mut pixmap =
        Pixmap::new(g.width, g.height).ok_or_else(|| "Failed to allocate canvas".to_string())?;
    fill_bg(&mut pixmap, BG);

    let wall_w = snap_even_width((g.cell / 2.5).max(1.0));
    let door_w = (g.cell / 10.0).max(1.0).round();

    if settings.export_show_grid {
        draw_grid(&mut pixmap, &g);
    }

    for &(x, y) in &scene.corridor_cells {
        fill_rect_cell(&mut pixmap, &g, x, y, CORRIDOR);
        if settings.colorblind_mode {
            let (cx, cy) = g.cell_center(x, y);
            draw_dot(&mut pixmap, cx, cy, g.cell * 0.14, BLACK);
        }
    }

    for room in &layout.rooms {
        fill_rect_room(&mut pixmap, &g, room, ROOM);
        if settings.colorblind_mode {
            draw_room_crosshatch(&mut pixmap, &g, room, 1.5, BLACK);
        }
    }

    draw_cell_walls(
        &mut pixmap,
        &g,
        &scene.corridor_cells,
        Some(&scene.corridor_edges),
        &scene.door_edges,
        wall_w,
        BLACK,
    );
    draw_cell_walls(
        &mut pixmap,
        &g,
        &scene.room_cells,
        Some(&scene.room_edges),
        &scene.door_edges,
        wall_w,
        BLACK,
    );

    for door in &layout.doors {
        let (color, style) = if settings.colorblind_mode {
            let s = if door.secret {
                DoorStyle::DashDotDot
            } else if door.archway {
                DoorStyle::Dotted
            } else if door.locked {
                DoorStyle::ShortDash
            } else {
                DoorStyle::LongDash
            };
            (BLACK, s)
        } else if door.secret {
            (SECRET_DOOR, DoorStyle::Solid)
        } else if door.archway {
            (ARCHWAY, DoorStyle::Solid)
        } else if door.locked {
            (LOCKED_DOOR, DoorStyle::Solid)
        } else {
            (OPEN_DOOR, DoorStyle::Solid)
        };
        draw_door(&mut pixmap, &g, door, door_w, color, style);
    }

    for window in &layout.windows {
        let style = if settings.colorblind_mode {
            DoorStyle::DashDot
        } else {
            DoorStyle::Solid
        };
        draw_window(&mut pixmap, &g, window, door_w, WINDOW, style);
    }

    draw_area_marker_group(
        &mut pixmap,
        &g,
        &layout.start_markers,
        "S",
        START_MARKER,
        settings.colorblind_mode,
    );
    draw_area_marker_group(
        &mut pixmap,
        &g,
        &layout.end_markers,
        "E",
        END_MARKER,
        settings.colorblind_mode,
    );
    draw_area_marker_group(
        &mut pixmap,
        &g,
        &layout.trap_markers,
        "T",
        TRAP_MARKER,
        settings.colorblind_mode,
    );
    draw_area_marker_group(
        &mut pixmap,
        &g,
        &layout.monster_markers,
        "M",
        MONSTER_MARKER,
        settings.colorblind_mode,
    );

    draw_staircase_group(
        &mut pixmap,
        &g,
        &layout.stairs,
        STAIRS,
        settings.colorblind_mode,
    );

    Ok(pixmap)
}

// Export composite image plus mask layers into a folder.
fn export_composite_masks(
    layout: &DungeonLayout,
    settings: &UiSettings,
    folder: &std::path::Path,
) -> Result<(), String> {
    std::fs::create_dir_all(folder).map_err(|e| format!("Failed creating folder: {e}"))?;

    let g = ExportGeometry::new(settings.cols, settings.rows);
    let scene = collect_scene_data(layout, settings);
    let wall_w = (g.cell / 2.5).max(1.0);
    let door_w = (g.cell / 10.0).max(1.0);

    // List of tasks to perform in parallel
    enum MaskTask {
        Floors,
        Walls,
        Doors,
        Archways,
        LockedDoors,
        SecretDoors,
        Windows,
        StartMarkers,
        EndMarkers,
        TrapMarkers,
        MonsterMarkers,
        Stairs,
        Grid,
    }

    let mut tasks = vec![
        MaskTask::Floors,
        MaskTask::Walls,
        MaskTask::Doors,
        MaskTask::Archways,
        MaskTask::LockedDoors,
        MaskTask::SecretDoors,
        MaskTask::Windows,
        MaskTask::StartMarkers,
        MaskTask::EndMarkers,
        MaskTask::TrapMarkers,
        MaskTask::MonsterMarkers,
        MaskTask::Stairs,
    ];

    if settings.export_show_grid {
        tasks.push(MaskTask::Grid);
    }

    // Process masks in parallel
    tasks.into_par_iter().try_for_each(|task| {
        let (stem, pixmap) = match task {
            MaskTask::Floors => {
                let mut pixmap = Pixmap::new(g.width, g.height).unwrap();
                fill_bg(&mut pixmap, BLACK);
                for &(x, y) in &scene.corridor_cells {
                    fill_rect_cell(&mut pixmap, &g, x, y, WHITE);
                }
                for room in &layout.rooms {
                    fill_rect_room(&mut pixmap, &g, room, WHITE);
                }
                ("floors_mask", pixmap)
            }
            MaskTask::Walls => {
                let mut pixmap = Pixmap::new(g.width, g.height).unwrap();
                fill_bg(&mut pixmap, BLACK);
                draw_cell_walls(
                    &mut pixmap,
                    &g,
                    &scene.corridor_cells,
                    Some(&scene.corridor_edges),
                    &scene.door_edges,
                    wall_w,
                    WHITE,
                );
                draw_cell_walls(
                    &mut pixmap,
                    &g,
                    &scene.room_cells,
                    None,
                    &scene.door_edges,
                    wall_w,
                    WHITE,
                );
                ("walls_mask", pixmap)
            }
            MaskTask::Doors => {
                let mut pixmap = blank_mask(&g, "doors").unwrap();
                for door in &layout.doors {
                    if !door.secret && !door.archway && !door.locked {
                        draw_door(&mut pixmap, &g, door, door_w, WHITE, DoorStyle::Solid);
                    }
                }
                ("doors_mask", pixmap)
            }
            MaskTask::Archways => {
                let mut pixmap = blank_mask(&g, "archways").unwrap();
                for door in &layout.doors {
                    if door.archway {
                        draw_door(&mut pixmap, &g, door, door_w, WHITE, DoorStyle::Solid);
                    }
                }
                ("archways_mask", pixmap)
            }
            MaskTask::LockedDoors => {
                let mut pixmap = blank_mask(&g, "locked doors").unwrap();
                for door in &layout.doors {
                    if door.locked {
                        draw_door(&mut pixmap, &g, door, door_w, WHITE, DoorStyle::Solid);
                    }
                }
                ("locked_doors_mask", pixmap)
            }
            MaskTask::SecretDoors => {
                let mut pixmap = blank_mask(&g, "secret doors").unwrap();
                for door in &layout.doors {
                    if door.secret {
                        draw_door(&mut pixmap, &g, door, door_w, WHITE, DoorStyle::Solid);
                    }
                }
                ("secret_doors_mask", pixmap)
            }
            MaskTask::Windows => {
                let mut pixmap = blank_mask(&g, "windows").unwrap();
                for window in &layout.windows {
                    draw_window(&mut pixmap, &g, window, door_w, WHITE, DoorStyle::Solid);
                }
                ("windows_mask", pixmap)
            }
            MaskTask::StartMarkers => {
                let mut pixmap = blank_mask(&g, "start markers").unwrap();
                fill_marker_mask(&mut pixmap, &g, &layout.start_markers);
                ("start_markers_mask", pixmap)
            }
            MaskTask::EndMarkers => {
                let mut pixmap = blank_mask(&g, "end markers").unwrap();
                fill_marker_mask(&mut pixmap, &g, &layout.end_markers);
                ("end_markers_mask", pixmap)
            }
            MaskTask::TrapMarkers => {
                let mut pixmap = blank_mask(&g, "trap markers").unwrap();
                fill_marker_mask(&mut pixmap, &g, &layout.trap_markers);
                ("trap_markers_mask", pixmap)
            }
            MaskTask::MonsterMarkers => {
                let mut pixmap = blank_mask(&g, "monster markers").unwrap();
                fill_marker_mask(&mut pixmap, &g, &layout.monster_markers);
                ("monster_markers_mask", pixmap)
            }
            MaskTask::Stairs => {
                let mut pixmap = blank_mask(&g, "stairs").unwrap();
                for staircase in &layout.stairs {
                    fill_rect_staircase(&mut pixmap, &g, staircase, WHITE);
                }
                ("stairs_mask", pixmap)
            }
            MaskTask::Grid => {
                let mut pixmap = Pixmap::new(g.width, g.height).unwrap();
                fill_bg(&mut pixmap, BLACK);
                draw_grid_color(&mut pixmap, &g, WHITE);
                ("grid_mask", pixmap)
            }
        };

        save_mask_image(folder, stem, &pixmap, settings)
    })?;

    // Also export the composite image
    let composite = render_pixmap(layout, settings)?;
    let composite_img = raster_image_from_pixmap(&composite, settings)?;
    let composite_ext = settings.export_format.extension();
    let composite_path = folder.join(format!("composite.{composite_ext}"));
    composite_img
        .save_with_format(&composite_path, export_image_format(settings.export_format))
        .map_err(|e| format!("Failed writing {}: {e}", composite_path.display()))?;

    Ok(())
}

fn blank_mask(g: &ExportGeometry, name: &str) -> Result<Pixmap, String> {
    let mut pixmap = Pixmap::new(g.width, g.height)
        .ok_or_else(|| format!("Failed to allocate {name} mask canvas"))?;
    fill_bg(&mut pixmap, BLACK);
    Ok(pixmap)
}

fn fill_marker_mask(pixmap: &mut Pixmap, g: &ExportGeometry, markers: &[AreaMarker]) {
    for marker in markers {
        fill_rect_marker(pixmap, g, marker, WHITE);
    }
}

// Write a single mask image to disk.
fn save_mask_image(
    folder: &std::path::Path,
    stem: &str,
    pixmap: &Pixmap,
    settings: &UiSettings,
) -> Result<(), String> {
    let ext = settings.mask_format.extension();
    let path = folder.join(format!("{stem}.{ext}"));
    let img = raster_mask_image_from_pixmap(pixmap, settings)?;
    img.save_with_format(&path, mask_image_format(settings.mask_format))
        .map_err(|e| format!("Failed writing {}: {e}", path.display()))?;
    Ok(())
}

// Map a mask format to an image encoding.
fn mask_image_format(format: MaskFormat) -> ImageFormat {
    match format {
        MaskFormat::Png => ImageFormat::Png,
        MaskFormat::Jpeg => ImageFormat::Jpeg,
        MaskFormat::Webp => ImageFormat::WebP,
    }
}

// Map an export format to an image encoding.
fn export_image_format(format: ExportFormat) -> ImageFormat {
    match format {
        ExportFormat::Png => ImageFormat::Png,
        ExportFormat::Jpeg => ImageFormat::Jpeg,
        ExportFormat::Webp => ImageFormat::WebP,
        ExportFormat::Svg | ExportFormat::Folder => ImageFormat::Png,
    }
}

// Convert a pixmap into a raster image at the target size.
fn raster_image_from_pixmap(
    pixmap: &Pixmap,
    settings: &UiSettings,
) -> Result<DynamicImage, String> {
    let rgba = RgbaImage::from_raw(pixmap.width(), pixmap.height(), pixmap.data().to_vec())
        .ok_or_else(|| "Failed to build image buffer".to_string())?;
    let mut img = DynamicImage::ImageRgba8(rgba);

    let target = raster_target_size(settings);
    if img.width() != target.0 || img.height() != target.1 {
        img = img.resize_exact(target.0, target.1, FilterType::Lanczos3);
    }

    Ok(img)
}

// Convert a pixmap into a thresholded mask image.
fn raster_mask_image_from_pixmap(
    pixmap: &Pixmap,
    settings: &UiSettings,
) -> Result<DynamicImage, String> {
    let rgba = RgbaImage::from_raw(pixmap.width(), pixmap.height(), pixmap.data().to_vec())
        .ok_or_else(|| "Failed to build image buffer".to_string())?;
    let mut img = DynamicImage::ImageRgba8(rgba);

    let target = raster_mask_target_size(settings, pixmap.width(), pixmap.height());
    if img.width() != target.0 || img.height() != target.1 {
        img = img.resize_exact(target.0, target.1, FilterType::Nearest);
    }

    let mut out = img.to_rgba8();
    for pixel in out.pixels_mut() {
        let on = pixel[0] > 127 || pixel[1] > 127 || pixel[2] > 127;
        let v = if on { 255 } else { 0 };
        *pixel = image::Rgba([v, v, v, 255]);
    }

    Ok(DynamicImage::ImageRgba8(out))
}

// Compute the mask target size based on export width scaling.
fn raster_mask_target_size(settings: &UiSettings, _src_w: u32, _src_h: u32) -> (u32, u32) {
    raster_target_size(settings)
}

// Compute the output raster size while honoring aspect settings.
fn raster_target_size(settings: &UiSettings) -> (u32, u32) {
    let width = settings.export_width.clamp(1, 10_000);
    let mut height = settings.export_height.clamp(1, 10_000);

    if !settings.allow_export_aspect_change {
        let cols = settings.cols.max(1) as f32;
        let rows = settings.rows.max(1) as f32;
        let ratio = cols / rows;
        height = ((width as f32) / ratio).round() as u32;
        height = height.clamp(1, 10_000);
    }

    (width, height)
}

// Build an SVG document for the current layout.
pub fn build_svg(layout: &DungeonLayout, settings: &UiSettings) -> String {
    let g = ExportGeometry::new(settings.cols, settings.rows);
    let wall_w = (g.cell / 2.5).max(1.0);
    let door_w = (g.cell / 10.0).max(1.0);

    let mut s = String::new();
    let _ = writeln!(
        s,
        "<svg xmlns='http://www.w3.org/2000/svg' width='{}' height='{}' viewBox='0 0 {} {}'>",
        g.width, g.height, g.width, g.height
    );
    let _ = writeln!(
        s,
        "<rect width='100%' height='100%' fill='rgb({},{},{})'/>",
        BG.0, BG.1, BG.2
    );

    if settings.export_show_grid {
        for c in 0..=g.cols {
            let x = g.left() + c as f32 * g.cell;
            let _ = writeln!(
                s,
                "<line x1='{x}' y1='{}' x2='{x}' y2='{}' stroke='rgb({},{},{})' stroke-width='1'/>",
                g.top(),
                g.top() + g.rows as f32 * g.cell,
                GRID.0,
                GRID.1,
                GRID.2
            );
        }
        for r in 0..=g.rows {
            let y = g.top() + r as f32 * g.cell;
            let _ = writeln!(
                s,
                "<line x1='{}' y1='{y}' x2='{}' y2='{y}' stroke='rgb({},{},{})' stroke-width='1'/>",
                g.left(),
                g.left() + g.cols as f32 * g.cell,
                GRID.0,
                GRID.1,
                GRID.2
            );
        }
    }

    let corridor_cells = corridor_cells(layout, settings.cols, settings.rows);
    let corridor_edges = corridor_edges_from_cells(&corridor_cells);
    let mut room_cells = HashSet::new();
    let room_edges = room_edges_from_rooms(&layout.rooms);
    let mut door_edges = HashSet::new();

    for door in &layout.doors {
        for edge in door_edges_for(door, settings.cols, settings.rows) {
            door_edges.insert(edge);
        }
    }
    // corridor_cells/corridor_edges already populated with corridor widths
    for room in &layout.rooms {
        for x in room.x..(room.x + room.width) {
            for y in room.y..(room.y + room.height) {
                room_cells.insert((x, y));
            }
        }
    }

    for &(x, y) in &corridor_cells {
        let px = g.left() + x as f32 * g.cell;
        let py = g.top() + y as f32 * g.cell;
        let _ = writeln!(
            s,
            "<rect x='{px}' y='{py}' width='{}' height='{}' fill='rgb({},{},{})'/>",
            g.cell, g.cell, CORRIDOR.0, CORRIDOR.1, CORRIDOR.2
        );
        if settings.colorblind_mode {
            let (cx, cy) = g.cell_center(x, y);
            let _ = writeln!(
                s,
                "<circle cx='{cx}' cy='{cy}' r='{}' fill='black'/>",
                g.cell * 0.14
            );
        }
    }

    for room in &layout.rooms {
        let px = g.left() + room.x as f32 * g.cell;
        let py = g.top() + room.y as f32 * g.cell;
        let _ = writeln!(
            s,
            "<rect x='{px}' y='{py}' width='{}' height='{}' fill='rgb({},{},{})'/>",
            room.width as f32 * g.cell,
            room.height as f32 * g.cell,
            ROOM.0,
            ROOM.1,
            ROOM.2
        );
        if settings.colorblind_mode {
            for x in room.x..(room.x + room.width) {
                for y in room.y..(room.y + room.height) {
                    let rx = g.left() + x as f32 * g.cell + 2.0;
                    let ry = g.top() + y as f32 * g.cell + 2.0;
                    let rr = g.cell - 4.0;
                    let _ = writeln!(
                        s,
                        "<line x1='{rx}' y1='{ry}' x2='{}' y2='{}' stroke='black' stroke-width='1.5'/>",
                        rx + rr,
                        ry + rr
                    );
                    let _ = writeln!(
                        s,
                        "<line x1='{}' y1='{ry}' x2='{rx}' y2='{}' stroke='black' stroke-width='1.5'/>",
                        rx + rr,
                        ry + rr
                    );
                }
            }
        }
    }

    append_svg_walls(
        &mut s,
        &g,
        &corridor_cells,
        Some(&corridor_edges),
        &door_edges,
        wall_w,
    );
    append_svg_walls(
        &mut s,
        &g,
        &room_cells,
        Some(&room_edges),
        &door_edges,
        wall_w,
    );

    for door in &layout.doors {
        let (color, dash) = if settings.colorblind_mode {
            if door.secret {
                ("black", Some("7,3,1,3,1,4"))
            } else if door.archway {
                ("black", Some("2,4"))
            } else if door.locked {
                ("black", Some("6,4"))
            } else {
                ("black", Some("14,7"))
            }
        } else if door.secret {
            ("rgb(170,80,170)", None)
        } else if door.archway {
            ("rgb(230,140,60)", None)
        } else if door.locked {
            ("rgb(220,70,70)", None)
        } else {
            ("rgb(80,200,120)", None)
        };

        if let Some((x1, y1, x2, y2)) =
            door_line_points(&g, door.from, door.to, door_render_width(door))
        {
            if let Some(pattern) = dash {
                let _ = writeln!(
                    s,
                    "<line x1='{x1}' y1='{y1}' x2='{x2}' y2='{y2}' stroke='{color}' stroke-width='{door_w}' stroke-dasharray='{pattern}'/>"
                );
            } else {
                let _ = writeln!(
                    s,
                    "<line x1='{x1}' y1='{y1}' x2='{x2}' y2='{y2}' stroke='{color}' stroke-width='{door_w}'/>"
                );
            }
        }
    }

    for window in &layout.windows {
        let dash = settings.colorblind_mode.then_some("7,3,1,4");
        if let Some((x1, y1, x2, y2)) = window_line_points(&g, window) {
            if let Some(pattern) = dash {
                let _ = writeln!(
                    s,
                    "<line x1='{x1}' y1='{y1}' x2='{x2}' y2='{y2}' stroke='rgb({},{},{})' stroke-width='{door_w}' stroke-dasharray='{pattern}'/>",
                    WINDOW.0, WINDOW.1, WINDOW.2
                );
            } else {
                let _ = writeln!(
                    s,
                    "<line x1='{x1}' y1='{y1}' x2='{x2}' y2='{y2}' stroke='rgb({},{},{})' stroke-width='{door_w}'/>",
                    WINDOW.0, WINDOW.1, WINDOW.2
                );
            }
        }
    }

    append_svg_area_marker_group(
        &mut s,
        &g,
        &layout.start_markers,
        "S",
        START_MARKER,
        settings.colorblind_mode,
    );
    append_svg_area_marker_group(
        &mut s,
        &g,
        &layout.end_markers,
        "E",
        END_MARKER,
        settings.colorblind_mode,
    );
    append_svg_area_marker_group(
        &mut s,
        &g,
        &layout.trap_markers,
        "T",
        TRAP_MARKER,
        settings.colorblind_mode,
    );
    append_svg_area_marker_group(
        &mut s,
        &g,
        &layout.monster_markers,
        "M",
        MONSTER_MARKER,
        settings.colorblind_mode,
    );

    // Draw stairs.
    for stair in &layout.stairs {
        let x = g.left() + stair.cell.0 as f32 * g.cell;
        let y = g.top() + stair.cell.1 as f32 * g.cell;
        let w = stair.size as f32 * g.cell;
        let h = stair.size as f32 * g.cell;

        let fill = if settings.colorblind_mode {
            "rgb(180,180,180)"
        } else {
            "rgb(200,150,50)"
        };
        let _ = writeln!(
            s,
            "<rect x='{x}' y='{y}' width='{w}' height='{h}' fill='{fill}' fill-opacity='0.35' stroke='rgb(150,100,30)' stroke-width='2'/>"
        );
        let steps = (stair.size as f32 * 2.0).round() as usize;
        let step_h = h / steps as f32;
        for i in 0..steps {
            let sy = y + (i as f32 * step_h) + step_h * 0.5;
            let _ = writeln!(
                s,
                "<line x1='{x1}' y1='{sy}' x2='{x2}' y2='{sy}' stroke='rgb(150,100,30)' stroke-width='1.5'/>",
                x1 = x + 2.0,
                x2 = x + w - 2.0
            );
        }
    }

    s.push_str("</svg>\n");
    s
}

// Fill the entire pixmap with a solid color.
fn fill_bg(pixmap: &mut Pixmap, color: (u8, u8, u8, u8)) {
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.0, color.1, color.2, color.3);
    let Some(rect) = Rect::from_xywh(0.0, 0.0, pixmap.width() as f32, pixmap.height() as f32)
    else {
        return;
    };
    pixmap.fill_rect(rect, &paint, Transform::identity(), None);
}

// Draw the default grid lines.
fn draw_grid(pixmap: &mut Pixmap, g: &ExportGeometry) {
    draw_grid_color(pixmap, g, GRID);
}

// Draw grid lines using a specific color.
fn draw_grid_color(pixmap: &mut Pixmap, g: &ExportGeometry, color: (u8, u8, u8, u8)) {
    let left = g.left();
    let top = g.top();
    let width = g.cols as f32 * g.cell;
    let height = g.rows as f32 * g.cell;

    let mut paint = Paint::default();
    paint.set_color_rgba8(color.0, color.1, color.2, color.3);

    for c in 0..=g.cols {
        let x = (left + c as f32 * g.cell).round();
        let Some(rect) = Rect::from_xywh(x, top, 1.0, height) else {
            continue;
        };
        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
    }
    for r in 0..=g.rows {
        let y = (top + r as f32 * g.cell).round();
        let Some(rect) = Rect::from_xywh(left, y, width, 1.0) else {
            continue;
        };
        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
    }
}

// Fill a single cell rectangle with a color.
fn fill_rect_cell(
    pixmap: &mut Pixmap,
    g: &ExportGeometry,
    col: usize,
    row: usize,
    color: (u8, u8, u8, u8),
) {
    let Some(rect) = g.cell_rect(col, row) else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.0, color.1, color.2, color.3);
    pixmap.fill_rect(rect, &paint, Transform::identity(), None);
}

// Fill an entire room rectangle with a color.
fn fill_rect_room(pixmap: &mut Pixmap, g: &ExportGeometry, room: &Room, color: (u8, u8, u8, u8)) {
    let Some(rect) = Rect::from_xywh(
        g.left() + room.x as f32 * g.cell,
        g.top() + room.y as f32 * g.cell,
        room.width as f32 * g.cell,
        room.height as f32 * g.cell,
    ) else {
        return;
    };

    let mut paint = Paint::default();
    paint.set_color_rgba8(color.0, color.1, color.2, color.3);
    pixmap.fill_rect(rect, &paint, Transform::identity(), None);
}

fn fill_rect_marker(
    pixmap: &mut Pixmap,
    g: &ExportGeometry,
    marker: &AreaMarker,
    color: (u8, u8, u8, u8),
) {
    let Some(rect) = Rect::from_xywh(
        g.left() + marker.cell.0 as f32 * g.cell,
        g.top() + marker.cell.1 as f32 * g.cell,
        marker.size as f32 * g.cell,
        marker.size as f32 * g.cell,
    ) else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.0, color.1, color.2, color.3);
    pixmap.fill_rect(rect, &paint, Transform::identity(), None);
}

fn draw_area_marker(
    pixmap: &mut Pixmap,
    g: &ExportGeometry,
    marker: &AreaMarker,
    label: &str,
    color: (u8, u8, u8, u8),
    colorblind_mode: bool,
) {
    let overlay = (
        color.0,
        color.1,
        color.2,
        if colorblind_mode { 56 } else { 90 },
    );
    fill_rect_marker(pixmap, g, marker, overlay);

    let x = g.left() + marker.cell.0 as f32 * g.cell;
    let y = g.top() + marker.cell.1 as f32 * g.cell;
    let w = marker.size as f32 * g.cell;
    let h = marker.size as f32 * g.cell;
    draw_wall_segment(pixmap, x, y, x + w, y, 3.0, color);
    draw_wall_segment(pixmap, x, y + h, x + w, y + h, 3.0, color);
    draw_wall_segment(pixmap, x, y, x, y + h, 3.0, color);
    draw_wall_segment(pixmap, x + w, y, x + w, y + h, 3.0, color);

    let symbol = if colorblind_mode { BLACK } else { WHITE };
    if label == "S" {
        draw_line(
            pixmap,
            x + w * 0.25,
            y + h * 0.75,
            x + w * 0.5,
            y + h * 0.25,
            4.0,
            symbol,
            DoorStyle::Solid,
        );
        draw_line(
            pixmap,
            x + w * 0.5,
            y + h * 0.25,
            x + w * 0.75,
            y + h * 0.75,
            4.0,
            symbol,
            DoorStyle::Solid,
        );
    } else if label == "E" {
        draw_line(
            pixmap,
            x + w * 0.25,
            y + h * 0.25,
            x + w * 0.75,
            y + h * 0.75,
            4.0,
            symbol,
            DoorStyle::Solid,
        );
        draw_line(
            pixmap,
            x + w * 0.75,
            y + h * 0.25,
            x + w * 0.25,
            y + h * 0.75,
            4.0,
            symbol,
            DoorStyle::Solid,
        );
    } else if label == "T" {
        // Horizontal bar
        draw_line(
            pixmap,
            x + w * 0.2,
            y + h * 0.25,
            x + w * 0.8,
            y + h * 0.25,
            4.0,
            symbol,
            DoorStyle::Solid,
        );
        // Vertical bar
        draw_line(
            pixmap,
            x + w * 0.5,
            y + h * 0.25,
            x + w * 0.5,
            y + h * 0.75,
            4.0,
            symbol,
            DoorStyle::Solid,
        );
    } else if label == "M" {
        draw_line(
            pixmap,
            x + w * 0.2,
            y + h * 0.75,
            x + w * 0.2,
            y + h * 0.25,
            4.0,
            symbol,
            DoorStyle::Solid,
        );
        draw_line(
            pixmap,
            x + w * 0.2,
            y + h * 0.25,
            x + w * 0.5,
            y + h * 0.5,
            4.0,
            symbol,
            DoorStyle::Solid,
        );
        draw_line(
            pixmap,
            x + w * 0.5,
            y + h * 0.5,
            x + w * 0.8,
            y + h * 0.25,
            4.0,
            symbol,
            DoorStyle::Solid,
        );
        draw_line(
            pixmap,
            x + w * 0.8,
            y + h * 0.25,
            x + w * 0.8,
            y + h * 0.75,
            4.0,
            symbol,
            DoorStyle::Solid,
        );
    }
}

fn fill_rect_staircase(
    pixmap: &mut Pixmap,
    g: &ExportGeometry,
    staircase: &Staircase,
    color: (u8, u8, u8, u8),
) {
    let Some(rect) = Rect::from_xywh(
        g.left() + staircase.cell.0 as f32 * g.cell,
        g.top() + staircase.cell.1 as f32 * g.cell,
        staircase.size as f32 * g.cell,
        staircase.size as f32 * g.cell,
    ) else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.0, color.1, color.2, color.3);
    pixmap.fill_rect(rect, &paint, Transform::identity(), None);
}

fn draw_staircase(
    pixmap: &mut Pixmap,
    g: &ExportGeometry,
    staircase: &Staircase,
    color: (u8, u8, u8, u8),
    colorblind_mode: bool,
) {
    let fill_color = if colorblind_mode {
        (180, 180, 180, 90)
    } else {
        (color.0, color.1, color.2, 90)
    };

    let Some(rect) = Rect::from_xywh(
        g.left() + staircase.cell.0 as f32 * g.cell,
        g.top() + staircase.cell.1 as f32 * g.cell,
        staircase.size as f32 * g.cell,
        staircase.size as f32 * g.cell,
    ) else {
        return;
    };

    let mut paint = Paint::default();
    paint.set_color_rgba8(fill_color.0, fill_color.1, fill_color.2, fill_color.3);
    pixmap.fill_rect(rect, &paint, Transform::identity(), None);

    let stroke_color = if colorblind_mode {
        BLACK
    } else {
        (150, 100, 30, 255)
    };
    let x = rect.left();
    let y = rect.top();
    let w = rect.width();
    let h = rect.height();

    draw_wall_segment(pixmap, x, y, x + w, y, 2.0, stroke_color);
    draw_wall_segment(pixmap, x, y + h, x + w, y + h, 2.0, stroke_color);
    draw_wall_segment(pixmap, x, y, x, y + h, 2.0, stroke_color);
    draw_wall_segment(pixmap, x + w, y, x + w, y + h, 2.0, stroke_color);

    let steps = (staircase.size as f32 * 2.0).round() as usize;
    let step_h = h / steps as f32;
    for i in 0..steps {
        let sy = (y + (i as f32 * step_h) + step_h * 0.5).round();
        draw_line(
            pixmap,
            x + 2.0,
            sy,
            x + w - 2.0,
            sy,
            1.5,
            stroke_color,
            DoorStyle::Solid,
        );
    }
}

fn draw_staircase_group(
    pixmap: &mut Pixmap,
    g: &ExportGeometry,
    stairs: &[Staircase],
    color: (u8, u8, u8, u8),
    colorblind_mode: bool,
) {
    for staircase in stairs {
        draw_staircase(pixmap, g, staircase, color, colorblind_mode);
    }
}

fn draw_area_marker_group(
    pixmap: &mut Pixmap,
    g: &ExportGeometry,
    markers: &[AreaMarker],
    label: &str,
    color: (u8, u8, u8, u8),
    colorblind_mode: bool,
) {
    for marker in markers {
        draw_area_marker(pixmap, g, marker, label, color, colorblind_mode);
    }
}

// Draw a filled circle for dot patterns.
fn draw_dot(pixmap: &mut Pixmap, cx: f32, cy: f32, radius: f32, color: (u8, u8, u8, u8)) {
    let mut pb = PathBuilder::new();
    pb.push_circle(cx, cy, radius);
    let Some(path) = pb.finish() else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.0, color.1, color.2, color.3);
    pixmap.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

// Draw a crosshatch overlay for a room.
fn draw_room_crosshatch(
    pixmap: &mut Pixmap,
    g: &ExportGeometry,
    room: &Room,
    width: f32,
    color: (u8, u8, u8, u8),
) {
    for x in room.x..(room.x + room.width) {
        for y in room.y..(room.y + room.height) {
            if let Some(rect) = g.cell_rect(x, y) {
                let pad = 2.0;
                draw_line(
                    pixmap,
                    rect.left() + pad,
                    rect.top() + pad,
                    rect.right() - pad,
                    rect.bottom() - pad,
                    width,
                    color,
                    DoorStyle::Solid,
                );
                draw_line(
                    pixmap,
                    rect.right() - pad,
                    rect.top() + pad,
                    rect.left() + pad,
                    rect.bottom() - pad,
                    width,
                    color,
                    DoorStyle::Solid,
                );
            }
        }
    }
}

// Draw wall segments around occupied cells.
fn draw_cell_walls(
    pixmap: &mut Pixmap,
    g: &ExportGeometry,
    cells: &HashSet<(usize, usize)>,
    connected_edges: Option<&HashSet<((usize, usize), (usize, usize))>>,
    door_edges: &HashSet<((usize, usize), (usize, usize))>,
    width: f32,
    color: (u8, u8, u8, u8),
) {
    for &(col, row) in cells {
        if let Some(rect) = g.cell_rect(col, row) {
            let right = (col + 1, row);
            let bottom = (col, row + 1);

            let should_left = col == 0
                || (!cells.contains(&(col - 1, row))
                    && !door_edges.contains(&norm_edge((col, row), (col - 1, row))));

            let right_connected = connected_edges
                .map(|set| set.contains(&norm_edge((col, row), right)))
                .unwrap_or(true);
            let should_right = (!cells.contains(&right) || !right_connected)
                && !door_edges.contains(&norm_edge((col, row), right));

            let should_top = row == 0
                || (!cells.contains(&(col, row - 1))
                    && !door_edges.contains(&norm_edge((col, row), (col, row - 1))));

            let bottom_connected = connected_edges
                .map(|set| set.contains(&norm_edge((col, row), bottom)))
                .unwrap_or(true);
            let should_bottom = (!cells.contains(&bottom) || !bottom_connected)
                && !door_edges.contains(&norm_edge((col, row), bottom));

            if should_left {
                draw_wall_segment(
                    pixmap,
                    rect.left(),
                    rect.top(),
                    rect.left(),
                    rect.bottom(),
                    width,
                    color,
                );
            }
            if should_right {
                draw_wall_segment(
                    pixmap,
                    rect.right(),
                    rect.top(),
                    rect.right(),
                    rect.bottom(),
                    width,
                    color,
                );
            }
            if should_top {
                draw_wall_segment(
                    pixmap,
                    rect.left(),
                    rect.top(),
                    rect.right(),
                    rect.top(),
                    width,
                    color,
                );
            }
            if should_bottom {
                draw_wall_segment(
                    pixmap,
                    rect.left(),
                    rect.bottom(),
                    rect.right(),
                    rect.bottom(),
                    width,
                    color,
                );
            }
        }
    }
}

// Draw a single wall segment as a filled rectangle.
fn draw_wall_segment(
    pixmap: &mut Pixmap,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    width: f32,
    color: (u8, u8, u8, u8),
) {
    let width = width.round().max(1.0);
    let half = width * 0.5;
    let x1 = x1.round();
    let y1 = y1.round();
    let x2 = x2.round();
    let y2 = y2.round();
    let rect = if (x1 - x2).abs() <= f32::EPSILON {
        let x = x1.min(x2);
        let y0 = y1.min(y2);
        let y1 = y1.max(y2);
        Rect::from_xywh(x - half, y0 - half, width, (y1 - y0) + width)
    } else {
        let y = y1.min(y2);
        let x0 = x1.min(x2);
        let x1 = x1.max(x2);
        Rect::from_xywh(x0 - half, y - half, (x1 - x0) + width, width)
    };

    let Some(rect) = rect else {
        return;
    };

    let mut paint = Paint::default();
    paint.set_color_rgba8(color.0, color.1, color.2, color.3);
    pixmap.fill_rect(rect, &paint, Transform::identity(), None);
}

// Round a width to the nearest even pixel size.
fn snap_even_width(value: f32) -> f32 {
    let mut w = value.round().max(1.0);
    if (w as u32) % 2 == 1 {
        w += 1.0;
    }
    w
}

// Append SVG wall rectangles for the given cells.
fn append_svg_walls(
    out: &mut String,
    g: &ExportGeometry,
    cells: &HashSet<(usize, usize)>,
    connected_edges: Option<&HashSet<((usize, usize), (usize, usize))>>,
    door_edges: &HashSet<((usize, usize), (usize, usize))>,
    width: f32,
) {
    for &(col, row) in cells {
        let left = g.left() + col as f32 * g.cell;
        let top = g.top() + row as f32 * g.cell;
        let right = left + g.cell;
        let bottom = top + g.cell;

        let right_cell = (col + 1, row);
        let bottom_cell = (col, row + 1);

        let left_edge = col == 0
            || (!cells.contains(&(col - 1, row))
                && !door_edges.contains(&norm_edge((col, row), (col - 1, row))));

        let right_connected = connected_edges
            .map(|set| set.contains(&norm_edge((col, row), right_cell)))
            .unwrap_or(true);
        let right_edge = (!cells.contains(&right_cell) || !right_connected)
            && !door_edges.contains(&norm_edge((col, row), right_cell));

        let top_edge = row == 0
            || (!cells.contains(&(col, row - 1))
                && !door_edges.contains(&norm_edge((col, row), (col, row - 1))));

        let bottom_connected = connected_edges
            .map(|set| set.contains(&norm_edge((col, row), bottom_cell)))
            .unwrap_or(true);
        let bottom_edge = (!cells.contains(&bottom_cell) || !bottom_connected)
            && !door_edges.contains(&norm_edge((col, row), bottom_cell));

        if left_edge {
            let _ = writeln!(
                out,
                "<rect x='{}' y='{}' width='{width}' height='{}' fill='black'/>",
                left - (width * 0.5),
                top - (width * 0.5),
                (bottom - top) + width
            );
        }
        if right_edge {
            let _ = writeln!(
                out,
                "<rect x='{}' y='{}' width='{width}' height='{}' fill='black'/>",
                right - (width * 0.5),
                top - (width * 0.5),
                (bottom - top) + width
            );
        }
        if top_edge {
            let _ = writeln!(
                out,
                "<rect x='{}' y='{}' width='{}' height='{width}' fill='black'/>",
                left - (width * 0.5),
                top - (width * 0.5),
                (right - left) + width
            );
        }
        if bottom_edge {
            let _ = writeln!(
                out,
                "<rect x='{}' y='{}' width='{}' height='{width}' fill='black'/>",
                left - (width * 0.5),
                bottom - (width * 0.5),
                (right - left) + width
            );
        }
    }
}

fn append_svg_area_marker(
    out: &mut String,
    g: &ExportGeometry,
    marker: &AreaMarker,
    label: &str,
    color: (u8, u8, u8, u8),
    colorblind_mode: bool,
) {
    let x = g.left() + marker.cell.0 as f32 * g.cell;
    let y = g.top() + marker.cell.1 as f32 * g.cell;
    let w = marker.size as f32 * g.cell;
    let h = marker.size as f32 * g.cell;
    let alpha = if colorblind_mode { 0.22 } else { 0.35 };
    let _ = writeln!(
        out,
        "<rect x='{x}' y='{y}' width='{w}' height='{h}' fill='rgb({},{},{})' fill-opacity='{alpha}' stroke='rgb({},{},{})' stroke-width='3'/>",
        color.0, color.1, color.2, color.0, color.1, color.2
    );
    let symbol = if colorblind_mode { "black" } else { "white" };
    if label == "S" {
        let _ = writeln!(
            out,
            "<polyline points='{},{} {},{} {},{}' fill='none' stroke='{}' stroke-width='4' stroke-linecap='round' stroke-linejoin='round'/>",
            x + w * 0.25,
            y + h * 0.75,
            x + w * 0.5,
            y + h * 0.25,
            x + w * 0.75,
            y + h * 0.75,
            symbol
        );
    } else if label == "E" {
        let _ = writeln!(
            out,
            "<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='{}' stroke-width='4' stroke-linecap='round'/>",
            x + w * 0.25,
            y + h * 0.25,
            x + w * 0.75,
            y + h * 0.75,
            symbol
        );
        let _ = writeln!(
            out,
            "<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='{}' stroke-width='4' stroke-linecap='round'/>",
            x + w * 0.75,
            y + h * 0.25,
            x + w * 0.25,
            y + h * 0.75,
            symbol
        );
    } else if label == "T" {
        // Horizontal bar
        let _ = writeln!(
            out,
            "<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='{}' stroke-width='4' stroke-linecap='round'/>",
            x + w * 0.2,
            y + h * 0.25,
            x + w * 0.8,
            y + h * 0.25,
            symbol
        );
        // Vertical bar
        let _ = writeln!(
            out,
            "<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='{}' stroke-width='4' stroke-linecap='round'/>",
            x + w * 0.5,
            y + h * 0.25,
            x + w * 0.5,
            y + h * 0.75,
            symbol
        );
    } else if label == "M" {
        let _ = writeln!(
            out,
            "<polyline points='{},{} {},{} {},{} {},{} {},{}' fill='none' stroke='{}' stroke-width='4' stroke-linecap='round' stroke-linejoin='round'/>",
            x + w * 0.2,
            y + h * 0.75,
            x + w * 0.2,
            y + h * 0.25,
            x + w * 0.5,
            y + h * 0.5,
            x + w * 0.8,
            y + h * 0.25,
            x + w * 0.8,
            y + h * 0.75,
            symbol
        );
    }
}

fn append_svg_area_marker_group(
    out: &mut String,
    g: &ExportGeometry,
    markers: &[AreaMarker],
    label: &str,
    color: (u8, u8, u8, u8),
    colorblind_mode: bool,
) {
    for marker in markers {
        append_svg_area_marker(out, g, marker, label, color, colorblind_mode);
    }
}

// Draw a door line onto the pixmap.
fn draw_door(
    pixmap: &mut Pixmap,
    g: &ExportGeometry,
    door: &Door,
    width: f32,
    color: (u8, u8, u8, u8),
    style: DoorStyle,
) {
    let Some((x1, y1, x2, y2)) = door_line_points(g, door.from, door.to, door_render_width(door))
    else {
        return;
    };
    draw_line(pixmap, x1, y1, x2, y2, width, color, style);
}

fn draw_window(
    pixmap: &mut Pixmap,
    g: &ExportGeometry,
    window: &Window,
    width: f32,
    color: (u8, u8, u8, u8),
    style: DoorStyle,
) {
    let Some((x1, y1, x2, y2)) = window_line_points(g, window) else {
        return;
    };
    draw_line(pixmap, x1, y1, x2, y2, width, color, style);
}

// Compute the door line endpoints in export space.
fn door_line_points(
    g: &ExportGeometry,
    a: (usize, usize),
    b: (usize, usize),
    width_cells: usize,
) -> Option<(f32, f32, f32, f32)> {
    if a.0.abs_diff(b.0) + a.1.abs_diff(b.1) != 1 {
        return None;
    }

    let width_cells = width_cells.max(1) as isize;
    let min_offset = -((width_cells - 1) / 2);
    let max_offset = width_cells / 2;

    if a.0 != b.0 {
        let x = g.left() + (a.0.max(b.0) as f32) * g.cell;
        let row = a.1 as isize;
        let y0_cell = (row + min_offset).clamp(0, g.rows.saturating_sub(1) as isize);
        let y1_cell = (row + max_offset).clamp(0, g.rows.saturating_sub(1) as isize);
        let y0 = g.top() + y0_cell as f32 * g.cell;
        let y1 = g.top() + (y1_cell as f32 + 1.0) * g.cell;
        Some((x, y0, x, y1))
    } else {
        let y = g.top() + (a.1.max(b.1) as f32) * g.cell;
        let col = a.0 as isize;
        let x0_cell = (col + min_offset).clamp(0, g.cols.saturating_sub(1) as isize);
        let x1_cell = (col + max_offset).clamp(0, g.cols.saturating_sub(1) as isize);
        let x0 = g.left() + x0_cell as f32 * g.cell;
        let x1 = g.left() + (x1_cell as f32 + 1.0) * g.cell;
        Some((x0, y, x1, y))
    }
}

fn window_line_points(g: &ExportGeometry, window: &Window) -> Option<(f32, f32, f32, f32)> {
    let width_cells = window_render_width(window).max(1) as isize;
    let min_offset = -((width_cells - 1) / 2);
    let max_offset = width_cells / 2;

    match window.side {
        WindowSide::Left => {
            let x = g.left() + window.cell.0 as f32 * g.cell;
            let row = window.cell.1 as isize;
            let y0_cell = (row + min_offset).clamp(0, g.rows.saturating_sub(1) as isize);
            let y1_cell = (row + max_offset).clamp(0, g.rows.saturating_sub(1) as isize);
            let y0 = g.top() + y0_cell as f32 * g.cell;
            let y1 = g.top() + (y1_cell as f32 + 1.0) * g.cell;
            Some((x, y0, x, y1))
        }
        WindowSide::Right => {
            let x = g.left() + (window.cell.0 as f32 + 1.0) * g.cell;
            let row = window.cell.1 as isize;
            let y0_cell = (row + min_offset).clamp(0, g.rows.saturating_sub(1) as isize);
            let y1_cell = (row + max_offset).clamp(0, g.rows.saturating_sub(1) as isize);
            let y0 = g.top() + y0_cell as f32 * g.cell;
            let y1 = g.top() + (y1_cell as f32 + 1.0) * g.cell;
            Some((x, y0, x, y1))
        }
        WindowSide::Top => {
            let y = g.top() + window.cell.1 as f32 * g.cell;
            let col = window.cell.0 as isize;
            let x0_cell = (col + min_offset).clamp(0, g.cols.saturating_sub(1) as isize);
            let x1_cell = (col + max_offset).clamp(0, g.cols.saturating_sub(1) as isize);
            let x0 = g.left() + x0_cell as f32 * g.cell;
            let x1 = g.left() + (x1_cell as f32 + 1.0) * g.cell;
            Some((x0, y, x1, y))
        }
        WindowSide::Bottom => {
            let y = g.top() + (window.cell.1 as f32 + 1.0) * g.cell;
            let col = window.cell.0 as isize;
            let x0_cell = (col + min_offset).clamp(0, g.cols.saturating_sub(1) as isize);
            let x1_cell = (col + max_offset).clamp(0, g.cols.saturating_sub(1) as isize);
            let x0 = g.left() + x0_cell as f32 * g.cell;
            let x1 = g.left() + (x1_cell as f32 + 1.0) * g.cell;
            Some((x0, y, x1, y))
        }
    }
}

// Draw a styled line in the pixmap.
fn draw_line(
    pixmap: &mut Pixmap,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    width: f32,
    color: (u8, u8, u8, u8),
    style: DoorStyle,
) {
    match style {
        DoorStyle::Solid => stroke_path(pixmap, x1, y1, x2, y2, width, color, None),
        DoorStyle::LongDash => stroke_path(pixmap, x1, y1, x2, y2, width, color, Some((14.0, 7.0))),
        DoorStyle::ShortDash => stroke_path(pixmap, x1, y1, x2, y2, width, color, Some((6.0, 4.0))),
        DoorStyle::Dotted => dotted_line(pixmap, x1, y1, x2, y2, width, color),
        DoorStyle::DashDot => stroke_path(pixmap, x1, y1, x2, y2, width, color, Some((7.0, 3.0))),
        DoorStyle::DashDotDot => {
            stroke_path(pixmap, x1, y1, x2, y2, width, color, Some((7.0, 3.0)))
        }
    }
}

// Stroke a path with optional dash patterns.
fn stroke_path(
    pixmap: &mut Pixmap,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    width: f32,
    color: (u8, u8, u8, u8),
    dash: Option<(f32, f32)>,
) {
    let mut pb = PathBuilder::new();
    pb.move_to(x1, y1);
    pb.line_to(x2, y2);
    let Some(path) = pb.finish() else {
        return;
    };

    let mut paint = Paint::default();
    paint.set_color_rgba8(color.0, color.1, color.2, color.3);

    let mut stroke = Stroke::default();
    stroke.width = width;
    if let Some((dash_len, gap_len)) = dash {
        stroke.dash = StrokeDash::new(vec![dash_len, gap_len], 0.0);
    }

    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
}

// Draw a dotted line using repeated circles.
fn dotted_line(
    pixmap: &mut Pixmap,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    width: f32,
    color: (u8, u8, u8, u8),
) {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 0.0 {
        return;
    }
    let ux = dx / len;
    let uy = dy / len;

    let mut d = 0.0_f32;
    let step = 6.0;
    let radius = (width * 0.35).max(1.0);
    while d <= len {
        draw_dot(pixmap, x1 + ux * d, y1 + uy * d, radius, color);
        d += step;
    }
}

// Normalize an edge tuple to a stable ordering.
fn norm_edge(a: (usize, usize), b: (usize, usize)) -> ((usize, usize), (usize, usize)) {
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
            edges.insert(norm_edge((col, row), right));
        }
        if corridor_cells.contains(&bottom) {
            edges.insert(norm_edge((col, row), bottom));
        }
    }
    edges
}

// Build a set of internal room edges from room rectangles.
fn room_edges_from_rooms(rooms: &[Room]) -> HashSet<((usize, usize), (usize, usize))> {
    let mut edges = HashSet::new();
    for room in rooms {
        let x_end = room.x + room.width;
        let y_end = room.y + room.height;
        for x in room.x..x_end {
            for y in room.y..y_end {
                if x + 1 < x_end {
                    edges.insert(norm_edge((x, y), (x + 1, y)));
                }
                if y + 1 < y_end {
                    edges.insert(norm_edge((x, y), (x, y + 1)));
                }
            }
        }
    }
    edges
}

// Expand a door into all grid edges it spans based on width.
fn door_edges_for(door: &Door, cols: usize, rows: usize) -> Vec<((usize, usize), (usize, usize))> {
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
            edges.push(norm_edge(a, b));
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
            edges.push(norm_edge(a, b));
        }
    }

    edges
}

// Determine the rendered door width in cells.
fn door_render_width(door: &Door) -> usize {
    if door.span_width {
        door.width.max(1)
    } else {
        1
    }
}

fn window_render_width(window: &Window) -> usize {
    if window.span_width {
        window.width.max(1)
    } else {
        1
    }
}
