# Desktop Dungeon Generator

Desktop Dungeon Generator is a Rust desktop app for building and exporting simple tabletop RPG dungeon layouts.

It combines deterministic generation with manual editing, so you can generate a baseline layout from a seed and then adjust rooms, corridors, doors, text labels, and start/end markers directly on the canvas.

## Current Features

- Deterministic dungeon generation from a master seed
- Room generation with configurable:
  - room count
  - minimum and maximum room size
  - square-only rooms
  - packed-room mode without corridors
- Corridor generation with configurable:
  - minimum and maximum corridor width
  - corridor randomness
  - dead-end room percentage
- Automatic door generation with configurable:
  - overall frequency
  - room/hallway chance
  - locked door chance
  - secret door chance
  - middle-corridor doors toggle
- Automatic window generation with configurable:
  - enable/disable
  - minimum and maximum width
  - overall frequency
  - room/hallway chance
  - internal windows toggle
- Generated start/end markers with configurable:
  - minimum and maximum size
  - minimum and maximum count
  - placement inside rooms
  - farthest-pair matching between start and end markers
- Manual editing tools for:
  - rooms
  - corridors
  - archways
  - doors
  - locked doors
  - secret doors
  - text labels
  - start markers
  - end markers
- Export support for:
  - `.png`
  - `.jpeg`
  - `.webp`
  - `.svg`
  - composite folder export with masks
- Accessibility mode with alternate visual encodings for rooms, corridors, walls, doors, archways, and windows

## Tabs

### Generate

Controls global generation and export:

- grid columns and rows
- master seed and random-seed button
- reset and clear actions
- export format, dimensions, and grid visibility

### Layout

Controls room, corridor, door, and window generation behavior.

Changing values in this tab regenerates the dungeon immediately.

### Start & End

Controls generated start/end marker behavior:

- minimum and maximum start marker size
- minimum and maximum end marker size
- minimum and maximum start marker count
- minimum and maximum end marker count

Changing values in this tab regenerates the dungeon immediately.

Generated markers are placed inside rooms. Matching start/end indices are paired to rooms that are as far apart as possible.

### Add

Contains manual placement tools for editing the current layout without regenerating it.

## Canvas Interaction

- Left click and drag a room to move it
- Left click and drag a start/end marker to move it
- Right click and drag a room to resize it
- Right click without dragging to cancel an active add tool
- `Delete` or `Backspace` removes the currently hovered room, corridor, door, text label, or marker
- `Ctrl+Z` undoes
- `Ctrl+Y` redoes

## Start / End Markers

The app currently supports two marker workflows:

- Generated markers:
  - created automatically during dungeon generation
  - always placed inside rooms
  - sized from the configured min/max ranges
  - paired by distance for each matching start/end index
- Manual markers:
  - added from the `Add` tab
  - use the configured minimum size for their marker type
  - can be repositioned by dragging on the canvas

## Folder Export Masks

Folder export writes a composite image plus separate masks for:

- floors
- walls
- doors
- archways
- locked doors
- secret doors
- windows
- start markers
- end markers
- grid, when enabled

## Run

```bash
cargo run
```

## Notes

- Settings are persisted between runs using the local OS application data directory.
- The app is still pre-release and the file layout and feature set may continue to change.
