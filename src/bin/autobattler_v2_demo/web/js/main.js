import { requestJson } from "./api.js";
import { renderCharacter, renderPresets } from "./components/character.js";
import { renderHud } from "./components/hud.js";
import { renderMap } from "./components/map.js";
import { renderEncounter, renderFight, renderInventory, renderLog, renderReward } from "./components/panels.js";

let state = null;
let autoTimer = null;
let autoBusy = false;

async function api(path, body) {
  state = await requestJson(path, body);
  render();
}

async function loadState() { await api("/api/state"); }

async function newRun() {
  const seedRaw = document.getElementById("seed").value.trim();
  await api("/api/new-run", {
    preset: document.getElementById("preset").value,
    name: document.getElementById("name").value.trim() || null,
    seed: seedRaw ? Number(seedRaw) : null,
  });
}

async function chooseNode(id) { await api("/api/choose-node", { node_id: id }); }
async function eventChoice(id) { await api("/api/event-choice", { choice_id: id }); }
async function startFight() { await api("/api/start-fight", {}); }
async function claimReward() { await api("/api/claim-reward", {}); }
async function fightCommand(command, seconds = 1) { await api("/api/fight-command", { command, seconds }); }

async function autoTick() {
  if (autoBusy) return;
  autoBusy = true;
  try {
    await fightCommand("tick", 1);
  } catch (err) {
    stopAutoTimer();
    renderError(err);
  } finally {
    autoBusy = false;
  }
}

function syncAutoTimer() {
  const shouldRun = Boolean(state && state.live_fight && state.live_fight.running);
  if (shouldRun && !autoTimer) autoTimer = setInterval(autoTick, 1000);
  if (!shouldRun) stopAutoTimer();
}

function stopAutoTimer() {
  if (!autoTimer) return;
  clearInterval(autoTimer);
  autoTimer = null;
}

function render() {
  renderPresets(state);
  renderHud(state);
  renderCharacter(state);
  renderMap(state);
  renderEncounter(state);
  renderInventory(state);
  renderReward(state);
  renderFight(state);
  renderLog(state);
  syncAutoTimer();
}

function renderError(err) {
  const el = document.getElementById("log");
  if (el) el.textContent = err.message;
}

Object.assign(window, {
  newRun,
  chooseNode,
  eventChoice,
  startFight,
  claimReward,
  fightCommand,
});

loadState().catch(renderError);
