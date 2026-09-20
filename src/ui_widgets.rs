use eframe::egui;
use std::hash::Hash;

pub fn searchable_select<T, I>(
    ui: &mut egui::Ui,
    id_source: impl Hash,
    selected_text: impl Into<egui::WidgetText>,
    selected: &mut T,
    options: I,
) -> bool
where
    T: Clone + PartialEq + 'static,
    I: IntoIterator<Item = (T, String, bool)>,
{
    let combo_id = egui::Id::new(id_source);
    let button_id = ui.make_persistent_id(egui::Id::new(combo_id));
    let popup_id = button_id.with("popup");
    let filter_id = combo_id.with("search_filter");
    let search_input_id = popup_id.with("search_input");
    let was_popup_open = ui.memory(|memory| memory.is_popup_open(popup_id));
    let mut changed = false;
    let mut selected_in_frame = false;
    let mut keep_popup_open = false;
    let options: Vec<(T, String, bool)> = options.into_iter().collect();
    egui::ComboBox::from_id_source(combo_id)
        .selected_text(selected_text)
        .show_ui(ui, |ui| {
            let mut filter = ui
                .data_mut(|data| data.get_persisted::<String>(filter_id))
                .unwrap_or_default();
            let response = ui.add(
                egui::TextEdit::singleline(&mut filter)
                    .id(search_input_id)
                    .hint_text("Type to filter"),
            );
            if !was_popup_open {
                response.request_focus();
                keep_popup_open = true;
            }
            if response.clicked() || response.changed() || response.has_focus() {
                keep_popup_open = true;
            }
            if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                filter.clear();
            }
            ui.data_mut(|data| data.insert_persisted(filter_id, filter.clone()));

            let filter = filter.trim().to_ascii_lowercase();
            let mut any_match = false;
            egui::ScrollArea::vertical()
                .max_height(220.0)
                .show(ui, |ui| {
                    for (value, label, enabled) in &options {
                        if !filter.is_empty() && !label.to_ascii_lowercase().contains(&filter) {
                            continue;
                        }
                        any_match = true;
                        let mut clicked = false;
                        ui.add_enabled_ui(*enabled, |ui| {
                            if ui.selectable_label(*selected == *value, label).clicked() {
                                clicked = true;
                            }
                        });
                        if clicked {
                            *selected = value.clone();
                            changed = true;
                            selected_in_frame = true;
                            ui.close_menu();
                        }
                    }
                });
            if !any_match {
                ui.label("No matches.");
            }
        });
    if ui.memory(|memory| memory.has_focus(search_input_id)) {
        keep_popup_open = true;
    }
    if keep_popup_open && !selected_in_frame {
        ui.memory_mut(|memory| {
            memory.open_popup(popup_id);
            memory.request_focus(search_input_id);
        });
    }
    changed
}
