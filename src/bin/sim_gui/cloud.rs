use super::*;
use hackmaster_sim::character_document::CharacterDocument;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(inline_js = "
export function cloud_enabled() { return !!window.hackmasterCloud; }
export function cloud_take_loads() { return window.hackmasterCloud.takeLoads(); }
export function cloud_publish(slot, text) { window.hackmasterCloud.publish(slot, text); }
export function cloud_loaded(slot, error) { window.hackmasterCloud.loaded(slot, error); }
export function cloud_error(message) { window.hackmasterCloud.error(message); }
")]
extern "C" {
    fn cloud_enabled() -> bool;
    fn cloud_take_loads() -> String;
    fn cloud_publish(slot: usize, text: &str);
    fn cloud_loaded(slot: usize, error: &str);
    fn cloud_error(message: &str);
}

#[derive(serde::Deserialize)]
struct Load { slot: usize, document: CharacterDocument }

#[derive(Default)]
pub(super) struct CloudState { published: [Option<PlayerConfig>; 2] }

impl SimGuiApp {
    pub(super) fn update_cloud(&mut self, ctx: &egui::Context) {
        if !cloud_enabled() { return; }
        // The HTML management controls can enqueue a load without an egui input event.
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
        let pending: Vec<serde_json::Value> = serde_json::from_str(&cloud_take_loads()).unwrap_or_default();
        for value in pending {
            let slot = value.get("slot").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            if slot >= self.players.len() { continue; }
            let result = serde_json::from_value::<Load>(value).map_err(|e| e.to_string()).and_then(|load| {
                if load.slot != slot { return Err("Invalid character slot".into()); }
                load.document.restore(&self.weapon_catalog, &self.armor_catalog, &self.shield_catalog, &self.npc_presets)
            });
            match result {
                Ok(player) => {
                    self.running = false;
                    self.players[slot] = player;
                    self.tactical_drafts[slot] = self.players[slot].tactical_policy.clone();
                    self.fighter_preset_names[slot] = self.players[slot].name.clone();
                    self.derived_cache = [None, None];
                    self.cloud.published[slot] = None;
                    self.show_player_editor[slot] = true;
                    cloud_loaded(slot, "");
                }
                Err(error) => cloud_loaded(slot, &format!("Character was not loaded: {error}")),
            }
        }
        for slot in 0..self.players.len() {
            if self.cloud.published[slot].as_ref() == Some(&self.players[slot]) { continue; }
            match CharacterDocument::capture(&self.players[slot], &self.weapon_catalog, &self.armor_catalog, &self.shield_catalog, &self.npc_presets)
                .and_then(|doc| serde_json::to_string(&doc).map_err(|e| e.to_string())) {
                Ok(text) => {
                    cloud_publish(slot, &text);
                    self.cloud.published[slot] = Some(self.players[slot].clone());
                }
                Err(error) => cloud_error(&format!("Local draft could not be captured: {error}")),
            }
        }
    }
}
