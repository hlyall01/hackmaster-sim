async function requestState() {
  const response = await fetch("/api/state");
  if (!response.ok) throw new Error(await response.text());
  return response.json();
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
    <span>Active max ${state.max_active}</span>
    <span>Bench max ${state.max_bench}</span>
    <span>${state.grid.tile_size_ft} ft squares</span>
  `;
  renderGrid(state.grid);
  document.getElementById("log").textContent = (state.log || []).join("\n");
}

requestState()
  .then(render)
  .catch((err) => {
    document.getElementById("log").textContent = err.message;
  });
