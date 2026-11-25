use super::{Combatant, CombatantSheet, SimState, StatIdI32};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum IntimidationPlan {
    Absent,
    FixedDuel { sources: [bool; 2] },
    Live,
}

impl IntimidationPlan {
    pub(super) fn new(roster: &[Combatant]) -> Self {
        if let [first, second] = roster {
            let sources = [first, second].map(|actor| {
                actor.sheet.modifiers.positive_at_zero(StatIdI32::FlagIntimidateAdversary)
            });
            if sources[0] || sources[1] {
                return Self::FixedDuel { sources };
            }
        }
        Self::Absent
    }

    pub(super) fn include_profile(&mut self, idx: usize, sheet: &CombatantSheet) {
        let emits = sheet.modifiers.positive_at_zero(StatIdI32::FlagIntimidateAdversary);
        match *self {
            Self::Absent if emits => *self = Self::Live,
            Self::FixedDuel { sources } if sources[idx] != emits => *self = Self::Live,
            _ => {}
        }
    }
}

impl SimState {
    /// All simulator combatants are treated as sentient. Overlapping auras
    /// impose the same -2 penalty once, rather than accumulating copies.
    pub(super) fn refresh_intimidation(&mut self) {
        match self.intimidation_plan() {
            IntimidationPlan::Absent => return,
            IntimidationPlan::FixedDuel { sources } => {
                let a = &self.combatants[0];
                let b = &self.combatants[1];
                let overrides = a.state.active_effects.capabilities().integers
                    | b.state.active_effects.capabilities().integers;
                if overrides & StatIdI32::FlagIntimidateAdversary.bit() == 0 {
                    let covered = a.team_id != b.team_id && a.state.hp > 0 && b.state.hp > 0
                        && self.distance_between(0, 1).is_some_and(|distance| distance <= 10.0);
                    if self.duel_in_intimidation_range != Some(covered) {
                        self.combatants[0].state.intimidated = covered && sources[1];
                        self.combatants[1].state.intimidated = covered && sources[0];
                        self.duel_in_intimidation_range = Some(covered);
                    }
                    return;
                }
                // Effects can grant or suppress a source independently of its equipment.
                self.duel_in_intimidation_range = None;
            }
            IntimidationPlan::Live => {
                if !self.combatants.iter().any(|c| c.combat_flag_active(StatIdI32::FlagIntimidateAdversary)) {
                    for actor in &mut self.combatants { actor.state.intimidated = false; }
                    return;
                }
            }
        }
        for target in 0..self.combatants.len() {
            let intimidated = self.combatants[target].state.hp > 0
                && self.combatants.iter().enumerate().any(|(source, actor)| {
                    actor.team_id != self.combatants[target].team_id
                        && actor.state.hp > 0
                        && actor.combat_flag_active(StatIdI32::FlagIntimidateAdversary)
                        && self.distance_between(source, target).is_some_and(|d| d <= 10.0)
                });
            self.combatants[target].state.intimidated = intimidated;
        }
    }
}

#[cfg(test)]
#[path = "intimidation_tests.rs"]
mod tests;
