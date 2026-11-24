import init, { run_web_bulk_shard } from './pkg/sim_gui.js';

const ready = init();
ready.then(() => self.postMessage({ ready: true })).catch(error => {
  self.postMessage({ Err: `Calculation failed: ${String(error)}` });
});
self.onmessage = async ({ data }) => {
  try {
    await ready;
    self.postMessage(run_web_bulk_shard(data.request, data.firstRun, data.runs));
  } catch (error) {
    self.postMessage(JSON.stringify({ Err: `Calculation failed: ${String(error)}` }));
  }
};
