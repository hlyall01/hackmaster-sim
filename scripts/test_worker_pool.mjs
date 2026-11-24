import test from 'node:test';
import assert from 'node:assert/strict';
import { bulkWorkerCount, bulkRanges } from '../web/worker-pool.js';

test('bulk workers leave a core free, cap memory use and keep small jobs serial', () => {
  assert.equal(bulkWorkerCount(1000, 32), 1);
  assert.equal(bulkWorkerCount(10000, 1), 1);
  assert.equal(bulkWorkerCount(10000, 2), 1);
  assert.equal(bulkWorkerCount(10000, 3), 2);
  assert.equal(bulkWorkerCount(10000, 32), 4);
  assert.equal(bulkWorkerCount(7500, 32), 3);
  assert.equal(bulkWorkerCount(undefined, 32), 1); // DPS and plots use the existing worker.
  assert.equal(bulkWorkerCount(10000, undefined), 1);
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
