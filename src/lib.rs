//! Shared library entrypoint for game rules and data.

pub mod assets;
pub mod character;
pub mod console;
pub mod core;
pub mod data;
pub mod game_logic;
pub mod sim;
pub mod ui_widgets;

#[cfg(target_arch = "wasm32")]
pub mod web_progress;

#[cfg(test)]
pub(crate) mod test_support;
