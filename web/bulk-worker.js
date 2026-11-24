import init, { run_web_bulk_shard } from './pkg/sim_gui.js';

let ready;
function initialize(module) {
  if (!ready) {
    ready = module ? init({ module_or_path: module }) : init();
    ready.then(() => self.postMessage({ ready: true })).catch(error => {
      self.postMessage({ Err: `Calculation failed: ${String(error)}` });
    });
  }
  return ready;
}
// Compatibility with a coordinator from an older, already-open page.
setTimeout(() => { if (!ready) initialize(); }, 25);
self.onmessage = async ({ data }) => {
  try {
    if (!data.request) { await initialize(data.module); return; }
    await initialize();
    self.postMessage(run_web_bulk_shard(data.request, data.firstRun, data.runs));
  } catch (error) {
    self.postMessage(JSON.stringify({ Err: `Calculation failed: ${String(error)}` }));
  }
};
