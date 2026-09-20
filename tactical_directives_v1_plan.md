# Tactical Directives v1 Plan

## Goal

Implement a native `sim_gui` Tactical Directives system that lets each character select learned weapon styles and combat maneuvers through prioritized conditional rules, with reusable JSON presets and deterministic simulation behavior.

## 1. Tactical rule model

Add a serializable tactical policy to each `PlayerConfig`:

```rust
TacticalPolicy {
    enabled: bool,
    rules: Vec<TacticalRule>,
}
```

Each rule contains:

```rust
TacticalRule {
    enabled: bool,
    decision: TacticalDecisionPoint,
    conditions: Vec<TacticalCondition>, // Maximum two, combined with AND.
    action: TacticalAction,
}
```

Decision points:

- Next attack opportunity
- Incoming attack reaction

Actions are separated into channels so compatible decisions can apply together:

- **Weapon style:** retain style, neutral style, or activate a learned style.
- **Attack mode:** normal attack or Jab.
- **Stance:** neutral or Fight Defensively at -2/-4/-6/-8.
- **Reaction:** stand ground or Give Ground.

The first legal matching rule in each channel wins. This allows one attack to use a selected weapon style, Jab, and Fight Defensively simultaneously.

Locked fallbacks:

- Style: retain current style.
- Attack: normal attack.
- Stance: neutral stance.
- Reaction: stand ground.

Invalid actions are skipped and evaluation continues with the next applicable rule.

## 2. Conditions

Support these initial conditions with appropriate numeric, enum, and boolean comparisons:

- Always
- My HP percentage
- Enemy HP percentage
- Current distance in feet
- My reach compared with enemy reach
- Retreat space available
- My weapon supports Jab
- Enemy weapon group
- Enemy has an active shield
- Enemy armor type
- Enemy is charging
- I have attacked at least once this combat
- Enemy time to reach me in seconds
- My active weapon style
- Enemy active weapon style
- Enemy DR
- Enemy primary attack speed in seconds
- Enemy attack speed is faster/slower than mine

Enemy DR uses the defender's current Armor DR after active effects but before attacker-specific penetration.

Attack-speed conditions use the current runtime speed after weapon styles and temporary modifiers. The UI must explain that lower seconds means faster.

Enemy time to reach uses the current distance, the enemy's melee reach, and its current movement speed. A stationary or incapacitated enemy has an infinite time to reach.

## 3. Learned weapon styles

Treat weapon-style knowledge and weapon-style activation as separate state.

- The Talents tab allows a character to learn any number of weapon styles while retaining normal acquisition requirements.
- Learning another style never deactivates, replaces, or removes an already learned style.
- The Combat Maneuvers tab owns the active selection:
  - **Default style** while Tactical Directives are disabled.
  - **Opening style** while Tactical Directives are enabled.
- The Default style remains active throughout ordinary combat.
- The Opening style is active at combat start and remains active until a directive changes it at the character's next attack opportunity.
- `No weapon style` is an explicit selection.
- Learned styles that do not work with the current weapon, offhand, shield, armor, or dual-wield mode remain visible but disabled with an explanation.
- An already selected style that becomes incompatible is preserved in the character/preset data. Combat falls back to no style until the loadout is compatible again or the user selects another style.
- Legacy fighter presets without an explicit style selection infer the same first learned style used by the old system. Saving the preset records the selection explicitly.
- Fighter presets save the Default/Opening selection; tactical presets may also carry an Opening style so a complete AI preset can establish its starting stance.

At runtime:

- Only one learned style may be active.
- Shield of Blades and Storm of Blades may be active together when the character has Perfect Two-Weapon Fighting.
- Style rules are evaluated when the character reaches their next attack opportunity.
- The selected style activates immediately before that attack and remains active afterward.
- Defenses before that opportunity continue using the previously active style.
- The new style determines the current attack and its subsequent recovery speed.
- Switching styles never rewinds or accelerates the already-scheduled attack opportunity.

Build combat-sheet profiles for every valid learned-style selection when combat resets. Switching styles swaps the active immutable profile while preserving HP, attack timers, temporary effects, and other runtime state.

Prevent style-switch exploits:

- Leaving a style clears counters requiring continuous use.
- Returning to a style does not repeatedly restore once-per-engagement opening benefits.
- Invalid styles caused by equipment changes remain in rules but are skipped with a warning.
- Invalid Default/Opening styles fall back to the neutral profile without deleting the saved selection.

## 4. Maneuver implementation

### Jab

Refactor weapon profiles to retain both normal and Jab attack data.

When selected:

- Use Jab recovery speed.
- Use Jab-specific damage when present; otherwise halve normal damage.
- Disable penetration.
- Fall back to the next attack rule if the weapon cannot Jab.

### Fight Defensively

Store the active defensive stance in runtime combat state.

- Apply the selected defense bonus while active.
- Apply the corresponding attack penalty, including talent reductions.
- If the stance is dropped, preserve its penalty for the next attack in the same engagement.
- Consume the pending attack penalty after that attack.

### Give Ground

Evaluate immediately before resolving an incoming attack.

Eligibility requires:

- A legal retreat position.
- The attacker is not charging.
- The attacker does not walk faster than the defender.
- The defender is not otherwise prevented from moving.

On use:

- Move the defender backward using the simulator's walking movement.
- Allow the attacker to advance with the retreat where space permits.
- Add +5 to the current Defense roll.
- Apply -1 to the defender's next Attack roll.
- Emit a combat event describing the matched directive and movement.
- If Give Ground is illegal, continue evaluating reaction rules and ultimately stand ground.

## 5. Native `sim_gui` UI

Add a **Tactics** tab to the existing Customize Character window.

Add a weapon-style selector to **Combat Maneuvers**, above the static maneuver controls:

