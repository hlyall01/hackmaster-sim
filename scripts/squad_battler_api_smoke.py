#!/usr/bin/env python3
"""HTTP smoke checks for the local squad_battler_demo server.

The script intentionally uses only the Python standard library so it can run in
fresh dev shells and CI jobs without extra package setup.
"""

from __future__ import annotations

import argparse
import json
import sys
import urllib.error
import urllib.request
from typing import Any


DEFAULT_BASE_URL = "http://127.0.0.1:8788"
DEFAULT_SEED = 8788


class SmokeFailure(RuntimeError):
    pass


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-url", default=DEFAULT_BASE_URL)
    parser.add_argument("--seed", type=int, default=DEFAULT_SEED)
    parser.add_argument("--timeout", type=float, default=5.0)
    args = parser.parse_args()

    client = Client(args.base_url.rstrip("/"), args.timeout)
    try:
        run_smoke(client, args.seed)
    except SmokeFailure as err:
        print(f"FAIL: {err}", file=sys.stderr)
        return 1
    except urllib.error.URLError as err:
        print(
            "FAIL: could not reach squad_battler_demo. "
            "Start it with: cargo run --bin squad_battler_demo -- --port 8788",
            file=sys.stderr,
        )
        print(f"detail: {err}", file=sys.stderr)
        return 1
    return 0


class Client:
    def __init__(self, base_url: str, timeout: float) -> None:
        self.base_url = base_url
        self.timeout = timeout

    def get(self, path: str) -> tuple[int, dict[str, str], str]:
        return self._request("GET", path, None)

    def post_json(self, path: str, body: dict[str, Any]) -> Any:
        status, headers, text = self._request("POST", path, json.dumps(body).encode("utf-8"))
        if status < 200 or status >= 300:
            raise SmokeFailure(f"POST {path} returned HTTP {status}: {text}")
        content_type = headers.get("content-type", "")
        if "application/json" not in content_type:
            raise SmokeFailure(f"POST {path} returned non-JSON content type {content_type!r}")
        try:
            return json.loads(text)
        except json.JSONDecodeError as err:
            raise SmokeFailure(f"POST {path} returned invalid JSON: {err}") from err

    def get_json(self, path: str) -> Any:
        status, headers, text = self.get(path)
        if status < 200 or status >= 300:
            raise SmokeFailure(f"GET {path} returned HTTP {status}: {text}")
        content_type = headers.get("content-type", "")
        if "application/json" not in content_type:
            raise SmokeFailure(f"GET {path} returned non-JSON content type {content_type!r}")
        try:
            return json.loads(text)
        except json.JSONDecodeError as err:
            raise SmokeFailure(f"GET {path} returned invalid JSON: {err}") from err

    def _request(
        self, method: str, path: str, body: bytes | None
    ) -> tuple[int, dict[str, str], str]:
        url = f"{self.base_url}{path}"
        headers = {}
        if body is not None:
            headers["Content-Type"] = "application/json"
        request = urllib.request.Request(url, data=body, headers=headers, method=method)
        try:
            with urllib.request.urlopen(request, timeout=self.timeout) as response:
                return response.status, normalize_headers(response.headers), decode(response.read())
        except urllib.error.HTTPError as err:
            return err.code, normalize_headers(err.headers), decode(err.read())


def run_smoke(client: Client, seed: int) -> None:
    print(f"base_url={client.base_url}")
    check_static_assets(client)

    state = client.get_json("/api/state")
    assert_state_shape(state, "state")
    print("ok GET /api/state")

    first = client.post_json("/api/new-run", {"seed": seed})
    second = client.post_json("/api/new-run", {"seed": seed})
    assert_state_shape(first, "new-run(first)")
    assert_state_shape(second, "new-run(second)")
    if first != second:
        raise SmokeFailure("fixed-seed /api/new-run responses differed")
    state = second
    print(
        "ok POST /api/new-run deterministic "
        f"seed={state['seed']} active={len(state['squad']['active'])}"
    )

    state = exercise_fight_flow(client, state)
    exercise_recruit_flow(client, seed + 1)
    exercise_route_progression(client, seed + 2)
    print(f"ok smoke complete phase={state['phase']} depth={state['depth']}")


