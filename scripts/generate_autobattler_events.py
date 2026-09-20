#!/usr/bin/env python3
"""Generate autobattler random events catalog and documentation."""

from __future__ import annotations

import json
import random
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVENTS_PATH = ROOT / "data" / "autobattler" / "events_v1.json"
DOCS_PATH = ROOT / "docs" / "autobattler_events.md"

RNG = random.Random(20260214)

STATS = [
    "strength",
    "intelligence",
    "wisdom",
    "dexterity",
    "constitution",
    "looks",
    "charisma",
]

SKILLS_BY_STAT = {
    "strength": ["Athletics", "Climbing", "Jumping", "Laborer"],
    "intelligence": ["Literacy", "Crafting", "Repairing", "Tactics"],
    "wisdom": ["Observation", "First Aid", "Survival", "Animal Training"],
    "dexterity": ["Acrobatics", "Stealth", "Riding", "Weapon Drills"],
    "constitution": ["Trauma", "Endurance", "Laboring", "Scouting"],
    "looks": ["Performing", "Carousing", "Begging", "Disguise"],
    "charisma": ["Persuasion", "Begging", "Gambling", "Performing"],
}

ITEMS = [
    "bandage roll",
    "iron ration",
    "sturdy rope",
    "throwing knife",
    "healing salve",
    "lantern oil",
    "lockpick set",
    "map scrap",
    "bone charm",
    "smithing nails",
    "signal whistle",
    "travel cloak",
]

CHAIN_TITLES = [
    "Ashen Sigil",
    "Broken Oath",
    "Salt Cartel",
    "Moonwell Pact",
    "Iron Witness",
    "Hollow Banner",
    "Cinder Choir",
    "Greenfire Idol",
    "Raven Toll",
    "Glass Pilgrim",
    "Pale Scriptorium",
    "Warden's Debt",
    "Black Orchard",
    "Thorn Tribunal",
    "Frost Reliquary",
    "Brass Covenant",
    "Shale Compass",
    "Mire Lantern",
    "Sable Tribunal",
    "Starless Mint",
]

CHAIN_STEP_NAMES = [
    "Rumor on the Road",
    "Hidden Mark",
    "Complication",
    "Turning Point",
    "Final Reckoning",
]

WORLD_PREFIXES = [
    "Wayfarer",
    "Dustbound",
    "Lantern",
    "Riverside",
    "Hinterland",
    "Coldwind",
    "Sunken",
    "Stonegate",
    "Highvale",
    "Nightmarket",
    "Marshroad",
    "Blackbarrow",
    "Oldfort",
    "Whispering",
    "Borderland",
    "Crosswind",
    "Deepwood",
    "Westwall",
    "Copperlane",
    "Grimwatch",
]

WORLD_NOUNS = [
    "Caravan",
    "Shrine",
    "Messenger",
    "Cache",
    "Patrol",
    "Archive",
    "Furnace",
    "Bridge",
    "Workshop",
    "Reliquary",
    "Camp",
    "Beacon",
    "Smithy",
    "Outpost",
    "Market",
    "Cairn",
    "Tunnel",
    "Harbor",
    "Garden",
    "Mausoleum",
]

WORLD_TAILS = [
    "at Dusk",
    "in Fog",
    "of Embers",
    "Under Watch",
    "Without Witness",
    "at the Ford",
    "in Ruins",
    "Behind the Gate",
    "of Quiet Knives",
    "in Rain",
    "on the Ridge",
    "of the Last Bell",
    "at First Light",
    "in Ash",
    "in the Thicket",
    "at Low Tide",
    "on Broken Stone",
    "in Bitter Wind",
    "of Crooked Paths",
    "Beyond the Wall",
]

