use crate::core::rng::SimRng;
use crate::core::sim::{CombatEvent, CombatEventKind, Combatant, SimConfig, SimState};
use crate::core::types::{EnemyProfile, Inventory, PlayerProfile};
use rand::RngCore;

use super::loot::LootTable;
use super::progression::{apply_xp, XpCurve};
use super::spawner::EnemySpawner;

#[derive(Clone, Debug)]
pub struct RunState {
    pub player: PlayerProfile,
    pub inventory: Inventory,
    pub run_depth: u32,
    pub wounds: Vec<Wound>,
}

impl RunState {
    pub fn new(player: PlayerProfile, inventory: Inventory) -> Self {
        Self {
            player,
            inventory,
            run_depth: 0,
            wounds: Vec::new(),
        }
    }

    pub fn apply_reward(&mut self, reward: &Reward) {
        self.inventory.add_gold(reward.gold);
        self.inventory.items.extend(reward.items.iter().cloned());
        self.player.xp = self.player.xp.saturating_add(reward.xp);
    }

    pub fn total_wound_damage(&self) -> u32 {
        self.wounds.iter().map(|wound| wound.damage).sum()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Wound {
    pub damage: u32,
    pub healing_progress_half_days: u32,
}

pub trait CombatantBuilder {
    fn build_player(&self, state: &RunState) -> Combatant;
    fn build_enemy(&self, enemy: &EnemyProfile) -> Combatant;
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reward {
    pub gold: u32,
    pub xp: u32,
    pub items: Vec<String>,
}

impl Reward {
    pub fn is_empty(&self) -> bool {
        self.gold == 0 && self.xp == 0 && self.items.is_empty()
    }
}

#[derive(Clone, Debug)]
pub struct FightResult {
    pub won: bool,
    pub remaining_hp: i32,
    pub turns: u32,
    pub events: Vec<CombatEvent>,
}

#[derive(Clone, Debug)]
pub struct RunOutcome {
    pub state: RunState,
    pub fight: FightResult,
    pub reward: Option<Reward>,
    pub enemy: Option<EnemyProfile>,
}

const RUN_DEPTH_PER_LEVEL: u32 = 2;

fn scaled_enemy_level(player_level: u8, run_depth: u32) -> u8 {
    let depth_bonus = (run_depth / RUN_DEPTH_PER_LEVEL) as u8;
    player_level.saturating_add(depth_bonus)
}

pub fn run_next_fight<B: CombatantBuilder>(
    mut state: RunState,
    spawner: &EnemySpawner,
    loot_table: &LootTable,
    xp_curve: Option<&XpCurve>,
    sim_config: SimConfig,
    max_seconds: u32,
    rest_days: u32,
    builder: &B,
    rng: &mut SimRng,
) -> RunOutcome {
    let effective_level = scaled_enemy_level(state.player.level, state.run_depth);
    let enemy = spawner.spawn_for_level(effective_level, rng);
    let Some(enemy_profile) = enemy else {
        return RunOutcome {
            state,
            fight: FightResult {
                won: false,
                remaining_hp: 0,
                turns: 0,
                events: Vec::new(),
            },
            reward: None,
            enemy: None,
        };
    };

    let player_combatant = builder.build_player(&state);
    let enemy_combatant = builder.build_enemy(&enemy_profile);
    let fight_seed = rng.next_u64();
    let mut sim = SimState::with_rng(sim_config, SimRng::from_seed(fight_seed));
    sim.reset_with_combatants([player_combatant, enemy_combatant]);
    while !sim.done && sim.elapsed_seconds < max_seconds {
        sim.update(1.0);
    }

    let player_hp = sim.combatants[0].state.hp;
    let enemy_hp = sim.combatants[1].state.hp;
    let won = sim.done && player_hp > 0 && enemy_hp <= 0;
    let fight = FightResult {
        won,
        remaining_hp: player_hp,
        turns: sim.elapsed_seconds,
        events: sim.combat_events.clone(),
    };

    let mut new_wounds = collect_wounds(&sim.combat_events);
    if !new_wounds.is_empty() {
        state.wounds.append(&mut new_wounds);
    }
    heal_wounds(&mut state.wounds, rest_days);

    let reward = if won {
        let loot = loot_table.roll(enemy_profile.level, rng);
        Some(Reward {
            gold: loot.gold,
            xp: loot.xp,
            items: loot.items,
        })
    } else {
        None
    };

    if let Some(reward) = reward.as_ref() {
        state.inventory.add_gold(reward.gold);
        state.inventory.items.extend(reward.items.iter().cloned());
        if reward.xp > 0 {
            if let Some(curve) = xp_curve {
                let _ = apply_xp(&mut state.player, curve, reward.xp);
            } else {
                state.player.xp = state.player.xp.saturating_add(reward.xp);
            }
        }
    }

    if won {
        state.run_depth = state.run_depth.saturating_add(1);
    }

    RunOutcome {
        state,
        fight,
        reward,
        enemy: Some(enemy_profile),
    }
}

fn collect_wounds(events: &[CombatEvent]) -> Vec<Wound> {
    let mut wounds = Vec::new();
    for event in events {
        if event.defender_idx != 0 {
            continue;
        }
        if let CombatEventKind::Attack(attack) = &event.kind {
            if attack.damage > 0 {
                wounds.push(Wound {
                    damage: attack.damage as u32,
                    healing_progress_half_days: 0,
                });
            }
        }
    }
    wounds
}

fn heal_wounds(wounds: &mut Vec<Wound>, rest_days: u32) {
    let rest_half_days = rest_days.saturating_mul(2);
    for wound in wounds.iter_mut() {
        if wound.damage == 0 {
            continue;
        }

        wound.damage = wound.damage.saturating_sub(1);
        if wound.damage == 0 {
            wound.healing_progress_half_days = 0;
            continue;
        }

        wound.healing_progress_half_days =
            wound.healing_progress_half_days.saturating_add(rest_half_days);
        while wound.damage > 0 {
            let required_half_days = wound.damage;
            if wound.healing_progress_half_days < required_half_days {
                break;
            }
            wound.healing_progress_half_days -= required_half_days;
            wound.damage -= 1;
        }

        if wound.damage == 0 {
            wound.healing_progress_half_days = 0;
        }
    }
    wounds.retain(|wound| wound.damage > 0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::sim::{
        AttackEvent, AttackRollBreakdown, CombatEventKind, DamageBreakdown, ShieldDamageBreakdown,
    };

    fn attack_event_with_damage(damage: i32, defender_idx: usize) -> CombatEvent {
        CombatEvent {
            time: 0,
            attacker_idx: 1,
            defender_idx,
            kind: CombatEventKind::Attack(AttackEvent {
                hit: true,
                shield_block: false,
                damage,
                shield_damage: 0,
                knockback_ft: 0.0,
                hold_at_bay: false,
                use_jab: false,
                is_ranged: false,
                trauma_applied: false,
                trauma_seconds: None,
                roll: AttackRollBreakdown {
                    attack_die: 1,
                    defense_die: 1,
                    attack_bonus: 0,
                    range_mod: 0,
                    defense_base: 0,
                    weapon_defense_bonus: 0,
                    shield_defense_bonus: 0,
                    attack_total: 1,
                    defense_total: 1,
                },
                damage_breakdown: Some(DamageBreakdown {
                    rolled_damage: damage,
                    strength_damage: 0,
                    raw_damage: damage,
                    armor_dr: 0,
                    armor_penetration: 0,
                    effective_armor_dr: 0,
                    final_damage: damage,
                }),
                shield_damage_breakdown: Some(ShieldDamageBreakdown {
                    rolled_damage: 0,
                    strength_damage: 0,
                    raw_damage: 0,
                    shield_dr: 0,
                    armor_dr: 0,
                    armor_penetration: 0,
                    effective_armor_dr: 0,
                    hp_damage: 0,
                    shield_broken: false,
                }),
                defender_hp_after: 0,
            }),
        }
    }

    #[test]
    fn collects_player_wounds_from_damage_events() {
        let events = vec![
            attack_event_with_damage(7, 0),
            attack_event_with_damage(0, 0),
            attack_event_with_damage(3, 1),
        ];

        let wounds = collect_wounds(&events);
        assert_eq!(
            wounds,
            vec![Wound {
                damage: 7,
                healing_progress_half_days: 0
            }]
        );
    }

    #[test]
    fn heals_wounds_with_rest_days() {
        let mut wounds = vec![Wound {
            damage: 7,
            healing_progress_half_days: 0,
        }];

        heal_wounds(&mut wounds, 1);
        assert_eq!(
            wounds,
            vec![Wound {
                damage: 6,
                healing_progress_half_days: 2
            }]
        );
    }
}
