//! Platform-independent part of native Pocket Shelf.
//!
//! The app draws into a [`gfx::Gfx`]: a list of commands (filled rectangles,
//! tinted texture parts, already cut to any clip) and the textures they read. A backend
//! uploads the textures and runs the commands: vita2d on the console, a
//! software rasterizer on the desktop for screenshots. Everything above those
//! two primitives (rounded shapes, shadows, button symbols, text) is built
//! here, so both backends draw the same pixels.
//!
//! The app talks to the platform through [`host::Host`].

pub mod app;
pub mod gfx;
pub mod host;
pub mod i18n;
pub mod library;
pub mod locales;
pub mod online;
pub mod shapes;
pub mod shelf;
pub mod stats;
pub mod text;
pub mod theme;
pub mod updates;

/// Logical screen size: the current UI's layout units. The Vita draws them
/// at 2× (960×544).
pub const W: f32 = 480.0;
pub const H: f32 = 272.0;
