# Autobattler UI + Character Creation + Level-Up Plan

## Goals
- Add a full UI-driven autobattler flow in the autobattler binary.
- Implement a detailed character creation wizard with BP/AP/LP/RP spending.
- Implement a level-up sequence that matches the rules given.
- Keep combat deterministic and data-driven.

## Decisions Locked In
- Percentiles for non-STR/DEX are tracked only for advancement and rolls, not for derived stats.
- Initiative Die progression is tied to the Initiative tier (no separate track).
- Skills, proficiencies, quirks, flaws, alignments, and item prices come later.
- Looks adjustments must modify the Charisma score (not only a modifier).
- Characters receive AP, LP and BP at creation and can spend them during creation.

## Architecture Changes
- Convert `src/bin/autobattler.rs` into an eframe UI app.
- Move the current CLI loop to `src/bin/autobattler_cli.rs` (or gate it behind `--cli`).
- Add a UI state machine for creation -> run -> level-up -> run.
- Reuse talent UI components from `src/bin/sim_gui.rs`.

## Core Data Model
- Add a full-ability struct that stores base + percentile for all 7 stats.
  - Percentiles for STR/DEX still drive derived stats as they do now.
  - Percentiles for INT/WIS/CON/LOOKS/CHA are tracked for advancement and rolls only.
- Extend `PlayerProfile` with:
  - `ability_scores_full` (base + percentile for all stats)
  - point pools: `bp`, `lp`, `ap`, `rp` and banked values
  - `progression` using existing `Progression`/`ProgressionTier` (attack/speed/initiative/health are already the advancement tracks)
  - `honor`, `alignment`, `race_id`, `background`, `quirks`, `flaws`
  - `skills`, `proficiencies` (stub lists until data arrives)
- Add conversion helpers:
  - Full ability scores -> `AbilitySet` for combat
  - Percentile normalization for STR/DEX to 01/51 for derived tables
- Store AP/LP/BP/RP spend events in creation and level-up for audit in UI.

## RNG + Dice Rules
- Use `SimRng` seeded by the UI seed for all rolls.
- Dice rules use penetrating dice where specified (e.g., d12p).
- Store percentiles as 0-99 (0 means 00). When adding percentiles:
  - If percentile >= 100, increment base by 1 per 100 and keep the remainder.
- Provide a reroll option for ability sets (two sets per creation).

## UI Flow
- Main layout: left column with wizard steps, right column with live character sheet.
- Locked steps stay visible but disabled until prior steps are complete.
- Show current points (BP/LP/AP/RP), banked totals, and remaining points.

## Character Creation Steps (Detailed)

### Step 1: Receive Points
- Initialize pools: 65 BP, 15 LP, 15 AP, 6 RP.
- Apply any race or background bonuses (when data exists).
- Show a ledger view of point sources.

### Step 2: Roll Ability Scores
- Roll 56d7 and count occurrences 1..7.
- Add +3 to each count to form 7 totals (min 4, max 18).
- If any count exceeds 18 (or would exceed 18 after +3), reroll excess dice:
  - Keep counts capped at 15 before +3 so totals cap at 18 after +3.
  - Reroll each excess die until it lands in a face below the cap.
- Repeat the whole process twice. Present both sets.
- Roll d100 for percentiles for each stat in each set. Store as 0-99.
- Allow the user to pick a set and assign scores to stats.

### Step 3: Choose Race
- Show race list from `data/races.json` with HP, size, pros, cons.
- Store `race_id` and note ability adjustments (applied in Step 5).
- Flag any racial talent categories for Step 9/10.

### Step 4: Choose Alignment (Deferred Data)
- For now: use a placeholder drop-down (e.g., Unaligned).
- Store alignment field; enable real data later.

### Step 5: Finalize Ability Scores
- Apply race adjustments to base stats.
- Spend BP to raise stats:
  - If base/percentile is below 10/01: +0/10 per BP.
  - From 10/01 up to 16/01: +0/05 per BP.
  - At 16/01 or above: +0/03 per BP.
- Apply Looks to Charisma adjustment (base) and clamp 1..25.
- Allow optional post-honor BP spending later (does not affect honor).

