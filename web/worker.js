import init, { run_web_job, merge_web_bulk_shards } from './pkg/sim_gui.js';
import { bulkWorkerCount, bulkRanges } from './worker-pool.js';

const ready = init();
const pool = [];

function clearPool() {
  for (const worker of pool) {
    worker.onmessage = null;
    worker.onerror = null;
    worker.terminate();
  }
  pool.length = 0;
}

async function preparePool(count) {
  try {
    while (pool.length < count) {
      const worker = new Worker('./bulk-worker.js', { type: 'module' });
      worker.ready = new Promise((resolve, reject) => {
        worker.onmessage = ({ data }) => data.ready ? resolve() : reject(new Error(data.Err));
        worker.onerror = event => reject(new Error(event.message));
      });
      pool.push(worker);
    }
    await Promise.all(pool.slice(0, count).map(worker => worker.ready));
    return true;
  } catch (_) {
    // Browsers without nested-worker support still calculate on this worker.
    clearPool();
    return false;
  }
}

async function runBulk(request, runs, count) {
  if (!await preparePool(count)) return run_web_job(request);
  const start = performance.now();
  const progress = new Array(count).fill(0);
  const ranges = bulkRanges(runs, count);
  const shards = await Promise.all(ranges.map(({ firstRun, runs: chunkRuns }, index) => {
    const worker = pool[index];
    return new Promise((resolve, reject) => {
      worker.onerror = event => reject(new Error(event.message));
      worker.onmessage = ({ data }) => {
        const value = typeof data === 'string' ? JSON.parse(data) : data;
        if (value.progress) {
          progress[index] = Math.min(chunkRuns, value.progress[0]);
          self.postMessage(JSON.stringify({ progress: [progress.reduce((a, b) => a + b, 0), runs] }));
        } else if (value.Err) {
          reject(new Error(value.Err));
        } else {
          progress[index] = chunkRuns;
          resolve(value.Ok);
        }
      };
      // Preserve the original u64 seed instead of re-encoding it as a JS number.
      worker.postMessage({ request, firstRun, runs: chunkRuns });
    });
  }));
  return merge_web_bulk_shards(request, JSON.stringify(shards), performance.now() - start);
}

// Install before initialization finishes so requests arriving early are retained.
self.onmessage = async ({ data: request }) => {
  try {
    await ready;
    const runs = JSON.parse(request).kind?.Bulk?.runs;
    const count = bulkWorkerCount(runs, self.navigator.hardwareConcurrency);
    const result = count > 1 ? await runBulk(request, runs, count) : run_web_job(request);
    self.postMessage(result);
  } catch (error) {
    clearPool();
    self.postMessage(JSON.stringify({ Err: `Calculation failed: ${String(error)}` }));
  }
};
