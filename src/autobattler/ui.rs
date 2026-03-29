use bevy::prelude::*;
use bevy_egui::EguiContexts;
use bevy_egui::egui::{self, Color32, Pos2, Rect as EguiRect};

use crate::autobattler::app::AutobattlerApp;
use crate::autobattler::weapon_mastery;
use std::collections::BTreeMap;

use crate::autobattler::constants::{
    LOG_DISPLAY_LIMIT, RUN_PANEL_WIDTH, START_AP, START_BP, START_LP, START_RP, STAT_COUNT,
    STAT_LABELS, SUMMARY_PANEL_WIDTH, TALENT_TAB_ALL, TALENT_TAB_RACIALS, WEAPON_GROUP_LABELS,
};
use crate::autobattler::logic::{
    apply_percentile, bp_increment, format_percentile, format_score, max_affordable_rank,
    race_adjustment_summary, starting_honor, stat_at_cap, subtract_percentile,
    talent_cost_for_rank, talent_display_label,
};
use crate::autobattler::screenshot::ScreenshotState;
use crate::autobattler::state::{
    AppScreen, CreationState, DamageFloat, DowntimeActivity, LiveFight, PointPool, RunAction,
    RunViewState,
};
use crate::autobattler::state::{AutobattlerState, CreationStep};
use crate::character::{
    AbilityScore, AbilitySetFull, InitiativeDieQuality, MasteryAspect, WeaponGroup,
};
use crate::core::rules::roll_damage_expr;
use crate::core::skills;
use crate::core::types::{PlayerProfile, RaceSpec, TalentSelection, TalentSpec};
use crate::game_logic::{self, PlayerConfig, TalentCatalog, WeaponCatalog};
use crate::sim;
use crate::ui_widgets::searchable_select;

