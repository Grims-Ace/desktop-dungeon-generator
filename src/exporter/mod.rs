/*
 * Main export module.
 * Orchestrates the export process, coordinating dialog selection,
 * background thread execution, and multi-format (SVG/Raster) generation.
 */

use image::ImageFormat;
use rayon::prelude::*;
use rfd::FileDialog;
use std::path::PathBuf;
use std::sync::mpsc;
use tiny_skia::Pixmap;

use crate::layout::DungeonLayout;
use crate::ui::{ExportFormat, MaskFormat, UiSettings};

pub mod raster;
pub mod svg;
pub mod types;
pub mod utils;

pub use svg::build_svg;
pub use types::*;

type ProgressSender = mpsc::Sender<ExportEvent>;

// Selects export target.
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

// Exports to target.
pub fn export_to_target(
    layouts: &[DungeonLayout],
    settings: &UiSettings,
    target: ExportTarget,
    progress_tx: &ProgressSender,
) -> Result<PathBuf, String> {
    let export_indices: Vec<usize> = if settings.export_level_index == 999 {
        (0..layouts.len()).collect()
    } else {
        let idx = settings
            .export_level_index
            .min(layouts.len().saturating_sub(1));
        vec![idx]
    };
    let total = export_image_count(settings, &target, export_indices.len());
    report_progress(progress_tx, 0, total);

    match target {
        ExportTarget::Folder(folder) => {
            if settings.export_format == ExportFormat::Folder {
                export_composite_masks_for_levels(
                    layouts,
                    settings,
                    &folder,
                    &export_indices,
                    progress_tx,
                )?;
            } else {
                export_level_images_to_folder(
                    layouts,
                    settings,
                    &folder,
                    &export_indices,
                    progress_tx,
                )?;
            }
            Ok(folder)
        }
        ExportTarget::File(path) => {
            let idx = export_indices[0];
            save_layout_to_file(&layouts[idx], settings, &path)?;
            report_progress(progress_tx, 1, total);
            Ok(path)
        }
    }
}

// Exports image count.
fn export_image_count(settings: &UiSettings, target: &ExportTarget, level_count: usize) -> usize {
    match target {
        ExportTarget::File(_) => 1,
        ExportTarget::Folder(_) if settings.export_format == ExportFormat::Folder => {
            level_count * composite_images_per_level(settings)
        }
        ExportTarget::Folder(_) => level_count,
    }
}

// Counts images per level.
fn composite_images_per_level(settings: &UiSettings) -> usize {
    let base_masks = 12;
    let grid_mask = usize::from(settings.export_show_grid);
    let composite = 1;
    base_masks + grid_mask + composite
}

// Sends an export progress update to the UI channel.
fn report_progress(progress_tx: &ProgressSender, completed: usize, total: usize) {
    let _ = progress_tx.send(ExportEvent::Progress(ExportProgress { completed, total }));
}

// Exports level images to folder.
fn export_level_images_to_folder(
    layouts: &[DungeonLayout],
    settings: &UiSettings,
    folder: &std::path::Path,
    export_indices: &[usize],
    progress_tx: &ProgressSender,
) -> Result<(), String> {
    std::fs::create_dir_all(folder).map_err(|err| format!("Failed creating folder: {err}"))?;
    let completed = std::sync::atomic::AtomicUsize::new(0);
    let total = export_indices.len();

    let ext = settings.export_format.extension();
    export_indices.par_iter().try_for_each(|&idx| {
        let filename = format!("level{}.{ext}", idx + 1);
        let path = folder.join(filename);
        save_layout_to_file(&layouts[idx], settings, &path)?;
        let done = completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        report_progress(progress_tx, done, total);
        Ok(())
    })
}

// Saves layout to file.
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
            let pixmap = raster::render_pixmap(layout, settings)?;
            let img = raster::raster_image_from_pixmap(&pixmap, settings)?;
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

