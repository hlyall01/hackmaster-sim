use bevy::prelude::*;
use bevy_egui::egui::{self, Color32, Pos2, Rect as EguiRect};
use bevy_egui::EguiContexts;

use crate::autobattler::app::AutobattlerApp;
use std::collections::BTreeMap;

use crate::autobattler::constants::{
    LOG_DISPLAY_LIMIT, RUN_PANEL_WIDTH, START_AP, START_BP, START_LP, START_RP, STAT_LABELS,
    STAT_COUNT, SUMMARY_PANEL_WIDTH, TALENT_TAB_ALL, TALENT_TAB_RACIALS, WEAPON_GROUP_LABELS,
};
use crate::autobattler::logic::{
    apply_percentile, bp_increment, format_percentile, format_score, max_affordable_rank,
    race_adjustment_summary, starting_honor, stat_at_cap, subtract_percentile,
    talent_cost_for_rank, talent_display_label,
};
use crate::autobattler::screenshot::ScreenshotState;
use crate::autobattler::state::{AppScreen, DamageFloat, LiveFight, PointPool, RunAction, RunViewState};
use crate::autobattler::state::{AutobattlerState, CreationStep};
use crate::character::InitiativeDieQuality;
use crate::core::types::{RaceSpec, TalentSelection, TalentSpec};
use crate::core::rules::roll_damage_expr;
use crate::character::WeaponGroup;
use crate::game_logic::{self, PlayerConfig, TalentCatalog, WeaponCatalog};
use crate::sim;

pub fn ui_system(
    mut contexts: EguiContexts,
    time: Res<Time>,
    mut state: ResMut<AutobattlerState>,
    mut screenshots: ResMut<ScreenshotState>,
) {
    let dt = time.delta_seconds();
    state
        .app
        .update_ui(contexts.ctx_mut(), dt, &mut screenshots);
}

