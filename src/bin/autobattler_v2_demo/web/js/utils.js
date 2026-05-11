export function escapeHtml(value) {
  return String(value).replace(/[&<>"']/g, c => ({
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    '"': "&quot;",
    "'": "&#39;",
  }[c]));
}

export function escapeJs(value) {
  return String(value).replace(/\\/g, "\\\\").replace(/'/g, "\\'");
}

export function clamp(value, min, max) {
  return Math.max(min, Math.min(max, Number.isFinite(value) ? value : min));
}

export function initials(value) {
  return String(value || "?")
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map(part => part[0].toUpperCase())
    .join("") || "?";
}

export function formatSeconds(value) {
  const number = Number(value || 0);
  return Number.isInteger(number) ? String(number) : number.toFixed(1);
}

export function formatFeet(value) {
  const number = Number(value || 0);
  return `${Number.isInteger(number) ? number : number.toFixed(1)}ft`;
}

export function rewardDetails(reward) {
  return `<div>Gold +${reward.gold}</div>
    <div>XP +${reward.xp}</div>
    <div>Items: ${reward.items.length ? reward.items.map(escapeHtml).join(", ") : "none"}</div>
    <div>${reward.level_gained ? "Level gained. Points granted." : "No level-up."}</div>`;
}

export function logLines(lines, emptyText) {
  const list = lines || [];
  if (!list.length) return `<div class="sub">${escapeHtml(emptyText)}</div>`;
  return list.map(line => `<div class="log-line">${escapeHtml(line)}</div>`).join("");
}
