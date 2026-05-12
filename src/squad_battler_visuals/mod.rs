//! Bevy client for the squad battler.
//!
//! The squad battler rules stay in `crate::squad_battler`; this module is only
//! responsible for rendering and animation.

pub mod app;
pub mod assets;
pub mod board;
pub mod camera;
pub mod combat_fx;
pub mod fight_preview;
pub mod hud;
pub mod rewards;
pub mod roster_ui;
pub mod route;
pub mod units;

pub use app::run;