WORLD_DESCRIPTORS = [
    "A tense crowd parts as you approach.",
    "The place feels picked clean but not abandoned.",
    "Tracks and discarded gear suggest recent violence.",
    "A survivor waves you over and whispers an offer.",
    "You hear steel on stone from somewhere nearby.",
    "A faded emblem marks this as disputed ground.",
    "The air smells of smoke, oil, and wet earth.",
    "A local elder claims this spot is cursed.",
    "Two rivals accuse each other and both want your help.",
    "A shuttered stall hides a locked chest under canvas.",
    "Stray dogs circle the area and refuse to leave.",
    "A torn map points to a cache no one has claimed.",
    "A messenger begs you to settle a dangerous dispute.",
    "Shallow graves nearby suggest someone failed this challenge.",
    "A sentry calls out and demands a reason to trust you.",
    "You find signs that scouts are shadowing this route.",
    "A half-buried relic hums with unstable energy.",
    "A missing patrol's banner hangs from a broken spear.",
    "A suspicious trader insists on a quick decision.",
    "Something valuable is here, but so is trouble.",
]


def maybe_skill(stat: str) -> str | None:
    if RNG.random() < 0.7:
        return RNG.choice(SKILLS_BY_STAT[stat])
    return None


def difficulty_from_dc(dc: int) -> str:
    if dc <= 10:
        return "easy"
    if dc <= 14:
        return "medium"
    if dc <= 18:
        return "hard"
    return "very_hard"


def compact_check(stat: str, dc: int, skill: str | None) -> dict:
    check = {
        "stat": stat,
        "dc": dc,
        "difficulty": difficulty_from_dc(dc),
        "require_trained": False,
    }
    if skill:
        check["skill"] = skill
    return check


def compact_result(
    *,
    gold: int = 0,
    xp: int = 0,
    honor: int = 0,
    wound: int = 0,
    heal: int = 0,
    item: str | None = None,
    set_flags: list[str] | None = None,
    clear_flags: list[str] | None = None,
    trigger_fight: bool = False,
    notes: list[str] | None = None,
) -> dict:
    result: dict = {}
    if gold:
        result["gold_delta"] = gold
    if xp:
        result["xp_delta"] = xp
    if honor:
        result["honor_delta"] = honor
    if wound:
        result["add_wound"] = wound
    if heal:
        result["heal_wound"] = heal
    if item:
        result["add_item"] = item
    if set_flags:
        result["set_flags"] = set_flags
    if clear_flags:
        result["clear_flags"] = clear_flags
    if trigger_fight:
        result["trigger_fight"] = True
    if notes:
        result["notes"] = notes
    return result


