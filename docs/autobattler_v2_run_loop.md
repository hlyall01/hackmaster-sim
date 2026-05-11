# Autobattler v2 Run Loop Design

## Goal
Define the v2 run loop as a deterministic sequence of typed nodes. A replay with the same run seed, config, content data, player state, and choices must produce the same node sequence, combat results, rewards, event outcomes, logs, and save checkpoints.

## State Machine
The run owns one `RunState` plus transient UI state for the active node.

```text
Start
  -> CreateOrLoadCharacter
  -> InitializeRun
  -> PrepareNextNode
  -> PresentNode
  -> ResolveNode
  -> PostNodeGate
  -> PrepareNextNode
```

Terminal and blocking branches:

- `ResolveNode -> RunComplete` when the player dies, the run is abandoned, or the configured win/end condition is met.
- `PostNodeGate -> LevelUp` when XP grants one or more levels; level-up must commit before downtime or the next node.
- `PostNodeGate -> Downtime` after a completed fight or event when the run continues.
- `Downtime -> PrepareNextNode` after rest or activity resolution.

State ownership:

- Persistent state: `RunState`, player profile, inventory, wounds, run depth, encounter index, event flags, seen event IDs, seed context, days elapsed, and save schema version.
- Transient state: pending encounter, pending event, live fight simulation, pending level-up allocation, selected downtime activity, previews, and animation/log buffers.
- Every transition that mutates persistent state writes through one resolver path and is eligible for an autosave checkpoint.

## Node Types
Each node is an atomic run step with `node_id`, `node_index`, `run_depth`, `tier`, `seed_context`, preview data, resolver input, resolver output, and log lines.

- `FightNode`: spawns an enemy, builds combatants, runs combat, applies wounds, XP, gold, items, weapon XP, depth advancement, and run-end checks.
- `EventNode`: selects a weighted event from depth/tier/state gates, presents choices, applies typed outcomes, sets flags, records seen IDs, and may trigger a forced fight.
- `DowntimeNode`: resolves rest or one activity, advances days, heals wounds, applies activity rewards/costs, and records feedback.
- `LevelUpNode`: blocks forward progress until level grants and point allocations are valid and committed.
- `ShopNode` (post-MVP): offers deterministic buy/sell/equip choices between dangerous nodes.
- `RunEndNode`: records final summary, terminal reason, final seed context, depth, elapsed days, rewards, and death/win state.

MVP node ordering is linear: at most one pending node, one live fight, one pending level-up, or one downtime choice exists at a time.

## Deterministic Seed Contract
The run has one public `run_seed: u64`. All randomness after run creation must come from `derive_seed(run_seed, domain, index)` and `SimRng::from_seed`.

Required domains:

- `creation`: character creation rolls, indexed by creation step or roll group.
- `event-spawn`: whether an event appears for an encounter index.
- `event-kind`: weighted event selection for that encounter index.
- `event-resolve`: checks and reward rolls inside the selected event.
- `spawn`: normal fight enemy selection.
- `event-forced-spawn`: enemy selection for fights forced by an event.
- `combat`: combat simulation RNG.
- `loot`: post-win loot rolls.
- `weapon-xp`: weapon mastery advancement after combat.
- `downtime-activity`: downtime activity checks and rewards.

Rules:

- Domain strings are stable API. Renaming a domain is a replay-breaking migration.
- Indexes are stable and explicit. Fight/event/downtime domains use the encounter index that caused the node; post-fight domains use the just-resolved encounter index.
- Resolvers do not use ambient entropy, wall-clock time, map iteration order, floating UI timers, or unordered collections for gameplay decisions.
- Saves persist `run_seed`, current `encounter_index`, `run_depth`, content/config version identifiers, and enough node state to resume before or after any resolver without consuming RNG differently.
- Debug UI/logs expose the active `run_seed` and relevant sub-seeds for reproduction.
- Golden replay tests compare same-seed node summaries; differential tests assert that changing `run_seed` changes at least one downstream spawn, event, combat, or reward result.

## MVP Scope
In scope:

- Create/load character into a seeded run.
- Linear run loop with deterministic event-or-fight preparation.
- Normal, elite, and boss encounter tiers by depth.
- Event choice resolution with flags, seen IDs, rewards, wounds, and optional forced fights.
- Live or fast-forwarded combat through the same resolver output.
- Post-fight/event downtime choice: rest or activity.
- Blocking level-up checkpoint before downtime or the next node.
- Autosave checkpoints at run start, post-node resolve, post-level-up, and post-downtime.
- Run summary with seed, depth, days, fights, events, rewards, and terminal reason.

Out of scope for MVP:

- Branching map paths, multiple simultaneous pending nodes, shops, equipment loadout swaps, party management, biomes, quests with map positioning, online leaderboards, and cross-version replay guarantees after content or resolver migrations.

## Acceptance Criteria
- Same seed, same content, same config, same character, and same choices produce byte-equivalent node summaries.
- Every run mutation occurs inside a node resolver or level-up commit.
- Reloading any autosave resumes without changing the next node or its sub-seeds.
- Failed runs and abandoned runs still produce complete summaries.
