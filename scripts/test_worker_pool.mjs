import test from 'node:test';
import assert from 'node:assert/strict';
import { bulkWorkerCount, bulkRanges, BulkQueue } from '../web/worker-pool.js';

test('bulk jobs use a fixed 24-worker budget regardless of old slider settings', () => {
  assert.equal(bulkWorkerCount(1000), 24);
  assert.equal(bulkWorkerCount(1000000), 24);
  assert.equal(bulkWorkerCount(10), 10);
  assert.equal(bulkWorkerCount(1), 1);
  assert.equal(bulkWorkerCount(0), 1);
  assert.equal(bulkWorkerCount(undefined), 1); // DPS and plots use the existing worker.
  assert.equal(bulkWorkerCount(10000, 1, 1), 24);
  assert.equal(bulkWorkerCount(10000, 32, 64), 24);
});

test('ready workers start without waiting and late workers take only unclaimed ranges', () => {
  const queue = new BulkQueue(10001, 64);
  const first = queue.take('coordinator');
  assert.equal(first.firstRun, 0);
  queue.update('coordinator', first.runs - 1);
  assert.equal(queue.progress(), first.runs - 1);
  queue.finish('coordinator', 'first result');
  const late = queue.take('late worker');
  assert.equal(late.firstRun, first.runs);
  assert.throws(() => queue.take('late worker'), /already has/);
  queue.update('late worker', late.runs);
  queue.update('late worker', 0); // Delayed progress cannot move the bar backwards.
  assert.equal(queue.progress(), first.runs + late.runs);
  queue.finish('late worker', 'second result');
  while (queue.hasWork) { queue.take('coordinator'); queue.finish('coordinator', 'rest'); }
  assert.equal(queue.done, true);
  assert.equal(queue.progress(), 10001);
  assert.equal(queue.take('very late worker'), null);
});

test('out-of-order completion covers every fight once across changing active worker counts', () => {
  for (const runs of [1, 31, 10001, 1000001, 0xffffffff]) {
    const queue = new BulkQueue(runs, 64);
    const assigned = [];
    let previousProgress = 0;
    let cycle = 0;
    while (!queue.done) {
      const ready = Math.min(64, 1 + cycle++);
      for (let worker = 0; worker < ready; worker++) {
        if (queue.active.has(worker)) continue;
        const range = queue.take(worker);
        if (range) assigned.push(range);
      }
      // Resolve in reverse order so allocation and completion orders differ.
      const worker = [...queue.active.keys()].at(-1);
      queue.update(worker, 1);
      assert.ok(queue.progress() >= previousProgress);
      queue.finish(worker, worker);
      previousProgress = queue.progress();
    }
    let expected = 0;
    for (const range of assigned) { assert.equal(range.firstRun, expected); expected += range.runs; }
    assert.equal(expected, runs);
    assert.equal(queue.progress(), runs);
  }
});

test('uneven bulk ranges cover every fight once without gaps or overlaps', () => {
  for (const runs of [0, 1, 31, 9999, 10000, 10001, 0xffffffff]) {
    for (const workers of [1, 2, 3, 4]) {
      let expected = 0;
      for (const range of bulkRanges(runs, workers)) {
        assert.equal(range.firstRun, expected);
        assert.ok(range.runs >= 0);
        expected += range.runs;
      }
      assert.equal(expected, runs);
    }
  }
});

test('large jobs shrink assignments toward the tail while respecting startup caps', () => {
  const queue = new BulkQueue(1000000, 24);
  assert.equal(queue.take('local', 250).runs, 250);
  queue.finish('local', null);
  const sizes = [];
  while (queue.hasWork) {
    sizes.push(queue.take('child').runs);
    queue.finish('child', null);
  }
  assert.equal(sizes[0], 20834);
  assert.ok(sizes.some(n => n < 20834 && n > 1000));
  assert.ok(sizes.includes(1000));
  assert.equal(queue.progress(), 1000000);
});
