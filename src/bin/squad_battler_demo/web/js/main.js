const $ = (id) => document.getElementById(id);
const asArray = (value) => Array.isArray(value) ? value : [];
const toNumber = (value, fallback = 0) => Number.isFinite(Number(value)) ? Number(value) : fallback;

const Api = {
  async requestState() {
    const response = await fetch("/api/state");
    return readJsonResponse(response);
  },

  async postJson(path, body = {}) {
    const response = await fetch(path, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    return readJsonResponse(response);
  },

  newRun(seed) {
    return this.postJson("/api/new-run", { seed });
  },

  chooseNode(nodeId) {
    return this.postJson("/api/choose-node", { node_id: nodeId });
  },

  startFight() {
    return this.postJson("/api/start-fight", {});
  },

  fightCommand(command, seconds = null) {
    return this.postJson("/api/fight-command", { command, seconds });
  },

  recruitChoice(candidateId, destination, replaceMemberId = null) {
    return this.postJson("/api/recruit-choice", {
      candidate_id: candidateId,
      destination,
      replace_member_id: replaceMemberId,
    });
  },

  rosterSwap(activeMemberId, benchMemberId) {
    return this.postJson("/api/roster-swap", {
      active_member_id: activeMemberId,
      bench_member_id: benchMemberId,
    });
  },

  rosterPromote(benchMemberId) {
    return this.postJson("/api/roster-promote", {
      bench_member_id: benchMemberId,
    });
  },

  rosterDismiss(benchMemberId) {
    return this.postJson("/api/roster-dismiss", {
      bench_member_id: benchMemberId,
    });
  },
};

const Store = {
  state: null,
  autoTimer: null,

  set(nextState) {
    this.state = normalizeState(nextState);
    if (!this.state.live_fight || this.state.live_fight.done) {
      this.stopAutoPlayback();
    }
    renderApp(this.state);
  },

  async refresh() {
    this.set(await Api.requestState());
  },

  async run(action) {
    try {
      const nextState = await action();
      this.set(nextState);
      return true;
    } catch (err) {
      this.reportError(err);
      return false;
    }
  },

  reportError(err) {
    const message = err instanceof Error ? err.message : String(err);
    const currentLog = asArray(this.state?.log);
    this.set({
      ...(this.state || {}),
      log: [...currentLog, `UI request failed: ${message}`],
    });
  },

  startAutoPlayback() {
    this.stopAutoPlayback();
    this.autoTimer = window.setInterval(() => {
      if (!this.state?.live_fight || this.state.live_fight.done) {
        this.stopAutoPlayback();
        return;
      }
      this.run(() => Api.fightCommand("tick", 1));
    }, 700);
  },

  stopAutoPlayback() {
    if (this.autoTimer) {
      window.clearInterval(this.autoTimer);
      this.autoTimer = null;
    }
  },

  async skipToNextInitiative() {
    await this.run(() => Api.fightCommand("skip_to_next_initiative", 1));
  },
};

const GridModule = {
  render(state) {
    const grid = state.live_fight?.grid || state.grid || {};
    const width = Math.max(1, Math.trunc(toNumber(grid.width, 12)));
    const height = Math.max(1, Math.trunc(toNumber(grid.height, 8)));
    const tileSize = toNumber(grid.tile_size_ft, 5);
    const el = $("battleGrid");

    el.style.setProperty("--cols", width);
    el.style.setProperty("--rows", height);
    el.setAttribute("aria-label", `${width} by ${height} tactical grid, ${tileSize} foot squares`);
    $("gridLabel").textContent = `${width} x ${height} - ${tileSize} ft squares`;

    const fragment = document.createDocumentFragment();
    for (let y = 0; y < height; y += 1) {
      for (let x = 0; x < width; x += 1) {
        const cell = document.createElement("div");
        cell.className = "grid-cell";
        cell.dataset.x = x;
        cell.dataset.y = y;
        fragment.appendChild(cell);
      }
    }

    for (const unit of asArray(state.live_fight?.combatants)) {
      fragment.appendChild(createToken(unit, width, height, state));
    }

    el.replaceChildren(fragment);
  },
};

const RosterModule = {
  render(state) {
    const active = asArray(state.squad?.active);
    const bench = asArray(state.squad?.bench);
    const maxActive = toNumber(state.squad?.max_active, 0);
    const maxBench = toNumber(state.squad?.max_bench, 0);
    const liveById = new Map(asArray(state.live_fight?.combatants).map((unit) => [unit.id, unit]));
    const liveActive = active.map((member) => liveById.has(member.id)
      ? mergeMemberWithUnit(member, liveById.get(member.id))
      : member);

    $("rosterStatus").textContent = `${active.length}/${maxActive || "-"} active`;
    $("benchStatus").textContent = `${bench.length}/${maxBench || "-"} reserve`;
    $("playerSquad").innerHTML = renderMembers(liveActive, "active", "No active squad.");
    $("benchSquad").innerHTML = renderMembers(bench, "bench", "No bench.");
  },
};

const RouteModule = {
  render(state) {
    $("routeDepth").textContent = `Depth ${toNumber(state.depth, 0)}`;
    const route = asArray(state.route);
    const el = $("routeMap");
    if (!route.length) {
      el.className = "route-map muted";
      el.textContent = "Roll a squad to reveal the route.";
      return;
    }

    const floors = [...new Set(route.map((node) => toNumber(node.floor, 0)))].sort((a, b) => a - b);
    el.style.setProperty("--lane-count", Math.max(1, floors.length));
    el.className = "route-map";
    el.innerHTML = floors.map((floor) => {
      const nodes = route
        .filter((node) => toNumber(node.floor, 0) === floor)
        .sort((a, b) => toNumber(a.lane, 0) - toNumber(b.lane, 0));
      return `<div class="route-floor">
        <div class="route-floor-title">Floor ${floor + 1}</div>
        ${nodes.map((node) => renderRouteNode(node, state)).join("")}
      </div>`;
    }).join("");
  },
};

const EncounterModule = {
  render(state) {
    const status = state.terminal || phaseLabel(state.phase);
    $("encounterStatus").textContent = status;
    const el = $("enemySquad");

    if (state.terminal) {
      el.innerHTML = `<div class="encounter-card"><strong>${escapeHtml(state.terminal)}</strong></div>`;
      return;
    }

    if (state.pending_fight) {
      const enemies = asArray(state.pending_fight.enemies);
      el.innerHTML = `<div class="encounter-card">
        <h3>${escapeHtml(state.pending_fight.tier || "Unknown")} enemy squad</h3>
        <div class="detail">${enemies.length || state.pending_fight.enemy_count || 0} hostile combatants sighted.</div>
      </div>
      ${enemies.map((enemy) => `<div class="member" tabindex="0" data-tooltip="${attr(`${enemy.name}\nLevel ${enemy.level || "?"}\nIntent: waiting for engagement`)}">
        <strong><span>${escapeHtml(enemy.name)}</span><span>Lv ${escapeHtml(enemy.level ?? "?")}</span></strong>
        <div class="detail">Pending deployment</div>
      </div>`).join("")}
      <button data-action="start-fight">Start Fight</button>`;
      return;
    }

    if (state.live_fight) {
      const enemies = asArray(state.live_fight.combatants)
        .filter((unit) => unit.team_id !== 0)
        .map((unit) => unitToMember(unit));
      el.innerHTML = renderMembers(enemies, "enemy", "No enemies.");
      return;
    }

    if (state.has_run) {
      el.innerHTML = `<div class="encounter-card">
        <h3>Choose a route node</h3>
        <div class="detail">Available route seals on the parchment map start the next encounter.</div>
      </div>`;
      return;
    }

    el.innerHTML = `<div class="muted">No enemy squad.</div>`;
  },
};

const RecruitModule = {
  render(state) {
    const offer = asArray(state.recruit_offer);
    const el = $("recruitPanel");
    if (!offer.length) {
      el.innerHTML = "";
      return;
    }

    const active = asArray(state.squad?.active);
    const bench = asArray(state.squad?.bench);
    const activeFull = active.length >= toNumber(state.squad?.max_active, 0);
    const benchFull = bench.length >= toNumber(state.squad?.max_bench, 0);

    el.innerHTML = `<div class="panel-heading">
      <h2>Recruit Offer</h2>
      <span class="tag">${offer.length} available</span>
    </div>
    ${offer.map((candidate) => renderRecruit(candidate, active, activeFull, benchFull)).join("")}`;
  },
};

const PlaybackModule = {
  render(state) {
    const live = state.live_fight;
    const el = $("playbackControls");
    if (!live) {
      el.innerHTML = `<button disabled>Play</button><button disabled>Step</button><button disabled>Next Initiative</button><span class="detail">Playback unlocks during combat.</span>`;
      return;
    }

    const elapsed = toNumber(live.elapsed_seconds, 0);
    const max = toNumber(live.max_seconds, 0);
    const done = Boolean(live.done);
    const running = Boolean(live.running) || Boolean(Store.autoTimer);
    el.innerHTML = `<button data-action="fight-command" data-command="play" ${done || running ? "disabled" : ""}>Play</button>
      <button data-action="fight-command" data-command="pause" ${done || !running ? "disabled" : ""}>Pause</button>
      <button data-action="fight-command" data-command="step" ${done ? "disabled" : ""}>Step</button>
      <button data-action="skip-initiative" ${done ? "disabled" : ""}>Next Initiative</button>
      <button data-action="fight-command" data-command="finish" ${done ? "disabled" : ""}>Finish</button>
      <span class="detail">${elapsed}s / ${max || "?"}s${done ? " - resolved" : ""}</span>`;
  },
};

const CombatLogModule = {
  render(state) {
    $("initiative").innerHTML = renderInitiative(asArray(state.live_fight?.initiative));
    $("reward").innerHTML = renderReward(state.last_reward);
    const runLog = asArray(state.log).map((line) => ({ line, type: "run" }));
    const fightLog = asArray(state.live_fight?.log_tail).map((line) => ({ line, type: "combat" }));
    const lines = [...runLog, ...fightLog];
    $("log").innerHTML = lines.length
      ? lines.map(({ line, type }) => `<div class="log-line ${type}">${escapeHtml(line)}</div>`).join("")
      : `<div class="muted">No log.</div>`;
  },
};

function renderApp(state) {
  $("phase").textContent = phaseLabel(state.phase);
  $("metrics").innerHTML = renderMetrics(state);
  GridModule.render(state);
  RosterModule.render(state);
  RouteModule.render(state);
  EncounterModule.render(state);
  RecruitModule.render(state);
  PlaybackModule.render(state);
  CombatLogModule.render(state);
}

function renderMetrics(state) {
  const active = asArray(state.squad?.active);
  const bench = asArray(state.squad?.bench);
  const live = state.live_fight;
  const seed = state.seed ?? "unrolled";
  const items = asArray(state.inventory?.items).length;
  return [
    `Depth ${toNumber(state.depth, 0)}`,
    `Gold ${toNumber(state.gold ?? state.inventory?.gold, 0)}`,
    `Active ${active.length}/${state.squad?.max_active ?? "-"}`,
    `Bench ${bench.length}/${state.squad?.max_bench ?? "-"}`,
    live ? `Time ${toNumber(live.elapsed_seconds, 0)}s` : `${toNumber(state.grid?.tile_size_ft, 5)} ft grid`,
    `Items ${items}`,
    `Seed ${seed}`,
  ].map((text) => `<span>${escapeHtml(text)}</span>`).join("");
}

function renderMembers(members, role, empty) {
  const list = asArray(members);
  if (!list.length) return `<div class="muted">${escapeHtml(empty)}</div>`;
  return list.map((member) => renderMember(member, role)).join("");
}

function renderMember(member, role) {
  const hp = toNumber(member.hp, 0);
  const maxHp = Math.max(1, toNumber(member.max_hp, 1));
  const level = member.level ?? "?";
  const stats = asArray(member.stats);
  const tooltip = memberTooltip(member, role);
  const actions = renderMemberActions(member, role);
  return `<div class="member" tabindex="0" data-member-id="${attr(member.id || "")}" data-tooltip="${attr(tooltip)}">
    <strong><span>${escapeHtml(member.name || "Unknown")}</span><span>${hp}/${maxHp}</span></strong>
    <div class="hpbar" style="--hp:${hpPct(member)}%"><span></span></div>
    <div class="detail">Lv ${escapeHtml(level)} - ${escapeHtml(member.weapon || "Unarmed")} - ${escapeHtml(member.status || "ready")}</div>
    ${stats.length ? `<div class="detail">${stats.map(escapeHtml).join(" - ")}</div>` : ""}
    ${actions}
  </div>`;
}

function renderMemberActions(member, role) {
  const state = Store.state || {};
  const active = asArray(state.squad?.active);
  const bench = asArray(state.squad?.bench);
  const rosterLocked = state.phase === "combat_playback" || state.phase === "fight_preview";
  const activeFull = active.length >= toNumber(state.squad?.max_active, 0);

  if (role === "active") {
    const selectId = `swap-${safeId(member.id)}`;
    return `<div class="member-actions">
      <button disabled>Active</button>
      <select id="${attr(selectId)}" data-swap-active="${attr(member.id)}" ${bench.length && !rosterLocked ? "" : "disabled"}>
        ${bench.map((benchMember) => `<option value="${attr(benchMember.id)}">${escapeHtml(benchMember.name)}</option>`).join("")}
      </select>
      <button data-action="roster-swap" data-active-member-id="${attr(member.id)}" ${bench.length && !rosterLocked ? "" : "disabled"}>Swap</button>
    </div>`;
  }
  if (role === "bench") {
    return `<div class="member-actions">
      <button disabled>Bench</button>
      <button data-action="roster-promote" data-bench-member-id="${attr(member.id)}" ${activeFull || rosterLocked ? "disabled" : ""}>Promote</button>
      <button data-action="roster-dismiss" data-bench-member-id="${attr(member.id)}" ${rosterLocked ? "disabled" : ""}>Dismiss</button>
    </div>`;
  }
  if (role === "enemy") {
    return `<div class="member-actions"><button disabled>Hostile</button></div>`;
  }
  return "";
}

function renderRouteNode(node, state) {
  const available = asArray(state.available_nodes).includes(node.id);
  const completed = Boolean(node.completed);
  const kind = String(node.kind || "unknown");
  const tooltip = `Floor ${toNumber(node.floor, 0) + 1}\nLane ${toNumber(node.lane, 0) + 1}\nStatus: ${completed ? "completed" : available ? "available" : "locked"}`;
  return `<div class="route-node ${available ? "available" : ""} ${completed ? "completed" : ""}" data-tooltip="${attr(tooltip)}">
    <button data-action="choose-node" data-node-id="${attr(node.id)}" ${available ? "" : "disabled"}>
      <span class="route-kind">${escapeHtml(kindLabel(kind))}</span>
      <span class="route-meta">${completed ? "Cleared" : available ? "Available" : "Locked"}</span>
    </button>
  </div>`;
}

function renderRecruit(candidate, active, activeFull, benchFull) {
  const hp = `${toNumber(candidate.hp, 0)}/${Math.max(1, toNumber(candidate.max_hp, 1))}`;
  const selectId = `replace-${safeId(candidate.id)}`;
  const canReplace = active.length > 0;
  return `<div class="recruit-card" tabindex="0" data-tooltip="${attr(memberTooltip(candidate, "recruit"))}">
    <strong><span>${escapeHtml(candidate.name)}</span><span>${hp}</span></strong>
    <div class="hpbar" style="--hp:${hpPct(candidate)}%"><span></span></div>
    <div class="detail">Lv ${escapeHtml(candidate.level ?? "?")} - ${escapeHtml(candidate.weapon || "Unarmed")}</div>
    <div class="detail">${asArray(candidate.stats).map(escapeHtml).join(" - ")}</div>
    <div class="recruit-actions">
      <button data-action="recruit-choice" data-candidate-id="${attr(candidate.id)}" data-destination="active" ${activeFull ? "disabled" : ""}>Add Active</button>
      <button data-action="recruit-choice" data-candidate-id="${attr(candidate.id)}" data-destination="bench" ${benchFull ? "disabled" : ""}>Add Bench</button>
      <button data-action="recruit-choice" data-candidate-id="${attr(candidate.id)}" data-destination="decline">Decline</button>
    </div>
    <div class="replacement-select">
      <label class="mini" for="${attr(selectId)}">Replace active member</label>
      <select id="${attr(selectId)}" data-replace-candidate="${attr(candidate.id)}" ${canReplace ? "" : "disabled"}>
        ${active.map((member) => `<option value="${attr(member.id)}">${escapeHtml(member.name)}</option>`).join("")}
      </select>
      <button data-action="recruit-choice" data-candidate-id="${attr(candidate.id)}" data-destination="replace" ${canReplace ? "" : "disabled"}>Replace</button>
    </div>
  </div>`;
}

function renderInitiative(rows) {
  if (!rows.length) return `<div class="muted">No combat timeline.</div>`;
  return rows
    .slice()
    .sort((a, b) => toNumber(a.next_action_in_seconds, 0) - toNumber(b.next_action_in_seconds, 0))
    .map((row) => `<div class="initiative-row" tabindex="0" data-tooltip="${attr(`Team: ${row.team_id === 0 ? "Company" : "Enemy"}\nIntent: ${row.ready ? "ready to act" : "waiting on initiative"}\nNext action: ${formatSeconds(row.next_action_in_seconds)}`)}">
      <strong><span>${escapeHtml(row.name)}</span><span>${row.ready ? "ready" : formatSeconds(row.next_action_in_seconds)}</span></strong>
      <div class="detail"><span class="team-tag">${row.team_id === 0 ? "Company" : "Enemy"}</span></div>
    </div>`).join("");
}

function renderReward(reward) {
  if (!reward) return `<div class="muted">No reward yet.</div>`;
  const deaths = asArray(reward.deaths);
  const levels = asArray(reward.level_ups);
  return `<div class="reward-panel">
    <strong>After-action spoils</strong>
    <div class="detail">Gold +${escapeHtml(reward.gold ?? 0)} - XP/survivor +${escapeHtml(reward.xp_per_survivor ?? 0)}</div>
    <div class="detail">Deaths: ${deaths.length ? deaths.map(escapeHtml).join(", ") : "none"}</div>
    <div class="detail">Level ups: ${levels.length ? levels.map(escapeHtml).join(", ") : "none"}</div>
  </div>`;
}

function createToken(unit, width, height, state) {
  const token = document.createElement("div");
  const x = Math.max(0, Math.min(width - 1, Math.trunc(toNumber(unit.x, 0))));
  const y = Math.max(0, Math.min(height - 1, Math.trunc(toNumber(unit.y, 0))));
  token.className = `unit-token team-${unit.team_id === 0 ? "0" : "1"}`;
  token.style.gridColumn = `${x + 1} / span 1`;
  token.style.gridRow = `${y + 1} / span 1`;
  token.setAttribute("role", "button");
  token.setAttribute("tabindex", "0");
  token.setAttribute("aria-disabled", toNumber(unit.hp, 0) <= 0 ? "true" : "false");
  token.setAttribute("aria-label", `${unit.name || "Unit"} at cell ${x + 1}, ${y + 1}`);
  token.dataset.tooltip = unitTooltip(unit, state);
  token.title = unitTooltip(unit, state).replaceAll("\n", " | ");
  token.textContent = initials(unit.name);
  return token;
}

function normalizeState(state) {
  const safe = state || {};
  return {
    ...safe,
    squad: {
      active: asArray(safe.squad?.active),
      bench: asArray(safe.squad?.bench),
      max_active: safe.squad?.max_active ?? 0,
      max_bench: safe.squad?.max_bench ?? 0,
    },
    route: asArray(safe.route),
    available_nodes: asArray(safe.available_nodes),
    recruit_offer: asArray(safe.recruit_offer),
    log: asArray(safe.log),
  };
}

function mergeMemberWithUnit(member, unit) {
  return {
    ...member,
    hp: unit.hp,
    max_hp: unit.max_hp,
    weapon: unit.weapon || member.weapon,
    status: unit.status || member.status,
    stats: [
      `cell ${toNumber(unit.x, 0) + 1},${toNumber(unit.y, 0) + 1}`,
      `${toNumber(unit.reach_ft, 0)} ft reach`,
      `${toNumber(unit.move_tiles, 0)} move`,
    ],
    intent: unitIntent(unit, Store.state),
  };
}

function unitToMember(unit) {
  return {
    id: unit.id,
    name: unit.name,
    hp: unit.hp,
    max_hp: unit.max_hp,
    level: "?",
    weapon: unit.weapon,
    status: unit.status,
    stats: [
      `cell ${toNumber(unit.x, 0) + 1},${toNumber(unit.y, 0) + 1}`,
      `${toNumber(unit.reach_ft, 0)} ft reach`,
      unit.max_range_ft ? `${toNumber(unit.max_range_ft, 0)} ft range` : `${toNumber(unit.move_tiles, 0)} move`,
    ],
    intent: unitIntent(unit, Store.state),
  };
}

function memberTooltip(member, role) {
  const wounds = Math.max(0, toNumber(member.max_hp, 0) - toNumber(member.hp, 0));
  const stats = asArray(member.stats).join(", ") || "stats unavailable";
  const intent = member.intent || (role === "enemy" ? "hostile formation" : role === "recruit" ? "awaiting contract" : "standing by");
  return `${member.name || "Unknown"}\nHP: ${member.hp ?? "?"}/${member.max_hp ?? "?"}\nWounds: ${wounds}\nWeapon: ${member.weapon || "unknown"}\nStatus: ${member.status || "unknown"}\nIntent: ${intent}\nStats: ${stats}`;
}

function unitTooltip(unit, state) {
  const wounds = Math.max(0, toNumber(unit.max_hp, 0) - toNumber(unit.hp, 0));
  const range = unit.max_range_ft ? `${toNumber(unit.max_range_ft, 0)} ft range` : `${toNumber(unit.reach_ft, 0)} ft reach`;
  return `${unit.name || "Unit"}\nHP: ${unit.hp ?? "?"}/${unit.max_hp ?? "?"}\nWounds: ${wounds}\nWeapon: ${unit.weapon || "unknown"}\nMove: ${unit.move_tiles ?? "?"} cells\nThreat: ${range}\nIntent: ${unitIntent(unit, state)}`;
}

function unitIntent(unit, state) {
  const row = asArray(state?.live_fight?.initiative).find((entry) => entry.combatant_id === unit.id);
  if (toNumber(unit.hp, 0) <= 0) return "downed";
  if (unit.intent && row?.ready) return `${unit.intent}; ready`;
  if (unit.intent) return String(unit.intent);
  if (row?.ready) return "ready to act";
  if (row) return `acts in ${formatSeconds(row.next_action_in_seconds)}`;
  return "advancing on nearest foe";
}

function hpPct(member) {
  return Math.max(0, Math.min(100, (toNumber(member.hp, 0) / Math.max(1, toNumber(member.max_hp, 1))) * 100)).toFixed(0);
}

function initials(name) {
  const letters = String(name || "?")
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase() || "")
    .join("");
  return letters || "?";
}

