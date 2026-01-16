use std::sync::Arc;

use eframe::egui::IconData;

#[derive(Clone, Copy)]
pub enum AppIcon {
    SimGui,
    WeaponPlot,
}

pub fn app_icon(icon: AppIcon) -> Option<Arc<IconData>> {
    let bytes: &[u8] = match icon {
        AppIcon::SimGui => include_bytes!("../assets/icon_sim_gui.png"),
        AppIcon::WeaponPlot => include_bytes!("../assets/icon_weapon_plot.png"),
    };
    match eframe::icon_data::from_png_bytes(bytes) {
        Ok(icon) => Some(Arc::new(icon)),
        Err(_) => None,
    }
}