```text
Weapon Style
Default style: [No weapon style / learned compatible style]
```

The label changes from `Default style` to `Opening style` when Tactical Directives are enabled. The selector lists every learned style, disables currently incompatible choices, and exposes the Shield of Blades + Storm of Blades pair only when Perfect Two-Weapon Fighting and the loadout make the pair legal.

Top toolbar:

```text
[Enable Tactical Directives]

Preset: [Select preset] [Load]
Save as: [Name____________] [Save]
[Rename] [Delete]
```

Ordered rule rows:

```text
[x] 1  NEXT ATTACK - STYLE
     IF [Enemy DR] [>=] [5]
    AND [Enemy speed] [is slower than mine]
   THEN [Use style] [Doomrazor]
   [Up] [Down] [Duplicate] [Delete]
```

Each row provides:

- Enabled checkbox
- Priority number
- Decision-point/channel badge
- Up to two condition editors
- Action editor
- Move up/down controls
- Duplicate and delete controls
- Inline compatibility warning

Editing behavior:

- Maintain an editor draft.
- **Save** validates and applies the complete list.
- **Cancel/Revert** discards changes since the editor was opened or last saved.
- Loading a preset replaces the draft after confirming unsaved changes.
- Tactical editing is locked after combat time has advanced; Reset unlocks it.
- Managed static checkboxes in Combat Maneuvers are disabled with a "Controlled by Tactical Directives" explanation while the policy is enabled.
- Display the current active style and last matched directive in the simulator status panel and combat log.

## 6. Tactical presets

Add a separate native preset file:

```text
data/sim/tactical_presets.json
```

Use the existing writable-data-path behavior used by fighter presets.

Preset schema:

```json
{
  "schema_version": 1,
  "presets": [
    {
      "name": "Armored Opponent",
      "opening_style_ids": ["doomrazor"],
      "rules": []
    }
  ]
}
```

Behavior:

- Tactical presets contain rules and an optional Opening style, but not character stats, talents, or equipment.
- Names are unique case-insensitively.
- Saving an existing name requires overwrite confirmation.
- Loading replaces the current draft.
- Deleting requires confirmation.
- Incompatible rules are preserved and shown with warnings.
- Preset load failures show an in-app error while leaving current tactics untouched.
- Fighter presets remain independent and do not overwrite tactics.

Bundled v1 presets:

1. Balanced
2. Cautious Defender
3. Last Stand
4. Quick Finisher
5. Rapid Jabs
6. Armored Opponent
7. Speed Counter
8. Reach Keeper
9. Hammer and Shield
10. Arthur - Armeroci Bridge
11. Perfect Blades

## 7. Compatibility

Add an explicit **Enable Tactical Directives** toggle.

When disabled:

- The selected Default style is used for the whole combat.
- Characters may still learn multiple styles.
- Existing fighter preset JSON remains valid.
- Legacy fighter presets infer their prior first-style behavior.

When enabled:

- Directives control learned styles, Normal/Jab selection, Fight Defensively, and Give Ground.
- Other existing maneuvers continue using their current static configuration until added to the directive system later.

## 8. Test plan

### Rule evaluator

- Priority ordering within each decision channel.
- Maximum of two `AND` conditions.
- Disabled rules are ignored.
- Invalid actions fall through to the next rule.
- Locked fallback behavior.
- HP, distance, reach, DR, equipment, style, and speed comparisons.
- Relative speed correctly treats fewer seconds as faster.
- Runtime modifiers affect DR and attack-speed conditions.

### Styles

- Multiple styles can be learned whether directives are enabled or disabled.
- The Default selection overrides learned-talent ordering in ordinary combat.
- The Opening selection supplies the initial tactical profile.
- Explicit `No weapon style` uses the neutral profile.
- Invalid saved selections remain serialized and fall back to no style at runtime.
- Legacy presets without the new field retain first-style behavior.
- Switch occurs only at the next attack opportunity.
- Previous style remains active defensively until the switch.
- New style affects the attack and following recovery time.
- Only one style activates normally.
- Shield of Blades and Storm of Blades form the sole valid pair.
- Equipment-incompatible styles are skipped.
- Switching cannot repeatedly restore opening benefits.
- Seeded simulations remain deterministic.

### Maneuvers

- Dynamic Normal/Jab switching changes speed and damage correctly.
- Jab remains non-penetrating.
- Fight Defensively applies the selected bonuses and lingering attack penalty.
- Give Ground applies movement, +5 Defense, and -1 next Attack.
- Give Ground is rejected without space, against Charge, and against a faster walker.
- Attacker pursuit preserves valid positions.

### Presets and UI

- JSON round-trip and schema-version handling.
- Save, overwrite, rename, delete, and replace-draft behavior.
- Invalid preset rules survive round-trip.
- Save and Cancel correctly isolate drafts.
- Editing locks after combat starts and unlocks after Reset.
- Both characters retain independent policies.
- Fighter preset round-trips preserve Default/Opening style selection.

## Acceptance criteria

- Each fighter can have an independent ordered directive list.
- A fighter can dynamically switch between learned weapon styles at attack opportunities.
- A fighter can select a persistent Default style without enabling Tactical Directives.
- A fighter can select an Opening style that applies before the first directive-driven switch.
- The special Shield of Blades/Storm of Blades pairing works and no other pair does.
- Rules can inspect current enemy DR and modified attack speed.
- Jab, Fight Defensively, and Give Ground can be selected conditionally.
- Rule decisions and style changes are visible in combat logs.
- Tactical presets survive application restarts.
- Disabling Tactical Directives preserves existing simulator behavior.
- All existing tests plus the new tactical test suite pass.

## Estimate

Approximately 10-15 development days. Dynamic learned-style profiles and Give Ground carry most of the implementation risk.