impl AutobattlerApp {
    pub fn update_ui(&mut self, ctx: &egui::Context, dt: f32, screenshots: &mut ScreenshotState) {
        match self.screen {
            AppScreen::Start => {
                if self.needs_save_refresh {
                    self.refresh_saves();
                    self.needs_save_refresh = false;
                }
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.heading("Autobattler");
                    ui.separator();
                    ui.label("Load an existing character or start a new one.");
                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui.button("New character").clicked() {
                            self.start_new_character();
                        }
                    });

                    ui.separator();
                    ui.label("Saved characters");
                    if self.save_entries.is_empty() {
                        ui.label("No saves found.");
                    } else {
                        egui::ScrollArea::vertical()
                            .max_height(240.0)
                            .show(ui, |ui| {
                                for (idx, entry) in self.save_entries.iter().enumerate() {
                                    let selected = self.selected_save == Some(idx);
                                    let label =
                                        format!("{} ({})", entry.display_name, entry.file_name);
                                    if ui.selectable_label(selected, label).clicked() {
                                        self.selected_save = Some(idx);
                                    }
                                }
                            });
                    }
                    ui.horizontal(|ui| {
                        let can_load = self.selected_save.is_some();
                        if ui.add_enabled(can_load, egui::Button::new("Load selected")).clicked() {
                            self.load_selected_character();
                        }
                    });
                    if let Some(status) = self.save_status.as_ref() {
                        ui.separator();
                        ui.label(status);
                    }

                    ui.separator();
                    ui.label("Saved runs");
                    if self.run_save_entries.is_empty() {
                        ui.label("No run saves found.");
                    } else {
                        egui::ScrollArea::vertical()
                            .max_height(240.0)
                            .show(ui, |ui| {
                                for (idx, entry) in self.run_save_entries.iter().enumerate() {
                                    let selected = self.selected_run_save == Some(idx);
                                    let label =
                                        format!("{} ({})", entry.display_name, entry.file_name);
                                    if ui.selectable_label(selected, label).clicked() {
                                        self.selected_run_save = Some(idx);
                                    }
                                }
                            });
                    }
                    ui.horizontal(|ui| {
                        let can_load = self.selected_run_save.is_some();
                        if ui.add_enabled(can_load, egui::Button::new("Load run")).clicked() {
                            self.load_selected_run();
                        }
                    });
                    if let Some(status) = self.run_save_status.as_ref() {
                        ui.separator();
                        ui.label(status);
                    }
                });
            }
            AppScreen::Creation => {
                let available_points = self.available_points();
                let (effective_cha, looks_delta) = self.effective_charisma();
                render_character_summary(
                    ctx,
                    &self.creation,
                    &self.race_catalog,
                    &self.talent_catalog,
                    available_points,
                    effective_cha,
                    looks_delta,
                    None,
                );

                egui::TopBottomPanel::top("creation_header").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Autobattler Character Creation");
                        ui.separator();
                        let step_number = self.creation_step.index() + 1;
                        let step_total = CreationStep::count();
                        ui.label(format!("Step {step_number} of {step_total}"));
                        ui.separator();
                        ui.label(self.creation_step.title());
                        if self.creation_done {
                            ui.separator();
                            ui.label("Creation complete");
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Reset to start").clicked() {
                                self.reset_creation();
                                self.screen = AppScreen::Start;
                                self.needs_save_refresh = true;
                            }
                        });
                    });
                });

                egui::CentralPanel::default().show(ctx, |ui| {
                    let step = self.creation_step;
                    let available_height = ui.available_height();
                    match step {
                        CreationStep::Points => {
                            ui.heading("Starting Points and Name");
                            ui.separator();
                            ui.horizontal(|ui| {
                                ui.label("Name");
                                let response = ui.text_edit_singleline(&mut self.creation.name);
                                if response.changed() {
                                    self.creation.sync_player_from_stats();
                                }
                            });
                            ui.separator();
                            ui.horizontal(|ui| {
                                ui.label("Run seed");
                                let mut seed_value =
                                    self.run_seed.min(i64::MAX as u64) as i64;
                                let response =
                                    ui.add(egui::DragValue::new(&mut seed_value).speed(1.0));
                                if response.changed() {
                                    self.run_seed = seed_value.max(0) as u64;
                                    self.seed_dirty = true;
                                }
                                if ui
                                    .add_enabled(self.seed_dirty, egui::Button::new("Apply seed"))
                                    .clicked()
                                {
                                    self.creation.reseed(self.run_seed);
                                    self.seed_dirty = false;
                                }
                            });
                            if self.seed_dirty {
                                ui.label("Apply the seed to reroll ability sets.");
                            } else {
                                ui.label(format!("Seed in use: {}", self.creation.run_seed));
                            }
                            ui.separator();
                            let spent = PointPool::new(START_BP, START_LP, START_AP, START_RP)
                                .sub(available_points);
                            ui.label(format!(
                                "Start: {START_BP} BP, {START_LP} LP, {START_AP} AP, {START_RP} RP"
                            ));
                            ui.label(format!(
                                "Spent: {} BP, {} LP, {} AP, {} RP",
                                spent.bp, spent.lp, spent.ap, spent.rp
                            ));
                            ui.label(format!(
                                "Remaining: {} BP, {} LP, {} AP, {} RP",
                                available_points.bp,
                                available_points.lp,
                                available_points.ap,
                                available_points.rp
                            ));
                        }
                        CreationStep::RollStats => {
                            ui.heading("Roll Ability Scores");
                            ui.separator();
                            ui.horizontal(|ui| {
                                if ui.button("Reroll").clicked() {
                                    self.creation.reset_rolls();
                                }
                            });
                            ui.label("Pick a set, then assign each roll to a stat.");
                            ui.horizontal(|ui| {
                                for set_idx in 0..2 {
                                    ui.group(|ui| {
                                        let label = format!("Set {}", set_idx + 1);
                                        ui.radio_value(
                                            &mut self.creation.selected_set,
                                            set_idx,
                                            label,
                                        );
                                        let set = &self.creation.rolled_sets[set_idx];
                                        for (idx, roll) in set.rolls.iter().enumerate() {
                                            ui.label(format!(
                                                "{}: {}",
                                                idx + 1,
                                                format_score(*roll)
                                            ));
                                        }
                                    });
                                }
                            });

                            ui.separator();
                            ui.label("Assignments");
                            let selected_set =
                                self.creation.rolled_sets[self.creation.selected_set];
                            ui.add_enabled_ui(!self.creation.stats_locked, |ui| {
                                for stat_idx in 0..STAT_COUNT {
                                    ui.horizontal(|ui| {
                                        ui.label(STAT_LABELS[stat_idx]);
                                        let selection = self.creation.assignments[stat_idx];
                                        let selected_text = selection
                                            .map(|idx| format_score(selected_set.rolls[idx]))
                                            .unwrap_or_else(|| "Select roll".to_string());
                                        egui::ComboBox::from_id_source(format!(
                                            "assign_{stat_idx}"
                                        ))
                                        .selected_text(selected_text)
                                        .show_ui(ui, |ui| {
                                            for roll_idx in 0..STAT_COUNT {
                                                let taken_elsewhere = self
                                                    .creation
                                                    .assignments
                                                    .iter()
                                                    .enumerate()
                                                    .any(|(idx, slot)| {
                                                        idx != stat_idx
                                                            && *slot == Some(roll_idx)
                                                    });
                                                let roll = selected_set.rolls[roll_idx];
                                                let label = format_score(roll);
                                                ui.add_enabled_ui(!taken_elsewhere, |ui| {
                                                    if ui
                                                        .selectable_label(
                                                            selection == Some(roll_idx),
                                                            label,
                                                        )
                                                        .clicked()
                                                    {
                                                        self.creation.assign_roll(
                                                            stat_idx, roll_idx,
                                                        );
                                                    }
                                                });
                                            }
                                        });
                                    });
                                }
                            });
                            if self.creation.stats_locked {
                                ui.label("Rolls locked. Use Reroll to generate new sets.");
                            }
                        }
                        CreationStep::ChooseRace => {
                            ui.heading("Choose Race");
                            ui.separator();
                            ui.add_enabled_ui(self.creation.stats_locked, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Race");
                                    let mut selection =
                                        self.creation.race_index.unwrap_or(usize::MAX);
                                    ui.add_enabled_ui(!self.creation.race_applied, |ui| {
                                        egui::ComboBox::from_id_source("race_select")
                                            .selected_text(
                                                self.race_catalog
                                                    .get(selection)
                                                    .map(|race| race.name.as_str())
                                                    .unwrap_or("None"),
                                            )
                                            .show_ui(ui, |ui| {
                                                ui.selectable_value(
                                                    &mut selection,
                                                    usize::MAX,
                                                    "None",
                                                );
                                                for (idx, race) in
                                                    self.race_catalog.iter().enumerate()
                                                {
                                                    ui.selectable_value(
                                                        &mut selection,
                                                        idx,
                                                        race.name.as_str(),
                                                    );
                                                }
                                            });
                                    });
                                    if !self.creation.race_applied {
                                        if selection == usize::MAX {
                                            self.creation.race_index = None;
                                        } else {
                                            self.creation.race_index = Some(selection);
                                        }
                                    }
                                    let can_confirm = self.creation.race_index.is_some()
                                        && !self.creation.race_applied;
                                    if ui
                                        .add_enabled(
                                            can_confirm,
                                            egui::Button::new("Confirm race"),
                                        )
                                        .clicked()
                                    {
                                        if let Some(index) = self.creation.race_index {
                                            if let Some(race) =
                                                self.race_catalog.get(index).cloned()
                                            {
                                                self.apply_race_adjustments(&race);
                                            }
                                        }
                                    }
                                });
                                if let Some(index) = self.creation.race_index {
                                    if let Some(race) = self.race_catalog.get(index) {
                                        ui.label(format!(
                                            "Base HP {} | {}",
                                            race.base_hp,
                                            race_adjustment_summary(race)
                                        ));
                                        if !race.pros.is_empty() {
                                            ui.separator();
                                            ui.label("Pros:");
                                            for entry in &race.pros {
                                                ui.label(format!("- {entry}"));
                                            }
                                        }
                                        if !race.cons.is_empty() {
                                            ui.separator();
                                            ui.label("Cons:");
                                            for entry in &race.cons {
                                                ui.label(format!("- {entry}"));
                                            }
                                        }
                                    }
                                }
                                if self.creation.race_applied {
                                    ui.separator();
                                    ui.label("Race confirmed. To change race, reroll stats.");
                                }
                            });
                        }
                        CreationStep::Alignment => {
                            ui.heading("Choose Alignment");
                            ui.separator();
                            ui.label("Alignment");
                            egui::ComboBox::from_id_source("alignment_select")
                                .selected_text(self.creation.alignment.as_str())
                                .show_ui(ui, |ui| {
                                    for option in ["Unaligned", "Lawful", "Neutral", "Chaotic"] {
                                        ui.selectable_value(
                                            &mut self.creation.alignment,
                                            option.to_string(),
                                            option,
                                        );
                                    }
                                });
                            ui.separator();
                            ui.label("Alignment effects are not implemented yet.");
                        }
                        CreationStep::FinalizeStats => {
                            ui.heading("Finalize Ability Scores");
                            ui.separator();
                            ui.label(format!("Remaining: {} BP", available_points.bp));
                            ui.label(format!(
                                "Current CHA after Looks: {} (Looks {:+})",
                                effective_cha, looks_delta
                            ));
                            ui.label("BP increments: +10 below 10/01, +5 up to 16/01, +3 at 16/01+.");
                            ui.separator();
                            ui.add_enabled_ui(self.creation.race_applied, |ui| {
                                for stat_idx in 0..STAT_COUNT {
                                    let label = STAT_LABELS[stat_idx];
                                    let score = self.creation.stats[stat_idx];
                                    let increment = bp_increment(&score);
                                    let can_add =
                                        available_points.bp > 0 && !stat_at_cap(&score);
                                    let can_remove =
                                        !self.creation.bp_history[stat_idx].is_empty();
                                    let mut remove_clicked = false;
                                    let mut add_clicked = false;
                                    ui.horizontal(|ui| {
                                        ui.label(format!("{label}: {}", format_score(score)));
                                        if ui
                                            .add_enabled(can_remove, egui::Button::new("-1 BP"))
                                            .clicked()
                                        {
                                            remove_clicked = true;
                                        }
                                        if ui
                                            .add_enabled(
                                                can_add,
                                                egui::Button::new(format!(
                                                    "+{} BP",
                                                    increment
                                                )),
                                            )
                                            .clicked()
                                        {
                                            add_clicked = true;
                                        }
                                    });
                                    if remove_clicked {
                                        if let Some(delta) =
                                            self.creation.bp_history[stat_idx].pop()
                                        {
                                            subtract_percentile(
                                                &mut self.creation.stats[stat_idx],
                                                delta,
                                            );
                                            self.creation.sync_player_from_stats();
                                        }
                                    }
                                    if add_clicked {
                                        apply_percentile(
                                            &mut self.creation.stats[stat_idx],
                                            increment,
                                        );
                                        self.creation.bp_history[stat_idx].push(increment);
                                        self.creation.sync_player_from_stats();
                                    }
                                }
                            });
                            if !self.creation.race_applied {
                                ui.label("Confirm race before spending BP.");
                            }
                        }
                        CreationStep::Honor => {
                            ui.heading("Calculate Starting Honor");
                            ui.separator();
                            let breakdown = starting_honor(&self.creation.stats, effective_cha);
                            self.creation.honor = breakdown.total;
                            ui.label(format!("Effective CHA: {}", effective_cha));
                            ui.label(format!("Base honor: {}", breakdown.base));
                            ui.label(format!("Looks mod: {:+}", breakdown.looks_mod));
                            ui.label(format!("CHA mod: {:+}", breakdown.cha_mod));
                            ui.separator();
                            ui.label(format!("Starting honor: {}", breakdown.total));
                            ui.label("Honor effects are not implemented yet.");
                        }
                        CreationStep::Priors => {
                            ui.heading("Priors and Particulars");
                            ui.separator();
                            ui.horizontal(|ui| {
                                ui.label("Background");
                                ui.text_edit_singleline(&mut self.creation.background);
                            });
                            ui.label("Background data is not implemented yet.");
                            ui.separator();
                            ui.horizontal(|ui| {
                                ui.label("Height");
                                ui.text_edit_singleline(&mut self.creation.height);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Weight");
                                ui.text_edit_singleline(&mut self.creation.weight);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Age");
                                ui.text_edit_singleline(&mut self.creation.age);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Handedness");
                                ui.text_edit_singleline(&mut self.creation.handedness);
                            });
                        }
                        CreationStep::QuirksFlaws => {
                            ui.heading("Quirks and Flaws");
                            ui.separator();
                            render_string_list(
                                ui,
                                "Quirks (placeholder)",
                                "Quirk",
                                &mut self.creation.quirk_input,
                                &mut self.creation.quirks,
                            );
                            ui.separator();
                            render_string_list(
                                ui,
                                "Flaws (placeholder)",
                                "Flaw",
                                &mut self.creation.flaw_input,
                                &mut self.creation.flaws,
                            );
                            ui.separator();
                            ui.label("Quirk and flaw effects are not implemented yet.");
                        }
                        CreationStep::AdvancementTalents => {
                            ui.heading("Record Advancement Talents");
                            ui.separator();
                            ui.label("Advancement talent data is not implemented yet.");
                            ui.label("No talents auto-granted.");
                        }
                        CreationStep::SkillsTalents => {
                            ui.heading("Skills, Talents, Proficiencies");
                            ui.separator();
                            render_string_list(
                                ui,
                                "Skills (placeholder)",
                                "Skill",
                                &mut self.creation.skill_input,
                                &mut self.creation.skills,
                            );
                            ui.separator();
                            render_string_list(
                                ui,
                                "Proficiencies (placeholder)",
                                "Proficiency",
                                &mut self.creation.proficiency_input,
                                &mut self.creation.proficiencies,
                            );
                            ui.separator();
                            ui.label("Talents");
                            let max_height = (available_height - 220.0).max(160.0);
                            render_talent_selector(
                                ui,
                                "talent_picker",
                                &mut self.creation.player,
                                &self.weapon_catalog,
                                &self.race_catalog,
                                &self.talent_catalog,
                                &mut self.creation.talent_category,
                                available_points,
                                max_height,
                            );
                        }
                        CreationStep::HitPoints => {
                            ui.heading("Determine Hit Points");
                            ui.separator();
                            let summary = game_logic::player_summary(
                                &self.creation.player,
                                &self.weapon_catalog,
                                &self.armor_catalog,
                                &self.shield_catalog,
                                &self.talent_catalog,
                            );
                            if !self.creation.race_applied {
                                ui.label("Select a race to finalize base HP.");
                            }
                            ui.label(format!("Base HP (race): {}", self.creation.player.base_hp));
                            ui.label(format!("CON: {}", self.creation.stats[4].base));
                            ui.label(format!(
                                "Health multiplier: {:.2}",
                                summary.derived.health_mult
                            ));
                            ui.label(format!("Total HP: {}", summary.derived.hit_points));
                        }
                        CreationStep::DerivedStats => {
                            ui.heading("Record Derived Statistics");
                            ui.separator();
                            let summary = game_logic::player_summary(
                                &self.creation.player,
                                &self.weapon_catalog,
                                &self.armor_catalog,
                                &self.shield_catalog,
                                &self.talent_catalog,
                            );
                            let sprint_duration = (self.creation.stats[4].base / 2) as u32;
                            ui.label(format!("Attack bonus: {}", summary.roll.attack_bonus));
                            ui.label(format!("Base damage: {}", summary.roll.strength_damage));
                            ui.label(format!("Defense (DV): {}", summary.derived.base_dv));
                            ui.label(format!(
                                "Initiative mod: {}",
                                summary.derived.initiative_mod
                            ));
                            ui.label(format!(
                                "Initiative die: {}",
                                initiative_die_label(summary.derived.initiative_die)
                            ));
                            ui.label(format!("Speed mod: {}", summary.derived.speed_mod));
                            ui.label(format!("Armor DR: {}", summary.derived.armor_dr));
                            ui.label(format!("Sprint duration: {} sec", sprint_duration));
                        }
                        CreationStep::MoneyGear => {
                            ui.heading("Money and Gear");
                            ui.separator();
                            ui.label("Starting money roll: 75 + 4d12p");
                            let mut roll_now = false;
                            if !self.creation.money_rolled {
                                if ui.button("Roll starting money").clicked() {
                                    roll_now = true;
                                }
                            } else {
                                ui.label(format!(
                                    "Starting money: {}",
                                    self.creation.starting_money
                                ));
                                if ui.button("Reroll").clicked() {
                                    roll_now = true;
                                }
                            }
                            if roll_now {
                                let roll = roll_damage_expr("4d12p", &mut self.creation.rng, false);
                                let total = 75i32.saturating_add(roll).max(0) as u32;
                                self.creation.starting_money = total;
                                self.creation.money_rolled = true;
                            }
                            let budget = if self.creation.money_rolled {
                                self.creation.starting_money
                            } else {
                                0
                            };
                            let total_cost = self.starter_gear_cost();
                            if self.creation.money_rolled {
                                ui.separator();
                                ui.label(format!("Starting money: {budget} gp"));
                                ui.label(format!("Gear total: {total_cost} gp"));
                                let remaining = budget as i32 - total_cost as i32;
                                if remaining >= 0 {
                                    ui.label(format!("Remaining: {remaining} gp"));
                                } else {
                                    ui.colored_label(
                                        Color32::from_rgb(190, 80, 80),
                                        format!("Over budget by {} gp", -remaining),
                                    );
                                }
                            } else {
                                ui.separator();
                                ui.label("Roll starting money to unlock gear selection.");
                            }

                            ui.separator();
                            ui.add_enabled_ui(self.creation.money_rolled, |ui| {
                                let mut weapon_cost = self
                                    .weapon_catalog
                                    .get(self.creation.player.weapon_id)
                                    .map(|weapon| weapon.price_gp)
                                    .unwrap_or(0);
                                let mut armor_cost = self
                                    .armor_catalog
                                    .get(self.creation.player.armor_id)
                                    .and_then(|entry| entry.armor.as_ref())
                                    .map(|armor| armor.price_gp)
                                    .unwrap_or(0);
                                let mut shield_cost = self
                                    .shield_catalog
                                    .get(self.creation.player.shield_id)
                                    .and_then(|entry| entry.shield.as_ref())
                                    .map(|shield| shield.price_gp)
                                    .unwrap_or(0);

                                ui.label("Weapon");
                                let mut weapon_index =
                                    self.weapon_catalog.index_of(self.creation.player.weapon_id);
                                let weapon_label = self
                                    .weapon_catalog
                                    .get(self.creation.player.weapon_id)
                                    .map(|weapon| gear_price_label(&weapon.name, weapon.price_gp))
                                    .unwrap_or_else(|| "Unknown".to_string());
                                egui::ComboBox::from_id_source("starter_weapon")
                                    .selected_text(weapon_label)
                                    .show_ui(ui, |ui| {
                                        for (idx, weapon) in
                                            self.weapon_catalog.entries().iter().enumerate()
                                        {
                                            let effective_shield_cost = if weapon.handedness
                                                == crate::game_logic::WeaponHandedness::TwoHanded
                                            {
                                                0
                                            } else {
                                                shield_cost
                                            };
                                            let new_total =
                                                weapon.price_gp + armor_cost + effective_shield_cost;
                                            let affordable =
                                                new_total <= budget || idx == weapon_index;
                                            ui.add_enabled_ui(affordable, |ui| {
                                                ui.selectable_value(
                                                    &mut weapon_index,
                                                    idx,
                                                    gear_price_label(
                                                        &weapon.name,
                                                        weapon.price_gp,
                                                    ),
                                                );
                                            });
                                        }
                                    });
                                if let Some(id) = self.weapon_catalog.id_from_index(weapon_index) {
                                    self.creation.player.weapon_id = id;
                                    if let Some(weapon) = self.weapon_catalog.get(id) {
                                        if weapon.handedness
                                            == crate::game_logic::WeaponHandedness::TwoHanded
                                        {
                                            self.creation.player.shield_id =
                                                crate::game_logic::ShieldId::new(0);
                                        }
                                    }
                                }
                                weapon_cost = self
                                    .weapon_catalog
                                    .get(self.creation.player.weapon_id)
                                    .map(|weapon| weapon.price_gp)
                                    .unwrap_or(0);
                                shield_cost = self
                                    .shield_catalog
                                    .get(self.creation.player.shield_id)
                                    .and_then(|entry| entry.shield.as_ref())
                                    .map(|shield| shield.price_gp)
                                    .unwrap_or(0);

                                ui.separator();
                                ui.label("Armor");
                                let mut armor_index =
                                    self.armor_catalog.index_of(self.creation.player.armor_id);
                                let armor_label = self
                                    .armor_catalog
                                    .get(self.creation.player.armor_id)
                                    .map(|entry| {
                                        let price = entry
                                            .armor
                                            .as_ref()
                                            .map(|armor| armor.price_gp)
                                            .unwrap_or(0);
                                        gear_price_label(&entry.label, price)
                                    })
                                    .unwrap_or_else(|| "Unknown".to_string());
                                egui::ComboBox::from_id_source("starter_armor")
                                    .selected_text(armor_label)
                                    .show_ui(ui, |ui| {
                                        for (idx, entry) in
                                            self.armor_catalog.entries().iter().enumerate()
                                        {
                                            let price = entry
                                                .armor
                                                .as_ref()
                                                .map(|armor| armor.price_gp)
                                                .unwrap_or(0);
                                            let new_total =
                                                weapon_cost + price + shield_cost;
                                            let affordable =
                                                new_total <= budget || idx == armor_index;
                                            ui.add_enabled_ui(affordable, |ui| {
                                                ui.selectable_value(
                                                    &mut armor_index,
                                                    idx,
                                                    gear_price_label(&entry.label, price),
                                                );
                                            });
                                        }
                                    });
                                if let Some(id) = self.armor_catalog.id_from_index(armor_index) {
                                    self.creation.player.armor_id = id;
                                }
                                armor_cost = self
                                    .armor_catalog
                                    .get(self.creation.player.armor_id)
                                    .and_then(|entry| entry.armor.as_ref())
                                    .map(|armor| armor.price_gp)
                                    .unwrap_or(0);

                                ui.separator();
                                let shield_allowed = self
                                    .weapon_catalog
                                    .get(self.creation.player.weapon_id)
                                    .map(|weapon| {
                                        weapon.handedness
                                            == crate::game_logic::WeaponHandedness::OneHanded
                                    })
                                    .unwrap_or(true);
                                if !shield_allowed {
                                    self.creation.player.shield_id =
                                        crate::game_logic::ShieldId::new(0);
                                }
                                ui.label("Shield");
                                ui.add_enabled_ui(shield_allowed, |ui| {
                                    let mut shield_index = self
                                        .shield_catalog
                                        .index_of(self.creation.player.shield_id);
                                    let shield_label = self
                                        .shield_catalog
                                        .get(self.creation.player.shield_id)
                                        .map(|entry| {
                                            let price = entry
                                                .shield
                                                .as_ref()
                                                .map(|shield| shield.price_gp)
                                                .unwrap_or(0);
                                            gear_price_label(&entry.label, price)
                                        })
                                        .unwrap_or_else(|| "Unknown".to_string());
                                    egui::ComboBox::from_id_source("starter_shield")
                                        .selected_text(shield_label)
                                        .show_ui(ui, |ui| {
                                            for (idx, entry) in
                                                self.shield_catalog.entries().iter().enumerate()
                                            {
                                                let price = entry
                                                    .shield
                                                    .as_ref()
                                                    .map(|shield| shield.price_gp)
                                                    .unwrap_or(0);
                                                let new_total =
                                                    weapon_cost + armor_cost + price;
                                                let affordable =
                                                    new_total <= budget || idx == shield_index;
                                                ui.add_enabled_ui(affordable, |ui| {
                                                    ui.selectable_value(
                                                        &mut shield_index,
                                                        idx,
                                                        gear_price_label(&entry.label, price),
                                                    );
                                                });
                                            }
                                        });
                                    if let Some(id) =
                                        self.shield_catalog.id_from_index(shield_index)
                                    {
                                        self.creation.player.shield_id = id;
                                    }
                                });
                                if !shield_allowed {
                                    ui.label("Two-handed weapons cannot use shields.");
                                }
                            });
                            ui.separator();
                            ui.label("Gear prices are placeholders loaded from data.");
                        }
                    }
                });

                egui::TopBottomPanel::bottom("creation_footer").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if let Some(prev) = self.creation_step.prev() {
                            if ui.button("Back").clicked() {
                                self.creation_step = prev;
                            }
                        }
                        if self.can_advance() {
                            if let Some(next) = self.creation_step.next() {
                                if ui.button("Next").clicked() {
                                    if self.creation_step == CreationStep::RollStats {
                                        self.creation.lock_assignments();
                                    }
                                    self.creation_step = next;
                                }
                            }
                        }
                        if self.can_finish() {
                            if ui.button("Finish").clicked() {
                                self.creation_done = true;
                                self.screen = AppScreen::Run;
                                self.start_run_from_creation();
                            }
                        }
                    });
                });
            }
            AppScreen::Run => {
                let available_points = self.available_points();
                let (effective_cha, looks_delta) = self.effective_charisma();
                let run_view = self.run_state.as_ref();
                render_character_summary(
                    ctx,
                    &self.creation,
                    &self.race_catalog,
                    &self.talent_catalog,
                    available_points,
                    effective_cha,
                    looks_delta,
                    run_view,
                );

                let mut next_action = None;
                let mut finish_fight = false;
                egui::SidePanel::left("run_panel")
                    .resizable(false)
                    .min_width(RUN_PANEL_WIDTH)
                    .default_width(RUN_PANEL_WIDTH)
                    .show(ctx, |ui| {
                        ui.heading("Autobattler Run");
                        ui.separator();
                        let Some(run_view) = self.run_state.as_mut() else {
                            ui.label("No active run.");
                            return;
                        };
                        if run_view.run_over {
                            ui.colored_label(Color32::from_rgb(190, 80, 80), "Run over!");
                        }
                        let action_enabled = run_view.live_fight.is_none() && !run_view.run_over;
                        ui.horizontal(|ui| {
                            if ui
                                .add_enabled(action_enabled, egui::Button::new("Fight on"))
                                .clicked()
                            {
                                next_action = Some(RunAction::FightOn);
                            }
                            if ui
                                .add_enabled(action_enabled, egui::Button::new("Rest"))
                                .clicked()
                            {
                                next_action = Some(RunAction::RestDay);
                            }
                            if ui
                                .add_enabled(action_enabled, egui::Button::new("Train"))
                                .clicked()
                            {
                                next_action = Some(RunAction::Train);
                            }
                        });
                        ui.separator();
                        if let Some(live) = run_view.live_fight.as_mut() {
                            live.ui_elapsed += dt;
                            ingest_live_events(live);
                            prune_floaters(live);
                            ui.horizontal(|ui| {
                                if ui
                                    .add_enabled(live.running, egui::Button::new("Pause"))
                                    .clicked()
                                {
                                    live.running = false;
                                }
                                if ui
                                    .add_enabled(!live.running, egui::Button::new("Resume"))
                                    .clicked()
                                {
                                    live.running = true;
                                }
                                if ui
                                    .add_enabled(live.running, egui::Button::new("Step"))
                                    .clicked()
                                {
                                    live.pending_step = true;
                                }
                            });
                            ui.add(
                                egui::Slider::new(&mut live.time_scale, 0.1..=3.0)
                                    .text("Speed")
                                    .logarithmic(true),
                            );
                            if live.running {
                                let mut step_dt = dt * live.time_scale;
                                while step_dt > 0.0 {
                                    let slice = step_dt.min(0.06);
                                    live.sim.update(slice);
                                    step_dt -= slice;
                                }
                            }
                            if live.pending_step {
                                live.sim.update(0.2);
                                live.pending_step = false;
                            }
                            if live.sim.done || live.sim.elapsed_seconds >= live.max_seconds {
                                finish_fight = true;
                            }

                            ui.separator();
                            ui.label("Combat log");
                            egui::ScrollArea::vertical()
                                .max_height(ui.available_height().min(180.0).max(120.0))
                                .show(ui, |ui| {
                                    let start = live
                                        .log_lines
                                        .len()
                                        .saturating_sub(LOG_DISPLAY_LIMIT);
                                    for line in &live.log_lines[start..] {
                                        ui.label(line);
                                    }
                                });
                        }
                    });
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(Color32::TRANSPARENT))
                    .show(ctx, |_| {});

                if finish_fight {
                    self.complete_live_fight();
                }
                if let Some(action) = next_action {
                    self.run_action(action);
                }

                egui::TopBottomPanel::bottom("run_footer").show(ctx, |ui| {
                    let (run_depth, can_save) = match self.run_state.as_ref() {
                        Some(run_view) => (run_view.run_state.run_depth, run_view.live_fight.is_none()),
                        None => return,
                    };
                    ui.horizontal(|ui| {
                        let suggested = format!("{}-depth{}", self.creation.name.trim(), run_depth);
                        if self.run_save_name.trim().is_empty() {
                            self.run_save_name = suggested;
                        }
                        ui.label("Run save");
                        ui.text_edit_singleline(&mut self.run_save_name);
                        if ui.add_enabled(can_save, egui::Button::new("Save run")).clicked() {
                            self.save_run();
                        }
                        if !can_save {
                            ui.label("Finish the fight to save.");
                        }
                    });
                    if let Some(status) = self.run_save_status.as_ref() {
                        ui.separator();
                        ui.label(status);
                    }
                });
            }
            AppScreen::SpriteReview => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.heading("Sprite Review");
                    ui.label("Generating sprite review screenshots.");
                    if let Some(path) = screenshots.last_path.as_ref() {
                        ui.label(format!("Last capture: {path}"));
                    }
                    if let Some(err) = screenshots.last_error.as_ref() {
                        ui.colored_label(Color32::from_rgb(190, 80, 80), err);
                    }
                });
            }
        }
    }
}