function kindLabel(kind) {
  return kind
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function phaseLabel(phase) {
  const labels = {
    start: "Roll a squad to begin",
    choose_node: "Choose a route",
    fight_preview: "Review enemy squad",
    combat_playback: "Combat playback",
    reward_review: "Review rewards",
    run_over: "Run over",
  };
  return labels[phase] || kindLabel(String(phase || "unknown"));
}

function formatSeconds(value) {
  return `${Math.ceil(toNumber(value, 0))}s`;
}

function safeId(value) {
  return String(value ?? "unknown").replace(/[^a-zA-Z0-9_-]/g, "_");
}

function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}

function attr(value) {
  return escapeHtml(value);
}

async function readJsonResponse(response) {
  const text = await response.text();
  let payload = {};
  try {
    payload = text ? JSON.parse(text) : {};
  } catch {
    payload = { error: text || response.statusText };
  }
  if (!response.ok) {
    throw new Error(payload.error || response.statusText || "Request failed");
  }
  return payload;
}

function handleAction(event) {
  const button = event.target.closest("[data-action]");
  if (!button || button.disabled) return;
  const action = button.dataset.action;

  if (action === "choose-node") {
    Store.run(() => Api.chooseNode(Number(button.dataset.nodeId)));
  } else if (action === "start-fight") {
    Store.run(() => Api.startFight());
  } else if (action === "fight-command") {
    handleFightCommand(button.dataset.command);
  } else if (action === "skip-initiative") {
    Store.skipToNextInitiative();
  } else if (action === "recruit-choice") {
    handleRecruitChoice(button);
  } else if (action === "roster-swap") {
    handleRosterSwap(button);
  } else if (action === "roster-promote") {
    Store.run(() => Api.rosterPromote(button.dataset.benchMemberId));
  } else if (action === "roster-dismiss") {
    Store.run(() => Api.rosterDismiss(button.dataset.benchMemberId));
  }
}

