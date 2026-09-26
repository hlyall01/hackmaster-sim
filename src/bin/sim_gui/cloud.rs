use super::*;
use hackmaster_sim::character_document::CharacterDocument;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(inline_js = "
export function cloud_snapshot(slot) { return window.hackmasterCloud.snapshot(slot); }
export function cloud_action(slot, action, id) { window.hackmasterCloud.action(slot, action, id); }
export function cloud_enabled() { return !!window.hackmasterCloud; }
export function cloud_take_loads() { return window.hackmasterCloud.takeLoads(); }
export function cloud_publish(slot, text) { window.hackmasterCloud.publish(slot, text); }
export function cloud_loaded(slot, error) { window.hackmasterCloud.loaded(slot, error); }
export function cloud_error(message) { window.hackmasterCloud.error(message); }
")]
extern "C" {
    fn cloud_enabled() -> bool;
    fn cloud_snapshot(slot: usize) -> String;
    fn cloud_action(slot: usize, action: &str, id: &str);
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

#[derive(serde::Deserialize)]
struct OnlineCharacter { id: String, name: String, is_owner: bool, can_edit: bool }
#[derive(serde::Deserialize)]
struct OnlineSnapshot {
    signed_in: bool, guest: bool, busy: bool, status: String, admin: bool,
    characters: Vec<OnlineCharacter>, loaded_id: String, loaded_name: String,
    can_save: bool, dirty: bool,
}

pub(super) fn show_core(ui: &mut egui::Ui, slot: usize) {
    if !cloud_enabled() { return; }
    let Ok(state) = serde_json::from_str::<OnlineSnapshot>(&cloud_snapshot(slot)) else { return; };
    ui.group(|ui| {
        ui.strong("Online Characters");
        if !state.signed_in && !state.guest {
            ui.label("Sign in or continue without logging in to use Party Members.");
            if ui.button("Sign in / Continue as guest").clicked() { cloud_action(slot, "manage", ""); }
            return;
        }
        ui.add_enabled_ui(!state.busy, |ui| {
            for (mine, label) in [(true, "My Characters"), (false, "Party Members")] {
                ui.add_enabled_ui(!mine || state.signed_in, |ui| { ui.horizontal(|ui| {
                    ui.label(label);
                    let items: Vec<_> = state.characters.iter().filter(|c| c.is_owner == mine).collect();
                    let selected = items.iter().find(|c| c.id == state.loaded_id)
                        .map(|c| c.name.as_str()).unwrap_or("Choose character…");
                    egui::ComboBox::from_id_source(("online", slot, mine)).selected_text(selected)
                        .width(210.0).show_ui(ui, |ui| {
                            if items.is_empty() { ui.label(if mine { "No characters created yet" } else { "No party characters yet" }); }
                            for row in items {
                                let text = if !mine && row.can_edit { format!("{} (editable)", row.name) } else { row.name.clone() };
                                if ui.selectable_label(row.id == state.loaded_id, text).clicked() {
                                    cloud_action(slot, "load", &row.id);
                                }
                            }
                        });
                }); });
            }
            ui.horizontal_wrapped(|ui| {
                if state.signed_in {
                if ui.add_enabled(state.can_save, egui::Button::new("Save online")).clicked() { cloud_action(slot, "save", ""); }
                if ui.button("Create my character").on_hover_text("Save the current fighter as a new character you own. Other signed-in players can load it for simulations.").clicked() { cloud_action(slot, "create", ""); }
                } else if ui.button("Sign in with Google").clicked() { cloud_action(slot, "signin", ""); }
                if ui.button("Refresh").clicked() { cloud_action(slot, "refresh", ""); }
                if state.signed_in && ui.button("Drafts / export").clicked() { cloud_action(slot, "manage", ""); }
                if state.admin && ui.button("Admin assignments").clicked() { cloud_action(slot, "admin", ""); }
            });
        });
        if !state.loaded_name.is_empty() {
            ui.small(format!("{} · {}{}", state.loaded_name, if state.can_save { "Editable" } else { "Simulation copy" }, if state.dirty { " · Unsaved changes" } else { "" }));
        }
        ui.small(&state.status);
    });
    ui.separator();
}
