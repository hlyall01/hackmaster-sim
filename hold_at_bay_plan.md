# Hold at Bay Maneuver Plan

## Scope
Add the first combat maneuver, **Hold at Bay**, with a clean extension point for future AI playstyles.

## Plan
1) Audit current combat state, reach/engagement flow, and UI controls to pick insertion points for maneuver selection and activation (sim tick + attack resolution).
2) Add maneuver selection plumbing (config/presets/state): define a `CombatManeuver`/`ManeuverPrefs` container, add `hold_at_bay` to `PlayerConfig`, fighter presets, and combatant sheets so AI playstyles can swap strategies later without touching UI.
3) Implement Hold at Bay rules in sim logic: detect entry into reach when attacker reach > defender reach, resolve a hold-at-bay attack that only deals jab damage if available, block defender advance/attacks while held, and add a “knock aside” contested roll using shieldless defense to break the hold and schedule next-second engagement.
4) Surface the maneuver in GUI/CLI: add a checkbox next to jab in `sim_gui`, persist it in presets, and log hold/knock-aside events for transparency.
5) Add targeted tests/sim scenarios for reach advantage, jab vs no-jab, and knock-aside flow; update docs/notes to describe the new maneuver and AI-extension points.
