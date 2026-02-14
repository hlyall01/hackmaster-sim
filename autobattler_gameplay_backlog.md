# Autobattler Gameplay Backlog

## Goal
Ship a playable autobattler run loop in Bevy with repeatable progression and clear player choices after each fight.

## Plan

### 1. Lock The Core Loop Contract
- Define one run cycle: `Prepare -> Fight -> Resolve Rewards -> Post-Fight Choice -> Next Encounter`.
- Finalize post-fight actions: `Progress`, `Rest`, `Train`, plus optional `Shop` if gold sinks are needed.
- Add run-end conditions: player death, voluntary retire, or milestone completion.

### 2. Finish Character Creation End-To-End
- Keep current step flow, but enforce completion gates (cannot start run with invalid state).
- Finalize talent selection validation (costs, rank caps, prerequisites).
- Finalize starter gear purchase validation (inventory slots, two-handed/shield rules, gold limits).
- Persist created character as canonical `PlayerProfile + Inventory + PointPools`.

### 3. Stabilize Combat + Encounter Pipeline
- Ensure spawn scaling is deterministic from `run_seed + encounter_index`.
- Define enemy bands by run depth and rarity.
- Add encounter metadata (normal, elite, boss) to drive rewards and event rates.
- Guarantee combat result payload includes: hp state, wounds, xp, gold, items, logs.

### 4. Rewards, Economy, And Progression
- Formalize reward tables: base xp/gold by enemy level, modifiers by encounter type.
- Add item reward tiers and drop weights.
- Ensure all rewards apply through one resolver to avoid drift.
- Add gold sinks: training costs, consumables/repairs, optional event costs.

### 5. Post-Fight Decision System
- `Progress`: immediate next encounter, no heal bonus.
- `Rest`: wound recovery/heal progression, chance of ambush/event.
- `Train`: spend time/gold for targeted growth (stat BP gain, talent progress, or combat bonuses).
- Expose risk/reward preview in UI before confirming choice.

### 6. Random Events Framework
- Add event engine with weighted pools by run depth and current state.
- Event categories: boon, hazard, trader, shrine, narrative fork.
- Each event must have deterministic roll input and explicit outcomes.
- Include at least 15 events for content variety in v1.

### 7. Level-Up And Advancement
- Use XP curve to trigger level-up in post-fight resolve.
- On level-up: apply hp progression, unlock talent opportunities, and any derived stat recompute.
- Add level-up UI checkpoint so player confirms allocations before next fight.
- Persist advancement decisions immediately.

### 8. UI/UX Pass For Full Loop
- Run HUD: current depth, HP/wounds, xp-to-next, gold, active modifiers.
- Post-fight panel: rewards summary + action choices + event result.
- Level-up modal with blocking decisions.
- Keep current low-fi visuals; prioritize clarity and speed over polish.

### 9. Save/Load + Recovery
- Save both character state and active run state at safe checkpoints.
- Add autosave at: end of fight, after decision resolve, after level-up confirm.
- Version save schema and add migration fallback for old saves.

### 10. Balance + Validation
- Add simulation tests for 100+ seeded runs to check economy/progression stability.
- Add invariants: no negative gold, no invalid talent ranks, no impossible gear states.
- Tune pacing targets: average fights-to-level, rest/train usage rate, run failure rate.

## Milestones
1. M1 (Playable): creation -> fights -> rewards -> post-fight choices -> basic level-up.
2. M2 (Progression): training/rest balancing, random events, stronger save/load.
3. M3 (Content): expanded enemy/event/item pools, balance pass, polish pass.

## Definition Of Done (v1)
- A new character can complete multiple encounters in one run.
- Rewards and level-ups work deterministically by seed.
- Post-fight choices materially change outcomes.
- Random events trigger and resolve correctly.
- Save/load fully restores an in-progress run without state loss.