#[derive(Clone, Copy)]
enum WeaponIcon {
    Sword,
    Dagger,
    Axe,
    Spear,
    Polearm,
    Bow,
    Crossbow,
    Blunt,
    Lash,
    Basic,
    Double,
    Ensnaring,
    Shield,
    Unarmed,
    Other,
}

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
                    if !self.startup_data_issues.is_empty() {
                        ui.separator();
                        ui.colored_label(
                            Color32::from_rgb(200, 90, 90),
                            "Missing required data files:",
                        );
                        for issue in &self.startup_data_issues {
                            ui.label(issue);
                        }
                    }
                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui.button("New character").clicked() {
                            self.start_new_character();
                        }
                    });
                    ui.separator();
                    ui.label("Quick starts");
                    if self.quick_start_presets.is_empty() {
                        ui.label("No quick-start presets found.");
                    } else {
                        egui::ScrollArea::vertical()
                            .max_height(160.0)
                            .show(ui, |ui| {
                                for (idx, preset) in
                                    self.quick_start_presets.entries().iter().enumerate()
                                {
                                    let selected = self.selected_quick_start == Some(idx);
                                    let label = format!(
                                        "{} (Lvl {}, {}, {})",
                                        preset.name, preset.level, preset.weapon, preset.armor
                                    );
                                    if ui.selectable_label(selected, label).clicked() {
                                        self.selected_quick_start = Some(idx);
                                    }
                                }
                            });
                    }
                    let can_start_quick = self.selected_quick_start.is_some();
                    if ui
                        .add_enabled(can_start_quick, egui::Button::new("Start quick run"))
                        .clicked()
                    {
                        self.start_run_from_selected_quick_start();
                    }
                    if let Some(status) = self.quick_start_status.as_ref() {
                        ui.label(status);
                    }

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
                        if ui
                            .add_enabled(can_load, egui::Button::new("Load selected"))
                            .clicked()
                        {
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
                        if ui
                            .add_enabled(can_load, egui::Button::new("Load run"))
                            .clicked()
                        {
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
                                let mut seed_value = self.run_seed.min(i64::MAX as u64) as i64;
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
                                        let mut selection = self.creation.assignments[stat_idx];
                                        let current_selection = selection;
                                        let selected_text = selection
                                            .map(|idx| format_score(selected_set.rolls[idx]))
                                            .unwrap_or_else(|| "Select roll".to_string());
                                        searchable_select(
                                            ui,
                                            format!("assign_{stat_idx}"),
                                            selected_text,
                                            &mut selection,
                                            (0..STAT_COUNT).map(|roll_idx| {
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
                                                (
                                                    Some(roll_idx),
                                                    label,
                                                    !taken_elsewhere
                                                        || current_selection == Some(roll_idx),
                                                )
                                            }),
                                        );
                                        if selection != self.creation.assignments[stat_idx] {
                                            if let Some(roll_idx) = selection {
                                                self.creation.assign_roll(stat_idx, roll_idx);
                                            }
                                        }
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
                                        searchable_select(
                                            ui,
                                            "race_select",
                                            self.race_catalog
                                                .get(selection)
                                                .map(|race| race.name.as_str())
                                                .unwrap_or("None"),
                                            &mut selection,
                                            std::iter::once((
                                                usize::MAX,
                                                "None".to_string(),
                                                true,
                                            ))
                                            .chain(
                                                self.race_catalog.iter().enumerate().map(
                                                    |(idx, race)| {
                                                        (idx, race.name.clone(), true)
                                                    },
                                                ),
                                            ),
                                        );
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
                                        .add_enabled(can_confirm, egui::Button::new("Confirm race"))
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
                            let alignment_label = self.creation.alignment.clone();
                            searchable_select(
                                ui,
                                "alignment_select",
                                alignment_label,
                                &mut self.creation.alignment,
                                ["Unaligned", "Lawful", "Neutral", "Chaotic"]
                                    .into_iter()
                                    .map(|option| (option.to_string(), option.to_string(), true)),
                            );
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
                            ui.label(
                                "BP increments: +10 below 10/01, +5 up to 16/01, +3 at 16/01+.",
                            );
                            ui.separator();
                            ui.add_enabled_ui(self.creation.race_applied, |ui| {
                                for stat_idx in 0..STAT_COUNT {
                                    let label = STAT_LABELS[stat_idx];
                                    let score = self.creation.stats[stat_idx];
                                    let increment = bp_increment(&score);
                                    let can_add = available_points.bp > 0 && !stat_at_cap(&score);
                                    let can_remove = !self.creation.bp_history[stat_idx].is_empty();
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
                                                egui::Button::new(format!("+{} BP", increment)),
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
                            render_skill_selector(ui, &mut self.creation, available_points);
                            ui.separator();
                            render_string_list(
                                ui,
                                "Proficiencies (placeholder)",
                                "Proficiency",
                                &mut self.creation.proficiency_input,
                                &mut self.creation.proficiencies,
                            );
                            self.creation.player.proficiencies =
                                self.creation.proficiencies.clone();
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
                            if self.creation.player.called_shot {
                                let (
                                    called_shot_light_bonus,
                                    called_shot_medium_bonus,
                                    called_shot_heavy_bonus,
                                ) = game_logic::called_shot_target_defense_bonuses_for_player(
                                    &self.creation.player,
                                );
                                let called_shot_self_penalty =
                                    game_logic::called_shot_self_defense_penalty_for_player(
                                        &self.creation.player,
                                    );
                                let called_shot_is_ranged = self
                                    .weapon_catalog
                                    .get(self.creation.player.weapon_id)
                                    .map(game_logic::is_ranged_weapon)
                                    .unwrap_or(false);
                                let called_shot_delay_expr = game_logic::called_shot_delay_expr_for_player(
                                    &self.creation.player,
                                    called_shot_is_ranged,
                                );
                                ui.label(format!(
                                    "Called shot target defense (light/medium/heavy): +{called_shot_light_bonus}/+{called_shot_medium_bonus}/+{called_shot_heavy_bonus}"
                                ));
                                ui.label(format!(
                                    "Called shot self defense: -{called_shot_self_penalty}"
                                ));
                                ui.label(format!(
                                    "Called shot speed: +{called_shot_delay_expr}"
                                ));
                                ui.label(format!(
                                    "Defense (DV while called shot): {}",
                                    summary.derived.base_dv - called_shot_self_penalty
                                ));
                            }
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
                                let weapon_cost: u32;
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
                                let selected_weapon_index = weapon_index;
                                searchable_select(
                                    ui,
                                    "starter_weapon",
                                    weapon_label,
                                    &mut weapon_index,
                                    self.weapon_catalog
                                        .entries()
                                        .iter()
                                        .enumerate()
                                        .map(|(idx, weapon)| {
                                            let effective_shield_cost = self
                                                .shield_catalog
                                                .get(self.creation.player.shield_id)
                                                .and_then(|entry| entry.shield.as_ref())
                                                .map(|shield| {
                                                    if game_logic::shield_option_allowed(
                                                        &self.creation.player,
                                                        weapon,
                                                        Some(shield),
                                                        &self.talent_catalog,
                                                        &self.weapon_catalog,
                                                    ) {
                                                        shield_cost
                                                    } else {
                                                        0
                                                    }
                                                })
                                                .unwrap_or(0);
                                            let new_total = weapon.price_gp
                                                + armor_cost
                                                + effective_shield_cost;
                                            let affordable =
                                                new_total <= budget
                                                    || idx == selected_weapon_index;
                                            (
                                                idx,
                                                gear_price_label(&weapon.name, weapon.price_gp),
                                                affordable,
                                            )
                                        }),
                                );
                                if let Some(id) = self.weapon_catalog.id_from_index(weapon_index) {
                                    self.creation.player.weapon_id = id;
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
                                let selected_armor_index = armor_index;
                                searchable_select(
                                    ui,
                                    "starter_armor",
                                    armor_label,
                                    &mut armor_index,
                                    self.armor_catalog.entries().iter().enumerate().map(
                                        |(idx, entry)| {
                                            let price = entry
                                                .armor
                                                .as_ref()
                                                .map(|armor| armor.price_gp)
                                                .unwrap_or(0);
                                            let new_total = weapon_cost + price + shield_cost;
                                            let affordable =
                                                new_total <= budget || idx == selected_armor_index;
                                            (
                                                idx,
                                                gear_price_label(&entry.label, price),
                                                affordable,
                                            )
                                        },
                                    ),
                                );
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
                                let weapon =
                                    self.weapon_catalog.get(self.creation.player.weapon_id);
                                let selected_shield_allowed = weapon
                                    .and_then(|weapon| {
                                        self.shield_catalog
                                            .get(self.creation.player.shield_id)
                                            .and_then(|entry| entry.shield.as_ref())
                                            .map(|shield| {
                                                game_logic::shield_option_allowed(
                                                    &self.creation.player,
                                                    weapon,
                                                    Some(shield),
                                                    &self.talent_catalog,
                                                    &self.weapon_catalog,
                                                )
                                            })
                                    })
                                    .unwrap_or(true);
                                if !selected_shield_allowed {
                                    self.creation.player.shield_id =
                                        crate::game_logic::ShieldId::new(0);
                                }
                                let shield_allowed = weapon
                                    .map(|weapon| {
                                        self.shield_catalog.entries().iter().any(|entry| {
                                            entry
                                                .shield
                                                .as_ref()
                                                .map(|shield| {
                                                    game_logic::shield_option_allowed(
                                                        &self.creation.player,
                                                        weapon,
                                                        Some(shield),
                                                        &self.talent_catalog,
                                                        &self.weapon_catalog,
                                                    )
                                                })
                                                .unwrap_or(false)
                                        })
                                    })
                                    .unwrap_or(true);
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
                                    let selected_shield_index = shield_index;
                                    searchable_select(
                                        ui,
                                        "starter_shield",
                                        shield_label,
                                        &mut shield_index,
                                        self.shield_catalog.entries().iter().enumerate().map(
                                            |(idx, entry)| {
                                                let price = entry
                                                    .shield
                                                    .as_ref()
                                                    .map(|shield| shield.price_gp)
                                                    .unwrap_or(0);
                                                let new_total = weapon_cost + armor_cost + price;
                                                let affordable =
                                                    new_total <= budget
                                                        || idx == selected_shield_index;
                                                let style_allowed = weapon
                                                    .and_then(|weapon| {
                                                        entry.shield.as_ref().map(|shield| {
                                                            game_logic::shield_option_allowed(
                                                                &self.creation.player,
                                                                weapon,
                                                                Some(shield),
                                                                &self.talent_catalog,
                                                                &self.weapon_catalog,
                                                            )
                                                        })
                                                    })
                                                    .unwrap_or(true);
                                                (
                                                    idx,
                                                    gear_price_label(&entry.label, price),
                                                    affordable && style_allowed,
                                                )
                                            },
                                        ),
                                    );
                                    if let Some(id) =
                                        self.shield_catalog.id_from_index(shield_index)
                                    {
                                        self.creation.player.shield_id = id;
                                    }
                                });
                                if !shield_allowed {
                                    ui.label("No shields are available for the current setup.");
                                } else if !selected_shield_allowed {
                                    ui.label("Only bucklers and small shields are allowed.");
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
                let run_snapshot = self.run_state.as_ref().map(|run_view| {
                    (
                        run_view.run_state.player.ability_scores_full,
                        run_view.run_state.player.honor,
                    )
                });
                if let Some((scores, honor)) = run_snapshot {
                    self.creation.stats[0] = scores.strength;
                    self.creation.stats[1] = scores.intelligence;
                    self.creation.stats[2] = scores.wisdom;
                    self.creation.stats[3] = scores.dexterity;
                    self.creation.stats[4] = scores.constitution;
                    self.creation.stats[5] = scores.looks;
                    self.creation.stats[6] = scores.charisma;
                    self.creation.honor = honor;
                    self.creation.sync_player_from_stats();
                }
                if let Some(run_view) = self.run_state.as_mut() {
                    if let Some(feedback) = run_view.downtime_feedback.as_mut() {
                        feedback.animation_seconds = (feedback.animation_seconds - dt).max(0.0);
                    }
                }
                let should_prepare_encounter = self
                    .run_state
                    .as_ref()
                    .map(|run_view| {
                        !run_view.run_over
                            && run_view.pending_levelup.is_none()
                            && !run_view.awaiting_downtime_choice
                            && run_view.live_fight.is_none()
                            && run_view.pending_encounter.is_none()
                            && run_view.pending_event.is_none()
                    })
                    .unwrap_or(false);
                if should_prepare_encounter {
                    self.prepare_next_encounter();
                }

                let mut next_action = None;
                let mut skip_encounter = false;
                let mut fight_encounter = false;
                let mut resolve_event_choice: Option<String> = None;
                let mut ignore_event = false;
                let mut confirm_levelup = false;
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
                        ui.label(format!("Run seed: {}", run_view.seed_context.run_seed));
                        if let Some(seed) = run_view.seed_context.spawn_seed {
                            ui.label(format!("Spawn seed: {seed}"));
                        }
                        if let Some(seed) = run_view.seed_context.combat_seed {
                            ui.label(format!("Combat seed: {seed}"));
                        }
                        if let Some(seed) = run_view.seed_context.loot_seed {
                            ui.label(format!("Loot seed: {seed}"));
                        }
                        if let Some(seed) = run_view.seed_context.event_seed {
                            ui.label(format!("Event seed: {seed}"));
                        }
                        ui.separator();
                        if let Some(checkpoint) = run_view.pending_levelup.as_mut() {
                            ui.heading("Level-Up Checkpoint");
                            ui.label(format!("Levels gained: {}", checkpoint.levels_gained));
                            ui.label(format!("Remaining slots: {}", checkpoint.remaining_slots()));
                            ui.horizontal(|ui| {
                                if ui
                                    .add_enabled(checkpoint.bp_slots > 0, egui::Button::new("-"))
                                    .clicked()
                                {
                                    checkpoint.bp_slots -= 1;
                                }
                                ui.label(format!("BP slots: {}", checkpoint.bp_slots));
                                if ui
                                    .add_enabled(
                                        checkpoint.remaining_slots() > 0,
                                        egui::Button::new("+"),
                                    )
                                    .clicked()
                                {
                                    checkpoint.bp_slots += 1;
                                }
                            });
                            ui.horizontal(|ui| {
                                if ui
                                    .add_enabled(checkpoint.lp_slots > 0, egui::Button::new("-"))
                                    .clicked()
                                {
                                    checkpoint.lp_slots -= 1;
                                }
                                ui.label(format!("LP slots: {}", checkpoint.lp_slots));
                                if ui
                                    .add_enabled(
                                        checkpoint.remaining_slots() > 0,
                                        egui::Button::new("+"),
                                    )
                                    .clicked()
                                {
                                    checkpoint.lp_slots += 1;
                                }
                            });
                            ui.horizontal(|ui| {
                                if ui
                                    .add_enabled(checkpoint.ap_slots > 0, egui::Button::new("-"))
                                    .clicked()
                                {
                                    checkpoint.ap_slots -= 1;
                                }
                                ui.label(format!("AP slots: {}", checkpoint.ap_slots));
                                if ui
                                    .add_enabled(
                                        checkpoint.remaining_slots() > 0,
                                        egui::Button::new("+"),
                                    )
                                    .clicked()
                                {
                                    checkpoint.ap_slots += 1;
                                }
                            });
                            ui.horizontal(|ui| {
                                if ui
                                    .add_enabled(checkpoint.rp_slots > 0, egui::Button::new("-"))
                                    .clicked()
                                {
                                    checkpoint.rp_slots -= 1;
                                }
                                ui.label(format!("RP slots: {}", checkpoint.rp_slots));
                                if ui
                                    .add_enabled(
                                        checkpoint.remaining_slots() > 0,
                                        egui::Button::new("+"),
                                    )
                                    .clicked()
                                {
                                    checkpoint.rp_slots += 1;
                                }
                            });
                            if ui.button("Auto-assign remaining to BP").clicked() {
                                checkpoint.bp_slots =
                                    checkpoint.bp_slots.saturating_add(checkpoint.remaining_slots());
                            }
                            let grants = checkpoint.grants();
                            ui.label(format!(
                                "Confirm grants: +{} BP, +{} LP, +{} AP, +{} RP",
                                grants.bp, grants.lp, grants.ap, grants.rp
                            ));
                            if ui
                                .add_enabled(
                                    checkpoint.remaining_slots() == 0,
                                    egui::Button::new("Confirm level-up"),
                                )
                                .clicked()
                            {
                                confirm_levelup = true;
                            }
                            ui.separator();
                        }
                        let action_enabled = run_view.live_fight.is_none()
                            && !run_view.run_over
                            && run_view.pending_levelup.is_none()
                            && run_view.awaiting_downtime_choice;
                        let downtime_days: u32 = 8;
                        let downtime_steps = downtime_days.saturating_mul(4);
                        ui.label(format!(
                            "Downtime per encounter: {downtime_days} days ({downtime_steps} steps)."
                        ));
                        if run_view.awaiting_downtime_choice {
                            ui.heading("Choose Downtime");
                            ui.horizontal(|ui| {
                                if ui
                                    .add_enabled(action_enabled, egui::Button::new("Rest"))
                                    .clicked()
                                {
                                    next_action = Some(RunAction::Rest);
                                }
                                searchable_select(
                                    ui,
                                    "downtime_activity_select",
                                    run_view.selected_activity.label(),
                                    &mut run_view.selected_activity,
                                    DowntimeActivity::ALL.into_iter().map(|activity| {
                                        (activity, activity.label().to_string(), true)
                                    }),
                                );
                                if ui
                                    .add_enabled(action_enabled, egui::Button::new("Activity"))
                                    .clicked()
                                {
                                    next_action = Some(RunAction::Activity);
                                }
                            });
                            ui.separator();
                            ui.heading("Selected Activity");
                            ui.label(
                                egui::RichText::new(run_view.selected_activity.label())
                                    .size(20.0)
                                    .strong(),
                            );
                            ui.label(
                                egui::RichText::new(run_view.selected_activity.description())
                                    .size(16.0),
                            );
                        } else if run_view.live_fight.is_none() && !run_view.run_over {
                            if let Some(event) = run_view.pending_event.as_ref() {
                                ui.heading("Encounter Event");
                                ui.label(format!("{}: {}", event.event.name, event.event.description));
                                ui.separator();
                                ui.label("Choices");
                                for choice in &event.event.choices {
                                    if ui.button(&choice.text).clicked() {
                                        resolve_event_choice = Some(choice.id.clone());
                                    }
                                }
                                if ui.button("Leave it").clicked() {
                                    ignore_event = true;
                                }
                            } else if let Some(encounter) = run_view.pending_encounter.as_ref() {
                                ui.heading("Encounter");
                                ui.label(format!(
                                    "You spot {} in {}, wielding {}. Try to avoid the fight or stand and fight?",
                                    encounter.enemy_name, encounter.armor_label, encounter.weapon_name
                                ));
                                ui.horizontal(|ui| {
                                    if ui.button("Run").clicked() {
                                        skip_encounter = true;
                                    }
                                    if ui.button("Fight").clicked() {
                                        fight_encounter = true;
                                    }
                                });
                            } else {
                                ui.label("Preparing encounter...");
                            }
                        }
                        ui.separator();
                        let fast_healer = run_view
                            .run_state
                            .player
                            .talents
                            .iter()
                            .any(|talent| talent.id == "fast_healer");
                        let mut wounds = run_view.run_state.wounds.clone();
                        if let Some(last_outcome) = run_view.last_outcome.as_ref() {
                            let result_text = if last_outcome.fight.won {
                                "Last result: Victory"
                            } else {
                                "Last result: Defeat"
                            };
                            ui.label(result_text);
                            ui.label(format!(
                                "Last fight: {}s, HP {}",
                                last_outcome.fight.turns, last_outcome.fight.remaining_hp
                            ));
                        }
                        ui.separator();
                        render_weapon_mastery_panel(
                            ui,
                            &mut run_view.run_state.player,
                            &self.creation.player,
                            &self.weapon_catalog,
                            &mut run_view.last_log,
                        );
                        if let Some(live) = run_view.live_fight.as_mut() {
                            let state_label = if live.running { "Running" } else { "Paused" };
                            ui.label(format!("Combat state: {state_label}"));
                            if live.pending_step {
                                ui.label("Step queued");
                            }
                            live.ui_elapsed += dt;
                            ingest_live_events(live);
                            prune_floaters(live);
                            wounds.extend(player_wounds_from_events(&live.sim.combat_events));
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
                                let frame_dt = (dt * live.time_scale).min(0.05);
                                live.sim.update(frame_dt);
                            }
                            if live.pending_step {
                                live.sim.update(0.2);
                                live.pending_step = false;
                            }
                            if live.sim.done || live.sim.elapsed_seconds >= live.max_seconds {
                                finish_fight = true;
                            }

                            ui.separator();
                            render_wound_status(ui, &wounds, fast_healer);
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
                egui::CentralPanel::default().show(ctx, |ui| {
                    let Some(run_view) = self.run_state.as_ref() else {
                        ui.label("No active run.");
                        return;
                    };
                    if let Some(feedback) = run_view.downtime_feedback.as_ref() {
                        let (rect, _) =
                            ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
                        draw_downtime_activity_scene(
                            ui,
                            rect,
                            feedback.activity,
                            feedback.animation_seconds,
                        );
                        let margin = 24.0;
                        let max_width = (rect.width() - margin * 2.0).max(320.0);
                        let max_height = (rect.height() - margin * 2.0).max(220.0);
                        let panel_size = egui::vec2(
                            (rect.width() * 0.62).clamp(520.0, max_width),
                            (rect.height() * 0.42).clamp(260.0, max_height),
                        );
                        let panel_rect = egui::Rect::from_min_size(
                            rect.min + egui::vec2(margin, margin),
                            panel_size,
                        );
                        let mut dismiss_feedback = false;
                        ui.allocate_ui_at_rect(panel_rect, |ui| {
                            egui::Frame::window(ui.style()).show(ui, |ui| {
                                ui.heading(&feedback.title);
                                for line in &feedback.lines {
                                    ui.label(format!("• {line}"));
                                }
                                if feedback.animation_seconds <= 0.0
                                    && ui.button("Continue").clicked()
                                {
                                    dismiss_feedback = true;
                                }
                            });
                        });
                        if dismiss_feedback {
                            if let Some(run_view_mut) = self.run_state.as_mut() {
                                run_view_mut.downtime_feedback = None;
                            }
                        }
                        return;
                    }
                    let Some(live) = run_view.live_fight.as_ref() else {
                        ui.centered_and_justified(|ui| {
                            if run_view.awaiting_downtime_choice {
                                ui.label("Encounter complete. Choose Rest or Activity.");
                            } else if run_view.pending_levelup.is_some() {
                                ui.label("Allocate level-up slots, then confirm.");
                            } else if run_view.pending_event.is_some() {
                                ui.label("Choose an event option or leave.");
                            } else if run_view.pending_encounter.is_some() {
                                ui.label("Choose Run or Fight to resolve the encounter.");
                            } else {
                                ui.label("Starting encounter...");
                            }
                        });
                        return;
                    };
                    let (rect, _) =
                        ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
                    let player_name = live
                        .sim
                        .combatants
                        .first()
                        .map(|c| c.sheet.name.as_str())
                        .unwrap_or("Player");
                    let enemy_name = live
                        .sim
                        .combatants
                        .get(1)
                        .map(|c| c.sheet.name.as_str())
                        .unwrap_or("Enemy");
                    draw_live_arena(
                        ui,
                        rect,
                        &live.sim,
                        &self.weapon_catalog,
                        player_name,
                        enemy_name,
                        &live.floaters,
                        live.ui_elapsed,
                    );
                });

                if finish_fight {
                    self.complete_live_fight();
                }
                if confirm_levelup {
                    self.confirm_level_up();
                }
                if let Some(choice_id) = resolve_event_choice {
                    self.resolve_pending_event_choice(&choice_id);
                }
                if ignore_event {
                    self.ignore_pending_event();
                }
                if skip_encounter {
                    self.skip_encounter();
                }
                if fight_encounter {
                    self.start_live_fight();
                }
                if let Some(action) = next_action {
                    self.run_action(action);
                }

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

                if self
                    .run_state
                    .as_ref()
                    .map(|run_view| {
                        run_view
                            .live_fight
                            .as_ref()
                            .map(|live| live.running)
                            .unwrap_or(false)
                            || run_view
                                .downtime_feedback
                                .as_ref()
                                .map(|f| f.animation_seconds > 0.0)
                                .unwrap_or(false)
                    })
                    .unwrap_or(false)
                {
                    ctx.request_repaint();
                }

                egui::TopBottomPanel::bottom("run_footer").show(ctx, |ui| {
                    let (run_depth, can_save) = match self.run_state.as_ref() {
                        Some(run_view) => {
                            (run_view.run_state.run_depth, run_view.live_fight.is_none())
                        }
                        None => return,
                    };
                    ui.horizontal(|ui| {
                        let suggested = format!("{}-depth{}", self.creation.name.trim(), run_depth);
                        if self.run_save_name.trim().is_empty() {
                            self.run_save_name = suggested;
                        }
                        ui.label("Run save");
                        ui.text_edit_singleline(&mut self.run_save_name);
                        if ui
                            .add_enabled(can_save, egui::Button::new("Save run"))
                            .clicked()
                        {
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
            if let Some(run_view) = run_view {
                let run_scores = run_view.run_state.player.ability_scores_full;
                let rows = [
                    ("STR", run_scores.strength),
                    ("INT", run_scores.intelligence),
                    ("WIS", run_scores.wisdom),
                    ("DEX", run_scores.dexterity),
                    ("CON", run_scores.constitution),
                    ("LKS", run_scores.looks),
                    ("CHA", run_scores.charisma),
                ];
                for (label, score) in rows {
                    ui.label(format!("{label}: {}", format_score(score)));
                }
            } else {
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
            ui.label(format!("Skills: {}", creation.skill_levels.len()));
            if !creation.skill_levels.is_empty() {
                let mut skill_rows: Vec<String> = creation
                    .skill_levels
                    .iter()
                    .filter_map(|entry| {
                        skills::skill_spec(&entry.id).map(|spec| {
                            let tier = skills::mastery_tier_for_level(entry.level).label();
                            format!("{} {}% ({tier})", spec.name, entry.level)
                        })
                    })
                    .collect();
                skill_rows.sort();
                egui::ScrollArea::vertical()
                    .max_height(90.0)
                    .show(ui, |ui| {
                        for row in skill_rows {
                            ui.label(row);
                        }
                    });
            }
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
                egui::ScrollArea::vertical()
                    .max_height(180.0)
                    .show(ui, |ui| {
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
                ui.label(format!(
                    "Wounds: {}",
                    run_view.run_state.total_wound_damage()
                ));
                ui.label(format!("XP: {}", run_view.run_state.player.xp));
                let unspent_mastery =
                    weapon_mastery::total_unspent_mastery_points(&run_view.run_state.player);
                ui.label(format!("Unspent mastery points: {}", unspent_mastery));
                ui.label(format!(
                    "Weapon mastery groups: {}",
                    run_view.run_state.player.weapon_masteries.len()
                ));
                ui.label(format!(
                    "Skills: {}",
                    run_view.run_state.player.skill_levels.len()
                ));
                ui.separator();
                let fast_healer = run_view
                    .run_state
                    .player
                    .talents
                    .iter()
                    .any(|talent| talent.id == "fast_healer");
                let mut wounds = run_view.run_state.wounds.clone();
                if let Some(live) = run_view.live_fight.as_ref() {
                    wounds.extend(player_wounds_from_events(&live.sim.combat_events));
                }
                render_wound_status(ui, &wounds, fast_healer);
                if run_view.training_days > 0 {
                    ui.label(format!("Training days: {}", run_view.training_days));
                }
            }
        });
}

fn render_wound_status(
    ui: &mut egui::Ui,
    wounds: &[crate::core::gameplay::Wound],
    fast_healer: bool,
) {
    ui.label("Wound Recovery");
    if wounds.is_empty() {
        ui.label("No active wounds.");
        return;
    }

    for (idx, wound) in wounds.iter().enumerate() {
        let required_steps = required_healing_steps(wound.damage, fast_healer);
        let progress_steps = wound.healing_progress_steps.min(required_steps);
        let pct = if required_steps == 0 {
            1.0
        } else {
            progress_steps as f32 / required_steps as f32
        };
        let remaining = required_steps.saturating_sub(progress_steps);
        ui.label(format!("Wound {}: {} dmg", idx + 1, wound.damage));
        ui.add(
            egui::ProgressBar::new(pct)
                .show_percentage()
                .text(format!("{} / {} steps", progress_steps, required_steps)),
        );
        ui.small(format!("Next heal in {} steps", remaining));
    }
}

fn required_healing_steps(damage: u32, fast_healer: bool) -> u32 {
    if fast_healer {
        if damage == 1 {
            1
        } else {
            damage.saturating_sub(1).saturating_mul(2)
        }
    } else {
        damage.saturating_mul(2)
    }
}

fn player_wounds_from_events(events: &[sim::CombatEvent]) -> Vec<crate::core::gameplay::Wound> {
    let mut wounds = Vec::new();
    for event in events {
        if event.defender_idx != 0 {
            continue;
        }
        let sim::CombatEventKind::Attack(attack) = &event.kind else {
            continue;
        };
        if attack.damage > 0 {
            wounds.push(crate::core::gameplay::Wound {
                damage: attack.damage as u32,
                healing_progress_steps: 0,
            });
        }
    }
    wounds
}

fn render_weapon_mastery_panel(
    ui: &mut egui::Ui,
    profile: &mut PlayerProfile,
    current_config: &PlayerConfig,
    weapon_catalog: &WeaponCatalog,
    last_log: &mut Vec<String>,
) {
    ui.heading("Weapon Mastery");
    let rows = weapon_mastery::mastery_rows(profile, weapon_catalog);
    if rows.is_empty() {
        ui.label("No weapon mastery progress yet.");
        return;
    }
    let active_group = weapon_catalog
        .get(current_config.weapon_id)
        .map(|weapon| weapon.group);
    let mut spend_requests: Vec<(WeaponGroup, MasteryAspect)> = Vec::new();
    egui::ScrollArea::vertical()
        .max_height(220.0)
        .show(ui, |ui| {
            for row in rows {
                ui.group(|ui| {
                    let mut header = row.label.to_string();
                    if active_group == Some(row.group) {
                        header.push_str(" (equipped)");
                    }
                    if row.at_max_tier {
                        header.push_str(" [max]");
                    }
                    ui.label(egui::RichText::new(header).strong());
                    ui.label(format!(
                        "XP {}/{} | Unspent {} | Tier {} / {}",
                        row.experience,
                        row.threshold,
                        row.unspent_points,
                        row.completed_tiers,
                        row.max_tier
                    ));
                    let pct = if row.threshold == 0 {
                        1.0
                    } else {
                        row.experience as f32 / row.threshold as f32
                    };
                    ui.add(
                        egui::ProgressBar::new(pct.clamp(0.0, 1.0))
                            .text(format!("{} / {}", row.experience, row.threshold)),
                    );
                    if row.group == WeaponGroup::Shields {
                        ui.label(format!("Defense +{}  Speed +{}", row.defense, row.speed));
                    } else {
                        ui.label(format!(
                            "Attack +{}  Defense +{}  Damage +{}  Speed +{}",
                            row.attack, row.defense, row.damage, row.speed
                        ));
                    }
                    if row.unspent_points > 0 {
                        if row.proficient {
                            ui.horizontal(|ui| {
                                if row.group != WeaponGroup::Shields
                                    && ui.small_button("+Atk").clicked()
                                {
                                    spend_requests.push((row.group, MasteryAspect::Attack));
                                }
                                if ui.small_button("+Def").clicked() {
                                    spend_requests.push((row.group, MasteryAspect::Defense));
                                }
                                if row.group != WeaponGroup::Shields
                                    && ui.small_button("+Dmg").clicked()
                                {
                                    spend_requests.push((row.group, MasteryAspect::Damage));
                                }
                                if ui.small_button("+Spd").clicked() {
                                    spend_requests.push((row.group, MasteryAspect::Speed));
                                }
                            });
                        } else {
                            ui.small("Needs matching proficiency to spend points.");
                        }
                    }
                });
            }
        });
    for (group, aspect) in spend_requests {
        match weapon_mastery::spend_mastery_point(profile, group, aspect, weapon_catalog) {
            Ok(lines) => {
                for line in lines {
                    last_log.push(line);
                }
            }
            Err(err) => last_log.push(err),
        }
    }
}

fn draw_downtime_activity_scene(
    ui: &mut egui::Ui,
    rect: EguiRect,
    activity: Option<DowntimeActivity>,
    animation_seconds: f32,
) {
    let painter = ui.painter_at(rect);
    let bg = Color32::from_rgb(22, 26, 33);
    painter.rect_filled(rect, 8.0, bg);
    let ground_y = rect.bottom() - 56.0;
    painter.line_segment(
        [
            Pos2::new(rect.left() + 24.0, ground_y),
            Pos2::new(rect.right() - 24.0, ground_y),
        ],
        (2.0, Color32::from_rgb(72, 88, 72)),
    );

    let t = (1.6 - animation_seconds).max(0.0);
    let sway = (t * 4.0).sin() * 8.0;
    let jump = ((t * 6.0).sin().abs()) * 10.0;
    let center = Pos2::new(rect.center().x - 40.0 + sway, ground_y - 18.0 - jump);
    draw_activity_stick_figure(&painter, center, activity, t);

    let title = activity
        .map(|a| format!("Downtime: {}", a.label()))
        .unwrap_or_else(|| "Downtime: Rest".to_string());
    painter.text(
        Pos2::new(rect.center().x, rect.top() + 26.0),
        egui::Align2::CENTER_CENTER,
        title,
        egui::FontId::proportional(26.0),
        Color32::from_rgb(230, 236, 245),
    );
}

fn draw_activity_stick_figure(
    painter: &egui::Painter,
    base: Pos2,
    activity: Option<DowntimeActivity>,
    t: f32,
) {
    let stroke = egui::Stroke::new(3.0, Color32::from_rgb(236, 236, 236));
    let accent = Color32::from_rgb(240, 187, 77);
    let head = Pos2::new(base.x, base.y - 34.0);
    let neck = Pos2::new(base.x, base.y - 24.0);
    let torso = Pos2::new(base.x, base.y - 4.0);
    let foot_l = Pos2::new(base.x - 8.0, base.y + 20.0);
    let foot_r = Pos2::new(base.x + 10.0, base.y + 20.0);
    painter.circle_filled(head, 7.0, Color32::from_rgb(228, 212, 178));
    painter.line_segment([neck, torso], stroke);
    painter.line_segment([torso, foot_l], stroke);
    painter.line_segment([torso, foot_r], stroke);

    let arm_swing = (t * 8.0).sin() * 10.0;
    let hand_l = Pos2::new(base.x - 12.0, base.y - 14.0 + arm_swing * 0.2);
    let hand_r = Pos2::new(base.x + 14.0, base.y - 14.0 - arm_swing * 0.2);
    painter.line_segment([neck, hand_l], stroke);
    painter.line_segment([neck, hand_r], stroke);

    let badge = |text: &str, x: f32, y: f32| {
        painter.text(
            Pos2::new(base.x + x, base.y + y),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(20.0),
            Color32::from_rgb(234, 230, 206),
        );
    };

    match activity {
        Some(DowntimeActivity::Acrobatics) => {
            painter.line_segment(
                [
                    Pos2::new(base.x + 14.0, base.y - 24.0),
                    Pos2::new(base.x + 42.0, base.y - 36.0),
                ],
                (3.0, accent),
            );
            badge("flip", 56.0, -30.0);
        }
        Some(DowntimeActivity::AnimalTraining) => {
            painter.circle_filled(
                Pos2::new(base.x + 52.0, base.y - 8.0),
                10.0,
                Color32::from_rgb(148, 120, 92),
            );
            badge("paw", 52.0, 12.0);
        }
        Some(DowntimeActivity::Athletics) => {
            painter.rect_filled(
                EguiRect::from_center_size(
                    Pos2::new(base.x + 54.0, base.y - 16.0),
                    egui::vec2(22.0, 8.0),
                ),
                2.0,
                Color32::from_rgb(104, 104, 104),
            );
            badge("lift", 54.0, 10.0);
        }
        Some(DowntimeActivity::Begging) => {
            painter.circle_filled(
                Pos2::new(base.x + 46.0, base.y - 8.0),
                9.0,
                Color32::from_rgb(176, 132, 72),
            );
            badge("coin", 46.0, 12.0);
        }
        Some(DowntimeActivity::Carousing) => {
            painter.rect_filled(
                EguiRect::from_center_size(
                    Pos2::new(base.x + 46.0, base.y - 14.0),
                    egui::vec2(10.0, 16.0),
                ),
                2.0,
                Color32::from_rgb(151, 102, 64),
            );
            badge("ale", 46.0, 10.0);
        }
        Some(DowntimeActivity::Climbing) => {
            painter.line_segment(
                [
                    Pos2::new(base.x + 56.0, base.y + 20.0),
                    Pos2::new(base.x + 56.0, base.y - 46.0),
                ],
                (3.0, Color32::from_rgb(128, 96, 64)),
            );
            badge("climb", 56.0, -54.0);
        }
        Some(DowntimeActivity::Crafting) => {
            painter.rect_filled(
                EguiRect::from_center_size(
                    Pos2::new(base.x + 52.0, base.y - 8.0),
                    egui::vec2(22.0, 12.0),
                ),
                2.0,
                Color32::from_rgb(118, 80, 56),
            );
            badge("forge", 52.0, 12.0);
        }
        Some(DowntimeActivity::Foraging) => {
            painter.circle_filled(
                Pos2::new(base.x + 52.0, base.y - 10.0),
                8.0,
                Color32::from_rgb(84, 140, 78),
            );
            badge("herb", 52.0, 12.0);
        }
        Some(DowntimeActivity::Gambling) => {
            painter.rect_filled(
                EguiRect::from_center_size(
                    Pos2::new(base.x + 52.0, base.y - 10.0),
                    egui::vec2(12.0, 12.0),
                ),
                1.0,
                Color32::from_rgb(216, 216, 216),
            );
            badge("d20", 52.0, 12.0);
        }
        Some(DowntimeActivity::Healing) => {
            painter.rect_filled(
                EguiRect::from_center_size(
                    Pos2::new(base.x + 52.0, base.y - 12.0),
                    egui::vec2(16.0, 16.0),
                ),
                1.0,
                Color32::from_rgb(172, 62, 62),
            );
            badge("+", 52.0, -12.0);
        }
        Some(DowntimeActivity::Hunting) => {
            painter.line_segment(
                [
                    Pos2::new(base.x + 14.0, base.y - 14.0),
                    Pos2::new(base.x + 56.0, base.y - 24.0),
                ],
                (2.5, accent),
            );
            badge("track", 56.0, 10.0);
        }
        Some(DowntimeActivity::Jumping) => {
            badge("jump", 54.0, -26.0);
        }
        Some(DowntimeActivity::Laboring) => {
            painter.rect_filled(
                EguiRect::from_center_size(
                    Pos2::new(base.x + 50.0, base.y - 12.0),
                    egui::vec2(18.0, 10.0),
                ),
                1.0,
                Color32::from_rgb(120, 92, 62),
            );
            badge("work", 50.0, 10.0);
        }
        Some(DowntimeActivity::Meditating) => {
            painter.circle_stroke(
                Pos2::new(base.x, base.y - 20.0),
                28.0,
                (2.0, Color32::from_rgb(120, 180, 220)),
            );
            badge("om", 52.0, 4.0);
        }
        Some(DowntimeActivity::Performing) => {
            badge("song", 52.0, -10.0);
        }
        Some(DowntimeActivity::Reading) => {
            let book = EguiRect::from_center_size(
                Pos2::new(base.x + 26.0, base.y - 18.0),
                egui::vec2(26.0, 18.0),
            );
            painter.rect_filled(book, 2.0, Color32::from_rgb(133, 88, 52));
            painter.line_segment(
                [
                    Pos2::new(book.center().x, book.top()),
                    Pos2::new(book.center().x, book.bottom()),
                ],
                (1.0, Color32::from_rgb(222, 205, 170)),
            );
            badge("read", 56.0, 4.0);
        }
        Some(DowntimeActivity::RepairingRefitting) => {
            painter.line_segment(
                [
                    Pos2::new(base.x + 18.0, base.y - 12.0),
                    Pos2::new(base.x + 44.0, base.y - 8.0),
                ],
                (3.0, accent),
            );
            badge("fix", 56.0, 8.0);
        }
        Some(DowntimeActivity::Riding) => {
            painter.circle_filled(
                Pos2::new(base.x + 52.0, base.y - 2.0),
                12.0,
                Color32::from_rgb(124, 96, 78),
            );
            badge("ride", 52.0, 16.0);
        }
        Some(DowntimeActivity::Scouting) => {
            badge("scan", 56.0, -10.0);
        }
        Some(DowntimeActivity::SkillTutoring) => {
            painter.circle_filled(
                Pos2::new(base.x + 52.0, base.y - 12.0),
                7.0,
                Color32::from_rgb(170, 170, 170),
            );
            badge("teach", 52.0, 10.0);
        }
        Some(DowntimeActivity::SkillTraining) => {
            badge("train", 56.0, 4.0);
        }
        Some(DowntimeActivity::Swimming) => {
            painter.circle_filled(
                Pos2::new(base.x + 38.0, base.y - 2.0),
                14.0,
                Color32::from_rgb(48, 122, 188),
            );
            painter.line_segment(
                [
                    Pos2::new(base.x + 24.0, base.y + 6.0),
                    Pos2::new(base.x + 54.0, base.y + 6.0),
                ],
                (3.0, Color32::from_rgb(62, 148, 220)),
            );
        }
        Some(DowntimeActivity::Sparring) => {
            let foe = Pos2::new(base.x + 62.0, base.y - 2.0);
            painter.circle_filled(
                Pos2::new(foe.x, foe.y - 34.0),
                7.0,
                Color32::from_rgb(170, 170, 170),
            );
            painter.line_segment(
                [
                    Pos2::new(foe.x, foe.y - 24.0),
                    Pos2::new(foe.x, foe.y - 4.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(foe.x, foe.y - 4.0),
                    Pos2::new(foe.x - 8.0, foe.y + 20.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(foe.x, foe.y - 4.0),
                    Pos2::new(foe.x + 10.0, foe.y + 20.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(base.x + 10.0, base.y - 16.0),
                    Pos2::new(foe.x - 12.0, foe.y - 16.0),
                ],
                (2.5, accent),
            );
        }
        Some(DowntimeActivity::WeaponDrills) => {
            painter.line_segment(
                [
                    Pos2::new(base.x + 20.0, base.y - 18.0),
                    Pos2::new(base.x + 58.0, base.y - 34.0),
                ],
                (3.0, accent),
            );
            painter.circle_stroke(
                Pos2::new(base.x + 70.0, base.y - 24.0),
                14.0,
                (2.0, Color32::from_rgb(110, 125, 140)),
            );
            badge("drill", 58.0, 8.0);
        }
        None => {
            painter.circle_filled(
                Pos2::new(base.x + 44.0, base.y - 24.0),
                10.0,
                Color32::from_rgb(90, 130, 90),
            );
            painter.text(
                Pos2::new(base.x + 44.0, base.y - 24.0),
                egui::Align2::CENTER_CENTER,
                "Z",
                egui::FontId::proportional(20.0),
                Color32::from_rgb(232, 246, 232),
            );
        }
    }
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
            if ui.button(format!("Remove##{input_label}_{idx}")).clicked() {
                remove_idx = Some(idx);
            }
        });
    }
    if let Some(idx) = remove_idx {
        entries.remove(idx);
    }
}

fn creation_ability_scores_full(creation: &CreationState) -> AbilitySetFull {
    AbilitySetFull {
        strength: creation.stats[0],
        intelligence: creation.stats[1],
        wisdom: creation.stats[2],
        dexterity: creation.stats[3],
        constitution: creation.stats[4],
        looks: creation.stats[5],
        charisma: AbilityScore::new(creation.player.charisma, creation.stats[6].percentile),
    }
}

fn render_skill_selector(
    ui: &mut egui::Ui,
    creation: &mut CreationState,
    available_points: PointPool,
) {
    ui.heading("Skills");
    ui.horizontal(|ui| {
        ui.label("Filter");
        ui.text_edit_singleline(&mut creation.skill_input);
        if ui.button("Clear").clicked() {
            creation.skill_input.clear();
        }
    });

    let query = creation.skill_input.trim().to_ascii_lowercase();
    let abilities = creation_ability_scores_full(creation);
    let mut to_learn: Option<String> = None;
    let mut to_remove: Option<String> = None;
    let mut specs: Vec<&crate::core::skills::SkillSpec> = skills::all_skill_specs()
        .iter()
        .filter(|spec| {
            query.is_empty()
                || spec.name.to_ascii_lowercase().contains(&query)
                || spec.id.contains(&query)
        })
        .collect();
    specs.sort_by(|a, b| a.name.cmp(b.name));

    egui::ScrollArea::vertical()
        .max_height(260.0)
        .show(ui, |ui| {
            for spec in specs {
                let current = skills::skill_level_in(&creation.skill_levels, spec.id);
                let tier = skills::mastery_tier_for_level(current);
                let learn_result = skills::can_learn_skill(
                    &creation.skill_levels,
                    creation.player.level.max(1),
                    spec.id,
                );
                let cap = skills::advancement_cap(
                    spec.id,
                    creation.player.level.max(1),
                    &creation.skill_levels,
                );
                let start_level = skills::starting_skill_level(&abilities, spec).min(cap);
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(spec.name).strong());
                        ui.label(format!("LP {}", spec.lp_cost));
                        ui.label(format!(
                            "Abilities: {}",
                            skills::describe_relevant_abilities(spec)
                        ));
                        if spec.universal {
                            ui.label("Universal");
                        }
                    });
                    if current > 0 {
                        ui.label(format!("Learned at {}% ({})", current, tier.label()));
                        if ui.button(format!("Unlearn {}", spec.name)).clicked() {
                            to_remove = Some(spec.id.to_string());
                        }
                    } else {
                        if let Err(err) = &learn_result {
                            ui.colored_label(Color32::from_rgb(190, 90, 90), err);
                        }
                        if available_points.lp < spec.lp_cost {
                            ui.colored_label(
                                Color32::from_rgb(190, 90, 90),
                                format!(
                                    "Need {} LP (have {})",
                                    spec.lp_cost,
                                    available_points.lp.max(0)
                                ),
                            );
                        }
                        let can_afford = available_points.lp >= spec.lp_cost;
                        let can_learn = learn_result.is_ok() && start_level > 0;
                        if ui
                            .add_enabled(
                                can_afford && can_learn,
                                egui::Button::new(format!("Learn at {}%", start_level)),
                            )
                            .clicked()
                        {
                            to_learn = Some(spec.id.to_string());
                        }
                    }
                });
            }
        });

    if let Some(skill_id) = to_remove {
        if skills::remove_skill(&mut creation.skill_levels, &skill_id) {
            let label = skills::skill_spec(&skill_id)
                .map(|spec| spec.name)
                .unwrap_or(skill_id.as_str());
            creation.skill_feedback = Some(format!("Removed skill: {label}"));
        }
    }

    if let Some(skill_id) = to_learn {
        match skills::learn_skill(
            &mut creation.skill_levels,
            &abilities,
            creation.player.level.max(1),
            &skill_id,
        ) {
            Ok(progress) => {
                let label = skills::skill_spec(&skill_id)
                    .map(|spec| spec.name)
                    .unwrap_or(skill_id.as_str());
                let tier = skills::mastery_tier_for_level(progress.level).label();
                creation.skill_feedback =
                    Some(format!("Learned {label}: {}% ({tier})", progress.level));
            }
            Err(err) => {
                creation.skill_feedback = Some(err);
            }
        }
    }

    if let Some(feedback) = creation.skill_feedback.as_ref() {
        ui.separator();
        ui.label(feedback);
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
    weapon_catalog: &WeaponCatalog,
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
    let gap = (x1 - x0).abs();
    let min_gap = 28.0;
    if gap < min_gap {
        let dir = if x1 >= x0 { 1.0 } else { -1.0 };
        if sim.combatants[0].sheet.offense.weapon.reach_ft
            >= sim.combatants[1].sheet.offense.weapon.reach_ft
        {
            x1 = x0 + dir * min_gap;
        } else {
            x0 = x1 - dir * min_gap;
        }
    }

    let fighter_positions = [(0usize, x0, 1.0_f32), (1usize, x1, -1.0_f32)];
    for (idx, x, facing) in fighter_positions {
        let combatant = &sim.combatants[idx];
        let player_color = if idx == 0 { player_color } else { enemy_color };
        let knocked_back = combatant.state.knockback_immobile_seconds > 0;
        let downed = combatant.state.hp <= 0 || combatant.state.trauma_remaining_seconds > 0;
        let weapon_icon = weapon_icon_for_combatant(combatant, weapon_catalog);
        draw_person(
            painter,
            Pos2::new(x, ground_y),
            facing,
            player_color,
            downed,
            knocked_back,
            weapon_icon,
        );
        if knocked_back && !downed {
            painter.text(
                Pos2::new(x, ground_y - 56.0),
                egui::Align2::CENTER_CENTER,
                "knocked",
                egui::TextStyle::Small.resolve(ui.style()),
                Color32::from_rgb(230, 160, 90),
            );
        }
    }

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

fn draw_person(
    painter: &egui::Painter,
    base: Pos2,
    facing: f32,
    color: Color32,
    downed: bool,
    knocked_back: bool,
    weapon_icon: WeaponIcon,
) {
    let head_color = color;
    let body_color = Color32::from_gray(230);
    let stroke = (2.0, body_color);

    if downed {
        let torso_start = Pos2::new(base.x - facing * 2.0, base.y - 4.0);
        let torso_end = Pos2::new(base.x + facing * 16.0, base.y - 4.0);
        let head = Pos2::new(base.x + facing * 22.0, base.y - 6.0);
        painter.line_segment([torso_start, torso_end], stroke);
        painter.line_segment(
            [torso_start, Pos2::new(base.x - facing * 6.0, base.y - 1.0)],
            stroke,
        );
        painter.line_segment(
            [torso_start, Pos2::new(base.x + facing * 4.0, base.y - 10.0)],
            stroke,
        );
        painter.circle_filled(head, 6.0, head_color);
        painter.line_segment(
            [torso_end, Pos2::new(base.x + facing * 10.0, base.y - 12.0)],
            stroke,
        );
        draw_weapon_icon(
            painter,
            Pos2::new(base.x + facing * 6.0, base.y - 10.0),
            facing,
            weapon_icon,
        );
        return;
    }

    if knocked_back {
        let torso_start = Pos2::new(base.x, base.y - 8.0);
        let torso_end = Pos2::new(base.x + facing * 16.0, base.y - 20.0);
        let head = Pos2::new(base.x + facing * 20.0, base.y - 24.0);
        painter.line_segment([torso_start, torso_end], stroke);
        painter.line_segment(
            [torso_start, Pos2::new(base.x - facing * 6.0, base.y - 2.0)],
            stroke,
        );
        painter.line_segment(
            [torso_start, Pos2::new(base.x + facing * 6.0, base.y - 2.0)],
            stroke,
        );
        painter.circle_filled(head, 6.0, head_color);
        painter.line_segment(
            [torso_end, Pos2::new(base.x + facing * 26.0, base.y - 14.0)],
            stroke,
        );
        draw_weapon_icon(
            painter,
            Pos2::new(base.x + facing * 26.0, base.y - 14.0),
            facing,
            weapon_icon,
        );
        return;
    }

    let head = Pos2::new(base.x, base.y - 34.0);
    let neck = Pos2::new(base.x, base.y - 26.0);
    let torso = Pos2::new(base.x, base.y - 14.0);
    painter.circle_filled(head, 6.5, head_color);
    painter.line_segment([neck, torso], stroke);
    painter.line_segment([torso, Pos2::new(base.x - 6.0, base.y - 2.0)], stroke);
    painter.line_segment([torso, Pos2::new(base.x + 6.0, base.y - 2.0)], stroke);
    let arm_start = Pos2::new(base.x, base.y - 22.0);
    let arm_end = Pos2::new(base.x + facing * 12.0, base.y - 18.0);
    painter.line_segment([arm_start, arm_end], stroke);
    draw_weapon_icon(painter, arm_end, facing, weapon_icon);
}

fn weapon_icon_for_combatant(
    combatant: &crate::core::sim::Combatant,
    weapon_catalog: &WeaponCatalog,
) -> WeaponIcon {
    let weapon_name = combatant.sheet.offense.weapon.name.as_str();
    let group = weapon_catalog
        .entries()
        .iter()
        .find(|weapon| weapon.name.eq_ignore_ascii_case(weapon_name))
        .map(|weapon| weapon.group);
    group.map(weapon_icon_kind).unwrap_or(WeaponIcon::Other)
}

fn weapon_icon_kind(group: WeaponGroup) -> WeaponIcon {
    match group {
        WeaponGroup::Unarmed => WeaponIcon::Unarmed,
        WeaponGroup::Axes => WeaponIcon::Axe,
        WeaponGroup::Basic => WeaponIcon::Basic,
        WeaponGroup::Blunt => WeaponIcon::Blunt,
        WeaponGroup::Bows => WeaponIcon::Bow,
        WeaponGroup::Crossbows => WeaponIcon::Crossbow,
        WeaponGroup::Double => WeaponIcon::Double,
        WeaponGroup::Ensnaring => WeaponIcon::Ensnaring,
        WeaponGroup::Lashes => WeaponIcon::Lash,
        WeaponGroup::LargeSwords => WeaponIcon::Sword,
        WeaponGroup::SmallSwords => WeaponIcon::Dagger,
        WeaponGroup::Polearms => WeaponIcon::Polearm,
        WeaponGroup::Spears => WeaponIcon::Spear,
        WeaponGroup::Shields => WeaponIcon::Shield,
    }
}

fn draw_weapon_icon(painter: &egui::Painter, pos: Pos2, facing: f32, icon: WeaponIcon) {
    let stroke = (2.0, Color32::from_gray(220));
    let accent = Color32::from_gray(170);
    match icon {
        WeaponIcon::Sword => {
            let tip = Pos2::new(pos.x + facing * 12.0, pos.y - 10.0);
            let guard = Pos2::new(pos.x + facing * 3.0, pos.y - 2.0);
            painter.line_segment([pos, tip], stroke);
            painter.line_segment(
                [
                    Pos2::new(guard.x - facing * 3.0, guard.y + 2.0),
                    Pos2::new(guard.x + facing * 3.0, guard.y - 2.0),
                ],
                stroke,
            );
            painter.circle_filled(Pos2::new(pos.x - facing * 1.0, pos.y + 1.0), 1.5, accent);
        }
        WeaponIcon::Dagger => {
            let tip = Pos2::new(pos.x + facing * 8.0, pos.y - 6.0);
            let guard = Pos2::new(pos.x + facing * 2.0, pos.y - 1.0);
            painter.line_segment([pos, tip], stroke);
            painter.line_segment(
                [
                    Pos2::new(guard.x - facing * 2.0, guard.y + 1.5),
                    Pos2::new(guard.x + facing * 2.0, guard.y - 1.5),
                ],
                stroke,
            );
        }
        WeaponIcon::Axe => {
            let handle_end = Pos2::new(pos.x + facing * 8.0, pos.y - 6.0);
            painter.line_segment([pos, handle_end], stroke);
            let blade_top = Pos2::new(handle_end.x + facing * 3.0, handle_end.y - 3.0);
            let blade_bottom = Pos2::new(handle_end.x + facing * 3.0, handle_end.y + 3.0);
            painter.line_segment([handle_end, blade_top], stroke);
            painter.line_segment([handle_end, blade_bottom], stroke);
        }
        WeaponIcon::Spear => {
            let tip = Pos2::new(pos.x + facing * 16.0, pos.y - 10.0);
            let base = Pos2::new(pos.x + facing * 11.0, pos.y - 7.0);
            painter.line_segment([pos, tip], stroke);
            painter.line_segment([tip, Pos2::new(base.x, base.y + 3.0)], stroke);
            painter.line_segment([tip, Pos2::new(base.x, base.y - 3.0)], stroke);
        }
        WeaponIcon::Polearm => {
            let tip = Pos2::new(pos.x + facing * 18.0, pos.y - 12.0);
            let blade_back = Pos2::new(tip.x - facing * 4.0, tip.y + 2.0);
            let blade_low = Pos2::new(tip.x - facing * 2.0, tip.y + 7.0);
            painter.line_segment([pos, tip], stroke);
            painter.line_segment([tip, blade_back], stroke);
            painter.line_segment([blade_back, blade_low], stroke);
            painter.line_segment([blade_low, tip], stroke);
            painter.line_segment(
                [
                    blade_back,
                    Pos2::new(blade_back.x - facing * 3.0, blade_back.y - 2.0),
                ],
                stroke,
            );
        }
        WeaponIcon::Bow => {
            let grip = Pos2::new(pos.x + facing * 2.0, pos.y - 4.0);
            let top = Pos2::new(grip.x, grip.y - 8.0);
            let mid = Pos2::new(grip.x + facing * 4.0, grip.y);
            let bottom = Pos2::new(grip.x, grip.y + 8.0);
            painter.line_segment([pos, grip], stroke);
            painter.line_segment([top, mid], stroke);
            painter.line_segment([mid, bottom], stroke);
            painter.line_segment([top, bottom], (1.0, accent));
        }
        WeaponIcon::Crossbow => {
            let stock_end = Pos2::new(pos.x + facing * 10.0, pos.y - 2.0);
            let limb_center = Pos2::new(pos.x + facing * 6.0, pos.y - 4.0);
            painter.line_segment([pos, stock_end], stroke);
            painter.line_segment(
                [
                    Pos2::new(limb_center.x, limb_center.y - 4.0),
                    Pos2::new(limb_center.x, limb_center.y + 4.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(limb_center.x - facing * 3.0, limb_center.y - 4.0),
                    Pos2::new(limb_center.x - facing * 3.0, limb_center.y + 4.0),
                ],
                (1.0, accent),
            );
            painter.line_segment(
                [
                    Pos2::new(pos.x + facing * 5.0, pos.y - 4.0),
                    Pos2::new(pos.x + facing * 9.0, pos.y - 4.0),
                ],
                (1.0, accent),
            );
        }
        WeaponIcon::Blunt => {
            let end = Pos2::new(pos.x + facing * 8.0, pos.y - 6.0);
            painter.line_segment([pos, end], stroke);
            let head = EguiRect::from_center_size(end, egui::vec2(5.0, 5.0));
            painter.rect_filled(head, 1.0, accent);
        }
        WeaponIcon::Lash => {
            let p1 = Pos2::new(pos.x + facing * 4.0, pos.y - 6.0);
            let p2 = Pos2::new(pos.x + facing * 8.0, pos.y - 2.0);
            let p3 = Pos2::new(pos.x + facing * 12.0, pos.y - 8.0);
            painter.line_segment([pos, p1], stroke);
            painter.line_segment([p1, p2], stroke);
            painter.line_segment([p2, p3], stroke);
            painter.circle_filled(p3, 2.0, accent);
        }
        WeaponIcon::Basic => {
            let end = Pos2::new(pos.x + facing * 9.0, pos.y - 7.0);
            painter.line_segment([pos, end], stroke);
            painter.circle_filled(Pos2::new(end.x - facing * 1.0, end.y), 1.5, accent);
        }
        WeaponIcon::Double => {
            let end = Pos2::new(pos.x + facing * 14.0, pos.y - 10.0);
            painter.line_segment([pos, end], stroke);
            painter.line_segment([pos, Pos2::new(pos.x + facing * 2.5, pos.y - 4.0)], stroke);
            painter.line_segment([end, Pos2::new(end.x - facing * 2.5, end.y + 4.0)], stroke);
        }
        WeaponIcon::Ensnaring => {
            let end = Pos2::new(pos.x + facing * 10.0, pos.y - 6.0);
            painter.line_segment([pos, end], stroke);
            let ring = EguiRect::from_center_size(end, egui::vec2(6.0, 6.0));
            painter.rect_stroke(ring, 3.0, (1.0, accent));
            painter.line_segment(
                [
                    Pos2::new(ring.left(), ring.center().y),
                    Pos2::new(ring.right(), ring.center().y),
                ],
                (1.0, accent),
            );
        }
        WeaponIcon::Shield => {
            let rect = EguiRect::from_center_size(
                Pos2::new(pos.x + facing * 6.0, pos.y - 6.0),
                egui::vec2(8.0, 12.0),
            );
            painter.rect_filled(rect, 2.0, Color32::from_gray(180));
            painter.line_segment(
                [
                    Pos2::new(rect.center().x, rect.top() + 2.0),
                    Pos2::new(rect.center().x, rect.bottom() - 2.0),
                ],
                (1.0, Color32::from_gray(120)),
            );
        }
        WeaponIcon::Unarmed | WeaponIcon::Other => {
            painter.circle_filled(pos, 2.5, Color32::from_gray(200));
        }
    }
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
    categories
        .entry(TALENT_TAB_RACIALS.to_string())
        .or_default();
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
    let proficiency_snapshot = player.proficiencies.clone();
    let context = game_logic::TalentContext {
        level: player.level,
        stats: &abilities,
        talents: &talent_snapshot,
        proficiencies: &proficiency_snapshot,
        weapon_catalog: Some(weapon_catalog),
    };
    let mut add_queue: Vec<TalentSelection> = Vec::new();
    let mut remove_queue: Vec<usize> = Vec::new();
    let default_group = weapon_catalog
        .get(player.weapon_id)
        .map(|weapon| weapon_group_label(weapon.group))
        .unwrap_or(WEAPON_GROUP_LABELS[0]);

    egui::ScrollArea::vertical()
        .max_height(max_height)
        .show(ui, |ui| {
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
    let style_conflict = current_rank == 0
        && game_logic::has_other_weapon_style_selected(player, spec, talent_catalog);
    let max_rank = spec.max_rank.max(1);
    let available_rank = max_affordable_rank(spec, available_points);
    let max_selectable = if style_conflict {
        0
    } else {
        max_rank.min(available_rank)
    };
    let can_adjust = (meets_requirements || current_rank > 0) && !style_conflict;
    let force_add_enabled = !meets_requirements && current_rank == 0 && !style_conflict;
    let muted_color = ui.style().visuals.weak_text_color();
    let name_color = if meets_requirements {
        ui.style().visuals.text_color()
    } else {
        Color32::from_rgb(190, 90, 90)
    };
    let mut force_add_requested = false;
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
        ui.label(format!(
            "Cost: {} BP, {} LP, {} RP",
            cost.bp, cost.lp, cost.rp
        ));
        if !meets_requirements {
            if let Some(failure) = failures.first() {
                ui.colored_label(
                    Color32::from_rgb(190, 90, 90),
                    format_talent_requirement_failure(failure, talent_catalog),
                );
            }
            if ui
                .add_enabled(force_add_enabled, egui::Button::new("Force add"))
                .clicked()
            {
                force_add_requested = true;
            }
        }
        if style_conflict {
            ui.colored_label(
                Color32::from_rgb(190, 90, 90),
                "Only one weapon style can be active at a time.",
            );
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
                searchable_select(
                    ui,
                    format!("{id_prefix}_talent_{}", spec.id),
                    selected_text,
                    &mut rank,
                    std::iter::once((0u8, "None".to_string(), true)).chain((1..=max_rank).map(
                        |rank_value| {
                            let can_select = rank_value <= max_selectable;
                            (rank_value, format!("{rank_value}"), can_select)
                        },
                    )),
                );
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
                    searchable_select(
                        ui,
                        format!("{id_prefix}_talent_weapon_group_{}", spec.id),
                        weapon_group.clone(),
                        &mut weapon_group,
                        WEAPON_GROUP_LABELS
                            .into_iter()
                            .map(|label| (label.to_string(), label.to_string(), true)),
                    );
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
                    searchable_select(
                        ui,
                        format!("{id_prefix}_talent_weapon_{}", spec.id),
                        selected_text,
                        &mut selection.weapon,
                        weapon_catalog
                            .entries()
                            .iter()
                            .map(|weapon| (Some(weapon.name.clone()), weapon.name.clone(), true)),
                    );
                });
            });
        }

        if force_add_requested {
            selection.rank = 1;
            add_queue.push(selection);
        } else if rank != selection.rank {
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
            format!("Requires {talent_name} rank {required_rank} (current {current_rank}).")
        }
        game_logic::TalentRequirementFailure::MissingSizeLLargeSwordProficiency => {
            "Requires proficiency in at least one type of size L large sword.".to_string()
        }
        game_logic::TalentRequirementFailure::MissingShieldProficiency => {
            "Requires shield proficiency.".to_string()
        }
        game_logic::TalentRequirementFailure::MissingArmerociPoleProficiency => {
            "Requires proficiency with a large sword or polearm with at least 5 feet of reach."
                .to_string()
        }
        game_logic::TalentRequirementFailure::MissingFymblwngerProficiency => {
            "Requires battle axe, executioner's axe, or greataxe proficiency.".to_string()
        }
        game_logic::TalentRequirementFailure::MissingHammererProficiency => {
            "Requires greathammer, hammer, maul, or warhammer proficiency.".to_string()
        }
        game_logic::TalentRequirementFailure::MissingHobblerProficiency => {
            "Requires proficiency in at least one polearm or spear.".to_string()
        }
        game_logic::TalentRequirementFailure::MissingIthicanPrinceProficiency => {
            "Requires shield proficiency and at least one small sword proficiency.".to_string()
        }
        game_logic::TalentRequirementFailure::MissingRegenstatProficiency => {
            "Requires proficiency in at least one size M small or large sword.".to_string()
        }
        game_logic::TalentRequirementFailure::MissingReturnerProficiency => {
            "Requires proficiency in at least one size L large sword.".to_string()
        }
        game_logic::TalentRequirementFailure::MissingSixPathsProficiency => {
            "Requires shield proficiency and at least one size M large sword proficiency."
                .to_string()
        }
        game_logic::TalentRequirementFailure::MissingThreeMountainsProficiency => {
            "Requires proficiency in at least one crushing melee weapon.".to_string()
        }
        game_logic::TalentRequirementFailure::MissingUnbreakableWallProficiency => {
            "Requires shield proficiency.".to_string()
        }
    }
}
