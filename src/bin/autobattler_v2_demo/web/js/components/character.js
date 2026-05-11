import { clamp, escapeHtml } from "../utils.js";

export function renderPresets(state) {
  const select = document.getElementById("preset");
  const previous = select.value;
  select.innerHTML = (state.presets || [])
    .map(name => `<option value="${escapeHtml(name)}">${escapeHtml(name)}</option>`)
    .join("");
  if (previous) select.value = previous;
}

export function renderCharacter(state) {
  const el = document.getElementById("character");
  if (!state.player) {
    el.innerHTML = `<div class="sub">No character rolled.</div>`;
    return;
  }
  const p = state.player;
  const xpPct = clamp((p.xp / Math.max(1, p.next_level_xp)) * 100, 0, 100);
  el.innerHTML = `
    <div class="stack">
      <div class="sheet-name">
        <div class="row"><h2>${escapeHtml(p.name)}</h2><span class="pill">Level ${p.level}</span></div>
        <div class="xpbar" style="--xp:${xpPct}%"><span></span></div>
        <div class="sub">XP ${p.xp} / ${p.next_level_xp}</div>
      </div>
      ${metricRow("Gold", p.gold)}
      ${metricRow("Depth", p.depth)}
      ${metricRow("Wounds", `<strong class="${p.wound_total ? "danger" : "ok"}">${p.wound_total || "none"}</strong>`)}
      ${metricRow("Seed", p.seed)}
      <div class="stat-grid">${p.stats.map(s => `<div class="stat">${escapeHtml(s)}</div>`).join("")}</div>
      <div class="sub">Points: BP ${p.bp}, LP ${p.lp}, AP ${p.ap}, RP ${p.rp}</div>
    </div>`;
}

function metricRow(label, value) {
  const rendered = typeof value === "string" && value.includes("<strong") ? value : `<strong>${escapeHtml(value)}</strong>`;
  return `<div class="metric"><span>${escapeHtml(label)}</span>${rendered}</div>`;
}
