import { escapeHtml, escapeJs, rewardDetails } from "../utils.js";
import { renderLiveFightScene } from "./combat.js";

const nodeIcons = { fight: "F", event: "?", rest: "R", elite: "E", boss: "B" };
const nodeLabels = { fight: "Fight", event: "Event", rest: "Rest", elite: "Elite", boss: "Boss" };

export function renderMap(state) {
  document.getElementById("routeStatus").textContent = state.has_run ? phaseLabel(state.phase) : "Route";
  const map = document.getElementById("map");
  if (state.live_fight) {
    map.innerHTML = renderLiveFightScene(state.live_fight);
    return;
  }
  if (state.phase === "reward_review" && state.last_reward) {
    map.innerHTML = renderRewardScene(state);
    return;
  }
  if (state.pending_event) {
    const event = state.pending_event;
    map.innerHTML = `<div class="node-scene event-scene">
      <div class="scene-mark">?</div>
      <h2>${escapeHtml(event.name)}</h2>
      <p>${escapeHtml(event.description)}</p>
      <div class="choice-list">${event.choices.map(c => `<button onclick="eventChoice('${escapeJs(c.id)}')">${escapeHtml(c.text)}</button>`).join("")}</div>
    </div>`;
    return;
  }
  if (state.pending_fight) {
    const fight = state.pending_fight;
    map.innerHTML = `<div class="node-scene fight-scene">
      <div class="scene-mark">F</div>
      <h2>${escapeHtml(fight.tier)} Fight</h2>
      <p>You have committed to a route and are sizing up ${escapeHtml(fight.enemy_name)}. Combat will move to a timeline from here.</p>
      <button onclick="startFight()">Fight</button>
    </div>`;
    return;
  }
  const floors = [0, 1, 2, 3];
  map.innerHTML = floors.map(floor => {
    const nodes = (state.map || []).filter(n => n.floor === floor);
    return `<div class="floor">${nodes.map(node => nodeHtml(node, state)).join("")}</div>`;
  }).join("");
}

function nodeHtml(node, state) {
  const available = (state.available_nodes || []).includes(node.id);
  const classes = ["node", `kind-${node.kind}`, available ? "available" : "", node.completed ? "completed" : ""].join(" ");
  const disabled = available ? "" : "disabled";
  return `<div class="${classes}">
    <div class="icon">${nodeIcons[node.kind]}</div>
    <div class="label">${nodeLabels[node.kind]}</div>
    <button ${disabled} onclick="chooseNode(${node.id})">Choose ${nodeLabels[node.kind]}</button>
  </div>`;
}

function renderRewardScene(state) {
  const reward = state.last_reward;
  const fight = state.last_fight;
  return `<div class="node-scene">
    <div class="reward-scene">
      <div class="combat-title">
        <div>
          <h2>${fight && fight.won ? "Victory Spoils" : "Encounter Reward"}</h2>
          <p>${fight ? `Defeated ${escapeHtml(fight.enemy)} in ${fight.turns}s.` : "Resolve the reward before choosing the next route."}</p>
        </div>
        <span class="pill">Reward</span>
      </div>
      <div class="loot-grid">
        <div class="loot-token"><span class="sub">Gold</span><strong>+${reward.gold}</strong></div>
        <div class="loot-token"><span class="sub">XP</span><strong>+${reward.xp}</strong></div>
        <div class="loot-token"><span class="sub">Level</span><strong>${reward.level_gained ? "Gained" : "Held"}</strong></div>
      </div>
      <div class="reward">${rewardDetails(reward)}</div>
      <div class="combat-controls"><button onclick="claimReward()">Continue Route</button></div>
    </div>
  </div>`;
}

function phaseLabel(phase) {
  return String(phase || "No run").replace(/_/g, " ");
}
