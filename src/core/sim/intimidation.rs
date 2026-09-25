use super::{SimState, StatIdI32};

impl SimState {
    /// All simulator combatants are treated as sentient. Overlapping auras
    /// impose the same -2 penalty once, rather than accumulating copies.
    pub(super) fn refresh_intimidation(&mut self) {
        for target in 0..self.combatants.len() {
            let intimidated = self.combatants[target].state.hp > 0
                && self.combatants.iter().enumerate().any(|(source, actor)| {
                    actor.team_id != self.combatants[target].team_id
                        && actor.state.hp > 0
                        && actor.apply_i32(StatIdI32::FlagIntimidateAdversary, 0) > 0
                        && self.distance_between(source, target).is_some_and(|d| d <= 10.0)
                });
            self.combatants[target].state.intimidated = intimidated;
        }
    }
}

#[cfg(test)]
#[path = "intimidation_tests.rs"]
mod tests;