fn render_character_summary(
    ctx: &egui::Context,
    creation: &crate::autobattler::state::CreationState,
    race_catalog: &[RaceSpec],
    talent_catalog: &TalentCatalog,
    available_points: PointPool,
    effective_cha: u8,
    looks_delta: i32,
    run_view: Option<&RunViewState>,
) {
    egui::SidePanel::right("character_summary")
        .resizable(false)
        .min_width(SUMMARY_PANEL_WIDTH)
        .default_width(SUMMARY_PANEL_WIDTH)
        .show(ctx, |ui| {
            ui.heading("Character Summary");
            ui.separator();
            ui.label(format!("Name: {}", creation.name));
            if let Some(race_idx) = creation.race_index {
                if let Some(race) = race_catalog.get(race_idx) {
                    ui.label(format!("Race: {}", race.name));
                    ui.label(format!("Base HP: {}", race.base_hp));
                } else {
                    ui.label("Race: None");
                }
            } else {
                ui.label("Race: None");
            }
            ui.separator();
            ui.label("Abilities");
            for (idx, label) in STAT_LABELS.iter().enumerate() {
                let score = creation.stats[idx];
                if idx == 6 {
                    let text = format!(
                        "{label}: {}/{} (raw {}/{}, looks {:+})",
                        effective_cha,
                        format_percentile(score.percentile),
                        score.base,
                        format_percentile(score.percentile),
                        looks_delta
                    );
                    ui.label(text);
                } else {
                    ui.label(format!("{label}: {}", format_score(score)));
                }
            }
            ui.separator();
            ui.label("Points");
            ui.label(format!("BP: {}", available_points.bp));
            ui.label(format!("LP: {}", available_points.lp));
            ui.label(format!("AP: {}", available_points.ap));
            ui.label(format!("RP: {}", available_points.rp));
            ui.separator();
            ui.label("Details");
            ui.label(format!("Seed: {}", creation.run_seed));
            ui.label(format!("Alignment: {}", creation.alignment));
            ui.label(format!("Honor: {}", creation.honor));
            if creation.background.trim().is_empty() {
                ui.label("Background: (none)");
            } else {
                ui.label(format!("Background: {}", creation.background));
            }
            ui.label(format!("Quirks: {}", creation.quirks.len()));
            ui.label(format!("Flaws: {}", creation.flaws.len()));
            ui.label(format!("Skills: {}", creation.skills.len()));
            ui.label(format!("Proficiencies: {}", creation.proficiencies.len()));
            if creation.money_rolled {
                ui.label(format!("Starting money: {}", creation.starting_money));
            } else {
                ui.label("Starting money: not rolled");
            }
            ui.separator();
            ui.label("Talents");
            if creation.player.talents.is_empty() {
                ui.label("None");
            } else {
                let mut labels: Vec<String> = creation
                    .player
                    .talents
                    .iter()
                    .map(|selection| talent_display_label(selection, talent_catalog))
                    .collect();
                labels.sort();
                egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                    for label in labels {
                        ui.label(label);
                    }
                });
            }
            if let Some(run_view) = run_view {
                ui.separator();
                ui.label("Run");
                ui.label(format!("Depth: {}", run_view.run_state.run_depth));
                ui.label(format!("Days: {}", run_view.days_elapsed));
                ui.label(format!("Gold: {}", run_view.run_state.inventory.gold));
                ui.label(format!("Wounds: {}", run_view.run_state.total_wound_damage()));
                ui.label(format!("XP: {}", run_view.run_state.player.xp));
                if run_view.training_days > 0 {
                    ui.label(format!("Training days: {}", run_view.training_days));
                }
            }
        });
}