def chain_event(chain_idx: int, step_idx: int) -> dict:
    chain_code = f"{chain_idx:02d}"
    step_code = f"{step_idx:02d}"
    event_id = f"evt_chain_{chain_code}_{step_code}"
    title = CHAIN_TITLES[chain_idx - 1]
    step_name = CHAIN_STEP_NAMES[step_idx - 1]

    requires = []
    if step_idx > 1:
        requires.append(f"quest_chain_{chain_code}_step_{step_idx - 1}_done")

    done_flag = f"quest_chain_{chain_code}_step_{step_idx}_done"
    success_flag = f"quest_chain_{chain_code}_step_{step_idx}_success"
    complete_flag = f"quest_chain_{chain_code}_complete"

    scout_stat = ["wisdom", "dexterity", "intelligence", "constitution", "charisma"][step_idx - 1]
    bold_stat = ["strength", "dexterity", "charisma", "constitution", "strength"][step_idx - 1]
    scout_dc = 10 + step_idx + (chain_idx % 3)
    bold_dc = 11 + step_idx + ((chain_idx + 1) % 3)

    tiers = {
        1: ["any"],
        2: ["normal", "elite"],
        3: ["normal", "elite", "boss"],
        4: ["elite", "boss"],
        5: ["boss", "elite"],
    }[step_idx]

    min_depth = (chain_idx - 1) % 7 + step_idx

    cautious_success_flags = [done_flag, success_flag]
    bold_success_flags = [done_flag, success_flag]
    cautious_failure_flags = [done_flag]
    bold_failure_flags = [done_flag]
    if step_idx == 5:
        cautious_success_flags.append(complete_flag)
        bold_success_flags.append(complete_flag)
        cautious_failure_flags.append(complete_flag)
        bold_failure_flags.append(complete_flag)

    cautious_success = compact_result(
        gold=6 + step_idx * 3 + (chain_idx % 4),
        xp=5 + step_idx * 3,
        honor=1 if step_idx in (2, 5) else 0,
        item=ITEMS[(chain_idx + step_idx) % len(ITEMS)] if step_idx in (3, 5) else None,
        set_flags=cautious_success_flags,
        notes=[
            f"{title}: step {step_idx} advances through careful planning.",
            "Your measured approach secures leverage for the next lead.",
        ],
    )
    cautious_failure = compact_result(
        gold=-(step_idx + chain_idx % 3),
        wound=1 + (step_idx // 2),
        set_flags=cautious_failure_flags,
        trigger_fight=step_idx >= 4,
        notes=[
            f"{title}: step {step_idx} slips, but the trail stays alive.",
            "You lose momentum and leave blood on the ground.",
        ],
    )

    bold_success = compact_result(
        gold=8 + step_idx * 4 + (chain_idx % 5),
        xp=6 + step_idx * 3,
        honor=1 if step_idx >= 3 else 0,
        wound=1 if step_idx == 5 and chain_idx % 2 == 0 else 0,
        set_flags=bold_success_flags,
        trigger_fight=step_idx >= 3 and chain_idx % 3 == 0,
        notes=[
            f"{title}: step {step_idx} is won by force.",
            "You take the direct route and claim immediate gains.",
        ],
    )
    bold_failure = compact_result(
        gold=-(2 + step_idx),
        wound=2 if step_idx >= 3 else 1,
        set_flags=bold_failure_flags,
        trigger_fight=True,
        notes=[
            f"{title}: step {step_idx} backfires and turns hostile.",
            "Your push creates enemies that answer with steel.",
        ],
    )

    description = (
        f"The {title.lower()} trail reaches {step_name.lower()}. "
        "You can play it carefully or force a quick outcome."
    )

    return {
        "id": event_id,
        "name": f"{title} - {step_name}",
        "description": description,
        "weight": 9 + step_idx,
        "min_depth": min_depth,
        "max_depth": 999,
        "tiers": tiers,
        "requires_flags": requires,
        "unique_once": True,
        "choices": [
            {
                "id": "scout",
                "text": "Scout and negotiate the situation",
                "check": compact_check(scout_stat, scout_dc, maybe_skill(scout_stat)),
                "success": cautious_success,
                "failure": cautious_failure,
            },
            {
                "id": "press",
                "text": "Press hard for an immediate result",
                "check": compact_check(bold_stat, bold_dc, maybe_skill(bold_stat)),
                "success": bold_success,
                "failure": bold_failure,
            },
        ],
    }


def world_event(event_idx: int) -> dict:
    code = f"{event_idx:03d}"
    event_id = f"evt_world_{code}"

    prefix = WORLD_PREFIXES[(event_idx - 1) % len(WORLD_PREFIXES)]
    noun = WORLD_NOUNS[((event_idx - 1) * 3) % len(WORLD_NOUNS)]
    tail = WORLD_TAILS[((event_idx - 1) * 7) % len(WORLD_TAILS)]
    name = f"{prefix} {noun} {tail} [{code}]"

    desc_a = WORLD_DESCRIPTORS[(event_idx - 1) % len(WORLD_DESCRIPTORS)]
    desc_b = WORLD_DESCRIPTORS[((event_idx - 1) * 5) % len(WORLD_DESCRIPTORS)]
    description = f"{desc_a} {desc_b}"

    stat_a = STATS[(event_idx - 1) % len(STATS)]
    stat_b = STATS[(event_idx + 2) % len(STATS)]
    dc_a = 9 + (event_idx % 8)
    dc_b = 11 + (event_idx % 9)

    choice_a_skill = maybe_skill(stat_a)
    choice_b_skill = maybe_skill(stat_b)

    min_depth = (event_idx - 1) % 24
    max_depth = 999

    if event_idx % 10 == 0:
        tiers = ["boss"]
    elif event_idx % 4 == 0:
        tiers = ["elite", "boss"]
    elif event_idx % 3 == 0:
        tiers = ["normal", "elite"]
    else:
        tiers = ["any"]

    requires: list[str] = []
    if event_idx <= 20:
        requires = [f"quest_chain_{event_idx:02d}_complete"]

    unique_once = event_idx <= 80

    set_seen = [f"world_event_{code}_resolved"] if unique_once else []

    success_a = compact_result(
        gold=4 + (event_idx % 9),
        xp=3 + (event_idx % 7),
        honor=1 if event_idx % 13 == 0 else 0,
        heal=1 if event_idx % 6 == 0 else 0,
        item=ITEMS[event_idx % len(ITEMS)] if event_idx % 5 == 0 else None,
        set_flags=set_seen,
        notes=[
            "You keep control and extract steady value from the encounter.",
        ],
    )

    failure_a = compact_result(
        gold=-(1 + event_idx % 5),
        wound=1 + (1 if event_idx % 9 == 0 else 0),
        trigger_fight=event_idx % 11 == 0,
        set_flags=set_seen,
        notes=[
            "Your cautious plan stalls, and the situation turns against you.",
        ],
    )

    success_b = compact_result(
        gold=6 + (event_idx % 12),
        xp=4 + (event_idx % 8),
        honor=1 if event_idx % 7 == 0 else 0,
        wound=1 if event_idx % 14 == 0 else 0,
        trigger_fight=event_idx % 8 == 0,
        set_flags=set_seen,
        notes=[
            "You gamble on momentum and seize a larger payoff.",
        ],
    )

    failure_b = compact_result(
        gold=-(2 + event_idx % 7),
        wound=1 + (event_idx % 3),
        trigger_fight=True if event_idx % 5 == 0 else event_idx % 2 == 0,
        set_flags=set_seen,
        notes=[
            "The aggressive move collapses into losses and open danger.",
        ],
    )

    return {
        "id": event_id,
        "name": name,
        "description": description,
        "weight": 6 + (event_idx % 10),
        "min_depth": min_depth,
        "max_depth": max_depth,
        "tiers": tiers,
        "requires_flags": requires,
        "unique_once": unique_once,
        "choices": [
            {
                "id": "careful",
                "text": "Take the careful approach",
                "check": compact_check(stat_a, dc_a, choice_a_skill),
                "success": success_a,
                "failure": failure_a,
            },
            {
                "id": "bold",
                "text": "Push for a bigger payoff",
                "check": compact_check(stat_b, dc_b, choice_b_skill),
                "success": success_b,
                "failure": failure_b,
            },
        ],
    }


def summarize_result(result: dict) -> str:
    pieces: list[str] = []
    if result.get("gold_delta"):
        gold = result["gold_delta"]
        pieces.append(f"gold {gold:+d}")
    if result.get("xp_delta"):
        pieces.append(f"xp +{result['xp_delta']}")
    if result.get("honor_delta"):
        honor = result["honor_delta"]
        pieces.append(f"honor {honor:+d}")
    if result.get("add_wound"):
        pieces.append(f"add wound {result['add_wound']}")
    if result.get("heal_wound"):
        pieces.append(f"heal wound {result['heal_wound']}")
    if result.get("add_item"):
        pieces.append(f"item: {result['add_item']}")
    for flag in result.get("set_flags", []):
        pieces.append(f"set flag `{flag}`")
    for flag in result.get("clear_flags", []):
        pieces.append(f"clear flag `{flag}`")
    if result.get("trigger_fight"):
        pieces.append("triggers fight")
    notes = result.get("notes", [])
    if notes:
        pieces.append("notes: " + " | ".join(notes))
    if not pieces:
        return "no mechanical effect"
    return "; ".join(pieces)


def path_label(event: dict) -> str:
    event_id = event["id"]
    if event_id.startswith("evt_chain_"):
        _, _, chain, step = event_id.split("_")
        return f"Quest chain {chain}, step {int(step)} of 5"
    requires = event.get("requires_flags", [])
    if requires:
        return "Follow-up event gated by: " + ", ".join(f"`{f}`" for f in requires)
    return "Standalone random event"


def event_doc_block(index: int, event: dict) -> str:
    lines: list[str] = []
    lines.append(f"## {index}. {event['name']} (`{event['id']}`)")
    lines.append(f"- Path: {path_label(event)}")
    lines.append(
        "- Availability: "
        f"depth {event['min_depth']}..{event['max_depth']}, "
        f"tiers {', '.join(event['tiers'])}, "
        f"unique_once={str(event['unique_once']).lower()}"
    )
    requires = event.get("requires_flags", [])
    if requires:
        lines.append("- Requires flags: " + ", ".join(f"`{flag}`" for flag in requires))
    else:
        lines.append("- Requires flags: none")
    lines.append("- Choices:")
    for choice in event["choices"]:
        check = choice.get("check")
        if check:
            skill = check.get("skill")
            difficulty = check.get("difficulty") or difficulty_from_dc(check.get("dc", 15))
            if skill:
                roll_line = (
                    f"d100 <= skill level + mastery die + ability mod "
                    f"({check['stat'].upper()}) + {difficulty} shift"
                )
            else:
                roll_line = (
                    f"d100 <= stat level + mastery die + ability mod "
                    f"({check['stat'].upper()}) + {difficulty} shift"
                )
        else:
            roll_line = "No roll"
        lines.append(f"1. {choice['text']}")
        lines.append(f"Roll: {roll_line}")
        lines.append(f"Success: {summarize_result(choice.get('success', {}))}")
        lines.append(f"Failure: {summarize_result(choice.get('failure', {}))}")
    lines.append("")
    return "\n".join(lines)


def build_catalog() -> dict:
    events: list[dict] = []

    for chain_idx in range(1, 21):
        for step_idx in range(1, 6):
            events.append(chain_event(chain_idx, step_idx))

    for event_idx in range(1, 121):
        events.append(world_event(event_idx))

    ids = [event["id"] for event in events]
    names = [event["name"] for event in events]
    if len(ids) != len(set(ids)):
        raise RuntimeError("Duplicate event IDs generated")
    if len(names) != len(set(names)):
        raise RuntimeError("Duplicate event names generated")
    if len(events) < 200:
        raise RuntimeError("Generated fewer than 200 events")

    chain_gated = [event for event in events if event.get("requires_flags")]
    if not chain_gated:
        raise RuntimeError("No prerequisite-gated events generated")

    fight_events = 0
    for event in events:
        for choice in event.get("choices", []):
            if choice.get("success", {}).get("trigger_fight") or choice.get("failure", {}).get(
                "trigger_fight"
            ):
                fight_events += 1
                break
    if fight_events < 40:
        raise RuntimeError("Too few fight-escalation events generated")

    for event in events:
        if len(event.get("choices", [])) < 2:
            raise RuntimeError(f"Event {event['id']} missing choices")
        for choice in event["choices"]:
            if "check" not in choice:
                raise RuntimeError(f"Choice {choice['id']} in {event['id']} missing check")

    return {"version": 1, "events": events}


def write_outputs(catalog: dict) -> None:
    EVENTS_PATH.parent.mkdir(parents=True, exist_ok=True)
    EVENTS_PATH.write_text(json.dumps(catalog, indent=2) + "\n", encoding="utf-8")

    DOCS_PATH.parent.mkdir(parents=True, exist_ok=True)
    events = catalog["events"]
    lines = [
        "# Autobattler Event Catalog v1",
        "",
        f"Generated event count: {len(events)}",
        "",
        "Roll model used by the resolver:",
        "- `roll = d100` and success when `roll <= target`.",
        "- `target = level + mastery_die_roll + ability_modifier + difficulty_shift`.",
        "- Skill checks use the player's skill percentile level; stat-only checks use event stat level.",
        "- Difficulty shifts: easy `+30`, medium `+15`, hard `+0`, very hard `-15`.",
        "",
        "Each event section below documents availability, path gating, checks, and both result branches.",
        "",
    ]

    for index, event in enumerate(events, start=1):
        lines.append(event_doc_block(index, event))

    DOCS_PATH.write_text("\n".join(lines), encoding="utf-8")


def main() -> None:
    catalog = build_catalog()
    write_outputs(catalog)


if __name__ == "__main__":
    main()
