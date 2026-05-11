import { escapeHtml } from "../utils.js";

export function renderHud(state) {
  document.getElementById("phase").textContent = state.terminal || phaseText(state.phase);
  document.getElementById("runStatus").textContent = state.has_run ? phaseLabel(state.phase) : "No run";
  const metrics = document.getElementById("hudMetrics");
  if (!state.player) {
    metrics.innerHTML = [
      hudCard("Level", "-"),
      hudCard("XP", "-"),
      hudCard("Gold", "-"),
      hudCard("Depth", "-"),
      hudCard("Wounds", "-"),
    ].join("");
    return;
  }
  const p = state.player;
  metrics.innerHTML = [
    hudCard("Level", p.level),
    hudCard("XP", `${p.xp}/${p.next_level_xp}`),
    hudCard("Gold", p.gold),
    hudCard("Depth", p.depth),
    hudCard("Wounds", p.wound_total || "none"),
  ].join("");
}

function hudCard(label, value) {
  return `<div class="hud-card"><span>${escapeHtml(label)}</span><strong>${escapeHtml(value)}</strong></div>`;
}

function phaseText(phase) {
  if (phase === "choose_node") return "Choose a route node.";
  if (phase === "event_choice") return "Resolve the event choice.";
  if (phase === "fight_preview") return "Fight scene selected.";
  if (phase === "combat_playback") return "Combat is running second by second.";
  if (phase === "reward_review") return "Claim rewards and progression.";
  if (phase === "run_over") return "Run over.";
  return "Roll a character to begin.";
}

function phaseLabel(phase) {
  return String(phase || "No run").replace(/_/g, " ");
}