def check_static_assets(client: Client) -> None:
    expected = [
        ("/", "text/html", "HackMaster Squad Battler"),
        ("/static/styles.css", "text/css", "battle-grid"),
        ("/static/js/main.js", "text/javascript", "fightCommand"),
    ]
    for path, content_type, body_marker in expected:
        status, headers, body = client.get(path)
        if status != 200:
            raise SmokeFailure(f"GET {path} returned HTTP {status}")
        actual_content_type = headers.get("content-type", "")
        if content_type not in actual_content_type:
            raise SmokeFailure(
                f"GET {path} content type {actual_content_type!r} did not include {content_type!r}"
            )
        if body_marker not in body:
            raise SmokeFailure(f"GET {path} body did not include {body_marker!r}")

    status, _, _ = client.get("/static/does-not-exist.txt")
    if status != 404:
        raise SmokeFailure(f"missing static asset returned HTTP {status}, expected 404")
    print("ok static asset routes")


def exercise_fight_flow(client: Client, state: dict[str, Any]) -> dict[str, Any]:
    fight_node = first_available_node(state, {"fight", "elite", "boss"})
    state = client.post_json("/api/choose-node", {"node_id": fight_node["id"]})
    assert_state_shape(state, "choose-node(fight)")
    assert_optional_object(state, "pending_fight", "choose-node(fight)")
    if state["phase"] != "fight_preview":
        raise SmokeFailure(f"choose-node fight phase was {state['phase']!r}")
    print(
        "ok POST /api/choose-node "
        f"node={fight_node['id']} kind={fight_node['kind']} enemies={state['pending_fight']['enemy_count']}"
    )

    state = client.post_json("/api/start-fight", {})
    assert_state_shape(state, "start-fight")
    assert_live_fight_shape(state["live_fight"], "start-fight.live_fight")
    assert_no_living_position_overlap(state, "start-fight")
    if state["phase"] != "combat_playback":
        raise SmokeFailure(f"start-fight phase was {state['phase']!r}")
    print("ok POST /api/start-fight")

    for seconds in (1, 5):
        state = client.post_json("/api/fight-command", {"command": "tick", "seconds": seconds})
        assert_state_shape(state, f"fight-command(tick {seconds})")
        assert_no_living_position_overlap(state, f"fight-command(tick {seconds})")
        print(f"ok POST /api/fight-command tick seconds={seconds}")

    state = client.post_json(
        "/api/fight-command", {"command": "skip_to_next_initiative", "seconds": 1}
    )
    assert_state_shape(state, "fight-command(skip_to_next_initiative)")
    assert_no_living_position_overlap(state, "fight-command(skip_to_next_initiative)")
    print("ok POST /api/fight-command skip_to_next_initiative")

    state = client.post_json("/api/fight-command", {"command": "finish", "seconds": 1})
    assert_state_shape(state, "fight-command(finish)")
    assert_no_living_position_overlap(state, "fight-command(finish)")
    if state["phase"] not in {"reward_review", "choose_node", "run_over"}:
        raise SmokeFailure(f"finish phase was {state['phase']!r}")
    if state["last_reward"] is not None:
        assert_reward_shape(state["last_reward"], "fight-command(finish).last_reward")
    print(f"ok POST /api/fight-command finish phase={state['phase']}")
    return state


