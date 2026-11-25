// A fixed budget keeps native and browser bulk simulations consistent.
export function bulkWorkerCount(runs) {
  return Math.max(1, Math.min(24, runs || 1));
}

// Work is allocated when a worker becomes available, never reserved for a worker
// that is still starting. Each global fight index is assigned exactly once.
export class BulkQueue {
  constructor(runs, workers) {
    this.runs = runs;
    this.workers = workers;
    this.chunkSize = Math.min(25000, Math.max(250, Math.ceil(runs / (workers * 2))));
    this.next = 0;
    this.completed = 0;
    this.active = new Map();
    this.results = [];
  }
  take(worker, maximum = this.chunkSize) {
    if (this.active.has(worker)) throw new Error('Worker already has a batch');
    if (this.next >= this.runs) return null;
    const remaining = this.runs - this.next;
    const tailSize = this.runs > 100000 ? Math.max(1000, Math.ceil(remaining / this.workers)) : this.chunkSize;
    const range = { firstRun: this.next, runs: Math.min(this.chunkSize, maximum, tailSize, remaining), progress: 0 };
    this.next += range.runs;
    this.active.set(worker, range);
    return range;
  }
  update(worker, completed) {
    const range = this.active.get(worker);
    if (range) range.progress = Math.max(range.progress, Math.min(range.runs, completed));
  }
  finish(worker, result) {
    const range = this.active.get(worker);
    if (!range) throw new Error('Unexpected batch result');
    this.completed += range.runs;
    this.active.delete(worker);
    this.results.push(result);
  }
  progress() {
    let value = this.completed;
    for (const range of this.active.values()) value += range.progress;
    return value;
  }
  get done() { return this.completed === this.runs; }
  get hasWork() { return this.next < this.runs; }
}

export function bulkRanges(runs, workers) {
  return Array.from({ length: workers }, (_, index) => {
    const firstRun = Math.floor(runs * index / workers);
    return { firstRun, runs: Math.floor(runs * (index + 1) / workers) - firstRun };
  });
}
