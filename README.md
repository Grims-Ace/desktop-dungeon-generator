# Desktop Dungeon Generator

**Pre-release Notice:** This program is incomplete and functionality is subject to change and it might break, you've been warned.

A Rust desktop app for generating simple tabletop dungeon layouts.

## Current Features

- Resizable split layout:
  - Left panel: generation settings
  - Right panel: vector-rendered grid + dungeon preview
- Grid controls:
  - Columns
  - Rows
- Room controls:
  - Room count
  - Minimum room size (in grid cells)
  - Maximum room size (in grid cells)
  - `Square Rooms Only` toggle
- Corridor controls:
  - `Corridor Randomness (%)` from `0` to `100`
  - `Dead-End Rooms (%)` from `0` to `50`
- Door controls:
  - `Door Frequency (%)`
  - `Room/Hallway Door Chance (%)`
  - `Locked Door Chance (%)`
  - `Allow Middle Corridor Doors` toggle
- Accessibility:
  - `Colorblind Mode` toggle (right panel)
  - Pattern/style encoding:
    - Rooms: crosshatch
    - Corridors: dotted fill
    - Walls: solid lines
    - Doors: long dashes
    - Locked doors: short dashes
    - Archways: dotted lines
- Generate control:
  - Master seed input (`u64`)
  - `Random` seed button
  - `Reset` button
  - `Export Image` button
  - Export format selector: `.png`, `.jpeg`, `.webp`, `.svg`
  - Export uses an OS folder picker to choose save location

## Generation Behavior

- Rooms are stored as rectangles (`x`, `y`, `width`, `height`) and placed without overlap.
- Corridors are stored separately as vector line segments between grid cells.
- Every generated room is connected into one navigable network when possible.
- A master seed drives deterministic generation, and internal subsystems derive their own sub-seeds from it.
- Doors are generated from corridor adjacency:
  - Open doors are rendered in red
  - Locked doors are rendered in green
  - Doors are rendered as line segments on grid boundaries

### Corridor Randomness

- `0%` randomness:
  - Uses shortest-path routing for room-to-room corridor paths.
  - Produces cleaner, direct corridor routes.
- `100%` randomness:
  - Uses noisy/randomized routing while still ensuring room connectivity.
  - Produces more wandering, less direct routes.
- Intermediate values blend between the two behaviors.

### Dead-End Rooms

- Controls how many rooms should end up as dead ends (rooms with only one room-to-room connection).
- `0%` targets no dead-end rooms (except tiny edge cases where topology makes this impossible, such as only 2 rooms).
- `50%` is the upper cap to avoid over-constraining connectivity.

## Run

```bash
cargo run
```

## Notes

- Rendering is vector-based in-app (`egui` painter), so zooming/scaling the preview remains crisp.
- Room and corridor vectors are stored separately to support future export workflows (for example, SVG export).
- Settings are persisted on exit and restored on startup from:
  - Windows: `%LOCALAPPDATA%/desktop_dungeon_generator/settings.json`
  - Linux: `~/.local/share/desktop_dungeon_generator/settings.json` (or distro-equivalent local data dir)