def exercise_recruit_flow(client: Client, seed: int) -> None:
    state = client.post_json("/api/new-run", {"seed": seed})
    assert_state_shape(state, "recruit new-run")
    recruit_node = first_available_node(state, {"recruit"})
    state = client.post_json("/api/choose-node", {"node_id": recruit_node["id"]})
    assert_state_shape(state, "choose-node(recruit)")
    if state["phase"] != "reward_review":
        raise SmokeFailure(f"recruit node phase was {state['phase']!r}")
    if not state["recruit_offer"]:
        raise SmokeFailure("recruit node did not return any candidates")

    candidate_id = state["recruit_offer"][0]["id"]
    state = client.post_json(
        "/api/recruit-choice",
        {
            "candidate_id": candidate_id,
            "destination": "bench",
            "replace_member_id": None,
        },
    )
    assert_state_shape(state, "recruit-choice")
    if len(state["squad"]["bench"]) != 1:
        raise SmokeFailure("bench recruit did not add exactly one reserve")
    print(
        "ok POST /api/recruit-choice "
        f"candidate={candidate_id} destination=bench remaining={len(state['recruit_offer'])}"
    )

    active_member_id = state["squad"]["active"][0]["id"]
    bench_member_id = state["squad"]["bench"][0]["id"]
    state = client.post_json(
        "/api/roster-swap",
        {"active_member_id": active_member_id, "bench_member_id": bench_member_id},
    )
    assert_state_shape(state, "roster-swap")
    if state["squad"]["active"][0]["id"] != bench_member_id:
        raise SmokeFailure("roster swap did not move bench member into active slot")
    print("ok POST /api/roster-swap")

    state = client.post_json(
        "/api/roster-dismiss", {"bench_member_id": active_member_id}
    )
    assert_state_shape(state, "roster-dismiss")
    if any(member["id"] == active_member_id for member in state["squad"]["bench"]):
        raise SmokeFailure("dismissed bench member still present")
    print("ok POST /api/roster-dismiss")

    if state["recruit_offer"]:
        promote_candidate_id = state["recruit_offer"][0]["id"]
        state = client.post_json(
            "/api/recruit-choice",
            {
                "candidate_id": promote_candidate_id,
                "destination": "bench",
                "replace_member_id": None,
            },
        )
        assert_state_shape(state, "recruit-choice(promote candidate)")
        bench_promote_id = state["squad"]["bench"][0]["id"]
        state = client.post_json(
            "/api/roster-promote", {"bench_member_id": bench_promote_id}
        )
        assert_state_shape(state, "roster-promote")
        if not any(member["id"] == bench_promote_id for member in state["squad"]["active"]):
            raise SmokeFailure("promoted bench member not found in active squad")
        print("ok POST /api/roster-promote")

    while state["recruit_offer"]:
        decline_id = state["recruit_offer"][0]["id"]
        state = client.post_json(
            "/api/recruit-choice",
            {
                "candidate_id": decline_id,
                "destination": "decline",
                "replace_member_id": None,
            },
        )
        assert_state_shape(state, "recruit-choice(decline remainder)")


def exercise_route_progression(client: Client, seed: int) -> None:
    state = client.post_json("/api/new-run", {"seed": seed})
    assert_state_shape(state, "route new-run")
    max_depth = state["depth"]
    saw_boss = False

    for step in range(12):
        if state["phase"] == "choose_node":
            node = preferred_available_node(
                state, ["recruit", "rest", "event", "fight", "elite", "boss"]
            )
            saw_boss = saw_boss or node["kind"] == "boss"
            state = client.post_json("/api/choose-node", {"node_id": node["id"]})
            assert_state_shape(state, f"route choose-node step={step}")
            max_depth = max(max_depth, state["depth"])

        if state["phase"] == "fight_preview":
            state = client.post_json("/api/start-fight", {})
            assert_state_shape(state, f"route start-fight step={step}")
            assert_no_living_position_overlap(state, f"route start-fight step={step}")

        if state["phase"] == "combat_playback":
            state = client.post_json(
                "/api/fight-command", {"command": "finish", "seconds": 1}
            )
            assert_state_shape(state, f"route finish fight step={step}")
            assert_no_living_position_overlap(state, f"route finish fight step={step}")
            max_depth = max(max_depth, state["depth"])

        if state["phase"] == "reward_review":
            state = resolve_recruit_rewards(client, state, f"route rewards step={step}")
            max_depth = max(max_depth, state["depth"])

        if state["phase"] == "run_over":
            break

    if max_depth < 4 and not saw_boss:
        raise SmokeFailure(
            f"route progression stalled before boss floor: max_depth={max_depth}, phase={state['phase']}"
        )
    print(f"ok route progression depth={max_depth} phase={state['phase']}")


def resolve_recruit_rewards(
    client: Client, state: dict[str, Any], label: str
) -> dict[str, Any]:
    while state["phase"] == "reward_review" and state["recruit_offer"]:
        candidate_id = state["recruit_offer"][0]["id"]
        active_count = len(state["squad"]["active"])
        bench_count = len(state["squad"]["bench"])
        if active_count < state["squad"]["max_active"]:
            destination = "active"
        elif bench_count < state["squad"]["max_bench"]:
            destination = "bench"
        else:
            destination = "decline"
        state = client.post_json(
            "/api/recruit-choice",
            {
                "candidate_id": candidate_id,
                "destination": destination,
                "replace_member_id": None,
            },
        )
        assert_state_shape(state, f"{label} recruit-choice")
    return state


