use eframe::egui::IconData;
use std::sync::Arc;

pub fn app_icon() -> Option<Arc<IconData>> {
    eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon_sim_gui.png"))
        .ok()
        .map(Arc::new)
}