fn render_string_list(
    ui: &mut egui::Ui,
    title: &str,
    input_label: &str,
    input: &mut String,
    entries: &mut Vec<String>,
) {
    ui.label(title);
    ui.horizontal(|ui| {
        ui.label(input_label);
        ui.text_edit_singleline(input);
        if ui.button(format!("Add##{input_label}")).clicked() {
            let trimmed = input.trim();
            if !trimmed.is_empty() {
                entries.push(trimmed.to_string());
                input.clear();
            }
        }
    });
    if entries.is_empty() {
        ui.label("None yet.");
        return;
    }
    let mut remove_idx = None;
    for (idx, entry) in entries.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.label(entry);
            if ui
                .button(format!("Remove##{input_label}_{idx}"))
                .clicked()
            {
                remove_idx = Some(idx);
            }
        });
    }
    if let Some(idx) = remove_idx {
        entries.remove(idx);
    }
}

fn gear_price_label(name: &str, price_gp: u32) -> String {
    format!("{name} - {price_gp} gp")
}

fn initiative_die_label(die: InitiativeDieQuality) -> &'static str {
    match die {
        InitiativeDieQuality::Standard => "Standard",
        InitiativeDieQuality::OneBetter => "One better",
        InitiativeDieQuality::TwoBetter => "Two better",
        InitiativeDieQuality::ThreeBetter => "Three better",
        InitiativeDieQuality::FourBetter => "Four better",
    }
}