def assert_state_shape(state: Any, label: str) -> None:
    require_type(state, dict, label)
    require_keys(
        state,
        label,
        [
            "has_run",
            "title",
            "phase",
            "seed",
            "depth",
            "gold",
            "inventory",
            "squad",
            "grid",
            "route",
            "available_nodes",
            "pending_fight",
            "live_fight",
            "last_reward",
            "recruit_offer",
            "terminal",
            "log",
        ],
    )
    require_type(state["has_run"], bool, f"{label}.has_run")
    require_type(state["title"], str, f"{label}.title")
    require_type(state["phase"], str, f"{label}.phase")
    require_optional_type(state["seed"], int, f"{label}.seed")
    require_type(state["depth"], int, f"{label}.depth")
    require_type(state["gold"], int, f"{label}.gold")
    assert_inventory_shape(state["inventory"], f"{label}.inventory")
    if state["gold"] != state["inventory"]["gold"]:
        raise SmokeFailure(f"{label}.gold does not match inventory.gold")
    assert_squad_shape(state["squad"], f"{label}.squad")
    assert_grid_shape(state["grid"], f"{label}.grid")
    require_type(state["route"], list, f"{label}.route")
    for idx, node in enumerate(state["route"]):
        assert_route_node_shape(node, f"{label}.route[{idx}]")
    require_type(state["available_nodes"], list, f"{label}.available_nodes")
    for idx, node_id in enumerate(state["available_nodes"]):
        require_type(node_id, int, f"{label}.available_nodes[{idx}]")
    require_optional_type(state["pending_fight"], dict, f"{label}.pending_fight")
    if state["pending_fight"] is not None:
        assert_pending_fight_shape(state["pending_fight"], f"{label}.pending_fight")
    require_optional_type(state["live_fight"], dict, f"{label}.live_fight")
    if state["live_fight"] is not None:
        assert_live_fight_shape(state["live_fight"], f"{label}.live_fight")
    require_optional_type(state["last_reward"], dict, f"{label}.last_reward")
    if state["last_reward"] is not None:
        assert_reward_shape(state["last_reward"], f"{label}.last_reward")
    require_type(state["recruit_offer"], list, f"{label}.recruit_offer")
    for idx, member in enumerate(state["recruit_offer"]):
        assert_member_shape(member, f"{label}.recruit_offer[{idx}]")
    require_optional_type(state["terminal"], str, f"{label}.terminal")
    require_type(state["log"], list, f"{label}.log")
    for idx, line in enumerate(state["log"]):
        require_type(line, str, f"{label}.log[{idx}]")


def assert_inventory_shape(value: Any, label: str) -> None:
    require_type(value, dict, label)
    require_keys(value, label, ["gold", "items"])
    require_type(value["gold"], int, f"{label}.gold")
    require_type(value["items"], list, f"{label}.items")


def assert_squad_shape(value: Any, label: str) -> None:
    require_type(value, dict, label)
    require_keys(value, label, ["active", "bench", "max_active", "max_bench"])
    require_type(value["active"], list, f"{label}.active")
    require_type(value["bench"], list, f"{label}.bench")
    require_type(value["max_active"], int, f"{label}.max_active")
    require_type(value["max_bench"], int, f"{label}.max_bench")
    for idx, member in enumerate(value["active"]):
        assert_member_shape(member, f"{label}.active[{idx}]")
    for idx, member in enumerate(value["bench"]):
        assert_member_shape(member, f"{label}.bench[{idx}]")


def assert_member_shape(value: Any, label: str) -> None:
    require_type(value, dict, label)
    require_keys(
        value,
        label,
        [
            "id",
            "name",
            "level",
            "xp",
            "next_level_xp",
            "role",
            "rarity",
            "hp",
            "max_hp",
            "weapon",
            "status",
            "wounds",
            "wound_total",
            "level_up_available",
            "stats",
        ],
    )
    require_type(value["id"], str, f"{label}.id")
    require_type(value["name"], str, f"{label}.name")
    require_type(value["level"], int, f"{label}.level")
    require_type(value["xp"], int, f"{label}.xp")
    require_type(value["next_level_xp"], int, f"{label}.next_level_xp")
    require_type(value["role"], str, f"{label}.role")
    require_type(value["rarity"], str, f"{label}.rarity")
    require_type(value["hp"], int, f"{label}.hp")
    require_type(value["max_hp"], int, f"{label}.max_hp")
    require_type(value["weapon"], str, f"{label}.weapon")
    require_type(value["status"], str, f"{label}.status")
    require_type(value["wounds"], list, f"{label}.wounds")
    require_type(value["wound_total"], int, f"{label}.wound_total")
    require_type(value["level_up_available"], bool, f"{label}.level_up_available")
    require_type(value["stats"], list, f"{label}.stats")


