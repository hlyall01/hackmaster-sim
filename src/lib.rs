//! Shared library entrypoint for game rules and data.

pub mod assets;
#[cfg(feature = "bevy")]
pub mod autobattler;
pub mod character;
pub mod console;
pub mod core;
pub mod data;
pub mod game_logic;
pub mod sim;
