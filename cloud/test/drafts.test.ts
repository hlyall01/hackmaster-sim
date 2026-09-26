import { test } from 'node:test';
import assert from 'node:assert/strict';
import { DraftStore } from '../../web/cloud-drafts.js';

function storage() {
  const data = new Map<string, string>();
  return { get length() { return data.size; }, key(i: number) { return [...data.keys()][i]; },
    getItem(k: string) { return data.get(k) ?? null; }, setItem(k: string, v: string) { data.set(k, v); }, removeItem(k: string) { data.delete(k); } };
}
test('drafts survive reloads, are isolated by user/tab/slot, and retain retry identities', () => {
  const disk = storage();
  const a = new DraftStore(disk, 'a');
  const one = a.save(null, 'character', 1, { name: 'First tab' });
  const two = a.save(null, 'character', 1, { name: 'Second tab' });
  assert.notEqual(one.id, two.id);
  assert.equal(new DraftStore(disk, 'a').list('character').length, 2);
  assert.deepEqual(new DraftStore(disk, 'b').list('character'), []);
  const retry = a.save(one, 'character', 1, { name: 'First tab' });
  assert.equal(retry.mutation_id, one.mutation_id);
  const edited = a.save(retry, 'character', 1, { name: 'Edited while saving' });
  assert.notEqual(edited.mutation_id, retry.mutation_id);
  a.acknowledge(retry);
  assert.equal(a.list('character').length, 2, 'old response cannot erase newer edits');
  a.acknowledge(edited);
  assert.deepEqual(a.list('character').map(d => d.document), [{ name: 'Second tab' }]);
});
test('a creation retry survives reload without duplicating the online character', () => {
  const disk = storage();
  const store = new DraftStore(disk, 'admin');
  const record = store.save(null, null, 0, { name: 'New character' });
  const recovered = new DraftStore(disk, 'admin').list(null)[0];
  const retry = store.save(recovered, null, 0, recovered.document);
  assert.equal(record.mutation_id, retry.mutation_id);
  store.acknowledge(retry);
  assert.deepEqual(store.list(null), []);
});
