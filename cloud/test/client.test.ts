import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { JSDOM } from 'jsdom';
import { DraftStore } from '../../web/cloud-drafts.js';

const source = readFileSync(new URL('../../web/cloud.js', import.meta.url), 'utf8').replace("import { DraftStore } from './cloud-drafts.js';", '');
const tick = () => new Promise(resolve => setImmediate(resolve));
async function fixture() {
  const dom = new JSDOM('<html><head><meta name="hackmaster-cloud" content="character-test"></head><body></body></html>', {
    url: 'https://characters-test.sim-gui.com', runScripts: 'outside-only',
  });
  const w = dom.window;
  Object.assign(w, { DraftStore, structuredClone, AbortSignal, Response });
  w.HTMLDialogElement.prototype.showModal = function () { this.open = true; };
  w.HTMLDialogElement.prototype.close = function () { this.open = false; };
  let row = { id: 'character', name: 'Original', version: 1, document: { schema_version: 1, player: { name: 'Original' } } };
  let failure = 0;
  let loseResponse = false;
  let writes = 0;
  const mutations = new Map();
  w.fetch = async (url, options) => {
    const path = String(url).replace('/api/', '');
    if (path === 'session') return Response.json({ user: { id: 'user', email: 'test@example.com', admin: true } });
    if (path === 'users') return Response.json([]);
    if (path.endsWith('/assignments') || path.endsWith('/revisions')) return Response.json([]);
    if (path === 'characters' && options.method === 'GET') return Response.json([row]);
    if (path === 'characters/character' && options.method === 'GET') return Response.json(row);
    if (options.method === 'PUT' || options.method === 'POST') {
      if (failure === -1) throw new TypeError('Network unavailable');
      if (failure) return Response.json({ error: failure === 409 ? 'Conflict: draft preserved' : 'Session expired' }, { status: failure });
      const body = JSON.parse(options.body);
      if (mutations.has(body.mutation_id)) return Response.json(mutations.get(body.mutation_id));
      row = { ...row, document: body.document, name: body.document.player.name, version: row.version + 1 };
      mutations.set(body.mutation_id, structuredClone(row)); writes++;
      if (loseResponse) throw new TypeError('Response lost after commit');
      return Response.json(row);
    }
    throw new Error(`Unexpected request ${options.method} ${url}`);
  };
  w.eval(source);
  await tick();
  const bridge = w.hackmasterCloud;
  bridge.publish(0, JSON.stringify(row.document));
  const click = async (label, index = 0) => {
    const buttons = [...w.document.querySelectorAll('button')].filter(b => b.textContent === label);
    assert.ok(buttons[index], label);
    assert.equal(buttons[index].disabled, false, label);
    buttons[index].click(); await tick();
  };
  const drafts = () => new DraftStore(w.localStorage, 'user').list('character');
  const load = async () => {
    await click('Load selected');
    const loads = JSON.parse(bridge.takeLoads()); assert.equal(loads.length, 1);
    bridge.loaded(0, ''); bridge.publish(0, JSON.stringify(loads[0].document));
  };
  return { w, bridge, click, load, drafts, failure: n => failure = n, lose: value => loseResponse = value, writes: () => writes };
}
test('client retains drafts on network/auth/conflict failure, then confirms committed save', async () => {
  const f = await fixture(); await f.load();
  f.bridge.publish(0, JSON.stringify({ schema_version: 1, player: { name: 'Edited' } }));
  for (const failure of [-1, 401, 409, 500]) {
    f.failure(failure); await f.click('Save online');
    assert.equal(f.drafts()[0].document.player.name, 'Edited');
    assert.ok(!f.w.document.querySelector('[role=status]').textContent.includes('saved online'));
  }
  f.failure(0); await f.click('Save online');
  assert.ok(f.w.document.querySelector('[role=status]').textContent.includes('saved online (revision 2)'));
  assert.equal(f.drafts().length, 0);
  f.w.close();
});
test('client retries a lost acknowledgement without a duplicate write', async () => {
  const f = await fixture(); await f.load();
  f.bridge.publish(0, JSON.stringify({ schema_version: 1, player: { name: 'Retry' } }));
  f.lose(true); await f.click('Save online');
  const mutation = f.drafts()[0].mutation_id;
  f.lose(false); await f.click('Save online');
  assert.equal(f.writes(), 1); assert.ok(mutation); assert.equal(f.drafts().length, 0);
  f.w.close();
});
test('initial publication never overwrites an existing creation draft', async () => {
  const f = await fixture();
  const store = new DraftStore(f.w.localStorage, 'user');
  const saved = store.save(null, null, 0, { schema_version: 1, player: { name: 'Previous session' } });
  f.bridge.publish(1, JSON.stringify({ schema_version: 1, player: { name: 'Default' } }));
  assert.deepEqual(store.list(null), [saved]);
  f.w.close();
});

test('storage failure prevents a save and gives an export recovery instruction', async () => {
  const f = await fixture(); await f.load();
  f.w.Storage.prototype.setItem = () => { throw new Error('Quota exceeded'); };
  f.bridge.publish(0, JSON.stringify({ schema_version: 1, player: { name: 'Cannot store' } }));
  await f.click('Save online');
  assert.equal(f.writes(), 0);
  assert.match(f.w.document.querySelector('[role=status]').textContent, /Export your character/);
  f.w.close();
});
