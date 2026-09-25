import init, { run_web_job } from './pkg/sim_gui.js';

// Install the handler before awaiting initialization so early requests are retained.
const ready = init();
self.onmessage = async ({ data }) => {
  try {
    await ready;
    self.postMessage(run_web_job(data));
  } catch (error) {
    self.postMessage(JSON.stringify({ Err: `Calculation failed: ${String(error)}` }));
  }
};
