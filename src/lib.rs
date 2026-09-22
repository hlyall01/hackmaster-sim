//! Shared library entrypoint for game rules and data.

pub mod assets;
pub mod character;
pub mod console;
pub mod core;
pub mod data;
pub mod game_logic;
pub mod sim;
pub mod ui_widgets;

#[cfg(test)]
pub(crate) mod test_support;
