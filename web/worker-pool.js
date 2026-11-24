// Leave one logical core for the UI. Cap memory use and keep small jobs serial.
export function bulkWorkerCount(runs, cores = 2) {
  return Math.max(1, Math.min(4, Math.max(1, (cores || 2) - 1), Math.floor((runs || 0) / 2500)));
}

export function bulkRanges(runs, workers) {
  return Array.from({ length: workers }, (_, index) => {
    const firstRun = Math.floor(runs * index / workers);
    return { firstRun, runs: Math.floor(runs * (index + 1) / workers) - firstRun };
  });
}