#[allow(dead_code)]
fn draw_live_arena(
    ui: &mut egui::Ui,
    rect: EguiRect,
    sim: &crate::core::sim::SimState,
    player_name: &str,
    enemy_name: &str,
    floaters: &[DamageFloat],
    ui_time: f32,
) {
    let padding = 16.0;
    if rect.width() <= padding * 2.0 || rect.height() <= padding * 2.0 {
        return;
    }
    let painter = ui.painter();
    let bg = ui.style().visuals.panel_fill;
    painter.rect_filled(rect, 0.0, bg);

    let left = rect.left() + padding;
    let right = rect.right() - padding;
    let arena_width = (right - left).max(1.0);
    let scale = arena_width / sim.config.start_distance.max(1.0);
    if !scale.is_finite() {
        return;
    }

    if sim.actors.len() < 2 {
        return;
    }

    let bar_height = 8.0;
    let gap = 16.0;
    let bar_width = ((right - left) - gap).max(1.0) * 0.5;
    let bar_y = rect.top() + padding * 0.5;
    let timeline_y = bar_y + bar_height + 18.0;

    draw_swing_timeline(ui, left, right, timeline_y, sim);

    let ground_y = rect.center().y + rect.height() * 0.1;
    painter.line_segment(
        [Pos2::new(left, ground_y), Pos2::new(right, ground_y)],
        (2.0, Color32::from_gray(80)),
    );

    let tile_size = sim.config.tile_size_ft.max(0.01);
    let start_tiles = (sim.config.start_distance / tile_size).ceil() as i32;
    let padding_tiles = ((sim.config.grid_width - 1 - start_tiles) / 2).max(0);
    let x0_ft = (sim.actors[0].position.x - padding_tiles) as f32 * tile_size;
    let x1_ft = (sim.actors[1].position.x - padding_tiles) as f32 * tile_size;
    let mut x0 = left + x0_ft * scale;
    let mut x1 = left + x1_ft * scale;
    x0 = x0.clamp(left, right);
    x1 = x1.clamp(left, right);
    let min_gap = 24.0;
    if (x1 - x0).abs() < min_gap {
        let dir = if x1 >= x0 { 1.0 } else { -1.0 };
        x1 = (x0 + dir * min_gap).clamp(left, right);
    }

    let player_color = Color32::from_rgb(214, 93, 69);
    let enemy_color = Color32::from_rgb(70, 140, 210);
    painter.circle_filled(Pos2::new(x0, ground_y - 12.0), 7.0, player_color);
    painter.circle_filled(Pos2::new(x1, ground_y - 12.0), 7.0, enemy_color);

    for idx in 0..2 {
        let hp = sim.combatants[idx].state.hp.max(0) as f32;
        let max_hp = sim.combatants[idx].sheet.vitals.max_hp.max(1) as f32;
        let ratio = (hp / max_hp).clamp(0.0, 1.0);
        let bar_x = if idx == 0 { left } else { right - bar_width };
        let bg_rect =
            EguiRect::from_min_size(Pos2::new(bar_x, bar_y), egui::vec2(bar_width, bar_height));
        painter.rect_filled(bg_rect, 2.0, Color32::from_gray(40));
        let fill_width = bar_width * ratio;
        let fill_x = if idx == 0 {
            bar_x
        } else {
            bar_x + (bar_width - fill_width)
        };
        let fill_rect =
            EguiRect::from_min_size(Pos2::new(fill_x, bar_y), egui::vec2(fill_width, bar_height));
        let bar_color = if idx == 0 { player_color } else { enemy_color };
        painter.rect_filled(fill_rect, 2.0, bar_color);
        let name = if idx == 0 { player_name } else { enemy_name };
        let align = if idx == 0 {
            egui::Align2::LEFT_CENTER
        } else {
            egui::Align2::RIGHT_CENTER
        };
        let text_x = if idx == 0 { bar_x } else { bar_x + bar_width };
        painter.text(
            Pos2::new(text_x, bar_y - 4.0),
            align,
            name,
            egui::TextStyle::Body.resolve(ui.style()),
            Color32::from_gray(220),
        );
    }

    draw_damage_floaters(ui, floaters, ui_time, x0, x1, ground_y - 26.0);
}

