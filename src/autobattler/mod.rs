pub mod app;
pub mod args;
pub mod constants;
pub mod logic;
pub mod persistence;
pub mod render;
pub mod screenshot;
pub mod sprite_review;
pub mod state;
pub mod ui;

pub use app::AutobattlerApp;
pub use args::AutobattlerArgs;
pub use constants::*;
pub use render::{setup_render_system, sync_render_system, RenderAssets, RenderConfig};
pub use screenshot::{
    HeadlessConfig, HeadlessRenderTarget, HeadlessScreenshotPlugin, ScreenshotState,
};
pub use sprite_review::sprite_review_system;
pub use state::{AutobattlerState, SpriteReviewState};
pub use ui::ui_system;

pub fn run() {
    app::run_app();
}
