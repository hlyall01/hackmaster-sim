//! Shared library entrypoint for game rules and data.

pub mod character;
pub mod core;
pub mod data;
pub mod game_logic;
pub mod assets;
#[cfg(feature = "bevy")]
pub mod autobattler;
pub mod console;
pub mod sim;