def assert_grid_shape(value: Any, label: str) -> None:
    require_type(value, dict, label)
    require_keys(value, label, ["width", "height", "tile_size_ft"])
    require_type(value["width"], int, f"{label}.width")
    require_type(value["height"], int, f"{label}.height")
    require_number(value["tile_size_ft"], f"{label}.tile_size_ft")
    if value["width"] <= 0 or value["height"] <= 0:
        raise SmokeFailure(f"{label} dimensions must be positive")


def assert_route_node_shape(value: Any, label: str) -> None:
    require_type(value, dict, label)
    require_keys(
        value,
        label,
        [
            "id",
            "floor",
            "lane",
            "kind",
            "completed",
            "required_depth",
            "tier",
            "difficulty",
            "reward",
        ],
    )
    require_type(value["id"], int, f"{label}.id")
    require_type(value["floor"], int, f"{label}.floor")
    require_type(value["lane"], int, f"{label}.lane")
    require_type(value["kind"], str, f"{label}.kind")
    require_type(value["completed"], bool, f"{label}.completed")
    require_type(value["required_depth"], int, f"{label}.required_depth")
    require_type(value["tier"], str, f"{label}.tier")
    require_type(value["difficulty"], int, f"{label}.difficulty")
    assert_reward_node_shape(value["reward"], f"{label}.reward")


def assert_pending_fight_shape(value: Any, label: str) -> None:
    require_type(value, dict, label)
    require_keys(value, label, ["tier", "enemy_count", "enemies"])
    require_type(value["tier"], str, f"{label}.tier")
    require_type(value["enemy_count"], int, f"{label}.enemy_count")
    require_type(value["enemies"], list, f"{label}.enemies")
    if value["enemy_count"] != len(value["enemies"]):
        raise SmokeFailure(f"{label}.enemy_count does not match enemies length")
    for idx, enemy in enumerate(value["enemies"]):
        require_type(enemy, dict, f"{label}.enemies[{idx}]")
        require_keys(enemy, f"{label}.enemies[{idx}]", ["name", "level"])
        require_type(enemy["name"], str, f"{label}.enemies[{idx}].name")
        require_type(enemy["level"], int, f"{label}.enemies[{idx}].level")


def assert_live_fight_shape(value: Any, label: str) -> None:
    require_type(value, dict, label)
    require_keys(
        value,
        label,
        [
            "grid",
            "elapsed_seconds",
            "max_seconds",
            "running",
            "done",
            "winner_team",
            "combatants",
            "initiative",
            "log_tail",
            "events_tail",
        ],
    )
    assert_grid_shape(value["grid"], f"{label}.grid")
    require_type(value["elapsed_seconds"], int, f"{label}.elapsed_seconds")
    require_type(value["max_seconds"], int, f"{label}.max_seconds")
    require_type(value["running"], bool, f"{label}.running")
    require_type(value["done"], bool, f"{label}.done")
    require_optional_type(value["winner_team"], int, f"{label}.winner_team")
    require_type(value["combatants"], list, f"{label}.combatants")
    for idx, unit in enumerate(value["combatants"]):
        assert_battle_unit_shape(unit, f"{label}.combatants[{idx}]")
    require_type(value["initiative"], list, f"{label}.initiative")
    require_type(value["log_tail"], list, f"{label}.log_tail")
    require_type(value["events_tail"], list, f"{label}.events_tail")


def assert_battle_unit_shape(value: Any, label: str) -> None:
    require_type(value, dict, label)
    require_keys(
        value,
        label,
        [
            "id",
            "name",
            "team_id",
            "x",
            "y",
            "hp",
            "max_hp",
            "status",
            "weapon",
            "reach_ft",
            "max_range_ft",
            "move_tiles",
            "initiative",
            "intent",
        ],
    )
    require_type(value["id"], str, f"{label}.id")
    require_type(value["name"], str, f"{label}.name")
    require_type(value["team_id"], int, f"{label}.team_id")
    require_type(value["x"], int, f"{label}.x")
    require_type(value["y"], int, f"{label}.y")
    require_type(value["hp"], int, f"{label}.hp")
    require_type(value["max_hp"], int, f"{label}.max_hp")
    require_type(value["status"], str, f"{label}.status")
    require_type(value["weapon"], str, f"{label}.weapon")
    require_number(value["reach_ft"], f"{label}.reach_ft")
    require_optional_number(value["max_range_ft"], f"{label}.max_range_ft")
    require_type(value["move_tiles"], int, f"{label}.move_tiles")
    require_number(value["initiative"], f"{label}.initiative")
    require_type(value["intent"], str, f"{label}.intent")