### Step 6: Calculate Starting Honor
- Convert each stat to a numeric value: base + (percentile / 100.0).
- Average the 7 values, floor to an integer.
- Apply honor modifiers from Looks and Charisma tables.
- Apply background/quirk modifiers later when data exists.
- Freeze honor before any post-honor BP spend.

### Step 7: Priors and Particulars (Deferred Data)
- Provide text fields for height, weight, age, handedness.
- Add placeholders for background data until the data set arrives.

### Step 8: Quirks and Flaws (Deferred Data)
- Provide a placeholder UI for now.
- When data exists, allow roll or pick and apply BP bonuses.

### Step 9: Record Advancement Talents
- Auto-grant rank 1 for talents flagged as advancement.
- Add a talent flag like `free_at_start` in `data/talents.json`.

### Step 10: Purchase Skills, Talents, Proficiencies
- Use the talent selector to spend BP/LP/RP on talents.
- Filter by race categories and race ids.
- Skills/proficiencies are stubbed for now; allow LP to remain banked.

### Step 11: Determine Hit Points
- HP = racial base HP + (CON base * health multiplier).
- Health multiplier comes from Health tier (default I).
- Apply talent and quirk modifiers.

### Step 12: Record Derived Statistics
- Use `game_logic::build_combatant` or `player_summary` for derived stats.
- Show sprint duration = floor(CON / 2) seconds.
- Display base attack, defense, initiative, speed, and base damage.

### Step 13: Money and Gear (Deferred Prices)
- Roll 75 + 4d12p for starting money.
- Show currency and bank it until prices exist.

## Level-Up Steps (Detailed)

### Step 1: Bump Stats
- Roll d20p, d12p, d10p, d8p, d6p, d4p once each.
- Assign each die to a different stat (no Looks).
- Allow one mulligan to reroll all dice.
- Add rolls to percentiles and carry to base as needed.

### Step 2: Receive Points
- Grant BP/LP/AP/RP based on target level:
  - 2-5: 20/5/5/1
  - 6-10: 25/6/5/1
  - 11-15: 30/7/5/1
  - 16-20: 35/8/5/1
  - 21+: 40/3/0/0
- Add to point pools; allow banking.

### Step 3: Extra Stat Bumps
- Spend BP with the same rules as creation.
- Allow 1 BP for +0/10 on any stat below 10/01.

### Step 4: New Skills (Deferred Data)
- When data exists, allow new skills if exposed/practiced.
- Spend BP/LP; roll mastery die with modifiers.

### Step 5: Improve Skills (Deferred Data)
- Allow improvement if used or mentored.
- Spend BP/LP; roll mastery die with modifiers.

### Step 6: New Proficiencies (Deferred Data)
- Add if exposed/practiced. Spend points.

### Step 7: New Talents
- Use the talent selector and spend BP/LP/RP.
- Respect talent requirements and ranks.

### Step 8: Finalize Advancement
- Spend AP on existing progression tiers:
  - Health I-V: 0, 5, 10, 15, 20 AP
  - Attack I-VI: 0, 2, 4, 6, 8, 15 AP
  - Speed I-VI: 0, 2, 4, 6, 8, 15 AP
  - Initiative I-V: 0, 3, 4, 5, 8 AP
- Initiative Die is tied to Initiative tier and uses the existing tables.
- Recompute derived stats and refresh the run screen.

## Deferred Content (Explicitly Out of Scope For Now)
- Skills data and mastery tables
- Proficiency list
- Quirks and flaws list and effects
- Alignment list and alignment effects
- Gear pricing and equipment catalog updates

## Implementation Milestones
1) Convert autobattler binary to UI and keep CLI in a new binary.
2) Add full ability score tracking and point pools.
3) Implement creation steps 1-6 with live sheet updates.
4) Implement talents and AP progression spending in creation using existing progression tiers.
5) Add run screen and level-up sequence using the new profile data.
6) Add tests for stat rolling, BP spending, and honor math.

## Files to Touch
- `src/bin/autobattler.rs` (UI app)
- `src/bin/autobattler_cli.rs` (existing CLI loop)
- `src/core/types.rs` (PlayerProfile fields)
- `src/character.rs` (ability data conversions)
- `src/core/gameplay/run.rs` (use updated profile)
- `src/game_logic.rs` (ability/percentile helpers)
- `data/talents.json` (add `free_at_start` flag)
- `autobattler_ui_plan.md` (this plan)
