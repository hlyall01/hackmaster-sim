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
  renderGrid(state.grid);
  document.getElementById("playerSquad").innerHTML = renderMembers(state.squad.active, "No active squad.");
  document.getElementById("benchSquad").innerHTML = renderMembers(state.squad.bench, "No bench.");
  document.getElementById("log").textContent = (state.log || []).join("\n");
}

function renderMembers(members, empty) {
  if (!members.length) return `<div class="muted">${empty}</div>`;
  return members.map(member => `<div class="member">
    <strong><span>${escapeHtml(member.name)}</span><span>${member.hp}/${member.max_hp}</span></strong>
    <div class="detail">Lv ${member.level} · ${escapeHtml(member.weapon)} · ${escapeHtml(member.status)}</div>
    <div class="detail">${(member.stats || []).map(escapeHtml).join(" · ")}</div>
  </div>`).join("");
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

requestState()
  .then(render)
  .catch((err) => {
    document.getElementById("log").textContent = err.message;
  });
