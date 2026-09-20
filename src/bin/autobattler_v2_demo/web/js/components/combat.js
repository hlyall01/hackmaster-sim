import { clamp, escapeHtml, formatFeet, formatSeconds, initials } from "../utils.js";

export function renderLiveFightScene(fight) {
  const player = fight.combatants.find(c => c.team_id === 0) || fight.combatants[0];
  const enemy = fight.combatants.find(c => c.team_id === 1) || fight.combatants[1];
  const distance = Number(fight.distance_ft || 0).toFixed(1);
  const enemyPosition = arenaEnemyPosition(fight.distance_ft);
  return `<div class="node-scene fight-scene">
    <div class="combat-scene">
      <div class="combat-title">
        <div>
          <h2>${escapeHtml(fight.tier)} Fight: ${escapeHtml(fight.enemy_name)}</h2>
          <p>${fight.elapsed_seconds}s elapsed of ${fight.max_seconds}s. Range ${distance} ft. Status: ${escapeHtml(fight.status)}.</p>
        </div>
        <span class="pill">${fight.running ? "Auto" : "Paused"}</span>
      </div>
      <div class="arena-track">
        <div class="fighter-token" style="left: 18%">${initials(player && player.name)}</div>
        <div class="fighter-token enemy" style="left: ${enemyPosition}%">${initials(enemy && enemy.name)}</div>
      </div>
      ${swingTimeline(fight)}
      <div class="combat-grid">
        ${combatantCard(player, false)}
        ${combatantCard(enemy, true)}
      </div>
      <div class="combat-controls">
        <button onclick="fightCommand('step', 1)">Step 1s</button>
        <button onclick="fightCommand('next_attack', 1)">Skip to Next Attack</button>
        <button onclick="fightCommand('play', 1)" ${fight.running ? "disabled" : ""}>Auto</button>
        <button onclick="fightCommand('pause', 1)" ${fight.running ? "" : "disabled"}>Pause</button>
        <button onclick="fightCommand('skip', 1)">Finish</button>
      </div>
      ${decisionHtml(fight.pending_decision)}
      <div class="log combat-log">${(fight.log_tail || []).map(escapeHtml).join("<br>") || "Combat is about to begin."}</div>
    </div>
  </div>`;
}

export function swingTimeline(fight) {
  const rows = (fight.combatants || []).map(combatant => {
    const speed = Math.max(1, Number(combatant.weapon_speed_seconds || 1));
    const next = combatant.next_attack_in_seconds;
    const queued = next !== null && next !== undefined;
    const remaining = queued ? Math.max(0, Number(next || 0)) : null;
    const progress = queued ? clamp(((speed - remaining) / speed) * 100, 0, 100) : 0;
    const ready = queued && remaining <= 0.05;
    const timeLabel = queued ? (ready ? "ready" : `${formatSeconds(remaining)}s`) : "closing";
    const side = combatant.team_id === 1 ? "enemy" : "player";
    return `<div class="timeline-row ${side}">
      <div class="timeline-name">
        <strong>${escapeHtml(combatant.name)}</strong>
        <span>${escapeHtml(combatant.weapon)} / ${formatSeconds(speed)}s</span>
      </div>
      <div class="timeline-track" style="--progress:${progress}%">
        <span class="timeline-fill"></span>
        <span class="timeline-tick"></span>
      </div>
      <div class="timeline-time ${ready ? "ready" : queued ? "" : "waiting"}">${escapeHtml(timeLabel)}</div>
    </div>`;
  }).join("");
  return `<div class="timeline">
    <div class="timeline-header"><strong>Swing Timeline</strong><span>${fight.elapsed_seconds}s</span></div>
    ${rows}
  </div>`;
}

function combatantCard(combatant, enemy) {
  if (!combatant) return `<div class="combatant"><div class="sub">No combatant.</div></div>`;
  const hp = `${combatant.hp} / ${combatant.max_hp}`;
  const hpPct = clamp((combatant.hp / Math.max(1, combatant.max_hp)) * 100, 0, 100);
  const tags = [
    combatant.weapon,
    `${formatSeconds(combatant.weapon_speed_seconds)}s speed`,
    `${formatFeet(combatant.reach_ft)} reach`,
    combatant.next_attack_in_seconds === null || combatant.next_attack_in_seconds === undefined
      ? null
      : `next ${formatSeconds(combatant.next_attack_in_seconds)}s`,
    shieldLabel(combatant),
    combatant.trauma_seconds > 0 ? `${combatant.trauma_seconds}s trauma` : null,
    combatant.knocked_seconds > 0 ? `${combatant.knocked_seconds}s knocked` : null,
  ].filter(Boolean);
  return `<div class="combatant ${enemy ? "enemy" : ""}">
    <div class="row"><h3>${escapeHtml(combatant.name)}</h3><strong>${hp}</strong></div>
    <div class="hpbar" style="--hp:${hpPct}%"><span></span></div>
    <div class="sub">${tags.map(escapeHtml).join(" | ")}</div>
  </div>`;
}

function shieldLabel(combatant) {
  if (!combatant || !combatant.shield_name) return null;
  return combatant.shield_intact ? `${combatant.shield_name} ready` : `${combatant.shield_name} broken`;
}

function decisionHtml(decision) {
  if (!decision) return `<div class="decision-slot">No tactical prompt this second.</div>`;
  return `<div class="decision-slot">
    <strong>Decision for actor ${decision.actor_idx}</strong>
    <div class="combat-controls">${decision.options.map(option => `<button>${escapeHtml(option)}</button>`).join("")}</div>
  </div>`;
}

function arenaEnemyPosition(distanceFt) {
  const playerPosition = 18;
  const contactPosition = playerPosition + 7;
  const farPosition = 82;
  const maxVisualRange = 20;
  const meleeRange = 1;
  const distance = clamp(Number(distanceFt || 0), 0, maxVisualRange);
  if (distance <= meleeRange) return contactPosition;
  return contactPosition + ((distance - meleeRange) / (maxVisualRange - meleeRange)) * (farPosition - contactPosition);
}
