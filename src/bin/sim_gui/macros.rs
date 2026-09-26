use super::*;
use game_logic::roll_macros::{DefenceMode, MacroSession, Modifier, RollKind};

pub(super) struct MacroPanel {
    pub session: MacroSession,
    scale: f32,
    #[cfg(target_arch = "wasm32")]
    copy_status: Option<std::rc::Rc<std::cell::RefCell<String>>>,
}

impl Default for MacroPanel {
    fn default() -> Self {
        Self {
            session: MacroSession::default(),
            scale: 1.0,
            #[cfg(target_arch = "wasm32")]
            copy_status: None,
        }
    }
}

impl MacroPanel {
    fn copy_text(&mut self, ctx: &egui::Context, text: String) {
        #[cfg(not(target_arch = "wasm32"))]
        ctx.output_mut(|output| output.copied_text = text);

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;

            // eframe 0.27 discards web copied_text unless web_sys_unstable_apis is
            // enabled, but that code is incompatible with our newer web-sys.
            // Use the stable API directly, starting the write in the click handler
            // so browsers that require a user gesture can accept it.
            let status = std::rc::Rc::new(std::cell::RefCell::new("Copying…".to_owned()));
            self.copy_status = Some(status.clone());
            let clipboard = web_sys::window().and_then(|window| {
                js_sys::Reflect::get(&window.navigator(), &"clipboard".into())
                    .ok()?
                    .dyn_into::<web_sys::Clipboard>()
                    .ok()
            });
            let Some(clipboard) = clipboard else {
                *status.borrow_mut() =
                    "Clipboard unavailable. Open this site over HTTPS in a browser with clipboard support."
                        .into();
                return;
            };
            let write = clipboard.write_text(&text);
            let ctx = ctx.clone();
            wasm_bindgen_futures::spawn_local(async move {
                *status.borrow_mut() = match wasm_bindgen_futures::JsFuture::from(write).await {
                    Ok(_) => "Copied to clipboard".into(),
                    Err(_) => "Copy failed. Allow clipboard access and try Copy last again.".into(),
                };
                ctx.request_repaint();
            });
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        player: &PlayerConfig,
        opponent: &PlayerConfig,
        weapons: &WeaponCatalog,
        armor: &ArmorCatalog,
        shields: &ShieldCatalog,
        npcs: &NpcPresetCatalog,
        talents: &TalentCatalog,
    ) {
        if ui.rect_contains_pointer(ui.clip_rect()) && ui.input(|i| i.modifiers.ctrl) {
            self.scale = (self.scale * ui.input(|i| i.zoom_delta())).clamp(0.5, 2.5);
        }
        ui.scope(|ui| {
            let scale = self.scale;
            let style = ui.style_mut();
            for font in style.text_styles.values_mut() {
                font.size *= scale;
            }
            style.spacing.interact_size *= scale;
            style.spacing.button_padding *= scale;
            style.spacing.item_spacing *= scale;
            style.spacing.icon_width *= scale;
            style.spacing.icon_width_inner *= scale;
            style.spacing.icon_spacing *= scale;
            style.spacing.combo_width *= scale;
            ui.heading("Combat macros");
            self.show_content(
                ui, player, opponent, weapons, armor, shields, npcs, talents, scale,
            );
        });
    }

