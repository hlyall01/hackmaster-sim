use super::*;
use std::sync::Arc;

#[derive(Default)]
pub(super) struct CombatLogView {
    names: Vec<(String, String, Option<String>)>,
    layout: Option<(
        egui::FontId,
        Option<egui::FontId>,
        Option<egui::TextStyle>,
        Color32,
        f32,
        f32,
        f32,
        bool,
    )>,
    rows: Vec<Arc<egui::Galley>>,
    offsets: Vec<f32>,
}

impl CombatLogView {
    pub fn clear(&mut self) {
        self.rows.clear();
        self.offsets.clear();
    }

    pub fn show(&mut self, ui: &mut egui::Ui, sim: &SimState) {
        egui::ScrollArea::vertical()
            .max_height(180.0)
            .show_viewport(ui, |ui, _viewport| {
                let width = ui.available_width();
                let spacing = ui.spacing().item_spacing.y;
                let layout = (
                    egui::TextStyle::Body.resolve(ui.style()),
                    ui.style().override_font_id.clone(),
                    ui.style().override_text_style.clone(),
                    ui.visuals().text_color(),
                    width,
                    ui.ctx().pixels_per_point(),
                    spacing,
                    ui.wrap_text(),
                );
                let names_match = self.names.len() == sim.combatants.len()
                    && self.names.iter().zip(&sim.combatants).all(
                        |((name, weapon, offhand), actor)| {
                            name == &actor.sheet.name
                                && weapon == &actor.sheet.offense.weapon.name
                                && offhand.as_deref()
                                    == actor
                                        .sheet
                                        .offense
                                        .offhand
                                        .as_ref()
                                        .map(|w| w.weapon.name.as_str())
                        },
                    );
                if !names_match {
                    self.names = sim
                        .combatants
                        .iter()
                        .map(|actor| {
                            (
                                actor.sheet.name.clone(),
                                actor.sheet.offense.weapon.name.clone(),
                                actor
                                    .sheet
                                    .offense
                                    .offhand
                                    .as_ref()
                                    .map(|w| w.weapon.name.clone()),
                            )
                        })
                        .collect();
                }
                if !names_match
                    || self.layout.as_ref() != Some(&layout)
                    || self.rows.len() > sim.combat_events.len()
                {
                    self.clear();
                    self.layout = Some(layout);
                }
                if self.offsets.is_empty() {
                    self.offsets.push(0.0);
                }
                // Existing events are immutable within a fight. Resetting the GUI's
                // simulation clears this cache, including resets of the same length.
                for event in &sim.combat_events[self.rows.len()..] {
                    let text = sim::format_combat_event_line(event, &sim.combatants);
                    let galley = egui::WidgetText::from(text).into_galley(
                        ui,
                        None,
                        width,
                        egui::TextStyle::Body,
                    );
                    self.offsets
                        .push(self.offsets.last().copied().unwrap() + galley.size().y + spacing);
                    self.rows.push(galley);
                }
                let total_height = (self.offsets.last().copied().unwrap_or(0.0) - spacing).max(0.0);
                let origin = ui.min_rect().min;
                // egui's clip rectangle can be taller on the first layout pass.
                // Match the labels it would paint while scroll bounds settle.
                let viewport = ui.clip_rect().translate(-origin.to_vec2());
                ui.set_min_height(total_height);
                let first = self
                    .offsets
                    .partition_point(|y| *y < viewport.min.y)
                    .saturating_sub(1);
                let end = self
                    .offsets
                    .partition_point(|y| *y <= viewport.max.y)
                    .min(self.rows.len());
                ui.skip_ahead_auto_ids(first);
                for index in first..end {
                    let rect = Rect::from_min_size(
                        origin + egui::vec2(0.0, self.offsets[index]),
                        egui::vec2(width, self.rows[index].size().y),
                    );
                    ui.allocate_ui_at_rect(rect, |ui| {
                        ui.add(egui::Label::new(self.rows[index].clone()));
                    });
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(output: egui::FullOutput) -> Vec<(String, Pos2)> {
        output
            .shapes
            .into_iter()
            .filter_map(|shape| match shape.shape {
                egui::Shape::Text(text) => Some((text.galley.text().to_owned(), text.pos)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn cached_log_matches_wrapped_labels_when_scrolling_resizing_and_resetting() {
        let mut sim = SimState::new(SimConfig::new(5.0, 1.0));
        sim.reset_with_combatants(vec![sim::Combatant::default()]);
        for time in 0..500 {
            sim.combat_events.push(sim::CombatEvent {
                time,
                attacker_idx: 0,
                defender_idx: 0,
                kind: sim::CombatEventKind::Tactical(sim::TacticalEvent {
                    rule_index: None,
                    action: "Attack".into(),
                    message: "A wrapped combat event with enough text to span several lines. "
                        .repeat((time % 3 + 1) as usize),
                }),
            });
        }
        let reference = egui::Context::default();
        let optimized = egui::Context::default();
        let mut view = CombatLogView::default();
        for frame in 0..12 {
            if frame == 9 {
                sim.reset();
                view.clear();
            }
            let mut input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    Pos2::ZERO,
                    egui::vec2(if frame < 6 { 320.0 } else { 220.0 }, 300.0),
                )),
                ..Default::default()
            };
            input
                .events
                .push(egui::Event::PointerMoved(Pos2::new(100.0, 100.0)));
            if frame == 3 {
                input
                    .events
                    .push(egui::Event::Scroll(egui::vec2(0.0, -450.0)));
            }
            let original = reference.run(input.clone(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(180.0)
                        .show(ui, |ui| {
                            for event in &sim.combat_events {
                                ui.label(sim::format_combat_event_line(event, &sim.combatants));
                            }
                        });
                });
            });
            let changed = optimized.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| view.show(ui, &sim));
            });
            assert_eq!(texts(changed), texts(original), "frame {frame}");
        }
    }
}
