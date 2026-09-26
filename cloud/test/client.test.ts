import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { JSDOM } from 'jsdom';
import { DraftStore } from '../../web/cloud-drafts.js';

const source = readFileSync(new URL('../../web/cloud.js', import.meta.url), 'utf8').replace("import { DraftStore } from './cloud-drafts.js';", '');
const tick = () => new Promise(resolve => setImmediate(resolve));
async function fixture(admin = true, owner = true, signedIn = true, initiallyDeleted = false) {
  const dom = new JSDOM('<html><head><meta name="hackmaster-cloud" content="character-test"></head><body></body></html>', {
    url: 'https://characters-test.sim-gui.com', runScripts: 'outside-only',
  });
  const w = dom.window;
  Object.assign(w, { DraftStore, structuredClone, AbortSignal, Response });
  w.HTMLDialogElement.prototype.showModal = function () { this.open = true; };
  w.HTMLDialogElement.prototype.close = function () { this.open = false; };
  let row = { id: 'character', name: 'Original', version: 1, document: { schema_version: 1, player: { name: 'Original' } } };
  let failure = 0;
  let sessionId = 'user';
  let assigned = false;
  let loseResponse = false;
  let writes = 0;
  let deleted = initiallyDeleted;
  let deletes = 0;
  w.confirm = () => true;
  const mutations = new Map();
  const assignments = [];
  w.fetch = async (url, options) => {
    if (String(url).startsWith('/guest/characters')) return Response.json(String(url) === '/guest/characters' ? [{...row,is_owner:0,can_edit:0}] : {...row,is_owner:0,can_edit:0});
    if (!signedIn) return Response.json({error:'Sign in'}, {status:401});
    const path = String(url).replace('/api/', '');
    if (path === 'session') return Response.json({ user: { id: sessionId, email: 'test@example.com', admin } });
    if (path === 'roster') return Response.json(deleted ? [] : [{...row,is_owner:Number(owner),is_assigned:Number(assigned),can_edit:Number(owner || assigned)}]);
    if (path === 'roster/character') return Response.json({...row,is_owner:Number(owner),is_assigned:Number(assigned),can_edit:Number(owner || assigned)});
    if (path === 'users') return Response.json([{id:'friend',email:'friend@example.com'}]);
    if (path.endsWith('/assignments')) { if (options.method !== 'GET') assignments.push({url,method:options.method,...JSON.parse(options.body)}); return Response.json([]); }
    if (path.endsWith('/revisions')) return Response.json([]);
    if (path === 'characters' && options.method === 'GET') return Response.json(deleted ? (admin ? [{...row,deleted:1}] : []) : [row]);
    if (path === 'characters/character' && options.method === 'GET') return Response.json(row);
    if (path === 'characters/character' && options.method === 'DELETE') {
      if (failure === -1) throw new TypeError('Network unavailable');
      if (failure) return Response.json({error:'Deletion failed'}, {status:failure});
      assert.equal(JSON.parse(options.body).version,row.version);
      deletes++; deleted = true; return Response.json({deleted:true,id:row.id});
    }
    if (path === 'characters/character/restore' && options.method === 'POST') {
      deleted = false; row.version++; return Response.json(row);
    }
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
    bridge.action(0,'load','character'); await tick();
    const loads = JSON.parse(bridge.takeLoads()); assert.equal(loads.length, 1);
    bridge.loaded(0, ''); bridge.publish(0, JSON.stringify(loads[0].document));
  };
  return { w, bridge, click, load, drafts, assignments, assign: value => assigned = value, account: id => sessionId = id, deletes: () => deletes, failure: n => failure = n, lose: value => loseResponse = value, writes: () => writes };
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

test('admin assignment is separate and names both the character and player', async () => {
  const f = await fixture();
  assert.equal(f.w.document.querySelector('[aria-label="Player to assign"]'), null);
  await f.click('Admin · Assign characters');
  assert.ok(f.w.document.querySelector('[aria-label="Character to manage"]'));
  assert.ok(f.w.document.querySelector('[aria-label="Player to assign"]'));
  assert.ok(![...f.w.document.querySelectorAll('button')].some(b => b.textContent === 'Load selected'));
  await f.click('Assign character');
  assert.deepEqual(f.assignments, [{url:'/api/characters/character/assignments',method:'PUT',user_id:'friend'}]);
  assert.match(f.w.document.querySelector('[role=status]').textContent, /Original assigned to friend@example.com/);
  f.w.close();
});

test('player creation is available with recoverable creation drafts and no admin controls', async () => {
  const f = await fixture(false);
  assert.equal(f.w.document.querySelector('button:nth-child(2)').hidden, true);
  f.bridge.action(0,'create',''); await tick();
  assert.equal(f.writes(), 1);
  const snapshot = JSON.parse(f.bridge.snapshot(0));
  assert.equal(snapshot.admin, false);
  assert.equal(snapshot.characters[0].is_owner, true);
  assert.equal(snapshot.can_save, true);
  f.w.close();
});

test('Core party load is a read-only simulation copy until explicitly created as own', async () => {
  const f = await fixture(false, false);
  f.bridge.action(0, 'load', 'character'); await tick();
  const loads = JSON.parse(f.bridge.takeLoads());
  assert.equal(loads.length, 1);
  f.bridge.loaded(0, ''); f.bridge.publish(0, JSON.stringify(loads[0].document));
  assert.equal(JSON.parse(f.bridge.snapshot(0)).can_save, false);
  f.bridge.action(0, 'save', ''); await tick();
  assert.equal(f.writes(), 0);
  f.bridge.action(0, 'create', ''); await tick();
  assert.equal(f.writes(), 1);
  f.w.close();
});

test('continue without logging in loads the whole party roster with no saves or admin controls', async () => {
  const f = await fixture(false,false,false);
  assert.equal(f.w.document.querySelector('dialog').open,true);
  await f.click('Continue without logging in');
  assert.equal(f.w.document.querySelector('dialog').open,false);
  const state = JSON.parse(f.bridge.snapshot(0));
  assert.equal(state.guest,true); assert.equal(state.signed_in,false);
  assert.equal(state.characters.length,1); assert.equal(state.characters[0].is_owner,false);
  f.bridge.action(0,'load','character'); await tick();
  const loads = JSON.parse(f.bridge.takeLoads()); assert.equal(loads.length,1);
  f.bridge.loaded(0,''); f.bridge.publish(0,JSON.stringify(loads[0].document));
  assert.equal(JSON.parse(f.bridge.snapshot(0)).can_save,false);
  for (const action of ['save','create','admin']) { f.bridge.action(0,action,''); await tick(); }
  assert.equal(f.writes(),0); assert.equal(f.w.localStorage.length,0);
  assert.ok(f.w.location.search.includes('guest=1'));
  f.w.close();
});

test('owner deletion confirms, retains drafts on failures, and detaches only after commit', async () => {
  const f = await fixture(false); await f.load();
  f.bridge.publish(0,JSON.stringify({schema_version:1,player:{name:'Unsaved edit'}}));
  assert.equal(JSON.parse(f.bridge.snapshot(0)).can_delete,true);
  f.w.confirm = () => false;
  f.bridge.action(0,'delete',''); await tick();
  assert.equal(f.deletes(),0);
  f.w.confirm = () => true;
  for (const failure of [-1,401,409]) {
    f.failure(failure);
    f.bridge.action(0,'delete',''); await tick();
    assert.equal(f.deletes(),0);
    assert.equal(JSON.parse(f.bridge.snapshot(0)).loaded_id,'character');
    assert.equal(f.drafts()[0].document.player.name,'Unsaved edit');
  }
  f.failure(0);
  f.bridge.action(0,'delete',''); await tick();
  assert.equal(f.deletes(),1);
  const state=JSON.parse(f.bridge.snapshot(0));
  assert.equal(state.characters.length,0);
  assert.equal(state.loaded_id,''); assert.equal(state.can_save,false); assert.equal(state.can_delete,false);
  assert.equal(f.drafts()[0].document.player.name,'Unsaved edit');
  f.w.close();
});
test('party members cannot be deleted through UI or bridge', async () => {
  const f=await fixture(false,false); await f.load();
  assert.equal(JSON.parse(f.bridge.snapshot(0)).can_delete,false);
  assert.ok(![...f.w.document.querySelectorAll('button')].some(b=>b.textContent==='Delete my character'));
  f.bridge.action(0,'delete',''); await tick();
  assert.equal(f.deletes(),0);
  f.w.close();
});

test('deleted characters have a separate recovery section and no assignment controls until restored', async () => {
  const f = await fixture(true,true,true,true);
  await f.click('Admin · Assign characters');
  assert.equal(f.w.document.querySelector('[aria-label="Character to manage"]').options.length,0);
  assert.equal(f.w.document.querySelector('[aria-label="Player to assign"]'),null);
  assert.equal(f.w.document.querySelector('[aria-label="Deleted character to restore"]').options[0].textContent,'Original');
  assert.ok(![...f.w.document.querySelectorAll('button')].some(b=>b.textContent==='Assign character'));
  await f.click('Restore deleted character');
  assert.equal(f.w.document.querySelector('[aria-label="Deleted character to restore"]'),null);
  assert.equal(f.w.document.querySelector('[aria-label="Character to manage"]').options[0].textContent,'Original');
  assert.ok(f.w.document.querySelector('[aria-label="Player to assign"]'));
  assert.match(f.w.document.querySelector('[role=status]').textContent,/restored/);
  f.w.close();
});

test('expired creation exposes new-tab sign-in and retries the preserved draft after authentication', async () => {
  const f = await fixture(false);
  f.failure(401);
  f.bridge.action(0,'create',''); await tick();
  assert.equal(f.writes(),0);
  assert.equal(JSON.parse(f.bridge.snapshot(0)).login_expired,true);
  assert.equal(f.w.document.querySelector('dialog').open,true);
  const link = f.w.document.querySelector('a[href="/api/login"]');
  assert.equal(link.target,'_blank'); assert.equal(link.rel,'noopener');
  const store = new DraftStore(f.w.localStorage,'user');
  const draft = store.list(null)[0];
  assert.equal(draft.document.player.name,'Original');
  f.failure(0);
  await f.click('Check sign-in');
  assert.equal(JSON.parse(f.bridge.snapshot(0)).login_expired,false);
  f.bridge.action(0,'create',''); await tick();
  assert.equal(f.writes(),1); assert.equal(store.list(null).length,0);
  assert.match(JSON.parse(f.bridge.snapshot(0)).status,/saved online/);
  f.w.close();
});
test('signing into a different account cannot submit the previous account draft', async () => {
  const f=await fixture(false);
  f.account('different-user');
  f.bridge.action(0,'create',''); await tick();
  assert.equal(f.writes(),0);
  assert.equal(new DraftStore(f.w.localStorage,'user').list(null).length,1);
  assert.match(JSON.parse(f.bridge.snapshot(0)).status,/different account/);
  f.w.close();
});

test('non-admin toolbar only offers sign out and assigned characters move into My Characters on focus', async () => {
  const f=await fixture(false,false);
  assert.deepEqual([...f.w.document.querySelector('body > div').querySelectorAll('button')].filter(b=>!b.hidden).map(b=>b.textContent),['Sign out']);
  assert.equal(f.w.document.querySelector('[aria-label="Assigned character"]'),null);
  assert.equal(JSON.parse(f.bridge.snapshot(0)).characters[0].is_mine,false);
  f.assign(true); f.w.dispatchEvent(new f.w.Event('focus')); await tick();
  assert.equal(JSON.parse(f.bridge.snapshot(0)).characters[0].is_mine,true);
  await f.load();
  assert.equal(JSON.parse(f.bridge.snapshot(0)).can_save,true);
  assert.equal(JSON.parse(f.bridge.snapshot(0)).can_delete,true);
  f.assign(false); f.w.dispatchEvent(new f.w.Event('focus')); await tick();
  assert.equal(JSON.parse(f.bridge.snapshot(0)).characters[0].is_mine,false);
  assert.equal(JSON.parse(f.bridge.snapshot(0)).can_save,false);
  f.w.close();
});

test('assigned characters have the same delete controls and behavior as created characters', async () => {
  const f=await fixture(false,false);
  f.assign(true); f.w.dispatchEvent(new f.w.Event('focus')); await tick(); await f.load();
  assert.equal(JSON.parse(f.bridge.snapshot(0)).can_delete,true);
  f.bridge.action(0,'delete',''); await tick();
  assert.equal(f.deletes(),1);
  assert.equal(JSON.parse(f.bridge.snapshot(0)).characters.length,0);
  f.w.close();
});
