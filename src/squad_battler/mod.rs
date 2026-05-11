//! Squad-based roguelite autobattler domain.
//!
//! This module intentionally stays separate from `sim_gui` and the existing
//! duel-oriented browser demo. It can reuse shared HackMaster catalogs and
//! combat helpers, but owns squad roster, route, and tactical battle state.

pub mod combat;
pub mod encounters;
pub mod rewards;
pub mod roster;
pub mod state;
pub mod view;
