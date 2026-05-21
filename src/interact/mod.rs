/*
 * Main interaction module facade.
 * Re-exports interaction submodules for dragging, resizing, hit-testing, and editing.
 */

pub use crate::exporter::types::{
    door_edges_for, door_render_width, norm_edge as normalized_edge, window_render_width,
};

pub mod add_delete;
pub mod drag;
pub mod geometry;
pub mod hit_test;
pub mod multi_select;
pub mod pathing;
pub mod resize;
pub mod types;

pub use add_delete::*;
pub use drag::*;
pub use geometry::*;
pub use hit_test::*;
pub use multi_select::*;
pub use pathing::*;
pub use resize::*;
pub use types::*;
