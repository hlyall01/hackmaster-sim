# Combat spells

Open **Customize → Spells**, add a spell, choose its empowerments and casting AI,
and apply the settings to reset combat. Spells are separate from Talents. EP is
ignored: no essence setup, spending, proficiency requirement, or EP-based
empowerment cap. Explicitly assigned spells are castable regardless of character level; casting
time, components, equipment failure, and fatigue still apply. Volatility is disabled.

## Available spells

- **Echo Strike:** level 10, somatic, 1-second cast, personal. Arms the next
  successful attack for 15 seconds; the attack echoes after 10 seconds for half
  its resolved wound, rounded down. Empowerments add duration, reduce delay
  (minimum 1 second), add echoes, and enable full damage.
- **Chronoblur:** level 4, verbal and somatic, 2-second cast, touch (the current
  casting policy targets self). Lasts 60 seconds, plus 30 seconds per duration
  empowerment. Movement in the previous second grants +4 melee defense and
  makes missile attacks treat the target as 20 feet farther away.
- **Streamline:** level 8, verbal and somatic, 5-second cast, personal field.
  Lasts 300 seconds, plus 60 seconds per duration empowerment. Radius is 30 feet,
  plus 10 feet per radius empowerment. Damage dice against targets in the field
  use their mathematical average, rounded down, before modifiers and mitigation.
  The field covers both sides. Both combat hosts use the empowered radius.

Old Chronoblur and Streamline talent selections are recognized and moved into
known spells. They now require casting; they are not free active buffs at the
start of combat. Volfango knows Echo Strike without requiring any essence setup. Both Volfango
presets have all fatigue talents, including five ranks of Diminish Spell Fatigue.
Utility spells with no supported combat effect are omitted from the picker.

## AI choices, per spell

- **Cast as often as possible:** cast whenever legal and the spell's buff is no
  longer active. Do not refresh an active buff. Discharged echoes continue
  independently and do not prevent another Echo Strike cast.
- **Cast at fight start:** queue one opening cast at the first legal opportunity.
  Opening spells wait for other casts and fatigue; they cannot all cast at once.
  This does not refresh a spell later, including after failure or cancellation.
- **Use spell's recommended timing:** Echo Strike waits for an enemy in melee
  reach and checks that weapon recovery can finish before the buff expires, then
  casts once. Chronoblur and Streamline use their opening cast.
- **Manual only:** use the cast button in the live combat panel.

All AI uses the character's selected empowerments. Temporary restrictions are
reconsidered without repeated rejection messages. Unused spells get priority
before repeats so one repeating spell cannot prevent another spell's opening.
The shared runtime owns these decisions; both combat hosts call it.

## Casting and talents

Somatic components must remain possible throughout casting; verbal components
are checked at completion. Casting/channelling uses d8p defense and permits only
walking, unless the relevant talent changes the restriction. Equipment failure
is 0/25/50/75% for no/light/moderate/heavy encumbrance, plus one 25-point penalty
for heavy armor or a shield; Combat Casting subtracts 25 percentage points.

Completed-spell fatigue starts the following second and lasts 5 + casting time
seconds, reduced by Diminish Spell Fatigue ranks. Default fatigue is -6 defense,
no attacks/casting, half movement without running/sprinting, -30% skill checks,
and doubled other-action time. Weapon recovery normally begins when fatigue
ends. Cancelling before combat time advances has no fatigue or weapon recovery
penalty. Once time has advanced, voluntary cancellation gives five seconds of
fatigue.

All thirteen supplied magic talents remain in Talents with their BP costs,
prerequisites, and rank limits. Mitigate improves fatigue defense and movement;
Decimate removes the other penalties and starts weapon recovery at cast
completion; Eliminate removes completed-spell fatigue but still resets weapon
recovery. Silent/Still Casting waive their components. Focus and resistance
bonuses are available to opposed magic saves; these three spells need no save.
General skill checks are not resolved by the combat engines, but fatigue's
skill/action penalties are exposed for future action systems.

## Echo details and assumptions

Misses and shield blocks do not discharge Echo Strike; successful direct hits,
including reactions and hits reduced to zero damage, do. Each echo makes a new
opposed attack roll, bypasses mundane DR, and uses the recorded wound rather
than rerolling damage. Magical Prescience/Precognition remain effective. Echoes
do not cause knockback, duplicate critical effects, recursively trigger Echo
Strike, reset weapon timers, or grant weapon mastery credit.

Additional echoes use successive delay intervals. Defense bonuses are +0, +2,
+4 within a cast, resetting for the next cast. Discharged echoes survive buff
expiry, movement, and caster death by default. Existing saved life/range
restrictions remain supported. Dead targets are skipped without retargeting.
Dismissal removes both armed and pending echoes. Combat waits for pending echoes
that can affect a living target, subject to existing host timeouts.

## Extending and testing

`core::magic::SPELL_CATALOG` supplies spell metadata and AI policy.
`MagicLoadout` stores known spells, per-spell AI, and empowerments.
`SpellRequest::from_loadout` constructs typed effects and canonical casting
metadata. Add new effects and targeting rules in the shared runtime, not in GUI
code. The generic runtime also supports instant casts, channelling and saves.
The optional essence rules API remains available for rule tests and explicit
programmatic callers; the spell editor does not enable it.

Run `cargo test --lib --bins`, `cargo test --features bevy --lib`, and
`cargo check --features bevy --bins`. Tests cover EP-free casting, empowerments,
AI scheduling/recasting, migration, preset persistence, components, fatigue,
talents, deterministic echoes, and radius coverage in both combat hosts.
