# Autobattler v2 Save/Load Schema

## Goal
Persist enough state to resume a v2 run at any checkpoint without changing the next node, sub-seeds, combat result, event result, rewards, or migration behavior. Saves are JSON, use stable snake_case field names, and store authoritative state only; previews, UI animation, expanded panels, and derived combat sheets are rebuilt on load.

## Root Object

```json
{
  "schema_version": 2,
  "save_kind": "autobattler_run",
  "created_at": "2026-05-11T00:00:00Z",
  "updated_at": "2026-05-11T00:00:00Z",
  "app_version": "0.1.0",
  "content": {
    "config_version": 1,
    "event_catalog_version": 1,
    "ruleset_id": "hackmaster_sim",
    "data_fingerprint": "optional-stable-content-hash"
  },
  "character_profile": {},
  "run_state": {},
  "map_state": {},
  "rng": {},
  "pending": {},
  "rewards": {},
  "migration": {}
}
```

Required top-level fields are `schema_version`, `save_kind`, `character_profile`, `run_state`, `rng`, and `pending`. `map_state`, `rewards`, and `migration` may be empty objects for early MVP saves but must be present.

## Character Profile
`character_profile` is the player identity and long-lived build. It should be loadable outside an active run.

Required fields:
- `character_id`: stable UUID or save-local ID.
- `name`, `level`, `xp`, `honor`, `alignment`, `race_id`, `background`.
- `ability_scores`: full STR, INT, WIS, DEX, CON, LOOKS, CHA values, including percentile where the rules support it.
- `progression`, `points`, and `banked_points`: BP/LP/AP/RP state.
- `talents`, `skills`, `proficiencies`, `weapon_masteries`.
- `equipment`: equipped weapon, armor, shield, and future loadout slots by content ID.
- `inventory`: gold and item content IDs or item instances.
- `creation`: creation seed context, BP spend history, selected quick-start or preset ID when applicable.

Do not store derived attack, defense, HP max, initiative, or combatant sheets unless they become player-authored data. Recompute them from rules and catalogs after load.

## Run State
`run_state` is the authoritative progress of the current run.

Required fields:
- `run_id`, `run_name`, `status`: `active`, `complete`, `defeated`, or `abandoned`.
- `run_depth`, `encounter_index`, `days_elapsed`, `training_days`.
- `last_encounter_tier`, `last_encounter_band`.
- `event_flags`, `seen_event_ids`.
- `wounds`: wound damage and healing progress steps.
- `pending_levelup`: null or levels gained plus BP/LP/AP/RP slot allocation.
- `last_action`: null, `rest`, `activity`, `fight`, `event`, `levelup`, or `map_choice`.
- `selected_activity`: downtime activity ID.
- `last_log`: concise player-facing resume log.
- `terminal_reason`: null until the run ends.

Run state must represent only committed mutations. If a resolver has started but not committed, save it under `pending` instead.

## Map State
`map_state` records route choice state. For the linear MVP this can be small, but the shape should support later branching maps.

Required fields:
- `map_seed`: derived from `run_seed` with the stable map domain.
- `current_floor` or `current_node_id`.
- `selected_node_id`: null unless a node is awaiting resolution.
- `available_node_ids`.
- `nodes`: stable node records with `node_id`, `floor`, `lane`, `kind`, `tier`, `depth_delta`, `reward_hint`, `risk_hint`, `revealed`, and `completed`.
- `edges`: optional list of directed node links for branching maps.

Node IDs must be deterministic for the same seed, content, and map generator version. Store completed and selected state, not only the map seed, so future map-generator changes do not corrupt existing saves.

## RNG
`rng` stores the replay contract, not live RNG internals.

Required fields:
- `run_seed`: public `u64` seed shown in debug/export.
- `seed_algorithm`: currently `derive_seed_fnv1a_splitmix64_v1`.
- `domains`: stable domain names used by resolvers.
- `active_subseeds`: the current node's derived seeds, when relevant.

Required domains:
- `creation`
- `map`
- `event-spawn`
- `event-kind`
- `event-resolve`
- `spawn`
- `event-forced-spawn`
- `combat`
- `loot`
- `weapon-xp`
- `downtime-activity`

`active_subseeds` should store the domain, index, and derived seed for pending work:

```json
{
  "active_subseeds": [
    { "domain": "event-resolve", "index": 4, "seed": 123456789 },
    { "domain": "combat", "index": 4, "seed": 987654321 }
  ]
}
```

Never persist `StdRng` bytes as the primary replay mechanism. On load, rebuild `SimRng` from the stored seed domain and index.

## Pending Choice/Fight
`pending` is the resumable boundary for uncommitted work.

```json
{
  "phase": "choose_node",
  "node": null,
  "event_choice": null,
  "fight": null,
  "reward_claim": null
}
```

Allowed `phase` values:
- `choose_node`: no resolver is active.
- `event_choice`: event is selected and waiting for a choice.
- `fight`: fight is selected or running and must resume deterministically.
- `reward`: node resolved and rewards are waiting for player confirmation.
- `levelup`: level-up allocation blocks progress.
- `downtime`: rest or activity choice blocks progress.
- `run_over`: terminal state.

`event_choice` stores `node_id`, `event_id`, `tier`, `choice_ids`, `default_choice_id`, `resolve_subseed`, and a copy of any event text needed for UI continuity. It must not apply outcomes until the choice resolver commits.

`fight` stores `node_id`, `tier`, `enemy_profile` or enemy content ID plus generated level, `spawn_subseed`, `combat_subseed`, `loot_subseed`, `rest_days`, `resting`, `max_seconds`, and `resume_mode`. MVP may resume a fight by rerunning from the start with the same seeds; if mid-fight resume is added, store a versioned combat snapshot under `fight.combat_snapshot`.

## Rewards
`rewards` tracks committed rewards and any player choice still pending.

Required fields:
- `last_reward`: last committed reward summary for resume UI.
- `pending_reward`: null or a reward requiring `take`, `equip`, `keep`, or `sell_later`.
- `history`: compact node reward records by `node_id` or `encounter_index`.

Reward records include `gold`, `xp`, `items`, `honor_delta`, `point_deltas`, `wounds_added`, `wounds_healed`, `talents_added`, `flags_set`, `flags_cleared`, `mastery_progress`, and `level_gained`. Values are deltas plus enough item IDs to avoid rerolling loot after load.

## Migration And Versioning
Versioning rules:
- `schema_version` is the save contract version. v2 loaders accept v2 and known older versions only.
- Future versions must fail with an actionable "unsupported save version" error.
- Missing optional fields use deterministic defaults and append a migration note.
- Domain names and seed derivation algorithm are stable API. Renaming either is replay-breaking and requires a schema migration.
- Content changes are tracked through `content`; exact replay is guaranteed only when content versions/fingerprints match.

Migration targets:
- v1 character saves map into `character_profile`, preserving creation fields, talents, skills, money, gear, and defaulting absent IDs to null or empty lists.
- v1 run saves map `seed` to `rng.run_seed`, `run_depth`, `encounter_index`, tier/band, flags, seen events, wounds, downtime, level-up, and logs into `run_state`.
- v1 saves without map state generate a deterministic MVP map from `run_seed`, mark no nodes completed beyond committed `encounter_index` unless explicit history exists, and set `pending.phase` to `choose_node`.
- v1 saves without pending resolver state are assumed to be checkpointed after the last committed mutation.

Every successful migration writes `migration.from_schema_version`, `migration.applied`, and `migration.warnings` in the loaded in-memory save and should rewrite autosaves as v2 on the next checkpoint.