fn draw_swing_timeline(
    ui: &egui::Ui,
    left: f32,
    right: f32,
    y: f32,
    sim: &crate::core::sim::SimState,
) {
    let painter = ui.painter();
    if right <= left {
        return;
    }
    let horizon = 8.0;
    let now = sim.elapsed_seconds as f32;
    let scale = (right - left) / horizon;
    let line_color = Color32::from_gray(70);
    painter.line_segment([Pos2::new(left, y), Pos2::new(right, y)], (2.0, line_color));

    for tick in 0..=8 {
        let x = left + tick as f32 * scale;
        let tick_h = if tick % 2 == 0 { 6.0 } else { 4.0 };
        painter.line_segment(
            [Pos2::new(x, y - tick_h), Pos2::new(x, y + tick_h)],
            (1.0, line_color),
        );
    }

    let player_color = Color32::from_rgb(214, 93, 69);
    let enemy_color = Color32::from_rgb(70, 140, 210);
    for idx in 0..2 {
        let color = if idx == 0 { player_color } else { enemy_color };
        if let Some(next) = sim.combatants[idx].state.next_attack_time_primary {
            let t = (next - now).max(0.0).min(horizon);
            let x = left + t * scale;
            let pos = Pos2::new(x, y - 14.0);
            painter.circle_filled(pos, 6.0, color);
        }
        if let Some(next) = sim.combatants[idx].state.next_attack_time_secondary {
            let t = (next - now).max(0.0).min(horizon);
            let x = left + t * scale;
            let pos = Pos2::new(x, y - 4.0);
            let secondary_color = Color32::from_rgb(
                ((color.r() as u16 + 255) / 2) as u8,
                ((color.g() as u16 + 255) / 2) as u8,
                ((color.b() as u16 + 255) / 2) as u8,
            );
            painter.circle_filled(pos, 4.0, secondary_color);
        }
    }
}

