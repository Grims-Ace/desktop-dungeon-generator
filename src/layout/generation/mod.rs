/*
 * Procedural generation module facade.
 * Re-exports room placement, passage, marker, and orchestration helpers.
 */

pub mod connections;
pub mod markers;
pub mod orchestration;
pub mod passages;
pub mod room_placement;

pub use connections::*;
pub use markers::*;
pub use orchestration::*;
pub use passages::*;
pub use room_placement::*;
