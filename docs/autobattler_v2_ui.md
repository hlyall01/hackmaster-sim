# Autobattler v2 UI/UX Design

## Goal
Make the autobattler playable from the first screen and present it as a run-based RPG, not a simulator/debug console. The player should always know:
- who they are playing
- where they can go next
- what risk/reward each choice implies
- what just happened in combat
- what changed after the encounter

Keep detailed seeds, logs, and sim controls available only through an explicit developer/debug drawer.

## First-Screen Playable Layout
The first screen is the run lobby and should be immediately playable.

Layout:
- Top bar: game title, current profile name, level, depth, gold, wounds, honor, save status.
- Left panel: character slot with portrait/sprite, core stats, weapon, armor, notable talents, and a `Roll Character` secondary action.
- Center panel: `Choose Your Path` map with three encounter cards already visible for the selected character or quick start.
- Right panel: compact run forecast showing depth band, expected enemy tier, rest status, and inventory highlights.
- Bottom bar: primary `Enter` button for the selected map card, secondary `Continue Run`, `Load`, and `Settings`.

Default state:
- Auto-select the first quick-start character if no character exists.
- Auto-select the safest map card.
- The primary action should start play without requiring the user to browse save lists, set seeds, or configure enemies.

Saved characters and saved runs:
- Show as compact cards with name, level, depth, weapon, and last played date.
- Hide file names unless the debug drawer is open.
- `New Character` opens the roll flow; `Quick Start` starts a run immediately.

## Character Roll Flow
This flow should feel like rolling a tabletop character, while keeping rule details inspectable.

Steps:
1. `Roll`: roll two ability sets from the existing 56d7 + 3 method and show both as named columns.
2. `Assign`: drag or click scores into STR, INT, WIS, DEX, CON, LOOKS, CHA. Show percentile values inline.
3. `Race`: choose a race card with HP, size, stat adjustments, and racial talent notes.
4. `Tune`: spend BP on stats with a clear before/after preview and remaining BP.
5. `Identity`: set name, alignment placeholder, priors/particulars, and optional portrait/sprite.
6. `Talents`: select required/free advancement talents, then optional starting talents.
7. `Finalize`: show derived HP, attack, defense, speed, initiative, honor, money, gear, and warnings.

UX rules:
- Show a live character sheet on the right through the whole flow.
- Every spend row needs `current`, `cost`, `result`, and `remaining points`.
- Locked or deferred systems should be collapsed as `Coming later`, not presented as empty forms.
- The final button is `Begin Run`, not `Save`.
- Character creation should save automatically when finalized.

## Map Choices
Map choices are the main between-fight decision surface.

Each map node/card should show:
- encounter name and short fiction line
- depth movement, tier, and biome/location
- expected enemy hint or event type
- reward hint: XP, gold, item, talent/mastery chance, or story flag
- risk hint: wounds, hard check, elite fight, boss fight, or resource loss
- one-line reason this route matters

Default three-card mix:
- `Safe Road`: lower rewards, lower tier, better healing/rest outcomes.
- `Opportunity`: normal fight/event with balanced reward.
- `Dangerous Lead`: higher tier, possible chain/event flag, better loot or honor.

Interactions:
- Selecting a card updates the right-side forecast.
- `Scout` can reveal more detail when the player has the relevant skill/talent.
- `Rest` and `Activity` live on the map screen as downtime choices before selecting the next node.
- Event choices should appear as focused decision cards, not raw text lists.

## Fight Screen
The fight screen should prioritize readable combat state over controls.

Layout:
- Center: large arena with player and enemy sprites, weapon direction, distance, wounds, knockback, and floating damage.
- Top HUD: player and enemy nameplates with HP, wounds, armor, weapon, and current state.
- Bottom timeline: upcoming actions, initiative/speed cues, last major event, and fight timer.
- Left rail: run context, selected map node, depth, tier, and retreat status.
- Right rail: concise combat feed with important events only.

Controls:
- Default mode is automatic playback.
- Visible controls: speed, pause/resume, skip to result.
- `Step` and raw frame controls belong in the debug drawer only.
- A defeat should resolve to a clear result state, not leave the player interpreting logs.

Combat feed:
- Prefer event summaries: `Orc hits shield for 7`, `Zorya takes 3 wound`, `Bandit knocked back 10 ft`.
- Hide raw roll math by default.
- Allow expanding a feed entry to show rolls, DR, AP, crit, and shield breakdown.

## Reward Screen
After every encounter, show a reward/result screen before returning to the map.

Sections:
- `Outcome`: victory/defeat/escaped, enemy defeated, fight duration, wounds gained/healed.
- `Rewards`: XP, gold, item drops, honor, flags, mastery progress, talent/proficiency unlocks.
- `Character Changes`: level progress, stat/talent changes, HP/wound status, inventory additions.
- `Choices`: take item, compare/equip, rest, train/activity, continue deeper, return to map.

Reward cards:
- Use one card per reward category.
- Show what changed with `before -> after`.
- For items, show current equipped item beside the new item and provide `Equip`, `Keep`, `Sell later`.
- For level-up, replace raw slot allocation with a guided level-up panel: points gained, spend options, confirm summary.

Exit actions:
- Primary: `Choose Next Path`.
- Secondary: `Review Fight`.
- Debug-only: export seed/log, copy combat transcript.

## Remove From Current Debug-Style UI
Remove or hide from the default player UI:
- Run seed, spawn seed, combat seed, loot seed, and event seed labels.
- File names in save/load lists.
- Text-heavy saved run and quick-start lists.
- Raw `Run` vs `Fight` encounter wording; use route/action language like `Avoid`, `Engage`, `Scout`.
- Always-visible combat log with raw line output.
- `Pause`, `Resume`, and `Step` as equal-weight buttons; keep pause/speed visible and move step to debug.
- Raw level-up slot counters for BP/LP/AP/RP.
- `Auto-assign remaining to BP` as a primary player action.
- Bottom `Run save` filename field during normal play.
- Placeholder labels for deferred systems when they do not affect the current decision.
- Developer sprite review entry points from the normal start screen.

Keep behind a debug drawer:
- all seeds and sub-seeds
- raw combat log
- roll formulas and full roll breakdowns
- sim stepping controls
- save file paths
- event IDs, flags, and catalog IDs
- screenshot/sprite review tooling

## Visual Direction
- Use a dense RPG tool layout, not a landing page.
- Keep cards compact with clear labels, icons, and status colors.
- Use the existing sim-style arena visuals as the fight centerpiece.
- Favor stable panels and fixed action areas so combat playback and reward changes do not shift the layout.
- Put fiction in short encounter and reward lines; put mechanics in expandable details.