#[allow(dead_code)]
fn draw_damage_floaters(
    ui: &egui::Ui,
    floaters: &[DamageFloat],
    ui_time: f32,
    player_x: f32,
    enemy_x: f32,
    base_y: f32,
) {
    let painter = ui.painter();
    let lifetime = 1.2;
    let rise_per_sec = 26.0;
    for floater in floaters {
        let age = ui_time - floater.start_time;
        if age < 0.0 || age > lifetime {
            continue;
        }
        let alpha = 1.0 - (age / lifetime);
        let alpha_u8 = (alpha * 255.0).clamp(0.0, 255.0) as u8;
        let color = if floater.is_shield {
            Color32::from_rgba_premultiplied(80, 180, 220, alpha_u8)
        } else {
            Color32::from_rgba_premultiplied(230, 70, 70, alpha_u8)
        };
        let x = if floater.target_idx == 0 {
            player_x
        } else {
            enemy_x
        } + floater.offset;
        let y = base_y - age * rise_per_sec;
        painter.text(
            Pos2::new(x, y),
            egui::Align2::CENTER_CENTER,
            floater.value.to_string(),
            egui::TextStyle::Heading.resolve(ui.style()),
            color,
        );
    }
}

fn ingest_live_events(live: &mut LiveFight) {
    let end = live.sim.combat_events.len();
    for idx in live.seen_events..end {
        let event = &live.sim.combat_events[idx];
        live.log_lines
            .push(sim::format_combat_event_line(event, &live.sim.combatants));
        let (damage, shield_damage) = match &event.kind {
            sim::CombatEventKind::Attack(attack) => (attack.damage, attack.shield_damage),
            _ => (0, 0),
        };
        let defender_idx = event.defender_idx;
        if damage > 0 {
            push_damage_float(live, damage, defender_idx, false);
        }
        if shield_damage > 0 {
            push_damage_float(live, shield_damage, defender_idx, true);
        }
    }
    live.seen_events = end;
}

fn prune_floaters(live: &mut LiveFight) {
    let lifetime = 1.2;
    live.floaters
        .retain(|floater| live.ui_elapsed - floater.start_time <= lifetime);
}

fn push_damage_float(live: &mut LiveFight, value: i32, target_idx: usize, is_shield: bool) {
    let offset = ((live.float_seed % 5) as f32 - 2.0) * 8.0;
    live.float_seed = live.float_seed.wrapping_add(1);
    live.floaters.push(DamageFloat {
        value,
        target_idx,
        start_time: live.ui_elapsed,
        offset,
        is_shield,
    });
}

fn render_talent_selector(
    ui: &mut egui::Ui,
    id_prefix: &str,
    player: &mut PlayerConfig,
    weapon_catalog: &WeaponCatalog,
    race_catalog: &[RaceSpec],
    talent_catalog: &TalentCatalog,
    active_category: &mut String,
    available_points: PointPool,
    max_height: f32,
) {
    if talent_catalog.is_empty() {
        ui.label("No talents loaded.");
        return;
    }

    let active_race = race_for_player(player, race_catalog);
    let mut categories: BTreeMap<String, Vec<&TalentSpec>> = BTreeMap::new();
    for spec in talent_catalog.entries() {
        if !racial_talent_matches(spec, active_race) {
            continue;
        }
        let category = if spec.category.trim().is_empty() {
            "Uncategorized"
        } else {
            spec.category.as_str()
        };
        categories
            .entry(category.to_string())
            .or_default()
            .push(spec);
    }
    categories.entry(TALENT_TAB_RACIALS.to_string()).or_default();
    let mut categories: Vec<(String, Vec<&TalentSpec>)> = categories.into_iter().collect();
    let total_count: usize = categories.iter().map(|(_, specs)| specs.len()).sum();
    categories.sort_by(|a, b| a.0.cmp(&b.0));

    if active_category != TALENT_TAB_ALL
        && !categories.iter().any(|(name, _)| name == active_category)
    {
        active_category.clear();
        active_category.push_str(TALENT_TAB_ALL);
    }

    ui.horizontal_wrapped(|ui| {
        let all_label = format!("{TALENT_TAB_ALL} ({total_count})");
        if ui
            .selectable_label(active_category.as_str() == TALENT_TAB_ALL, all_label)
            .clicked()
        {
            active_category.clear();
            active_category.push_str(TALENT_TAB_ALL);
        }
        for (category, specs) in &categories {
            let label = format!("{category} ({})", specs.len());
            if ui
                .selectable_label(active_category.as_str() == category.as_str(), label)
                .clicked()
            {
                active_category.clear();
                active_category.push_str(category);
            }
        }
    });
    ui.separator();

    let abilities = game_logic::ability_set_from_player(player);
    let talent_snapshot = player.talents.clone();
    let context = game_logic::TalentContext {
        level: player.level,
        stats: &abilities,
        talents: &talent_snapshot,
    };
    let mut add_queue: Vec<TalentSelection> = Vec::new();
    let mut remove_queue: Vec<usize> = Vec::new();
    let default_group = weapon_catalog
        .get(player.weapon_id)
        .map(|weapon| weapon_group_label(weapon.group))
        .unwrap_or(WEAPON_GROUP_LABELS[0]);

    egui::ScrollArea::vertical().max_height(max_height).show(ui, |ui| {
        if active_category.as_str() == TALENT_TAB_ALL {
            for (category, specs) in &categories {
                if specs.is_empty() {
                    continue;
                }
                ui.separator();
                ui.label(category.as_str());
                for spec in specs {
                    render_talent_entry(
                        ui,
                        id_prefix,
                        player,
                        default_group,
                        weapon_catalog,
                        talent_catalog,
                        spec,
                        &context,
                        available_points,
                        &mut add_queue,
                        &mut remove_queue,
                    );
                }
            }
        } else if let Some((name, specs)) =
            categories.iter().find(|(name, _)| name == active_category)
        {
            if name == TALENT_TAB_RACIALS && specs.is_empty() {
                if active_race.is_some() {
                    ui.label("No racial talents available for the selected race.");
                } else {
                    ui.label("Select a race to view racial talents.");
                }
            }
            for spec in specs {
                render_talent_entry(
                    ui,
                    id_prefix,
                    player,
                    default_group,
                    weapon_catalog,
                    talent_catalog,
                    spec,
                    &context,
                    available_points,
                    &mut add_queue,
                    &mut remove_queue,
                );
            }
        }
    });

    if !add_queue.is_empty() {
        player.talents.extend(add_queue);
    }

    if !remove_queue.is_empty() {
        remove_queue.sort_unstable();
        remove_queue.dedup();
        for idx in remove_queue.into_iter().rev() {
            if idx < player.talents.len() {
                player.talents.remove(idx);
            }
        }
    }
}

