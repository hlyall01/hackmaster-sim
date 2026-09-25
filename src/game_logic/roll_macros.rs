//! Manual table-play state. Character arithmetic stays in the simulator's resolved profiles.
use super::PlayerConfig;
pub use crate::core::sim::combat::WeaponDamageMode as DamageMode;
use crate::core::sim::{Combatant, StatIdI32, WeaponSlot, combat};

pub fn aggressive_attack_bonus(profile: &Combatant) -> i32 {
    combat::aggressive_attack_bonus(profile)
}
pub fn aggressive_compatible(profile: &Combatant) -> bool {
    let m = &profile.sheet.maneuvers;
    !m.called_shot
        && !m.power_attack
        && !m.fight_defensively
        && !m.full_parry
        && !m.fighting_withdrawal
        && !m.flee
        && !profile.sheet.offense.weapon.uses_projectiles
}
pub fn defensive_penalties(
    player: &PlayerConfig,
    weapons: &super::WeaponCatalog,
    talents: &super::TalentCatalog,
) -> [i32; 5] {
    let modifiers = super::resolve_talent_modifiers(player, talents, weapons);
    [0, 2, 4, 6, 8].map(|choice| {
        let mut p = player.clone();
        p.fight_defensively = choice > 0;
        p.fight_defensively_penalty = choice;
        super::fight_defensively_attack_penalty_with_modifiers(&p, &modifiers)
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RollKind {
    Attack,
    Defence,
    Damage,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modifier {
    GiveGround,
    ScamperBack,
    TacticalMove,
    Aggressive,
    Charge,
    PowerAttack,
    Extra(RollKind),
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DefenceMode {
    #[default]
    Melee,
    RangedStationary,
    RangedMoving,
}

/// Only one-shot choices survive Attack; formulas and character stats never do.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DamageModifiers {
    pub aggressive: bool,
}

#[derive(Clone, Debug)]
pub struct MacroState {
    pub ground: u32,
    pub scamper: u32,
    pub moves: u32,
    pub reactions: Vec<Modifier>,
    pub aggressive: bool,
    pub aggressive_defence: bool,
    pub charge: bool,
    pub charge_defence: bool,
    pub stance: i32,
    pub owed_stance: i32,
    pub extra: [i32; 3],
    pub slot: WeaponSlot,
    pub defence_mode: DefenceMode,
    pub range_penalty: i32,
    pub ranged_attack: bool,
    pub weapon_ready: bool,
    pub shield_available: bool,
    pub jab: bool,
    pub power_attack: bool,
    pub weapon_style_ids: Option<Vec<String>>,
    pub damage_mode: DamageMode,
    pub target_retreat: Option<bool>,
    pub pending_damage: DamageModifiers,
    pub last_macro: Option<String>,
}
impl Default for MacroState {
    fn default() -> Self {
        Self {
            ground: 0,
            scamper: 0,
            moves: 0,
            reactions: Vec::new(),
            aggressive: false,
            aggressive_defence: false,
            charge: false,
            charge_defence: false,
            stance: 0,
            owed_stance: 0,
            extra: [0; 3],
            slot: WeaponSlot::Primary,
            defence_mode: DefenceMode::Melee,
            range_penalty: 0,
            ranged_attack: false,
            weapon_ready: true,
            shield_available: true,
            jab: false,
            power_attack: false,
            weapon_style_ids: None,
            damage_mode: DamageMode::Normal,
            target_retreat: None,
            pending_damage: DamageModifiers::default(),
            last_macro: None,
        }
    }
}
impl RollKind {
    fn index(self) -> usize {
        match self {
            Self::Attack => 0,
            Self::Defence => 1,
            Self::Damage => 2,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Attack => "Attack",
            Self::Defence => "Defence",
            Self::Damage => "Damage",
        }
    }
}
impl Modifier {
    pub fn label(self) -> &'static str {
        match self {
            Self::GiveGround => "Give Ground",
            Self::ScamperBack => "Scamper Back",
            Self::TacticalMove => "Tactical Move",
            Self::Aggressive => "Aggressive Attack",
            Self::Charge => "Charge",
            Self::PowerAttack => "Power Attack",
            Self::Extra(k) => k.label(),
        }
    }
}
#[derive(Clone, Default)]
pub struct MacroSession {
    pub state: MacroState,
    history: Vec<MacroState>,
}
impl MacroSession {
    pub fn edit(&mut self, f: impl FnOnce(&mut MacroState)) {
        let before = (
            self.state.power_attack,
            self.state.aggressive,
            self.state.charge,
        );
        self.transition(f);
        let after = (
            self.state.power_attack,
            self.state.aggressive,
            self.state.charge,
        );
        if before != after {
            // An explicit new attack choice replaces the previous attack's choices.
            self.state.pending_damage = DamageModifiers::default();
        }
    }
    fn transition(&mut self, f: impl FnOnce(&mut MacroState)) {
        self.history.push(self.state.clone());
        if self.history.len() > 50 {
            self.history.remove(0);
        }
        f(&mut self.state);
    }
    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }
    pub fn undo(&mut self) {
        if let Some(s) = self.history.pop() {
            self.state = s;
        }
    }
    pub fn reset(&mut self) {
        self.edit(|s| *s = MacroState::default());
    }
    pub fn set_weapon_styles(&mut self, styles: Vec<String>) {
        self.edit(|s| {
            s.weapon_style_ids = Some(styles);
        });
    }
    pub fn set_jab(&mut self, jab: bool) {
        if self.state.jab != jab {
            self.edit(|s| {
                s.jab = jab;
            });
        }
    }
    pub fn set_stance(&mut self, penalty: i32, effective_penalty: i32) {
        if self.state.aggressive {
            return;
        }
        self.edit(|s| {
            s.owed_stance = s.owed_stance.max(effective_penalty);
            s.stance = penalty;
        });
    }
    pub fn adjust(&mut self, modifier: Modifier, add: bool) {
        if add && !self.state.can_add(modifier) {
            return;
        }
        self.edit(|s| match modifier {
            Modifier::Extra(kind) => {
                s.extra[kind.index()] =
                    s.extra[kind.index()].saturating_add(if add { 1 } else { -1 })
            }
            Modifier::Aggressive => {
                s.aggressive = add;
                s.aggressive_defence = add;
                if !add {
                    s.pending_damage.aggressive = false;
                }
            }
            Modifier::Charge => {
                s.charge = add;
                s.charge_defence = add;
            }
            Modifier::PowerAttack => {
                s.power_attack = add;
            }
            _ => {
                let count = match modifier {
                    Modifier::GiveGround => &mut s.ground,
                    Modifier::ScamperBack => &mut s.scamper,
                    _ => &mut s.moves,
                };
                *count = if add {
                    count.saturating_add(1).min(1000)
                } else {
                    count.saturating_sub(1)
                };
                if modifier != Modifier::TacticalMove {
                    if add {
                        s.reactions.push(modifier);
                    } else if let Some(i) = s.reactions.iter().rposition(|m| *m == modifier) {
                        s.reactions.remove(i);
                    }
                }
            }
        });
    }
    /// Calling this is the explicit copy action, never a preview or repeat-copy.
    pub fn copy(
        &mut self,
        kind: RollKind,
        profile: &Combatant,
        target: &Combatant,
    ) -> Result<String, String> {
        let preview = self.state.preview(profile, target)?;
        let text = match kind {
            RollKind::Attack => preview.attack,
            RollKind::Defence => preview.defence,
            RollKind::Damage => preview.damage,
        };
        self.transition(|s| {
            s.last_macro = Some(text.clone());
            match kind {
                RollKind::Attack => {
                    s.pending_damage = DamageModifiers {
                        aggressive: s.aggressive,
                    };
                    s.damage_mode = DamageMode::Normal;
                    s.ground = 0;
                    s.scamper = 0;
                    s.moves = 0;
                    s.owed_stance = 0;
                    s.aggressive = false;
                    s.charge = false;
                }
                RollKind::Defence => {
                    s.reactions.clear();
                    s.aggressive_defence = false;
                }
                RollKind::Damage => {
                    s.target_retreat = None;
                    s.pending_damage = DamageModifiers::default();
                }
            }
            s.extra[kind.index()] = 0;
        });
        Ok(text)
    }
}
impl MacroState {
    pub fn damage_modifiers(&self) -> DamageModifiers {
        if self.aggressive {
            DamageModifiers { aggressive: true }
        } else {
            self.pending_damage
        }
    }
    pub fn can_add(&self, modifier: Modifier) -> bool {
        match modifier {
            Modifier::Aggressive => {
                !self.aggressive
                    && !self.charge
                    && !self.charge_defence
                    && self.ground + self.scamper + self.moves == 0
                    && self.stance == 0
                    && self.owed_stance == 0
                    && !self.jab
                    && !self.power_attack
                    && !self.ranged_attack
            }
            Modifier::Charge => {
                !self.aggressive && !self.charge && !self.charge_defence && !self.ranged_attack
            }
            Modifier::PowerAttack => !self.aggressive && !self.power_attack && !self.ranged_attack,
            Modifier::Extra(_) => true,
            _ => !self.aggressive,
        }
    }
    pub fn count(&self, modifier: Modifier) -> i32 {
        match modifier {
            Modifier::GiveGround => self.ground as i32,
            Modifier::ScamperBack => self.scamper as i32,
            Modifier::TacticalMove => self.moves as i32,
            Modifier::Aggressive => i32::from(self.aggressive || self.aggressive_defence),
            Modifier::Charge => i32::from(self.charge || self.charge_defence),
            Modifier::PowerAttack => i32::from(self.power_attack),
            Modifier::Extra(k) => self.extra[k.index()],
        }
    }
    pub fn profile_config(&self, player: &PlayerConfig) -> PlayerConfig {
        let mut p = player.clone();
        if let Some(styles) = &self.weapon_style_ids {
            p.default_weapon_style_ids = Some(styles.clone());
            p.active_weapon_style_ids = Some(styles.clone());
        }
        p.tactical_policy.enabled = false;
        p.aggressive_attack = false;
        p.charge = false;
        p.give_ground = false;
        p.scamper_back = false;
        p.tactical_move = false;
        p.fight_defensively = self.stance > 0;
        p.fight_defensively_penalty = self.stance;
        p.use_jab = self.jab;
        p.power_attack = self.power_attack && !self.ranged_attack;
        p
    }
    pub fn power_attack_available(
        &self,
        player: &PlayerConfig,
        weapons: &super::WeaponCatalog,
    ) -> bool {
        let weapon_id = if self.slot == WeaponSlot::Secondary {
            player.offhand_weapon_id.unwrap_or(player.weapon_id)
        } else {
            player.weapon_id
        };
        !self.ranged_attack
            && weapons
                .get(weapon_id)
                .is_some_and(|weapon| super::power_attack_available_for_player(player, weapon))
    }
    pub fn pending_label(&self) -> String {
        let mut parts = Vec::new();
        for (count, name) in [
            (self.ground, "Give Ground"),
            (self.scamper, "Scamper Back"),
            (self.moves, "Tactical Move"),
        ] {
            if count > 0 {
                parts.push(format!("{name} ×{count}"));
            }
        }
        if self.owed_stance > 0 {
            parts.push(format!("Defensive attack −{}", self.owed_stance));
        }
        if !self.reactions.is_empty() {
            parts.push("Next defence +5".into());
        }
        if self.aggressive {
            parts.push("Aggressive attack".into());
        }
        if self.aggressive_defence {
            parts.push("Next defence −2".into());
        }
        if self.charge {
            parts.push("Charge +4 ATK".into());
        }
        if self.charge_defence {
            parts.push("No Dex defence · 5s".into());
        }
        if self.power_attack {
            parts.push("Power Attack".into());
        }
        if self.pending_damage.aggressive {
            parts.push("Damage: Aggressive Attack".into());
        }
        parts.join(" · ")
    }
    pub fn preview(&self, profile: &Combatant, target: &Combatant) -> Result<MacroPreview, String> {
        if self.aggressive && !aggressive_compatible(profile) {
            return Err("Aggressive Attack cannot combine with the selected combat manoeuvre or projectile weapon.".into());
        }
        let mut c = profile.clone();
        c.state.shield_intact &= self.shield_available;
        let slot = if self.slot == WeaponSlot::Secondary && c.sheet.offense.offhand.is_some() {
            self.slot
        } else {
            WeaponSlot::Primary
        };
        let weapon = match slot {
            WeaponSlot::Primary => &c.sheet.offense.weapon,
            WeaponSlot::Secondary => &c.sheet.offense.offhand.as_ref().unwrap().weapon,
        };
        let projectile = weapon.uses_projectiles;
        let ranged = self.ranged_attack || projectile;
        let aggressive = self.aggressive && !ranged;
        let charge = self.charge && !ranged;
        let power_attack = self.power_attack && !ranged && !weapon.is_small_weapon;
        let penalty = (self.ground + 4 * self.scamper + self.moves) as i32
            + (self.owed_stance - combat::fight_defensively_attack_penalty(&c)).max(0);
        let attack_bonus = combat::preview_attack_bonus(&c, slot) - penalty
            + self.extra[0]
            + if aggressive {
                combat::aggressive_attack_bonus(&c)
            } else {
                0
            }
            + if charge && !c.sheet.maneuvers.mounted {
                4
            } else {
                0
            }
            + if ranged { self.range_penalty } else { 0 };
        let die = if c.apply_i32(StatIdI32::FlagFallingSunStyle, 0) > 0 {
            "1d20!p>19"
        } else {
            "1d20!p"
        };
        let mut attack_labels = Vec::new();
        for (count, name) in [
            (self.ground, "Give Ground"),
            (self.scamper, "Scamper Back"),
            (self.moves, "Tactical Move"),
        ] {
            if count > 0 {
                attack_labels.push(format!("{name} ×{count}"));
            }
        }
        if self.stance > 0 || self.owed_stance > 0 {
            attack_labels.push("Fight Defensively".into());
        }
        if c.sheet.maneuvers.called_shot {
            attack_labels.push("Called shot".into());
        }
        if power_attack {
            attack_labels.push("Power attack".into());
        }
        let attack_title = if aggressive {
            "Aggressive attack"
        } else if charge {
            "Charge"
        } else {
            "Attack"
        };
        let attack = labelled(
            format!("{die}{attack_bonus:+}"),
            attack_title,
            &attack_labels,
        );
        let d = combat::preview_defense(&c);
        let (sides, mut defence_bonus) = match self.defence_mode {
            DefenceMode::Melee => {
                let shield = if c.state.shield_intact {
                    4 + c.apply_i32(
                        StatIdI32::ShieldDefenseBonus,
                        c.sheet.defense.shield_defense_bonus,
                    )
                } else {
                    0
                };
                let ready = c.sheet.offense.weapon.defense_bonus_always
                    || (self.weapon_ready
                        && (c.sheet.offense.weapon.two_hand_grip
                            || c.sheet.maneuvers.defensive_dualwielding));
                (
                    d.melee_die,
                    d.melee_bonus + shield + if ready { 4 } else { 0 },
                )
            }
            DefenceMode::RangedStationary => (d.ranged_stationary_die, d.ranged_bonus),
            DefenceMode::RangedMoving => (d.ranged_moving_die, d.ranged_bonus),
        };
        let mut defence_labels = Vec::new();
        if self.defence_mode == DefenceMode::Melee
            && !self.weapon_ready
            && !c.sheet.offense.weapon.defense_bonus_always
            && (c.sheet.offense.weapon.two_hand_grip || c.sheet.maneuvers.defensive_dualwielding)
        {
            defence_labels.push("between attacks -4".into());
        }
        if let Some(reaction) = self.reactions.last() {
            defence_bonus += 5;
            defence_labels.push(reaction.label().into());
        }
        defence_bonus -= self.moves as i32;
        if self.moves > 0 {
            defence_labels.push(format!("Tactical Move ×{}", self.moves));
        }
        if self.aggressive_defence {
            defence_bonus -= 2;
            defence_labels.push("Aggressive Attack".into());
        }
        if self.charge_defence
            && self.defence_mode == DefenceMode::Melee
            && c.sheet.defense.dex_defense_bonus > 0
        {
            defence_bonus -= c.sheet.defense.dex_defense_bonus.max(0);
            defence_labels.push("Charge, no Dex bonus".into());
        }
        if self.stance > 0 {
            defence_labels.push("Fight Defensively".into());
        }
        defence_bonus += self.extra[1];
        let defence_die = if sides == 20 {
            die.to_string()
        } else {
            format!("1d{sides}!p")
        };
        let defence = labelled(
            format!("{defence_die}{defence_bonus:+}"),
            "Defence",
            &defence_labels,
        );
        let modifiers = self.damage_modifiers();
        let damage_aggressive =
            modifiers.aggressive && !ranged && !weapon.use_jab && aggressive_compatible(&c);
        let confidence = c.apply_i32(StatIdI32::FlagProjectConfidence, 0) > 0;
        let options = combat::weapon_roll20_damage_options(&c, target, slot, ranged)
            .map_err(|e| e.to_string())?;
        // Damage has its own modifiers; attack penalties must not leak into it.
        let mut effect_labels = Vec::new();
        if weapon.use_jab {
            effect_labels.push("Jab");
        }
        if power_attack {
            effect_labels.push("Power attack");
        }
        let damage_options = options
            .iter()
            .map(|option| {
                let expression = &option.expression;
                let mut damage_expression = if self.extra[2] != 0 {
                    format!("({expression}){:+}", self.extra[2])
                } else {
                    expression.clone()
                };
                let mut damage_labels = Vec::new();
                if option.mode != DamageMode::Normal {
                    damage_labels.push(option.label.clone());
                }
                for label in &effect_labels {
                    damage_labels.push(label.to_string());
                }
                if damage_aggressive && self.target_retreat.is_some() {
                    let scamper = self.target_retreat == Some(true);
                    if scamper || !confidence {
                        damage_expression = format!("floor(({damage_expression})/2)");
                        damage_labels.push(format!(
                            "Aggressive attack vs {}",
                            if scamper {
                                "Scamper Back"
                            } else {
                                "Give Ground"
                            }
                        ));
                    }
                }
                DamageOption {
                    mode: option.mode,
                    label: option.label.clone(),
                    roll: labelled(damage_expression, "Damage", &damage_labels),
                }
            })
            .collect::<Vec<_>>();
        let damage = damage_options
            .iter()
            .find(|option| option.mode == self.damage_mode)
            .unwrap_or(&damage_options[0])
            .roll
            .clone();
        Ok(MacroPreview {
            attack,
            defence,
            damage,
            attack_bonus,
            defence_bonus,
            damage_options,
        })
    }
}
fn labelled(expression: String, title: &str, labels: &[String]) -> String {
    format!(
        "[[{expression}]] {title}{}",
        if labels.is_empty() {
            String::new()
        } else {
            format!(" — {}", labels.join(", "))
        }
    )
}
pub struct DamageOption {
    pub mode: DamageMode,
    pub label: String,
    pub roll: String,
}
pub struct MacroPreview {
    pub attack: String,
    pub defence: String,
    pub damage: String,
    pub attack_bonus: i32,
    pub defence_bonus: i32,
    pub damage_options: Vec<DamageOption>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::rules::DamageExprCache;
    use crate::core::sim::{ModifierOpI32, TemporaryEffect};
    use std::sync::Arc;
    fn actor() -> Combatant {
        let mut c = Combatant::default();
        c.sheet.offense.attack_bonus = 7;
        c.sheet.offense.strength_damage = 3;
        c.sheet.defense.defense_mod = 3;
        c.sheet.defense.dex_defense_bonus = 2;
        let w = Arc::make_mut(&mut c.sheet.offense.weapon);
        w.damage_expr = "2d6p".into();
        w.damage_expr_cache = DamageExprCache::new("2d6p");
        c
    }
    #[test]
    fn later_defence_toggle_is_manual_and_only_changes_melee_defence() {
        for dual_wield in [false, true] {
            let mut c = actor();
            Arc::make_mut(&mut c.sheet.offense.weapon).two_hand_grip = !dual_wield;
            c.sheet.maneuvers.defensive_dualwielding = dual_wield;
            let mut s = MacroSession::default();
            let first = s.state.preview(&c, &c).unwrap();
            s.edit(|state| state.weapon_ready = false);
            let later = s.state.preview(&c, &c).unwrap();
            assert_eq!(later.defence_bonus, first.defence_bonus - 4);
            assert!(later.defence.contains("between attacks -4"));
            assert_eq!(later.attack, first.attack);
            assert_eq!(later.damage, first.damage);
            for kind in [
                RollKind::Defence,
                RollKind::Defence,
                RollKind::Attack,
                RollKind::Damage,
            ] {
                s.copy(kind, &c, &c).unwrap();
                assert!(!s.state.weapon_ready);
                assert_eq!(s.state.preview(&c, &c).unwrap().defence, later.defence);
            }
            s.state.defence_mode = DefenceMode::RangedStationary;
            let ranged = s.state.preview(&c, &c).unwrap().defence;
            s.edit(|state| state.weapon_ready = true);
            assert_eq!(s.state.preview(&c, &c).unwrap().defence, ranged);
            assert!(!ranged.contains("between attacks"));
            s.state.defence_mode = DefenceMode::Melee;
            assert_eq!(s.state.preview(&c, &c).unwrap().defence, first.defence);
            s.state.weapon_ready = false;
            s.reset();
            assert!(s.state.weapon_ready);
        }
    }
    #[test]
    fn ordinary_and_always_ready_weapons_keep_their_defence_between_rolls() {
        for always in [false, true] {
            let mut c = actor();
            Arc::make_mut(&mut c.sheet.offense.weapon).defense_bonus_always = always;
            let mut s = MacroSession::default();
            s.copy(RollKind::Attack, &c, &c).unwrap();
            let defence = s.state.preview(&c, &c).unwrap().defence;
            for _ in 0..3 {
                assert_eq!(s.copy(RollKind::Defence, &c, &c).unwrap(), defence);
            }
        }
    }
    #[test]
    fn separate_consumption_right_click_and_undo_preserve_other_pending_effects() {
        let c = actor();
        let mut s = MacroSession::default();
        s.adjust(Modifier::GiveGround, true);
        s.adjust(Modifier::GiveGround, true);
        let p = s.state.preview(&c, &c).unwrap();
        assert_eq!(p.attack_bonus, 5);
        assert_eq!(p.defence_bonus, 8);
        s.adjust(Modifier::GiveGround, false);
        assert_eq!(s.state.ground, 1);
        assert_eq!(
            s.copy(RollKind::Attack, &c, &c).unwrap(),
            "[[1d20!p+6]] Attack — Give Ground ×1"
        );
        assert_eq!(s.state.ground, 0);
        assert_eq!(s.state.preview(&c, &c).unwrap().defence_bonus, 8);
        s.copy(RollKind::Defence, &c, &c).unwrap();
        assert!(s.state.reactions.is_empty());
        s.undo();
        assert_eq!(s.state.reactions.len(), 1);
        s.adjust(Modifier::GiveGround, false);
        assert!(s.state.reactions.is_empty());
        s.adjust(Modifier::GiveGround, false);
        assert_eq!(s.state.ground, 0);
    }
    #[test]
    fn defensive_stance_keeps_owed_attack_penalty_without_affecting_damage() {
        let mut c = actor();
        let mut s = MacroSession::default();
        s.set_stance(4, 4);
        c.sheet.maneuvers.fight_defensively = true;
        c.sheet.maneuvers.fight_defensively_attack_penalty = 4;
        c.sheet.maneuvers.fight_defensively_defense_bonus = 2;
        s.copy(RollKind::Attack, &c, &c).unwrap();
        s.set_stance(0, 4);
        c.sheet.maneuvers.fight_defensively = false;
        let p = s.state.preview(&c, &c).unwrap();
        assert!(p.damage.starts_with("[[2d6!p+3]]"));
        assert!(!p.damage.contains("Fight Defensively"));
        assert_eq!(p.attack_bonus, 3); // Dropping a stance cannot forgive the owed attack.
        s.copy(RollKind::Attack, &c, &c).unwrap();
        assert_eq!(s.state.owed_stance, 0);
        assert_eq!(
            s.state.preview(&c, &c).unwrap().damage,
            "[[2d6!p+3]] Damage"
        );
    }
    #[test]
    fn charge_defence_survives_rolls_until_manually_expired() {
        let c = actor();
        let mut s = MacroSession::default();
        s.adjust(Modifier::Charge, true);
        s.adjust(Modifier::Charge, true);
        assert_eq!(
            s.copy(RollKind::Attack, &c, &c).unwrap(),
            "[[1d20!p+11]] Charge"
        );
        for _ in 0..2 {
            assert!(
                s.copy(RollKind::Defence, &c, &c)
                    .unwrap()
                    .starts_with("[[1d20!p+1]]")
            );
        }
        assert_eq!(
            s.state.preview(&c, &c).unwrap().damage,
            "[[2d6!p+3]] Damage"
        );
        s.adjust(Modifier::Charge, false);
        assert_eq!(s.state.preview(&c, &c).unwrap().defence_bonus, 3);
    }
    #[test]
    fn aggressive_macros_apply_talent_and_reaction_to_damage_after_attack() {
        for confidence in [false, true] {
            let mut c = actor();
            if confidence {
                c.sheet
                    .modifiers
                    .add_i32(StatIdI32::FlagProjectConfidence, ModifierOpI32::Set(1));
            }
            let mut s = MacroSession::default();
            s.adjust(Modifier::Aggressive, true);
            assert_eq!(
                s.copy(RollKind::Attack, &c, &c).unwrap(),
                format!(
                    "[[1d20!p+{}]] Aggressive attack",
                    if confidence { 13 } else { 12 }
                )
            );
            assert_eq!(
                s.state.preview(&c, &c).unwrap().damage,
                "[[2d6!p+3]] Damage"
            );
            s.state.target_retreat = Some(false);
            assert_eq!(
                s.state.preview(&c, &c).unwrap().damage.contains("floor("),
                !confidence
            );
            assert_eq!(
                s.state
                    .preview(&c, &c)
                    .unwrap()
                    .damage
                    .contains("Aggressive attack vs Give Ground"),
                !confidence
            );
            s.state.target_retreat = Some(true);
            assert!(s.state.preview(&c, &c).unwrap().damage.contains("floor("));
            assert_eq!(s.state.preview(&c, &c).unwrap().defence_bonus, 1);
            s.copy(RollKind::Defence, &c, &c).unwrap();
            assert_eq!(s.state.preview(&c, &c).unwrap().defence_bonus, 3);
        }
    }
    #[test]
    fn movement_penalties_do_not_label_damage_before_or_after_attack_copy() {
        let c = actor();
        for modifier in [
            Modifier::GiveGround,
            Modifier::ScamperBack,
            Modifier::TacticalMove,
        ] {
            let mut session = MacroSession::default();
            session.adjust(modifier, true);
            let preview = session.state.preview(&c, &c).unwrap();
            assert!(preview.attack.contains(modifier.label()));
            assert!(preview.defence.contains(modifier.label()));
            assert_eq!(preview.damage, "[[2d6!p+3]] Damage");
            session.copy(RollKind::Attack, &c, &c).unwrap();
            assert_eq!(
                session.copy(RollKind::Damage, &c, &c).unwrap(),
                "[[2d6!p+3]] Damage"
            );
        }
        let mut c = actor();
        c.sheet.defense.dex_defense_bonus = 0;
        let mut session = MacroSession::default();
        session.adjust(Modifier::Charge, true);
        assert!(
            !session
                .state
                .preview(&c, &c)
                .unwrap()
                .defence
                .contains("Charge")
        );
    }
    #[test]
    fn macro_resolution_includes_sheet_and_active_effect_modifiers_once() {
        let mut c = actor();
        c.sheet
            .modifiers
            .add_i32(StatIdI32::AttackBonus, ModifierOpI32::Add(2));
        let mut effect = TemporaryEffect::new("test buff", 30);
        effect
            .modifiers
            .add_i32(StatIdI32::AttackBonus, ModifierOpI32::Add(3));
        c.state.add_effect(effect);
        let s = MacroState::default();
        assert_eq!(s.preview(&c, &c).unwrap().attack_bonus, 12);
    }
    #[test]
    fn jab_labels_damage_without_changing_the_attack_macro() {
        let mut c = actor();
        let state = MacroState::default();
        let normal = state.preview(&c, &c).unwrap();
        Arc::make_mut(&mut c.sheet.offense.weapon).use_jab = true;
        let jab = state.preview(&c, &c).unwrap();
        assert_eq!(jab.attack, normal.attack);
        assert!(jab.damage.contains("Jab"));
        assert!(!jab.damage.contains("!p"));
    }
    #[test]
    fn defensive_stance_preserves_penetration_for_normal_and_rohavalan_damage() {
        let mut c = actor();
        let weapon = Arc::make_mut(&mut c.sheet.offense.weapon);
        weapon.use_close_hit_damage_expr = Some("2d4p".into());
        weapon.use_close_hit_damage_expr_cache = Some(DamageExprCache::new("2d4p"));
        weapon.use_close_hit_margin_less_than = 10;
        for close_hit in [false, true] {
            let mut session = MacroSession::default();
            session.state.damage_mode = if close_hit {
                DamageMode::CloseHit
            } else {
                DamageMode::Normal
            };
            c.sheet.maneuvers.fight_defensively = false;
            let normal = session.state.preview(&c, &c).unwrap().damage;
            c.sheet.maneuvers.fight_defensively = true;
            session.set_stance(8, 8);
            let defensive = session.state.preview(&c, &c).unwrap().damage;
            assert_eq!(defensive, normal);
            assert!(defensive.contains("!p"));
            session.copy(RollKind::Attack, &c, &c).unwrap();
            session.state.damage_mode = if close_hit {
                DamageMode::CloseHit
            } else {
                DamageMode::Normal
            };
            assert_eq!(session.copy(RollKind::Damage, &c, &c).unwrap(), normal);
        }
    }
    #[test]
    fn spear_jab_refreshes_previous_attack_damage_without_halving_or_penetration() {
        let (weapons, armor, shields) = crate::data::load_catalogs().unwrap();
        let talents = crate::data::load_talents(crate::data::TALENTS_PATH).unwrap();
        let npcs = super::super::NpcPresetCatalog::new(vec![]);
        for name in ["Hasta", "Spear", "Short Spear"] {
            let id = weapons
                .entries()
                .iter()
                .position(|w| w.name == name)
                .and_then(|i| weapons.id_from_index(i))
                .unwrap();
            let mut player = PlayerConfig::new(name, id);
            player.strength_base = 18;
            let mut session = MacroSession::default();
            let build = |session: &MacroSession| {
                super::super::build_combatant(
                    &session.state.profile_config(&player),
                    &weapons,
                    &armor,
                    &shields,
                    &npcs,
                    &talents,
                )
            };
            let normal = build(&session);
            let normal_rolls = session.state.preview(&normal, &normal).unwrap();
            session.copy(RollKind::Attack, &normal, &normal).unwrap();
            session.set_jab(true);
            let jab = build(&session);
            assert!(!jab.sheet.offense.weapon.halves_damage_for_attack());
            let rolls = session.state.preview(&jab, &normal).unwrap();
            assert_eq!(rolls.attack, normal_rolls.attack);
            assert_eq!(
                rolls.damage,
                format!("{} — Jab", normal_rolls.damage.replace("!p", ""))
            );
            session.copy(RollKind::Attack, &jab, &normal).unwrap();
            assert_eq!(
                session.copy(RollKind::Damage, &jab, &normal).unwrap(),
                rolls.damage
            );
            session.set_jab(false);
            assert_eq!(
                session.state.preview(&normal, &normal).unwrap().damage,
                normal_rolls.damage
            );
        }
    }
    #[test]
    fn power_attack_is_an_eligible_persistent_toggle() {
        let (w, a, sh) = crate::data::load_catalogs().unwrap();
        let talents = crate::data::load_talents(crate::data::TALENTS_PATH).unwrap();
        let id = w
            .entries()
            .iter()
            .position(|w| w.name == "Halberd")
            .and_then(|i| w.id_from_index(i))
            .unwrap();
        let mut player = PlayerConfig::new("Power attacker", id);
        player.strength_base = 18;
        player.dex_base = 16;
        player.intelligence = 16;
        player.weapon_material_tier = 5;
        player.mastery_mut(w.get(id).unwrap().group).damage = 4;
        let mut session = MacroSession::default();
        assert!(!session.state.power_attack_available(&player, &w));
        player.talents.push(crate::core::types::TalentSelection {
            id: "power_attack".into(),
            rank: 1,
            weapon: None,
        });
        assert!(session.state.power_attack_available(&player, &w));
        player.strength_base = 12;
        assert!(!session.state.power_attack_available(&player, &w));
        player.strength_base = 18;
        session.state.ranged_attack = true;
        assert!(!session.state.power_attack_available(&player, &w));
        session.state.ranged_attack = false;
        let dagger = w
            .entries()
            .iter()
            .position(|w| w.name == "Dagger")
            .and_then(|i| w.id_from_index(i))
            .unwrap();
        player.offhand_weapon_id = Some(dagger);
        session.state.slot = WeaponSlot::Secondary;
        assert!(!session.state.power_attack_available(&player, &w));
        session.state.slot = WeaponSlot::Primary;
        player.offhand_weapon_id = None;
        let build = |state: &MacroState| {
            super::super::build_combatant(
                &state.profile_config(&player),
                &w,
                &a,
                &sh,
                &super::super::NpcPresetCatalog::new(vec![]),
                &talents,
            )
        };
        let normal = build(&session.state);
        let normal_rolls = session.state.preview(&normal, &normal).unwrap();
        session.adjust(Modifier::PowerAttack, true);
        assert!(!session.state.can_add(Modifier::Aggressive));
        session.adjust(Modifier::PowerAttack, false);
        assert!(!session.state.power_attack);
        session.adjust(Modifier::PowerAttack, true);
        let powered = build(&session.state);
        let powered_rolls = session.state.preview(&powered, &normal).unwrap();
        let character = super::super::build_character(&player, &w, &a, &sh, &talents);
        let penalty = character.ability_mods.intelligence.attack.max(0)
            + character.ability_mods.dexterity.attack.max(0);
        assert_eq!(
            powered_rolls.attack_bonus,
            normal_rolls.attack_bonus - penalty
        );
        assert_eq!(
            powered.sheet.offense.strength_damage - normal.sheet.offense.strength_damage,
            character.ability_mods.strength.damage
        );
        assert!(powered_rolls.attack.contains("Power attack"));
        assert!(powered_rolls.damage.contains("Power attack"));
        assert_eq!(powered_rolls.defence, normal_rolls.defence);
        session.copy(RollKind::Attack, &powered, &normal).unwrap();
        assert!(session.state.power_attack);
        assert_eq!(
            session.copy(RollKind::Damage, &powered, &normal).unwrap(),
            powered_rolls.damage
        );
        assert!(session.state.power_attack);
        assert_eq!(
            session.copy(RollKind::Attack, &powered, &normal).unwrap(),
            powered_rolls.attack
        );
        session.adjust(Modifier::PowerAttack, false);
        assert_eq!(
            session.state.preview(&normal, &normal).unwrap().attack,
            normal_rolls.attack
        );
    }
    #[test]
    fn rohavalan_damage_can_be_chosen_after_attack_and_uses_current_bonuses() {
        let mut c = actor();
        let mut session = MacroSession::default();
        assert_eq!(
            session.state.preview(&c, &c).unwrap().damage_options.len(),
            1
        );
        let weapon = Arc::make_mut(&mut c.sheet.offense.weapon);
        weapon.use_close_hit_damage_expr = Some("2d4p".into());
        weapon.use_close_hit_damage_expr_cache = Some(DamageExprCache::new("2d4p"));
        weapon.use_close_hit_margin_less_than = 10;
        assert_eq!(
            session.state.preview(&c, &c).unwrap().damage_options.len(),
            2
        );
        session.copy(RollKind::Attack, &c, &c).unwrap();
        // Pick the margin only after seeing the attack result on Roll20.
        session.edit(|s| s.damage_mode = DamageMode::CloseHit);
        c.sheet.offense.strength_damage = 99;
        assert_eq!(
            session.copy(RollKind::Damage, &c, &c).unwrap(),
            "[[2d4!p+99]] Damage — Close hit · margin <10"
        );
        session.edit(|s| s.damage_mode = DamageMode::Normal);
        assert_eq!(
            session.copy(RollKind::Damage, &c, &c).unwrap(),
            "[[2d6!p+99]] Damage"
        );
        session.edit(|s| s.damage_mode = DamageMode::CloseHit);
        session.copy(RollKind::Attack, &c, &c).unwrap();
        assert_eq!(session.state.damage_mode, DamageMode::Normal);
    }
    #[test]
    fn opening_damage_options_follow_resolved_dice_and_keep_attack_bonuses() {
        let (w, a, sh) = crate::data::load_catalogs().unwrap();
        let talents = crate::data::load_talents(crate::data::TALENTS_PATH).unwrap();
        let id = w
            .entries()
            .iter()
            .position(|w| w.name == "Halberd")
            .and_then(|i| w.id_from_index(i))
            .unwrap();
        let mut player = PlayerConfig::new("Opening strike", id);
        player.strength_base = 18;
        player.proficiencies = vec!["Halberd".into()];
        player.default_weapon_style_ids = Some(vec!["armeroci_pole".into()]);
        for id in ["armeroci_pole", "power_attack"] {
            player.talents.push(crate::core::types::TalentSelection {
                id: id.into(),
                rank: 1,
                weapon: None,
            });
        }
        let mut session = MacroSession::default();
        session.adjust(Modifier::PowerAttack, true);
        let mut c = super::super::build_combatant(
            &session.state.profile_config(&player),
            &w,
            &a,
            &sh,
            &super::super::NpcPresetCatalog::new(vec![]),
            &talents,
        );
        let original = session.state.preview(&c, &c).unwrap();
        let opening = original
            .damage_options
            .iter()
            .find(|o| o.mode == DamageMode::OpeningEngagement)
            .unwrap();
        assert!(
            opening
                .roll
                .starts_with(&format!("[[3d10!p+3+{}]]", c.sheet.offense.strength_damage)),
            "{}",
            opening.roll
        );
        assert!(opening.roll.contains("Power attack"));
        let current_damage = opening.roll.replace(
            &format!("+{}]]", c.sheet.offense.strength_damage),
            &format!("+{}]]", c.sheet.offense.strength_damage + 20),
        );
        session.copy(RollKind::Attack, &c, &c).unwrap();
        c.sheet.offense.strength_damage += 20;
        session.state.damage_mode = DamageMode::OpeningEngagement;
        assert_eq!(
            session.copy(RollKind::Damage, &c, &c).unwrap(),
            current_damage
        );
        session.copy(RollKind::Attack, &c, &c).unwrap();
        assert_eq!(session.state.damage_mode, DamageMode::Normal);

        session.state.ranged_attack = true;
        assert!(
            !session
                .state
                .preview(&c, &c)
                .unwrap()
                .damage_options
                .iter()
                .any(|o| o.mode == DamageMode::OpeningEngagement)
        );
    }
    #[test]
    fn generic_damage_options_preserve_replacements_and_extra_die_rules() {
        let mut c = actor();
        c.sheet.modifiers.add_i32(
            StatIdI32::OpeningEngagementExtraDamageDice,
            ModifierOpI32::Set(2),
        );
        let weapon = Arc::make_mut(&mut c.sheet.offense.weapon);
        weapon.damage_expr = "1d4p+2d8p+5".into();
        weapon.damage_expr_cache =
            DamageExprCache::new_with_max_minus_one_penetration(&weapon.damage_expr);
        weapon.use_close_hit_damage_expr_cache = Some(DamageExprCache::new("3d3p"));
        weapon.use_close_hit_margin_less_than = 7;
        let mut state = MacroState::default();
        let preview = state.preview(&c, &c).unwrap();
        assert_eq!(preview.damage_options.len(), 3);
        let opening = preview
            .damage_options
            .iter()
            .find(|o| o.mode == DamageMode::OpeningEngagement)
            .unwrap();
        assert!(
            opening.roll.starts_with("[[2d4!p>3+3d8!p>7+5+3]]"),
            "{}",
            opening.roll
        );
        let close = preview
            .damage_options
            .iter()
            .find(|o| o.mode == DamageMode::CloseHit)
            .unwrap();
        assert_eq!(close.roll, "[[3d3!p+3]] Damage — Close hit · margin <7");
        state.damage_mode = DamageMode::OpeningEngagement;
        let weapon = Arc::make_mut(&mut c.sheet.offense.weapon);
        weapon.use_jab = true;
        let jab = state.preview(&c, &c).unwrap();
        assert!(
            jab.damage.starts_with("[[floor((1d4+2d8+5+3)/2)+1d4+1d8]]"),
            "{}",
            jab.damage
        );
        assert!(jab.damage.contains("Jab"));
        assert_eq!(
            DamageExprCache::new("lower of 1d8p")
                .roll20_with_added_dice(false, &[(8, true)])
                .unwrap(),
            "{[[1d8!p]],[[1d8!p]]}kl1+1d8!p"
        );
    }
    #[test]
    fn resolved_rohavalan_style_enables_the_alternate_damage_pool() {
        let (w, a, sh) = crate::data::load_catalogs().unwrap();
        let talents = crate::data::load_talents(crate::data::TALENTS_PATH).unwrap();
        let id = w
            .entries()
            .iter()
            .position(|w| w.name == "Staff")
            .and_then(|i| w.id_from_index(i))
            .unwrap();
        let mut player = PlayerConfig::new("Arthur", id);
        player.proficiencies = vec!["Staff".into()];
        player.talents.push(crate::core::types::TalentSelection {
            id: "rohavalan_bridge".into(),
            rank: 1,
            weapon: None,
        });
        let mut state = MacroState::default();
        state.damage_mode = DamageMode::CloseHit;
        let c = super::super::build_combatant(
            &state.profile_config(&player),
            &w,
            &a,
            &sh,
            &super::super::NpcPresetCatalog::new(vec![]),
            &talents,
        );
        let preview = state.preview(&c, &c).unwrap();
        assert!(
            preview
                .damage_options
                .iter()
                .any(|o| o.mode == DamageMode::CloseHit)
        );
        assert!(preview.damage.starts_with("[[2d4!p"));
        assert!(preview.damage.contains("Close hit · margin <10"));
        assert!(!preview.attack.contains("Rohavalan"));
    }
    #[test]
    fn roll20_translation_preserves_special_penetration_and_lower_of() {
        assert_eq!(
            DamageExprCache::new_with_max_minus_one_penetration("2d8p+1")
                .roll20_expression(false)
                .unwrap(),
            "2d8!p>7+1"
        );
        assert_eq!(
            DamageExprCache::new_with_d6_penetration_triggers("3d6p+1", &[4, 5, 6])
                .roll20_expression(false)
                .unwrap(),
            "3d6!p>4+1"
        );
        assert_eq!(
            DamageExprCache::new("lower of 1d8p")
                .roll20_expression(false)
                .unwrap(),
            "{[[1d8!p]],[[1d8!p]]}kl1"
        );
        assert_eq!(
            DamageExprCache::new_with_max_minus_one_penetration("2d8p+1")
                .roll20_expression(true)
                .unwrap(),
            "2d8+1"
        );
    }
    #[test]
    fn damage_follows_current_inputs_after_an_attack_for_every_kind_of_change() {
        type Change = fn(&mut MacroSession, &mut Combatant, &mut Combatant);
        let cases: &[(&str, Change)] = &[
            ("stats", |_, c, _| c.sheet.offense.strength_damage += 10),
            ("weapon", |_, c, _| {
                let weapon = Arc::make_mut(&mut c.sheet.offense.weapon);
                weapon.damage_expr = "4d8p+2".into();
                weapon.damage_expr_cache = DamageExprCache::new(&weapon.damage_expr);
            }),
            ("off hand", |s, c, _| {
                let mut weapon = c.sheet.offense.weapon.as_ref().clone();
                weapon.damage_expr = "1d4p".into();
                weapon.damage_expr_cache = DamageExprCache::new(&weapon.damage_expr);
                c.sheet.offense.offhand = Some(crate::core::sim::OffhandProfile {
                    attack_bonus: 3,
                    strength_damage: 1,
                    weapon: Arc::new(weapon),
                });
                s.edit(|s| s.slot = WeaponSlot::Secondary);
            }),
            ("thrown", |s, _, _| s.edit(|s| s.ranged_attack = true)),
            ("mounted", |_, c, _| c.sheet.maneuvers.mounted = false),
            ("opponent size", |_, _, target| {
                target.sheet.defense.is_medium_sized = false
            }),
            ("style pool", |s, c, _| {
                let weapon = Arc::make_mut(&mut c.sheet.offense.weapon);
                weapon.use_close_hit_damage_expr_cache = Some(DamageExprCache::new("2d4p"));
                weapon.use_close_hit_margin_less_than = 10;
                s.edit(|s| s.damage_mode = DamageMode::CloseHit);
            }),
            ("talent", |s, c, _| {
                c.sheet.modifiers.add_i32(
                    StatIdI32::OpeningEngagementExtraDamageDice,
                    ModifierOpI32::Set(2),
                );
                s.edit(|s| s.damage_mode = DamageMode::OpeningEngagement);
            }),
            ("aggressive", |s, _, _| {
                s.adjust(Modifier::Aggressive, true);
                s.edit(|s| s.target_retreat = Some(true));
            }),
        ];
        for (name, change) in cases {
            let mut c = actor();
            Arc::make_mut(&mut c.sheet.offense.weapon).has_weapon = true;
            c.sheet.maneuvers.mounted = true;
            let mut target = actor();
            target.sheet.defense.is_medium_sized = true;
            let mut session = MacroSession::default();
            let before = session.state.preview(&c, &target).unwrap().damage;
            session.copy(RollKind::Attack, &c, &target).unwrap();
            change(&mut session, &mut c, &mut target);
            let current = session.state.preview(&c, &target).unwrap();
            assert_ne!(current.damage, before, "{name} must refresh Damage");
            let mut fresh = session.state.clone();
            fresh.pending_damage = DamageModifiers::default();
            let expected = fresh.preview(&c, &target).unwrap();
            assert_eq!(current.damage, expected.damage, "{name}");
            assert_eq!(
                current
                    .damage_options
                    .iter()
                    .map(|o| &o.roll)
                    .collect::<Vec<_>>(),
                expected
                    .damage_options
                    .iter()
                    .map(|o| &o.roll)
                    .collect::<Vec<_>>(),
                "{name}"
            );
            assert_eq!(
                session.copy(RollKind::Damage, &c, &target).unwrap(),
                expected.damage,
                "{name}"
            );
        }
    }
    #[test]
    fn consumed_aggressive_effect_uses_live_damage_and_has_an_explicit_lifetime() {
        let mut c = actor();
        let mut session = MacroSession::default();
        session.adjust(Modifier::Aggressive, true);
        session.copy(RollKind::Attack, &c, &c).unwrap();
        assert!(!session.state.aggressive);
        assert!(session.state.pending_damage.aggressive);
        session.edit(|s| s.target_retreat = Some(true));
        c.sheet.offense.strength_damage = 11;
        session.copy(RollKind::Defence, &c, &c).unwrap();
        assert!(session.state.pending_damage.aggressive);
        assert_eq!(
            session.copy(RollKind::Damage, &c, &c).unwrap(),
            "[[floor((2d6!p+11)/2)]] Damage — Aggressive attack vs Scamper Back"
        );
        assert!(!session.state.pending_damage.aggressive);
        session.undo();
        assert!(session.state.pending_damage.aggressive);
        assert!(
            session
                .state
                .preview(&c, &c)
                .unwrap()
                .damage
                .contains("floor(")
        );
        session.copy(RollKind::Attack, &c, &c).unwrap();
        assert!(!session.state.pending_damage.aggressive);
        assert!(
            !session
                .state
                .preview(&c, &c)
                .unwrap()
                .damage
                .contains("floor(")
        );
    }
    #[test]
    fn every_catalog_weapon_generates_macros_from_resolved_character_values() {
        let (weapons, armor, shields) = crate::data::load_catalogs().unwrap();
        let talents = crate::data::load_talents(crate::data::TALENTS_PATH).unwrap();
        let npcs = super::super::NpcPresetCatalog::new(vec![]);
        for (index, w) in weapons.entries().iter().enumerate() {
            let mut p = PlayerConfig::new("Macro", weapons.id_from_index(index).unwrap());
            p.proficiencies = vec![w.name.clone()];
            p.mastery_mut(w.group).attack = 3;
            p.mastery_mut(w.group).damage = 2;
            let s = MacroState::default();
            let c = super::super::build_combatant(
                &s.profile_config(&p),
                &weapons,
                &armor,
                &shields,
                &npcs,
                &talents,
            );
            let preview = s.preview(&c, &Combatant::default()).unwrap();
            let mut previously_attacked = MacroSession::default();
            previously_attacked
                .copy(RollKind::Attack, &actor(), &actor())
                .unwrap();
            assert_eq!(
                previously_attacked
                    .state
                    .preview(&c, &Combatant::default())
                    .unwrap()
                    .damage,
                preview.damage,
                "{} after switching weapons",
                w.name
            );
            assert_eq!(
                preview.attack_bonus,
                combat::preview_attack_bonus(&c, WeaponSlot::Primary),
                "{}",
                w.name
            );
            assert!(!preview.damage.contains('^'), "{}", w.name);
        }
    }
    #[test]
    fn project_confidence_is_resolved_from_the_catalog_selection() {
        let (w, a, s) = crate::data::load_catalogs().unwrap();
        let t = crate::data::load_talents(crate::data::TALENTS_PATH).unwrap();
        let mut p = PlayerConfig::new("Katlakehan", w.id_from_index(0).unwrap());
        p.race_id = Some("katlakehan".into());
        p.talents.push(crate::core::types::TalentSelection {
            id: "project_confidence".into(),
            rank: 1,
            weapon: None,
        });
        let c = super::super::build_combatant(
            &p,
            &w,
            &a,
            &s,
            &super::super::NpcPresetCatalog::new(vec![]),
            &t,
        );
        assert_eq!(aggressive_attack_bonus(&c), 6);
    }
    #[test]
    fn defensive_stance_retains_the_talent_reduced_penalty_when_dropped() {
        let (w, a, sh) = crate::data::load_catalogs().unwrap();
        let talents = crate::data::load_talents(crate::data::TALENTS_PATH).unwrap();
        let mut player = PlayerConfig::new("Expert", w.id_from_index(0).unwrap());
        player.talents.push(crate::core::types::TalentSelection {
            id: "combat_expertise".into(),
            rank: 1,
            weapon: None,
        });
        let penalties = defensive_penalties(&player, &w, &talents);
        assert_eq!(penalties, [0, 1, 2, 3, 4]);
        let build = |state: &MacroState| {
            super::super::build_combatant(
                &state.profile_config(&player),
                &w,
                &a,
                &sh,
                &super::super::NpcPresetCatalog::new(vec![]),
                &talents,
            )
        };
        let mut session = MacroSession::default();
        let normal = build(&session.state);
        let base = session
            .state
            .preview(&normal, &normal)
            .unwrap()
            .attack_bonus;
        session.set_stance(6, penalties[3]);
        let defensive = build(&session.state);
        assert_eq!(
            session
                .state
                .preview(&defensive, &normal)
                .unwrap()
                .attack_bonus,
            base - 3
        );
        session.set_stance(0, penalties[3]);
        assert_eq!(
            session
                .state
                .preview(&normal, &normal)
                .unwrap()
                .attack_bonus,
            base - 3
        );
        session.copy(RollKind::Attack, &normal, &normal).unwrap();
        assert_eq!(
            session
                .state
                .preview(&normal, &normal)
                .unwrap()
                .attack_bonus,
            base
        );
    }
}
