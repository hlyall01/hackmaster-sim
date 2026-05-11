async function requestState() {
  const response = await fetch("/api/state");
  if (!response.ok) throw new Error(await response.text());
  return response.json();
}

async function postJson(path, body) {
  const response = await fetch(path, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!response.ok) throw new Error(await response.text());
  return response.json();
}

async function newRun() {
  const seedRaw = document.getElementById("seed").value.trim();
  const state = await postJson("/api/new-run", {
    seed: seedRaw ? Number(seedRaw) : null,
  });
  render(state);
}

async function chooseNode(id) {
  render(await postJson("/api/choose-node", { node_id: id }));
}

async function startFight() {
  render(await postJson("/api/start-fight", {}));
}

async function fightCommand(command, seconds = 1) {
  render(await postJson("/api/fight-command", { command, seconds }));
}

function renderGrid(grid) {
  const el = document.getElementById("battleGrid");
  el.style.gridTemplateColumns = `repeat(${grid.width}, 1fr)`;
  el.style.gridTemplateRows = `repeat(${grid.height}, 1fr)`;
  el.style.aspectRatio = `${grid.width} / ${grid.height}`;
  el.innerHTML = "";
  for (let y = 0; y < grid.height; y += 1) {
    for (let x = 0; x < grid.width; x += 1) {
      const cell = document.createElement("div");
      cell.className = "grid-cell";
      cell.dataset.x = x;
      cell.dataset.y = y;
      el.appendChild(cell);
    }
  }
}

function render(state) {
  document.getElementById("phase").textContent = state.phase;
  document.getElementById("metrics").innerHTML = `
    <span>Depth ${state.depth}</span>
    <span>Gold ${state.gold}</span>
    <span>Active ${state.squad.active.length}/${state.squad.max_active}</span>
    <span>Bench ${state.squad.bench.length}/${state.squad.max_bench}</span>
    <span>${state.grid.tile_size_ft} ft squares</span>
  `;
  document.getElementById("gridLabel").textContent = `${state.grid.width} x ${state.grid.height} · ${state.grid.tile_size_ft} ft`;
  renderGrid(state.grid);
  document.getElementById("playerSquad").innerHTML = renderPlayerSquad(state);
  document.getElementById("benchSquad").innerHTML = renderMembers(state.squad.bench, "No bench.");
  document.getElementById("enemySquad").innerHTML = renderEnemies(state);
  renderCombatants(state);
  document.getElementById("initiative").innerHTML = renderInitiative(state.live_fight?.initiative || []);
  const fightLog = state.live_fight?.log_tail || [];
  document.getElementById("log").textContent = [...(state.log || []), ...fightLog].join("\n");
}

function renderMembers(members, empty) {
  if (!members.length) return `<div class="muted">${empty}</div>`;
  return members.map(member => `<div class="member">
    <strong><span>${escapeHtml(member.name)}</span><span>${member.hp}/${member.max_hp}</span></strong>
    <div class="hpbar" style="--hp:${hpPct(member)}%"><span></span></div>
    <div class="detail">Lv ${member.level} · ${escapeHtml(member.weapon)} · ${escapeHtml(member.status)}</div>
    <div class="detail">${(member.stats || []).map(escapeHtml).join(" · ")}</div>
  </div>`).join("");
}

function renderPlayerSquad(state) {
  if (!state.live_fight) return renderMembers(state.squad.active, "No active squad.");
  const live = state.live_fight.combatants
    .filter(unit => unit.team_id === 0)
    .map(unit => ({
      id: unit.id,
      name: unit.name,
      hp: unit.hp,
      max_hp: unit.max_hp,
      level: state.squad.active.find(member => member.id === unit.id)?.level || 1,
      weapon: unit.weapon,
      status: unit.status,
      stats: [`cell ${unit.x + 1},${unit.y + 1}`, `${unit.reach_ft} ft reach`],
    }));
  return renderMembers(live, "No active squad.");
}

