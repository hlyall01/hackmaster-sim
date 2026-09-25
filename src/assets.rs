#[cfg(not(target_arch = "wasm32"))]
use eframe::egui::IconData;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
pub fn app_icon() -> Option<Arc<IconData>> {
    eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon_sim_gui.png"))
        .ok()
        .map(Arc::new)
}