// Exports composite masks for levels.
fn export_composite_masks_for_levels(
    layouts: &[DungeonLayout],
    settings: &UiSettings,
    folder: &std::path::Path,
    export_indices: &[usize],
    progress_tx: &ProgressSender,
) -> Result<(), String> {
    let contexts = export_indices
        .iter()
        .map(|&idx| {
            let subfolder = if settings.export_level_index == 999 {
                folder.join(format!("level {}", idx + 1))
            } else {
                folder.to_path_buf()
            };
            std::fs::create_dir_all(&subfolder)
                .map_err(|err| format!("Failed creating folder: {err}"))?;

            let geometry = ExportGeometry::new(settings.cols, settings.rows);
            let scene = utils::collect_scene_data(&layouts[idx], settings);
            Ok(CompositeExportContext {
                layout: &layouts[idx],
                folder: subfolder,
                geometry,
                scene,
                wall_width: (geometry.cell / 2.5).max(1.0),
                door_width: (geometry.cell / 5.0).max(1.0),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

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

    let export_tasks = contexts
        .iter()
        .enumerate()
        .flat_map(|(context_idx, _)| {
            tasks
                .iter()
                .copied()
                .map(move |task| (context_idx, CompositeTask::Mask(task)))
                .chain(std::iter::once((context_idx, CompositeTask::Composite)))
        })
        .collect::<Vec<_>>();
    let completed = std::sync::atomic::AtomicUsize::new(0);
    let total = export_tasks.len();

    export_tasks
        .into_par_iter()
        .try_for_each(|(context_idx, task)| {
            let context = &contexts[context_idx];
            match task {
                CompositeTask::Mask(mask_task) => {
                    let (stem, pixmap) = render_mask_pixmap(context, mask_task)?;
                    save_mask_image(&context.folder, stem, &pixmap, settings)
                }
                CompositeTask::Composite => save_composite_image(context, settings),
            }?;
            let done = completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            report_progress(progress_tx, done, total);
            Ok::<(), String>(())
        })?;

    Ok(())
}

// Saves composite image.
fn save_composite_image(
    context: &CompositeExportContext<'_>,
    settings: &UiSettings,
) -> Result<(), String> {
    let composite = raster::render_pixmap_with_scene(
        context.layout,
        settings,
        &context.geometry,
        &context.scene,
    )?;
    let composite_img = raster::raster_image_from_pixmap(&composite, settings)?;
    let composite_ext = settings.mask_format.extension();
    let composite_path = context.folder.join(format!("composite.{composite_ext}"));
    composite_img
        .save_with_format(&composite_path, mask_image_format(settings.mask_format))
        .map_err(|e| format!("Failed writing {}: {e}", composite_path.display()))?;

    Ok(())
}

// Renders mask pixmap.
fn render_mask_pixmap(
    context: &CompositeExportContext<'_>,
    task: MaskTask,
) -> Result<(&'static str, Pixmap), String> {
    let g = &context.geometry;
    let scene = &context.scene;
    let layout = context.layout;

    let result = match task {
        MaskTask::Floors => {
            let mut pixmap = blank_mask(g, "floors")?;
            for &(x, y) in &scene.corridor_cells {
                raster::fill_rect_cell(&mut pixmap, g, x, y, WHITE);
            }
            for room in &layout.rooms {
                raster::fill_rect_room(&mut pixmap, g, room, WHITE);
            }
            ("floors_mask", pixmap)
        }
        MaskTask::Walls => {
            let mut pixmap = blank_mask(g, "walls")?;
            raster::draw_cell_walls(
                &mut pixmap,
                g,
                &scene.corridor_cells,
                Some(&scene.corridor_edges),
                &scene.door_edges,
                context.wall_width,
                WHITE,
            );
            raster::draw_cell_walls(
                &mut pixmap,
                g,
                &scene.room_cells,
                Some(&scene.room_edges),
                &scene.door_edges,
                context.wall_width,
                WHITE,
            );
            ("walls_mask", pixmap)
        }
        MaskTask::Doors => {
            let mut pixmap = blank_mask(g, "doors")?;
            for door in &layout.doors {
                if !door.secret && !door.archway && !door.locked {
                    raster::draw_door(
                        &mut pixmap,
                        g,
                        door,
                        context.door_width,
                        WHITE,
                        DoorStyle::Solid,
                    );
                }
            }
            ("doors_mask", pixmap)
        }
        MaskTask::Archways => {
            let mut pixmap = blank_mask(g, "archways")?;
            for door in &layout.doors {
                if door.archway {
                    raster::draw_door(
                        &mut pixmap,
                        g,
                        door,
                        context.door_width,
                        WHITE,
                        DoorStyle::Solid,
                    );
                }
            }
            ("archways_mask", pixmap)
        }
        MaskTask::LockedDoors => {
            let mut pixmap = blank_mask(g, "locked doors")?;
            for door in &layout.doors {
                if door.locked {
                    raster::draw_door(
                        &mut pixmap,
                        g,
                        door,
                        context.door_width,
                        WHITE,
                        DoorStyle::Solid,
                    );
                }
            }
            ("locked_doors_mask", pixmap)
        }
        MaskTask::SecretDoors => {
            let mut pixmap = blank_mask(g, "secret doors")?;
            for door in &layout.doors {
                if door.secret {
                    raster::draw_door(
                        &mut pixmap,
                        g,
                        door,
                        context.door_width,
                        WHITE,
                        DoorStyle::Solid,
                    );
                }
            }
            ("secret_doors_mask", pixmap)
        }
        MaskTask::Windows => {
            let mut pixmap = blank_mask(g, "windows")?;
            for window in &layout.windows {
                raster::draw_window(
                    &mut pixmap,
                    g,
                    window,
                    context.door_width,
                    WHITE,
                    DoorStyle::Solid,
                );
            }
            ("windows_mask", pixmap)
        }
        MaskTask::StartMarkers => {
            let mut pixmap = blank_mask(g, "start markers")?;
            fill_marker_mask(&mut pixmap, g, &layout.start_markers);
            ("start_markers_mask", pixmap)
        }
        MaskTask::EndMarkers => {
            let mut pixmap = blank_mask(g, "end markers")?;
            fill_marker_mask(&mut pixmap, g, &layout.end_markers);
            ("end_markers_mask", pixmap)
        }
        MaskTask::TrapMarkers => {
            let mut pixmap = blank_mask(g, "trap markers")?;
            fill_marker_mask(&mut pixmap, g, &layout.trap_markers);
            ("trap_markers_mask", pixmap)
        }
        MaskTask::MonsterMarkers => {
            let mut pixmap = blank_mask(g, "monster markers")?;
            fill_marker_mask(&mut pixmap, g, &layout.monster_markers);
            ("monster_markers_mask", pixmap)
        }
        MaskTask::Stairs => {
            let mut pixmap = blank_mask(g, "stairs")?;
            for staircase in &layout.stairs {
                raster::fill_rect_staircase(&mut pixmap, g, staircase, WHITE);
            }
            ("stairs_mask", pixmap)
        }
        MaskTask::Grid => {
            let mut pixmap = blank_mask(g, "grid")?;
            raster::draw_grid_color(&mut pixmap, g, WHITE);
            ("grid_mask", pixmap)
        }
    };

    Ok(result)
}

// Creates an empty pixmap for a named export mask.
fn blank_mask(g: &ExportGeometry, name: &str) -> Result<Pixmap, String> {
    let mut pixmap = Pixmap::new(g.width, g.height)
        .ok_or_else(|| format!("Failed to allocate {name} mask canvas"))?;
    raster::fill_bg(&mut pixmap, BLACK);
    Ok(pixmap)
}

// Fills marker mask.
fn fill_marker_mask(
    pixmap: &mut Pixmap,
    g: &ExportGeometry,
    markers: &[crate::layout::AreaMarker],
) {
    for marker in markers {
        raster::fill_rect_marker(pixmap, g, marker, WHITE);
    }
}

// Saves mask image.
fn save_mask_image(
    folder: &std::path::Path,
    stem: &str,
    pixmap: &Pixmap,
    settings: &UiSettings,
) -> Result<(), String> {
    let ext = settings.mask_format.extension();
    let path = folder.join(format!("{stem}.{ext}"));
    let img = raster::raster_mask_image_from_pixmap(pixmap, settings)?;
    img.save_with_format(&path, mask_image_format(settings.mask_format))
        .map_err(|e| format!("Failed writing {}: {e}", path.display()))?;
    Ok(())
}

// Selects image format.
fn mask_image_format(format: MaskFormat) -> ImageFormat {
    match format {
        MaskFormat::Png => ImageFormat::Png,
        MaskFormat::Jpeg => ImageFormat::Jpeg,
        MaskFormat::Webp => ImageFormat::WebP,
    }
}