fn render_talent_entry(
    ui: &mut egui::Ui,
    id_prefix: &str,
    player: &mut PlayerConfig,
    default_group: &str,
    weapon_catalog: &WeaponCatalog,
    talent_catalog: &TalentCatalog,
    spec: &TalentSpec,
    context: &game_logic::TalentContext,
    available_points: PointPool,
    add_queue: &mut Vec<TalentSelection>,
    remove_queue: &mut Vec<usize>,
) {
    let (current_rank, selection_idx) = player
        .talents
        .iter()
        .enumerate()
        .find(|(_, selection)| selection.id == spec.id)
        .map(|(idx, selection)| (selection.rank, Some(idx)))
        .unwrap_or((0, None));
    let failures = game_logic::evaluate_talent_requirements(spec, context);
    let meets_requirements = failures.is_empty();
    let max_rank = spec.max_rank.max(1);
    let available_rank = max_affordable_rank(spec, available_points);
    let max_selectable = max_rank.min(available_rank);
    let can_adjust = meets_requirements || current_rank > 0;
    let muted_color = ui.style().visuals.weak_text_color();
    let name_color = if meets_requirements {
        ui.style().visuals.text_color()
    } else {
        Color32::from_rgb(190, 90, 90)
    };
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(&spec.name).color(name_color));
            if spec.category == TALENT_TAB_RACIALS {
                ui.label(egui::RichText::new("Racial").color(muted_color));
            }
        });
        if !spec.description.trim().is_empty() {
            ui.label(&spec.description);
        }
        let cost = talent_cost_for_rank(spec, 1);
        ui.label(format!("Cost: {} BP, {} LP, {} RP", cost.bp, cost.lp, cost.rp));
        if !meets_requirements {
            if let Some(failure) = failures.first() {
                ui.colored_label(
                    Color32::from_rgb(190, 90, 90),
                    format_talent_requirement_failure(failure, talent_catalog),
                );
            }
        }

        let mut selection = TalentSelection {
            id: spec.id.clone(),
            rank: current_rank,
            weapon: None,
        };

        let mut rank = selection.rank;
        ui.horizontal(|ui| {
            ui.label("Rank");
            let mut selected_text = if rank == 0 {
                "None".to_string()
            } else {
                format!("{rank}")
            };
            if rank > max_selectable {
                selected_text.push_str(" (cap)");
            }
            ui.add_enabled_ui(can_adjust, |ui| {
                egui::ComboBox::from_id_source(format!("{id_prefix}_talent_{}", spec.id))
                    .selected_text(selected_text)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut rank, 0, "None");
                        for rank_value in 1..=max_rank {
                            let label = format!("{rank_value}");
                            let can_select = rank_value <= max_selectable;
                            ui.add_enabled_ui(can_select, |ui| {
                                ui.selectable_value(&mut rank, rank_value, label);
                            });
                        }
                    });
            });
        });

        if game_logic::talent_requires_weapon_group(spec) {
            let mut weapon_group = selection
                .weapon
                .clone()
                .unwrap_or_else(|| default_group.to_string());
            ui.horizontal(|ui| {
                ui.label("Weapon group");
                ui.add_enabled_ui(can_adjust, |ui| {
                    egui::ComboBox::from_id_source(format!(
                        "{id_prefix}_talent_weapon_group_{}",
                        spec.id
                    ))
                    .selected_text(weapon_group.clone())
                    .show_ui(ui, |ui| {
                        for label in WEAPON_GROUP_LABELS {
                            ui.selectable_value(&mut weapon_group, label.to_string(), label);
                        }
                    });
                });
            });
            selection.weapon = Some(weapon_group);
        } else if game_logic::talent_requires_weapon(spec) {
            let requires_group = game_logic::talent_requires_weapon_group(spec);
            if selection.weapon.is_none() {
                selection.weapon = if requires_group {
                    Some(default_group.to_string())
                } else if game_logic::talent_requires_weapon(spec) {
                    weapon_catalog
                        .get(player.weapon_id)
                        .map(|weapon| weapon.name.clone())
                        .or_else(|| {
                            weapon_catalog
                                .entries()
                                .first()
                                .map(|weapon| weapon.name.clone())
                        })
                } else {
                    None
                };
            }
            let selected_text = selection
                .weapon
                .clone()
                .unwrap_or_else(|| "Select weapon".to_string());
            ui.horizontal(|ui| {
                ui.label("Weapon");
                ui.add_enabled_ui(can_adjust, |ui| {
                    egui::ComboBox::from_id_source(format!("{id_prefix}_talent_weapon_{}", spec.id))
                        .selected_text(selected_text)
                        .show_ui(ui, |ui| {
                            for weapon in weapon_catalog.entries() {
                                ui.selectable_value(
                                    &mut selection.weapon,
                                    Some(weapon.name.clone()),
                                    weapon.name.as_str(),
                                );
                            }
                        });
                });
            });
        }

        if rank != selection.rank {
            if rank == 0 {
                if let Some(idx) = selection_idx {
                    remove_queue.push(idx);
                }
            } else {
                selection.rank = rank;
                if let Some(idx) = selection_idx {
                    if let Some(existing) = player.talents.get_mut(idx) {
                        existing.rank = rank;
                        existing.weapon = selection.weapon.clone();
                    }
                } else {
                    add_queue.push(selection);
                }
            }
        }
    });
}

fn race_for_player<'a>(
    player: &PlayerConfig,
    race_catalog: &'a [RaceSpec],
) -> Option<&'a RaceSpec> {
    player
        .race_id
        .as_ref()
        .and_then(|id| race_catalog.iter().find(|race| race.id == *id))
}

fn weapon_group_label(group: WeaponGroup) -> &'static str {
    match group {
        WeaponGroup::Unarmed => "Unarmed",
        WeaponGroup::Axes => "Axes",
        WeaponGroup::Basic => "Basic",
        WeaponGroup::Blunt => "Blunt",
        WeaponGroup::Bows => "Bows",
        WeaponGroup::Crossbows => "Crossbows",
        WeaponGroup::Double => "Double",
        WeaponGroup::Ensnaring => "Ensnaring",
        WeaponGroup::Lashes => "Lashes",
        WeaponGroup::LargeSwords => "Large swords",
        WeaponGroup::SmallSwords => "Small swords",
        WeaponGroup::Polearms => "Polearms",
        WeaponGroup::Spears => "Spears",
        WeaponGroup::Shields => "Shields",
    }
}

fn racial_talent_matches(spec: &TalentSpec, race: Option<&RaceSpec>) -> bool {
    if spec.category != TALENT_TAB_RACIALS {
        return true;
    }
    let Some(race) = race else {
        return false;
    };
    if spec.race_ids.iter().any(|race_id| race_id == &race.id) {
        return true;
    }
    if spec
        .race_categories
        .iter()
        .any(|category| category.eq_ignore_ascii_case(&race.category))
    {
        return true;
    }
    false
}

fn format_talent_requirement_failure(
    failure: &game_logic::TalentRequirementFailure,
    talent_catalog: &TalentCatalog,
) -> String {
    match failure {
        game_logic::TalentRequirementFailure::MinLevel { required, current } => {
            format!("Requires level {required} (current {current}).")
        }
        game_logic::TalentRequirementFailure::MinStatBase {
            stat,
            required,
            current,
        } => {
            format!("Requires {} {required}+ (current {current}).", stat.label())
        }
        game_logic::TalentRequirementFailure::MinStatPercentile {
            stat,
            required,
            current,
        } => {
            let current_label = current
                .map(format_percentile)
                .unwrap_or_else(|| "??".to_string());
            format!(
                "Requires {} percentile {required}+ (current {current_label}).",
                stat.label()
            )
        }
        game_logic::TalentRequirementFailure::RequiresTalent {
            id,
            required_rank,
            current_rank,
        } => {
            let talent_name = talent_catalog
                .entries()
                .iter()
                .find(|talent| talent.id == *id)
                .map(|talent| talent.name.as_str())
                .unwrap_or(id.as_str());
            format!(
                "Requires {talent_name} rank {required_rank} (current {current_rank})."
            )
        }
    }
}