def assert_reward_node_shape(value: Any, label: str) -> None:
    require_type(value, dict, label)
    require_keys(
        value,
        label,
        [
            "gold_min",
            "gold_max",
            "xp_per_survivor",
            "recruit_chance_percent",
            "item_chance_percent",
            "reward_multiplier_percent",
        ],
    )
    for key in (
        "gold_min",
        "gold_max",
        "xp_per_survivor",
        "recruit_chance_percent",
        "item_chance_percent",
        "reward_multiplier_percent",
    ):
        require_type(value[key], int, f"{label}.{key}")


def assert_reward_shape(value: Any, label: str) -> None:
    require_type(value, dict, label)
    require_keys(value, label, ["gold", "xp_per_survivor", "deaths", "level_ups"])
    require_type(value["gold"], int, f"{label}.gold")
    require_type(value["xp_per_survivor"], int, f"{label}.xp_per_survivor")
    require_type(value["deaths"], list, f"{label}.deaths")
    require_type(value["level_ups"], list, f"{label}.level_ups")


def assert_no_living_position_overlap(state: dict[str, Any], label: str) -> None:
    live_fight = state.get("live_fight")
    if live_fight is None:
        return
    grid = live_fight["grid"]
    seen: dict[tuple[int, int], str] = {}
    for unit in live_fight["combatants"]:
        if not is_living_unit(unit):
            continue
        pos = (unit["x"], unit["y"])
        if not (0 <= unit["x"] < grid["width"] and 0 <= unit["y"] < grid["height"]):
            raise SmokeFailure(f"{label}: living unit {unit['id']} is outside grid at {pos}")
        if pos in seen:
            raise SmokeFailure(
                f"{label}: living units {seen[pos]} and {unit['id']} overlap at {pos}"
            )
        seen[pos] = unit["id"]


def is_living_unit(unit: dict[str, Any]) -> bool:
    return unit.get("hp", 0) > 0 and unit.get("status") == "alive"


def first_available_node(state: dict[str, Any], kinds: set[str]) -> dict[str, Any]:
    available = set(state["available_nodes"])
    for node in state["route"]:
        if node["id"] in available and node["kind"] in kinds:
            return node
    raise SmokeFailure(f"no available route node found for kinds {sorted(kinds)}")


def preferred_available_node(state: dict[str, Any], preferred_kinds: list[str]) -> dict[str, Any]:
    available = set(state["available_nodes"])
    candidates = [node for node in state["route"] if node["id"] in available]
    for kind in preferred_kinds:
        for node in candidates:
            if node["kind"] == kind:
                return node
    if candidates:
        return candidates[0]
    raise SmokeFailure("no available route node found")


def assert_optional_object(state: dict[str, Any], key: str, label: str) -> None:
    if state.get(key) is None:
        raise SmokeFailure(f"{label}.{key} was null")


def require_keys(value: dict[str, Any], label: str, keys: list[str]) -> None:
    missing = [key for key in keys if key not in value]
    if missing:
        raise SmokeFailure(f"{label} missing keys: {', '.join(missing)}")


def require_type(value: Any, expected: type, label: str) -> None:
    if expected is int:
        ok = type(value) is int
    else:
        ok = isinstance(value, expected)
    if not ok:
        raise SmokeFailure(f"{label} expected {expected.__name__}, got {type(value).__name__}")


def require_optional_type(value: Any, expected: type, label: str) -> None:
    if value is not None:
        require_type(value, expected, label)


def require_number(value: Any, label: str) -> None:
    if type(value) not in {int, float}:
        raise SmokeFailure(f"{label} expected number, got {type(value).__name__}")


def require_optional_number(value: Any, label: str) -> None:
    if value is not None:
        require_number(value, label)


def normalize_headers(headers: Any) -> dict[str, str]:
    return {str(key).lower(): str(value) for key, value in headers.items()}


def decode(body: bytes) -> str:
    return body.decode("utf-8", errors="replace")


if __name__ == "__main__":
    raise SystemExit(main())