function renderEnemies(state) {
  if (state.terminal) {
    return `<div class="member"><strong>${escapeHtml(state.terminal)}</strong></div>`;
  }
  if (state.recruit_offer && state.recruit_offer.length) {
    return `<h2>Recruit Offer</h2>${state.recruit_offer.map(candidate => `<div class="member">
      <strong><span>${escapeHtml(candidate.name)}</span><span>${candidate.hp}/${candidate.max_hp}</span></strong>
      <div class="hpbar" style="--hp:${hpPct(candidate)}%"><span></span></div>
      <div class="detail">Lv ${candidate.level} · ${escapeHtml(candidate.weapon)}</div>
      <button onclick="recruitChoice('${escapeJs(candidate.id)}', 'active')">Active</button>
      <button onclick="recruitChoice('${escapeJs(candidate.id)}', 'bench')">Bench</button>
      <button onclick="recruitChoice('${escapeJs(candidate.id)}', 'decline')">Decline</button>
    </div>`).join("")}`;
  }
  if (state.live_fight) {
    const controls = `<div class="combat-controls">
      <button onclick="fightCommand('step', 1)">Step 1s</button>
      <button onclick="fightCommand('tick', 5)">Step 5s</button>
      <button onclick="fightCommand('finish', 1)">Finish</button>
    </div>`;
    return renderMembers(
      state.live_fight.combatants.filter(unit => unit.team_id === 1).map(unit => ({
        id: unit.id,
        name: unit.name,
        hp: unit.hp,
        max_hp: unit.max_hp,
        level: 1,
        weapon: unit.weapon,
        status: unit.status,
        stats: [`${unit.x},${unit.y}`],
      })),
      "No enemies."
    ) + controls;
  }
  if (state.pending_fight) {
    const enemies = state.pending_fight.enemies.map(enemy => `<div class="member">
      <strong><span>${escapeHtml(enemy.name)}</span><span>Lv ${enemy.level}</span></strong>
      <div class="detail">${escapeHtml(state.pending_fight.tier)} squad</div>
    </div>`).join("");
    return `${enemies}<button onclick="startFight()">Start Fight</button>`;
  }
  if (state.has_run) {
    return `<div class="route-list">${state.route.map(node => {
      const available = state.available_nodes.includes(node.id);
      return `<button ${available ? "" : "disabled"} onclick="chooseNode(${node.id})">${escapeHtml(node.kind)} ${node.id + 1}</button>`;
    }).join("")}</div>`;
  }
  return `<div class="muted">No enemy squad.</div>`;
}

function renderCombatants(state) {
  const grid = document.getElementById("battleGrid");
  if (!state.live_fight) return;
  for (const unit of state.live_fight.combatants) {
    const token = document.createElement("div");
    token.className = `unit-token team-${unit.team_id}`;
    token.style.gridColumn = `${unit.x + 1}`;
    token.style.gridRow = `${unit.y + 1}`;
    token.setAttribute("aria-disabled", unit.hp <= 0 ? "true" : "false");
    token.textContent = initials(unit.name);
    token.title = `${unit.name} ${unit.hp}/${unit.max_hp}`;
    grid.appendChild(token);
  }
}

function renderInitiative(rows) {
  if (!rows.length) return `<div class="muted">No combat timeline.</div>`;
  return rows.map(row => `<div class="member">
    <strong><span>${escapeHtml(row.name)}</span><span>${row.ready ? "ready" : `${row.next_action_in_seconds.toFixed(0)}s`}</span></strong>
    <div class="detail"><span class="team-tag">Team ${row.team_id === 0 ? "Squad" : "Enemy"}</span></div>
  </div>`).join("");
}

function hpPct(member) {
  return Math.max(0, Math.min(100, (Number(member.hp || 0) / Math.max(1, Number(member.max_hp || 1))) * 100));
}

function initials(name) {
  return String(name || "?")
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map(part => part[0].toUpperCase())
    .join("");
}

async function recruitChoice(candidateId, destination, replaceMemberId = null) {
  render(await postJson("/api/recruit-choice", {
    candidate_id: candidateId,
    destination,
    replace_member_id: replaceMemberId,
  }));
}

function escapeJs(value) {
  return String(value ?? "").replaceAll("\\", "\\\\").replaceAll("'", "\\'");
}

function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}

document.getElementById("newRun").addEventListener("click", () => {
  newRun().catch((err) => {
    document.getElementById("log").textContent = err.message;
  });
});

Object.assign(window, { chooseNode, startFight, fightCommand, recruitChoice });

requestState()
  .then(render)
  .catch((err) => {
    document.getElementById("log").textContent = err.message;
  });
