import init, { run_web_job, run_web_bulk_shard_quiet, merge_web_bulk_shards } from './pkg/sim_gui.js';
import { bulkWorkerCount, BulkQueue } from './worker-pool.js';

let ready;
const initialize = module => ready ??= module ? init({ module_or_path: module }) : init();
const pool = [];
let activeJob = null;
let nestedWorkersAvailable = true;
const localWorker = {};
// Yield to worker messages between local batches without nested timer clamping.
const localTasks = [];
const localChannel = new MessageChannel();
localChannel.port1.onmessage = () => localTasks.shift()?.();
function deferLocal(task) {
  localTasks.push(task);
  localChannel.port2.postMessage(0);
}

function terminate(entry) {
  entry.dead = true;
  entry.worker.onmessage = null;
  entry.worker.onerror = null;
  entry.worker.terminate();
}
function clearPool() {
  for (const entry of pool) terminate(entry);
  pool.length = 0;
}
function fail(job, error) {
  if (activeJob !== job) return;
  activeJob = null;
  clearPool();
  nestedWorkersAvailable = true;
  job.reject(error);
}
function report(job) {
  self.postMessage(JSON.stringify({ progress: [job.queue.progress(), job.runs] }));
}
function finish(job) {
  if (!job.queue.done) return false;
  try {
    const result = merge_web_bulk_shards(job.request, JSON.stringify(job.queue.results), performance.now() - job.start);
    activeJob = null;
    job.resolve(result);
  } catch (error) { fail(job, error); }
  return true;
}
function assign(entry) {
  const job = activeJob;
  if (!job || entry.dead || !entry.ready || entry.assignment) return;
  const range = job.queue.take(entry);
  if (!range) return;
  entry.assignment = job;
  entry.worker.postMessage({ request: job.request, firstRun: range.firstRun, runs: range.runs });
}
function workerError(entry, error) {
  const job = entry.assignment;
  if (job) { fail(job, error); return; }
  // An unavailable nested worker never blocks the coordinator's own calculation.
  terminate(entry);
  nestedWorkersAvailable = false;
  if (activeJob) scheduleLocal(activeJob);
}
function startWorkers(job) {
  if (activeJob !== job || !job.queue.hasWork || !nestedWorkersAvailable) return;
  while (pool.length < job.count && pool.filter(entry => !entry.dead && !entry.ready).length < 4) {
    try {
      const worker = new Worker('./bulk-worker.js', { type: 'module' });
      const entry = { worker, ready: false, dead: false, assignment: null };
      pool.push(entry);
      worker.onerror = event => {
        event.preventDefault();
        workerError(entry, new Error(event.message));
      };
      worker.onmessage = ({ data }) => {
        try {
          const value = typeof data === 'string' ? JSON.parse(data) : data;
          if (value.ready) {
            entry.ready = true;
            assign(entry);
            if (activeJob) startWorkers(activeJob);
          } else if (value.Err) {
            workerError(entry, new Error(value.Err));
          } else {
            const owner = entry.assignment;
            if (!owner || activeJob !== owner) return;
            if (value.progress) {
              owner.queue.update(entry, value.progress[0]);
              report(owner);
            } else {
              owner.queue.finish(entry, value.Ok);
              entry.assignment = null;
              report(owner);
              if (!finish(owner)) assign(entry);
            }
          }
        } catch (error) { workerError(entry, error); }
      };
      // Clone compiled code; each worker still has its own private WASM memory.
      try { worker.postMessage({ module: init.__wbindgen_wasm_module }); }
      catch (_) { worker.postMessage({ initialize: true }); }
    } catch (_) {
      nestedWorkersAvailable = false;
      return;
    }
  }
}
function scheduleLocal(job) {
  if (activeJob !== job || !job.queue.hasWork || job.localScheduled) return;
  job.localScheduled = true;
  deferLocal(() => {
    job.localScheduled = false;
    if (activeJob !== job) return;
    // Once the full pool is ready, keep the coordinator free for dispatch and merging.
    if (pool.filter(entry => !entry.dead && entry.ready).length >= job.count) return;
    const range = job.queue.take(localWorker, 250);
    if (!range) return;
    try {
      const result = JSON.parse(run_web_bulk_shard_quiet(job.request, range.firstRun, range.runs));
      if (result.Err) throw new Error(result.Err);
      job.queue.finish(localWorker, result.Ok);
      report(job);
      if (!finish(job)) {
        startWorkers(job);
        scheduleLocal(job);
      }
    } catch (error) { fail(job, error); }
  });
}
function runBulk(request, runs, count) {
  return new Promise((resolve, reject) => {
    const job = { request, runs, count, queue: new BulkQueue(runs, count), start: performance.now(), localScheduled: false, resolve, reject };
    activeJob = job;
    nestedWorkersAvailable = true;
    for (let index = pool.length - 1; index >= 0; index--) {
      if (pool[index].dead) pool.splice(index, 1);
    }
    // Lowering the slider releases surplus idle workers and their memory.
    while (pool.length > count) terminate(pool.pop());
    if (runs <= 100000 && pool.filter(entry => entry.ready).length === count) {
      job.queue.chunkSize = Math.ceil(runs / count);
    }
    for (const entry of pool) assign(entry);
    startWorkers(job);
    // Begin here immediately, even if no child has finished starting.
    scheduleLocal(job);
  });
}

self.onmessage = async ({ data: request }) => {
  if (typeof request !== 'string') {
    if (request.module instanceof WebAssembly.Module && WebAssembly.Module.exports(request.module)
      .some(value => value.name === 'run_web_bulk_shard_quiet')) {
      initialize(request.module).catch(() => { ready = undefined; });
    }
    return;
  }
  try {
    await initialize();
    const bulk = JSON.parse(request).kind?.Bulk;
    const count = bulkWorkerCount(bulk?.runs, self.navigator.hardwareConcurrency, bulk?.workers);
    if (bulk && count === 1) clearPool();
    const result = bulk && count > 1 ? await runBulk(request, bulk.runs, count) : run_web_job(request);
    self.postMessage(result);
  } catch (error) {
    clearPool();
    activeJob = null;
    self.postMessage(JSON.stringify({ Err: `Calculation failed: ${String(error)}` }));
  }
};
