use crate::core::rng::{SimRng, derive_seed};
use crate::core::sim::{CombatEvent, CombatEventKind, Combatant, SimConfig, SimState};
use crate::core::types::{EnemyProfile, Inventory, PlayerProfile};
use serde::{Deserialize, Serialize};

use super::loot::LootTable;
use super::progression::{XpCurve, apply_xp};
use super::spawner::EnemySpawner;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncounterTier {
    Normal,
    Elite,
    Boss,
}

impl Default for EncounterTier {
    fn default() -> Self {
        Self::Normal
    }
}

impl EncounterTier {
    fn reward_ratio(self) -> (u32, u32) {
        match self {
            EncounterTier::Normal => (1, 1),
            EncounterTier::Elite => (3, 2),
            EncounterTier::Boss => (9, 4),
        }
    }

    fn level_bonus(self) -> u8 {
        match self {
            EncounterTier::Normal => 0,
            EncounterTier::Elite => 1,
            EncounterTier::Boss => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DepthBand {
    Novice,
    Veteran,
    Champion,
    Mythic,
}

impl Default for DepthBand {
    fn default() -> Self {
        Self::Novice
    }
}

impl DepthBand {
    fn level_bonus(self) -> u8 {
        match self {
            DepthBand::Novice => 0,
            DepthBand::Veteran => 1,
            DepthBand::Champion => 2,
            DepthBand::Mythic => 3,
        }
    }
}

pub fn depth_band_for_depth(depth: u32) -> DepthBand {
    match depth {
        0..=4 => DepthBand::Novice,
        5..=11 => DepthBand::Veteran,
        12..=23 => DepthBand::Champion,
        _ => DepthBand::Mythic,
    }
}

pub fn encounter_tier_for_depth(depth: u32) -> EncounterTier {
    if depth > 0 && depth % 10 == 0 {
        EncounterTier::Boss
    } else if depth > 0 && depth % 4 == 0 {
        EncounterTier::Elite
    } else {
        EncounterTier::Normal
    }
}

#[derive(Clone, Debug)]
pub struct RunState {
    pub player: PlayerProfile,
    pub inventory: Inventory,
    pub run_depth: u32,
    pub run_seed: u64,
    pub encounter_index: u32,
    pub last_encounter_tier: EncounterTier,
    pub last_encounter_band: DepthBand,
    pub event_flags: Vec<String>,
    pub seen_event_ids: Vec<String>,
    pub wounds: Vec<Wound>,
}

impl RunState {
    pub fn new(player: PlayerProfile, inventory: Inventory, run_seed: u64) -> Self {
        Self {
            player,
            inventory,
            run_depth: 0,
            run_seed,
            encounter_index: 0,
            last_encounter_tier: EncounterTier::Normal,
            last_encounter_band: DepthBand::Novice,
            event_flags: Vec::new(),
            seen_event_ids: Vec::new(),
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
    pub healing_progress_steps: u32,
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

fn scaled_enemy_level(player_level: u8, run_depth: u32, tier: EncounterTier) -> u8 {
    let depth_bonus = (run_depth / RUN_DEPTH_PER_LEVEL) as u8;
    let band_bonus = depth_band_for_depth(run_depth).level_bonus();
    player_level
        .saturating_add(depth_bonus)
        .saturating_add(band_bonus)
        .saturating_add(tier.level_bonus())
}

fn resolve_reward_for_tier(base: Reward, tier: EncounterTier) -> Reward {
    let (num, den) = tier.reward_ratio();
    let scale = |value: u32| value.saturating_mul(num) / den.max(1);
    Reward {
        gold: scale(base.gold),
        xp: scale(base.xp),
        items: base.items,
    }
}

pub fn run_next_fight<B: CombatantBuilder>(
    state: RunState,
    spawner: &EnemySpawner,
    loot_table: &LootTable,
    xp_curve: Option<&XpCurve>,
    sim_config: SimConfig,
    max_seconds: u32,
    rest_days: u32,
    resting: bool,
    tier: EncounterTier,
    builder: &B,
) -> RunOutcome {
    let effective_level = scaled_enemy_level(state.player.level, state.run_depth, tier);
    let encounter_index = state.encounter_index as u64;
    let mut spawn_rng = SimRng::from_seed(derive_seed(state.run_seed, "spawn", encounter_index));
    let enemy = spawner.spawn_for_level(effective_level, &mut spawn_rng);
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

    let mut player_combatant = builder.build_player(&state);
    let mut enemy_combatant = builder.build_enemy(&enemy_profile);
    player_combatant.team_id = 0;
    enemy_combatant.team_id = 1;
    let fight_seed = derive_seed(state.run_seed, "combat", encounter_index);
    let mut sim = SimState::with_rng(sim_config, SimRng::from_seed(fight_seed));
    sim.reset_with_combatants(vec![player_combatant, enemy_combatant]);
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

    apply_fight_result(
        state,
        Some(enemy_profile),
        fight,
        loot_table,
        xp_curve,
        rest_days,
        resting,
        tier,
    )
}

pub fn apply_fight_result(
    mut state: RunState,
    enemy: Option<EnemyProfile>,
    fight: FightResult,
    loot_table: &LootTable,
    xp_curve: Option<&XpCurve>,
    rest_days: u32,
    resting: bool,
    tier: EncounterTier,
) -> RunOutcome {
    let Some(enemy_profile) = enemy else {
        return RunOutcome {
            state,
            fight,
            reward: None,
            enemy: None,
        };
    };

    let encounter_band = depth_band_for_depth(state.run_depth);
    state.last_encounter_tier = tier;
    state.last_encounter_band = encounter_band;

    let mut new_wounds = collect_wounds(&fight.events);
    if !new_wounds.is_empty() {
        state.wounds.append(&mut new_wounds);
    }
    let fast_healer = player_has_talent(&state.player, "fast_healer");
    heal_wounds(&mut state.wounds, rest_days, fast_healer, resting);

    let reward = if fight.won {
        let encounter_index = state.encounter_index as u64;
        let mut loot_rng = SimRng::from_seed(derive_seed(state.run_seed, "loot", encounter_index));
        let loot = loot_table.roll(enemy_profile.level, &mut loot_rng);
        let base = Reward {
            gold: loot.gold,
            xp: loot.xp,
            items: loot.items,
        };
        Some(resolve_reward_for_tier(base, tier))
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

    if fight.won {
        state.run_depth = state.run_depth.saturating_add(1);
    }
    state.encounter_index = state.encounter_index.saturating_add(1);

    RunOutcome {
        state,
        fight,
        reward,
        enemy: Some(enemy_profile),
    }
}

pub fn apply_downtime(state: &mut RunState, rest_days: u32, resting: bool) {
    let fast_healer = player_has_talent(&state.player, "fast_healer");
    heal_wounds(&mut state.wounds, rest_days, fast_healer, resting);
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
                    healing_progress_steps: 0,
                });
            }
        }
    }
    wounds
}

fn player_has_talent(player: &PlayerProfile, id: &str) -> bool {
    player.talents.iter().any(|talent| talent.id == id)
}

pub fn heal_wounds(wounds: &mut Vec<Wound>, rest_days: u32, fast_healer: bool, resting: bool) {
    let mut rest_steps = rest_days.saturating_mul(4);
    if !resting {
        rest_steps /= 2;
    }
    for wound in wounds.iter_mut() {
        if wound.damage == 0 {
            continue;
        }

        let mut healing_progress = wound.healing_progress_steps.saturating_add(rest_steps);

        while wound.damage > 0 {
            let required_steps = required_healing_steps(wound.damage, fast_healer);
            if healing_progress < required_steps {
                break;
            }
            healing_progress = healing_progress.saturating_sub(required_steps);
            wound.damage -= 1;
        }

        if wound.damage == 0 {
            wound.healing_progress_steps = 0;
        } else {
            wound.healing_progress_steps = healing_progress;
        }
    }
    wounds.retain(|wound| wound.damage > 0);
}

pub fn required_healing_steps(damage: u32, fast_healer: bool) -> u32 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::gameplay::EnemySpawnEntry;
    use crate::core::ids::NpcPresetId;
    use crate::core::sim::{
        AttackEvent, AttackRollBreakdown, CombatEvent, CombatEventKind, Combatant, DamageBreakdown,
        ShieldDamageBreakdown, WeaponSlot,
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
                is_charge: false,
                weapon_slot: WeaponSlot::Primary,
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
                critical: None,
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
                healing_progress_steps: 0
            }]
        );
    }

    #[test]
    fn heals_wounds_with_rest_days() {
        let mut wounds = vec![Wound {
            damage: 7,
            healing_progress_steps: 0,
        }];

        heal_wounds(&mut wounds, 1, false, true);
        assert_eq!(
            wounds,
            vec![Wound {
                damage: 7,
                healing_progress_steps: 4
            }]
        );
    }

    #[test]
    fn halves_healing_without_rest() {
        let mut wounds = vec![Wound {
            damage: 7,
            healing_progress_steps: 0,
        }];

        heal_wounds(&mut wounds, 1, false, false);
        assert_eq!(
            wounds,
            vec![Wound {
                damage: 7,
                healing_progress_steps: 2
            }]
        );
    }

    #[test]
    fn fast_healer_recovers_wounds_faster() {
        let mut normal = vec![Wound {
            damage: 3,
            healing_progress_steps: 0,
        }];
        let mut fast = normal.clone();

        heal_wounds(&mut normal, 1, false, true);
        heal_wounds(&mut fast, 1, true, true);

        assert_eq!(
            normal,
            vec![Wound {
                damage: 3,
                healing_progress_steps: 4
            }]
        );
        assert_eq!(
            fast,
            vec![Wound {
                damage: 2,
                healing_progress_steps: 0
            }]
        );
    }

    #[derive(Clone, Copy)]
    struct DummyBuilder;

    impl CombatantBuilder for DummyBuilder {
        fn build_player(&self, _state: &RunState) -> Combatant {
            let mut combatant = Combatant::default();
            combatant.sheet.name = "Player".to_string();
            combatant
        }

        fn build_enemy(&self, _enemy: &EnemyProfile) -> Combatant {
            let mut combatant = Combatant::default();
            combatant.sheet.name = "Enemy".to_string();
            combatant
        }
    }

    fn canonical_event_lines(events: &[CombatEvent]) -> Vec<String> {
        events
            .iter()
            .map(|event| match &event.kind {
                CombatEventKind::Attack(attack) => format!(
                    "t={} a={} d={} hit={} sb={} dmg={} sd={} kb={:.1} charge={} ranged={} hp={}",
                    event.time,
                    event.attacker_idx,
                    event.defender_idx,
                    attack.hit,
                    attack.shield_block,
                    attack.damage,
                    attack.shield_damage,
                    attack.knockback_ft,
                    attack.is_charge,
                    attack.is_ranged,
                    attack.defender_hp_after
                ),
                CombatEventKind::KnockAside(knock) => format!(
                    "t={} a={} d={} knock_success={} atk={} def={}",
                    event.time,
                    event.attacker_idx,
                    event.defender_idx,
                    knock.success,
                    knock.roll.attack_total,
                    knock.roll.defense_total
                ),
            })
            .collect()
    }

    #[test]
    fn run_next_fight_is_deterministic_for_same_seed() {
        let state = RunState::new(PlayerProfile::default(), Inventory::default(), 424242);
        let spawner = EnemySpawner::new(vec![EnemySpawnEntry {
            preset_id: NpcPresetId::new(0),
            min_level: 1,
            max_level: 10,
            weight: 1,
        }]);
        let loot_table = LootTable {
            gold_range: 7..=11,
            xp_per_level: 3,
            item_table: Vec::new(),
        };
        let builder = DummyBuilder;
        let sim_config = SimConfig::new(20.0, 1.0);

        let a = run_next_fight(
            state.clone(),
            &spawner,
            &loot_table,
            None,
            sim_config,
            40,
            8,
            true,
            EncounterTier::Normal,
            &builder,
        );
        let b = run_next_fight(
            state,
            &spawner,
            &loot_table,
            None,
            sim_config,
            40,
            8,
            true,
            EncounterTier::Normal,
            &builder,
        );

        assert_eq!(a.enemy, b.enemy);
        assert_eq!(a.reward, b.reward);
        assert_eq!(a.fight.won, b.fight.won);
        assert_eq!(a.fight.remaining_hp, b.fight.remaining_hp);
        assert_eq!(a.fight.turns, b.fight.turns);
        assert_eq!(
            canonical_event_lines(&a.fight.events),
            canonical_event_lines(&b.fight.events)
        );
    }

    #[test]
    fn encounter_tier_breakpoints_match_depth() {
        assert_eq!(encounter_tier_for_depth(0), EncounterTier::Normal);
        assert_eq!(encounter_tier_for_depth(4), EncounterTier::Elite);
        assert_eq!(encounter_tier_for_depth(8), EncounterTier::Elite);
        assert_eq!(encounter_tier_for_depth(10), EncounterTier::Boss);
    }

    #[test]
    fn reward_scaling_increases_by_tier() {
        let base = Reward {
            gold: 20,
            xp: 12,
            items: vec!["Potion".to_string()],
        };
        let normal = resolve_reward_for_tier(base.clone(), EncounterTier::Normal);
        let elite = resolve_reward_for_tier(base.clone(), EncounterTier::Elite);
        let boss = resolve_reward_for_tier(base, EncounterTier::Boss);
        assert!(normal.gold <= elite.gold && elite.gold <= boss.gold);
        assert!(normal.xp <= elite.xp && elite.xp <= boss.xp);
        assert_eq!(normal.items.len(), 1);
        assert_eq!(elite.items.len(), 1);
        assert_eq!(boss.items.len(), 1);
    }

    #[test]
    fn depth_band_progression_is_stable() {
        assert_eq!(depth_band_for_depth(0), DepthBand::Novice);
        assert_eq!(depth_band_for_depth(7), DepthBand::Veteran);
        assert_eq!(depth_band_for_depth(14), DepthBand::Champion);
        assert_eq!(depth_band_for_depth(30), DepthBand::Mythic);
    }

    #[test]
    fn scaled_enemy_level_increases_with_depth_band_and_tier() {
        let normal = scaled_enemy_level(3, 2, EncounterTier::Normal);
        let elite = scaled_enemy_level(3, 8, EncounterTier::Elite);
        let boss = scaled_enemy_level(3, 20, EncounterTier::Boss);
        assert!(normal < elite);
        assert!(elite < boss);
    }

    #[test]
    fn different_seed_changes_downstream_outcome() {
        let state_a = RunState::new(PlayerProfile::default(), Inventory::default(), 111);
        let state_b = RunState::new(PlayerProfile::default(), Inventory::default(), 222);
        let spawner = EnemySpawner::new(vec![
            EnemySpawnEntry {
                preset_id: NpcPresetId::new(0),
                min_level: 1,
                max_level: 10,
                weight: 1,
            },
            EnemySpawnEntry {
                preset_id: NpcPresetId::new(1),
                min_level: 1,
                max_level: 10,
                weight: 2,
            },
        ]);
        let loot_table = LootTable {
            gold_range: 4..=16,
            xp_per_level: 3,
            item_table: Vec::new(),
        };
        let builder = DummyBuilder;
        let sim_config = SimConfig::new(20.0, 1.0);
        let a = run_next_fight(
            state_a,
            &spawner,
            &loot_table,
            None,
            sim_config,
            45,
            8,
            true,
            EncounterTier::Normal,
            &builder,
        );
        let b = run_next_fight(
            state_b,
            &spawner,
            &loot_table,
            None,
            sim_config,
            45,
            8,
            true,
            EncounterTier::Normal,
            &builder,
        );
        let changed = a.enemy != b.enemy
            || a.reward != b.reward
            || canonical_event_lines(&a.fight.events) != canonical_event_lines(&b.fight.events);
        assert!(changed, "expected at least one downstream difference");
    }
}
