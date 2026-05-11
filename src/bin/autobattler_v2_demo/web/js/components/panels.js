import { escapeHtml, escapeJs, logLines, rewardDetails } from "../utils.js";
import { swingTimeline } from "./combat.js";

export function renderEncounter(state) {
  const el = document.getElementById("encounter");
  if (state.terminal) {
    el.innerHTML = `<h2>Run Complete</h2><div class="sub">${escapeHtml(state.terminal)}</div><button onclick="newRun()">Roll Again</button>`;
    return;
  }
  if (state.live_fight) {
    const fight = state.live_fight;
    el.innerHTML = `<h2>Live Combat</h2>
      <div class="sub">${escapeHtml(fight.enemy_name)} is active at ${fight.elapsed_seconds}s. Watch the log, step time forward, or let auto-play tick.</div>
      <div class="combat-controls">
        <button onclick="fightCommand('step', 1)">Step 1s</button>
        <button onclick="fightCommand('next_attack', 1)">Skip to Next Attack</button>
        <button onclick="fightCommand('play', 1)" ${fight.running ? "disabled" : ""}>Auto</button>
        <button onclick="fightCommand('pause', 1)" ${fight.running ? "" : "disabled"}>Pause</button>
        <button onclick="fightCommand('skip', 1)">Finish</button>
      </div>
      ${swingTimeline(fight)}`;
    return;
  }
  if (state.pending_event) {
    const event = state.pending_event;
    el.innerHTML = `<h2>${escapeHtml(event.name)}</h2>
      <div class="sub">${escapeHtml(event.description)}</div>
      <div class="choice-list">${event.choices.map(c => `<button onclick="eventChoice('${escapeJs(c.id)}')">${escapeHtml(c.text)}</button>`).join("")}</div>`;
    return;
  }
  if (state.pending_fight) {
    const fight = state.pending_fight;
    el.innerHTML = `<h2>${escapeHtml(fight.tier)} Fight</h2>
      <div class="sub">Enemy scouted: ${escapeHtml(fight.enemy_name)}. This is now a fight scene, not an instant node resolution.</div>
      <button onclick="startFight()">Fight</button>`;
    return;
  }
  if (state.phase === "reward_review" && state.last_reward) {
    el.innerHTML = `<div class="section-title"><h2>Reward Review</h2><span class="pill">Claim</span></div>
      <div class="reward">${rewardDetails(state.last_reward)}</div>
      <button onclick="claimReward()">Continue Route</button>`;
    return;
  }
  if (state.phase === "choose_node") {
    el.innerHTML = `<h2>Choose Node</h2><div class="sub">Pick an available route node on the map. Fights resolve through the existing HackMaster combat engine.</div>`;
  } else {
    el.innerHTML = `<h2>Encounter</h2><div class="sub">Roll a character or resolve the pending choice.</div>`;
  }
}

export function renderInventory(state) {
  const el = document.getElementById("inventory");
  const inventory = state.inventory;
  if (!inventory) {
    el.innerHTML = `<div class="sub">No run inventory yet.</div>`;
    return;
  }
  const items = countedItems(inventory.items || []);
  el.innerHTML = `<div class="inventory-panel">
    <div class="inventory-gold"><span>Gold</span><strong>${inventory.gold}</strong></div>
    <div class="inventory-list">
      ${items.length
        ? items.map(item => `<div class="inventory-item"><span>${escapeHtml(item.name)}</span><strong>${item.count > 1 ? `x${item.count}` : ""}</strong></div>`).join("")
        : `<div class="sub">Pack is empty.</div>`}
    </div>
  </div>`;
}

export function renderReward(state) {
  const el = document.getElementById("reward");
  if (!state.last_reward) {
    el.innerHTML = `<div class="sub">No reward yet.</div>`;
    return;
  }
  el.innerHTML = `<div class="reward">${rewardDetails(state.last_reward)}</div>`;
}

export function renderFight(state) {
  const el = document.getElementById("fight");
  if (state.live_fight) {
    const fight = state.live_fight;
    el.innerHTML = `<div class="fight fight-summary">
      <div class="fight-result-line"><strong>${escapeHtml(fight.status)}</strong><span>vs ${escapeHtml(fight.enemy_name)}</span></div>
      <div class="summary-grid">
        <div><span>Time</span><strong>${fight.elapsed_seconds}s</strong></div>
        <div><span>Range</span><strong>${Number(fight.distance_ft || 0).toFixed(1)} ft</strong></div>
      </div>
      <div class="combat-log-mini">${logLines(fight.log_tail, "No strikes yet.")}</div>
    </div>`;
    return;
  }
  if (!state.last_fight) {
    el.innerHTML = `<div class="sub">No fight resolved yet.</div>`;
    return;
  }
  const f = state.last_fight;
  el.innerHTML = `<div class="fight fight-summary">
    <div class="fight-result-line"><strong class="${f.won ? "ok" : "danger"}">${f.won ? "Victory" : "Defeat"}</strong><span>vs ${escapeHtml(f.enemy)}</span></div>
    <div class="summary-grid">
      <div><span>Turns</span><strong>${f.turns}s</strong></div>
      <div><span>HP left</span><strong>${f.remaining_hp}</strong></div>
      <div><span>Dealt</span><strong>${escapeHtml(f.hits_dealt)}</strong></div>
      <div><span>Taken</span><strong>${escapeHtml(f.hits_taken)}</strong></div>
    </div>
    <div class="combat-log-mini">${logLines(f.combat_log, "No combat log.")}</div>
  </div>`;
}

export function renderLog(state) {
  const el = document.getElementById("log");
  el.innerHTML = (state.last_log || []).map(escapeHtml).join("<br>") || "No log.";
}

function countedItems(items) {
  const counts = new Map();
  for (const item of items) counts.set(item, (counts.get(item) || 0) + 1);
  return Array.from(counts.entries())
    .map(([name, count]) => ({ name, count }))
    .sort((a, b) => a.name.localeCompare(b.name));
}
