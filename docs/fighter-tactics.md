# Default fighter tactics

Fighter entries in `data/sim/fighter_presets.json` can include a
`tactical_policy`. Loading the fighter applies it, and saving a fighter from
the simulator preserves it. Older entries without the field load with tactics
disabled. Existing saved user overrides retain their own settings.

Both Volfango presets use:

```json
"tactical_policy": {
  "enabled": true,
  "rules": [
    {
      "conditions": [
        { "kind": "enemy_dr", "comparison": "greater", "value": 7.0 }
      ],
      "action": { "kind": "called_shot" }
    },
    {
      "conditions": [{ "kind": "always" }],
      "action": { "kind": "fight_defensively", "penalty": 8 }
    }
  ]
}
```

This chooses Called Shot above 7 DR and normal attacks at 7 or below. The
separate stance rule preserves Volfango's Fight Defensively setting. Each
channel uses its first legal matching rule; the default attack fallback is
Normal attack, so a second low-DR rule is unnecessary.

Enemy DR uses the target's effective `ArmorDr`, including temporary modifiers,
before attacker armor penetration. Called Shot uses the existing precision,
defense-penalty, and aiming-delay rules. Choosing it through tactics does not
skip the opening aiming delay; switching it on during recovery also adds the
required aiming time. Tactics can be edited in the simulator's Tactical
Directives panel, including the new Called shot action.

Reusable policies also live in `data/sim/tactical_presets.json`; these use the
same rule/action schema and can be selected in the tactics editor. A fighter's
default policy is stored directly on its fighter entry.