    fn show_content(
        &mut self,
        ui: &mut egui::Ui,
        player: &PlayerConfig,
        opponent: &PlayerConfig,
        weapons: &WeaponCatalog,
        armor: &ArmorCatalog,
        shields: &ShieldCatalog,
        npcs: &NpcPresetCatalog,
        talents: &TalentCatalog,
        scale: f32,
    ) {
        let style_config = self.session.state.profile_config(player);
        let learned = game_logic::learned_weapon_style_ids(player, talents);
        let compatible = game_logic::compatible_weapon_style_ids(
            &style_config,
            talents,
            weapons,
            armor,
            shields,
        );
        let selected = game_logic::effective_default_weapon_style_ids(
            &style_config,
            talents,
            weapons,
            armor,
            shields,
        );
        ui.horizontal_wrapped(|ui| {
            ui.label("Weapon style");
            egui::ComboBox::from_id_source("macro_weapon_style")
                .selected_text(weapon_style_selection_label(&selected, talents))
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(selected.is_empty(), "No weapon style")
                        .clicked()
                    {
                        self.session.set_weapon_styles(vec![]);
                    }
                    for style in &learned {
                        let available = compatible.iter().any(|id| id.eq_ignore_ascii_case(style));
                        let name = weapon_style_display_name(style, talents);
                        let label = if available {
                            name
                        } else {
                            format!("{name} — unavailable with current equipment")
                        };
                        if ui
                            .add_enabled(
                                available,
                                egui::SelectableLabel::new(
                                    selected.len() == 1 && selected[0].eq_ignore_ascii_case(style),
                                    label,
                                ),
                            )
                            .clicked()
                        {
                            self.session.set_weapon_styles(vec![style.clone()]);
                        }
                    }
                    if game_logic::tactical_style_pair_allowed(player, &learned) {
                        let pair = vec!["shield_of_blades".into(), "storm_of_blades".into()];
                        if ui
                            .add_enabled(
                                game_logic::tactical_style_pair_allowed(player, &compatible),
                                egui::SelectableLabel::new(
                                    selected == pair,
                                    weapon_style_selection_label(&pair, talents),
                                ),
                            )
                            .clicked()
                        {
                            self.session.set_weapon_styles(pair);
                        }
                    }
                });
        });
        let can_power_attack = self.session.state.power_attack_available(player, weapons);
        if self.session.state.power_attack && !can_power_attack {
            self.session.adjust(Modifier::PowerAttack, false);
        }
        let config = self.session.state.profile_config(player);
        let actor = &game_logic::build_combatant(&config, weapons, armor, shields, npcs, talents);
        let target = &game_logic::build_combatant(opponent, weapons, armor, shields, npcs, talents);
        let penalties = game_logic::roll_macros::defensive_penalties(&config, weapons, talents);
        let preview = self.session.state.preview(actor, target);
        ui.add_space(8.0);
        match &preview {
            Ok(rolls) => {
                let roll_buttons = [
                    (RollKind::Attack, &rolls.attack),
                    (RollKind::Defence, &rolls.defence),
                    (RollKind::Damage, &rolls.damage),
                ];
                let mut draw_roll = |ui: &mut egui::Ui, kind, text: &String| {
                    let button_text = text.rsplit_once("]] ").map_or_else(
                        || text.clone(),
                        |(expression, label)| format!("{label}\n{expression}]]"),
                    );
                    if ui
                        .add_sized(
                            [ui.available_width(), 65.0 * scale],
                            egui::Button::new(egui::RichText::new(button_text).size(20.0 * scale)),
                        )
                        .clicked()
                    {
                        if let Ok(text) = self.session.copy(kind, actor, target) {
                            self.copy_text(ui.ctx(), text);
                        }
                    }
                };
                if ui.available_width() < 540.0 * scale {
                    for (kind, text) in roll_buttons {
                        draw_roll(ui, kind, text);
                    }
                } else {
                    ui.columns(3, |columns| {
                        for (i, (kind, text)) in roll_buttons.into_iter().enumerate() {
                            draw_roll(&mut columns[i], kind, text);
                        }
                    });
                }
            }
            Err(error) => {
                ui.colored_label(Color32::LIGHT_RED, error);
            }
        }
        ui.add_space(8.0);
        if let Ok(rolls) = &preview
            && rolls.damage_options.len() > 1
        {
            ui.group(|ui| {
                ui.strong("Damage options");
                let mut mode = rolls
                    .damage_options
                    .iter()
                    .find(|option| option.mode == self.session.state.damage_mode)
                    .unwrap_or(&rolls.damage_options[0])
                    .mode;
                for option in &rolls.damage_options {
                    let expression = option
                        .roll
                        .rsplit_once("]] ")
                        .map(|(expr, _)| format!("{expr}]]"))
                        .unwrap_or_else(|| option.roll.clone());
                    ui.radio_value(
                        &mut mode,
                        option.mode,
                        format!("{}   {expression}", option.label),
                    );
                }
                if mode != self.session.state.damage_mode {
                    self.session.edit(|s| s.damage_mode = mode);
                }
            });
        }
        ui.horizontal_wrapped(|ui| {
            ui.label(self.session.state.pending_label());
            if ui
                .add_enabled(self.session.can_undo(), egui::Button::new("Undo"))
                .clicked()
            {
                self.session.undo();
            }
        });
        ui.separator();
        ui.strong("Combat manoeuvres");
        ui.horizontal_wrapped(|ui| {
            for m in [
                Modifier::GiveGround,
                Modifier::ScamperBack,
                Modifier::TacticalMove,
                Modifier::Aggressive,
                Modifier::Charge,
            ] {
                let n = self.session.state.count(m);
                let effects = match m {
                    Modifier::GiveGround => "+5 DEF / −1 ATK".into(),
                    Modifier::ScamperBack => "+5 DEF / −4 ATK".into(),
                    Modifier::TacticalMove => "−1 ATK / −1 DEF".into(),
                    Modifier::Aggressive => format!(
                        "+{} ATK / −2 DEF",
                        game_logic::roll_macros::aggressive_attack_bonus(actor)
                    ),
                    Modifier::Charge => "+4 ATK / no Dex DEF, 5s".into(),
                    Modifier::PowerAttack => "×2 STR damage / no +INT, +DEX ATK".into(),
                    _ => String::new(),
                };
                let text = format!(
                    "{}{}\n{effects}",
                    m.label(),
                    if n > 0 {
                        format!(" ×{n}")
                    } else {
                        String::new()
                    }
                );
                // Active controls stay enabled so secondary-click can remove them.
                let compatible = m != Modifier::Aggressive
                    || game_logic::roll_macros::aggressive_compatible(actor);
                let enabled = (self.session.state.can_add(m) && compatible) || n > 0;
                let stacks = ui.ctx().animate_value_with_time(
                    ui.id().with(("manoeuvre-outline", m.label())),
                    n as f32,
                    0.16,
                );
                let strength = 1.0 - (-0.4 * (stacks - 1.0).max(0.0)).exp();
                let fade = stacks.clamp(0.0, 1.0);
                let green = Color32::from_rgb(
                    (170.0 - 100.0 * strength) as u8,
                    (220.0 + 35.0 * strength) as u8,
                    (180.0 - 65.0 * strength) as u8,
                );
                let mut button = egui::Button::new(text)
                    .min_size(egui::vec2(150.0 * scale, 48.0 * scale))
                    .rounding(8.0);
                if fade > 0.0 {
                    button = button
                        .stroke(egui::Stroke::new(
                            1.3 + strength,
                            green.gamma_multiply(fade),
                        ))
                        .fill(Color32::from(egui::lerp(
                            egui::Rgba::from(ui.visuals().widgets.inactive.weak_bg_fill)
                                ..=egui::Rgba::from(green),
                            (0.06 + 0.07 * strength) * fade,
                        )));
                }
                let response = ui.add_enabled(enabled, button);
                if fade > 0.0 {
                    // Soft outer glow grows brighter as the manoeuvre stacks.
                    for (spread, opacity) in [(3.0, 0.05), (1.7, 0.10), (0.6, 0.18)] {
                        ui.painter().rect_stroke(
                            response.rect.expand(spread),
                            8.0 + spread,
                            egui::Stroke::new(
                                1.5,
                                green.gamma_multiply(opacity * fade * (0.5 + strength)),
                            ),
                        );
                    }
                }
                if response.secondary_clicked()
                    || response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Delete))
                {
                    self.session.adjust(m, false);
                } else if response.clicked() && compatible {
                    self.session.adjust(m, true);
                }
            }
        });
        ui.add_space(8.0);
        ui.strong("Fight defensively");
        ui.horizontal_wrapped(|ui| {
            let mut stance = self.session.state.stance;
            ui.add_enabled_ui(!self.session.state.aggressive, |ui| {
                for (i, n) in [0, 2, 4, 6, 8].into_iter().enumerate() {
                    ui.radio_value(
                        &mut stance,
                        n,
                        if n == 0 {
                            "Normal".into()
                        } else {
                            format!("−{} ATK / +{} DEF", penalties[i], n / 2)
                        },
                    );
                }
            });
            if stance != self.session.state.stance {
                let effective = penalties[(stance / 2) as usize]
                    .max(penalties[(self.session.state.stance / 2) as usize]);
                self.session.set_stance(stance, effective);
            }
        });
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            if can_power_attack {
                let enabled = !self.session.state.aggressive;
                let selected = self.session.state.power_attack;
                let mut button =
                    egui::Button::new("Power Attack\n×2 STR damage / no +INT, +DEX ATK")
                        .selected(selected);
                if selected {
                    button =
                        button.stroke(egui::Stroke::new(1.5, Color32::from_rgb(170, 220, 180)));
                }
                let response = ui.add_enabled(enabled, button);
                if response.secondary_clicked() {
                    self.session.adjust(Modifier::PowerAttack, false);
                } else if response.clicked() {
                    self.session.adjust(Modifier::PowerAttack, !selected);
                }
            }
            if actor.sheet.offense.offhand.is_some() {
                let mut slot = self.session.state.slot;
                ui.radio_value(&mut slot, sim::WeaponSlot::Primary, "Main hand");
                ui.radio_value(&mut slot, sim::WeaponSlot::Secondary, "Off hand");
                if slot != self.session.state.slot {
                    self.session.edit(|s| {
                        s.slot = slot;
                        s.jab = false;
                    });
                }
            }
            if self.session.state.slot == sim::WeaponSlot::Primary
                && weapons
                    .get(player.weapon_id)
                    .is_some_and(|w| w.jab_speed.is_some())
            {
                let mut jab = self.session.state.jab;
                ui.add_enabled_ui(!self.session.state.aggressive, |ui| {
                    ui.radio_value(&mut jab, false, "Normal attack");
                    ui.radio_value(&mut jab, true, "Jab");
                });
                if jab != self.session.state.jab {
                    self.session.set_jab(jab);
                }
            }
        });
        let weapon = match self.session.state.slot {
            sim::WeaponSlot::Secondary => actor
                .sheet
                .offense
                .offhand
                .as_ref()
                .map(|o| o.weapon.as_ref())
                .unwrap_or(&actor.sheet.offense.weapon),
            _ => &actor.sheet.offense.weapon,
        };
        let has_range = weapon.range_bands_feet.is_some()
            || sim::max_range_for_weapon_name(&weapon.name).is_some();
        if has_range {
            ui.horizontal_wrapped(|ui| {
                if !weapon.uses_projectiles {
                    let mut ranged = self.session.state.ranged_attack;
                    ui.radio_value(&mut ranged, false, "Melee");
                    ui.radio_value(&mut ranged, true, "Thrown");
                    if ranged != self.session.state.ranged_attack {
                        self.session.edit(|s| {
                            s.ranged_attack = ranged;
                            if ranged {
                                s.aggressive = false;
                                s.charge = false;
                            }
                        });
                    }
                }
                let mut range = self.session.state.range_penalty;
                for (n, label) in [
                    (0, "Short"),
                    (-4, "Medium −4"),
                    (-6, "Long −6"),
                    (-8, "Extreme −8"),
                ] {
                    ui.radio_value(&mut range, n, label);
                }
                if range != self.session.state.range_penalty {
                    self.session.edit(|s| s.range_penalty = range);
                }
            });
        }
        ui.horizontal_wrapped(|ui| {
            ui.label("Defence:");
            let mut mode = self.session.state.defence_mode;
            for (m, label) in [
                (DefenceMode::Melee, "Melee"),
                (DefenceMode::RangedStationary, "Ranged · stationary"),
                (DefenceMode::RangedMoving, "Ranged · moving"),
            ] {
                ui.radio_value(&mut mode, m, label);
            }
            if mode != self.session.state.defence_mode {
                self.session.edit(|s| s.defence_mode = mode);
            }
        });
        ui.horizontal_wrapped(|ui| {
            if actor.sheet.defense.shield_name.is_some() {
                if ui
                    .selectable_label(self.session.state.shield_available, "Shield available")
                    .clicked()
                {
                    self.session
                        .edit(|s| s.shield_available = !s.shield_available);
                }
            }
            if !actor.sheet.offense.weapon.defense_bonus_always
                && (actor.sheet.offense.weapon.two_hand_grip
                    || actor.sheet.maneuvers.defensive_dualwielding)
            {
                if ui
                    .selectable_label(!self.session.state.weapon_ready, "between attacks -4")
                    .clicked()
                {
                    self.session.edit(|s| s.weapon_ready = !s.weapon_ready);
                }
            }
            if self.session.state.charge_defence && ui.button("Charge: 5s elapsed").clicked() {
                self.session.edit(|s| s.charge_defence = false);
            }
        });
        ui.separator();
        ui.strong("Extra modifiers");
        if self.session.state.damage_modifiers().aggressive {
            ui.horizontal_wrapped(|ui| {
                ui.label("Target:");
                let mut retreat = self.session.state.target_retreat;
                ui.radio_value(&mut retreat, None, "Stand ground");
                ui.radio_value(&mut retreat, Some(false), "Give Ground");
                ui.radio_value(&mut retreat, Some(true), "Scamper Back");
                if retreat != self.session.state.target_retreat {
                    self.session.edit(|s| s.target_retreat = retreat);
                }
            });
        }
        ui.horizontal_wrapped(|ui| {
            for kind in [RollKind::Attack, RollKind::Defence, RollKind::Damage] {
                let m = Modifier::Extra(kind);
                let r = ui.button(format!(
                    "{} {:+}",
                    kind.label(),
                    self.session.state.count(m)
                ));
                if r.secondary_clicked()
                    || r.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Delete))
                {
                    self.session.adjust(m, false);
                } else if r.clicked() {
                    self.session.adjust(m, true);
                }
            }
        });
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    self.session.state.last_macro.is_some(),
                    egui::Button::new("Copy last again"),
                )
                .clicked()
            {
                self.copy_text(ui.ctx(), self.session.state.last_macro.clone().unwrap());
            }
            if ui.button("Reset combat").clicked() {
                self.session.reset();
                #[cfg(target_arch = "wasm32")]
                {
                    self.copy_status = None;
                }
            }
        });
        #[cfg(target_arch = "wasm32")]
        if let Some(status) = &self.copy_status {
            ui.label(status.borrow().as_str());
        }
        if let Some(text) = &self.session.state.last_macro {
            ui.add(egui::Label::new(egui::RichText::new(text).monospace()).wrap(true));
        }
    }
}