function handleFightCommand(command) {
  if (command === "play") {
    Store.run(() => Api.fightCommand("play")).then((ok) => {
      if (ok) Store.startAutoPlayback();
    });
  } else if (command === "pause") {
    Store.stopAutoPlayback();
    Store.run(() => Api.fightCommand("pause"));
  } else if (command === "step") {
    Store.run(() => Api.fightCommand("step", 1));
  } else if (command === "finish") {
    Store.stopAutoPlayback();
    Store.run(() => Api.fightCommand("finish", 1));
  }
}

function handleRecruitChoice(button) {
  const candidateId = button.dataset.candidateId;
  const destination = button.dataset.destination;
  let replaceMemberId = null;
  if (destination === "replace") {
    const select = [...document.querySelectorAll("select[data-replace-candidate]")]
      .find((current) => current.dataset.replaceCandidate === candidateId);
    replaceMemberId = select?.value || null;
  }
  Store.run(() => Api.recruitChoice(candidateId, destination, replaceMemberId));
}

function handleRosterSwap(button) {
  const activeMemberId = button.dataset.activeMemberId;
  const select = [...document.querySelectorAll("select[data-swap-active]")]
    .find((current) => current.dataset.swapActive === activeMemberId);
  const benchMemberId = select?.value || null;
  if (!benchMemberId) return;
  Store.run(() => Api.rosterSwap(activeMemberId, benchMemberId));
}

$("newRun").addEventListener("click", () => {
  const seedRaw = $("seed").value.trim();
  const seed = seedRaw ? Number(seedRaw) : null;
  Store.stopAutoPlayback();
  Store.run(() => Api.newRun(seed));
});

$("seed").addEventListener("keydown", (event) => {
  if (event.key === "Enter") {
    $("newRun").click();
  }
});

document.addEventListener("click", handleAction);

Store.refresh().catch((err) => Store.reportError(err));
