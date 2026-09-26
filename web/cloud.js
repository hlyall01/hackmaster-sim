// Enabled only by the isolated character-test deployment's HTML marker.
import { DraftStore } from './cloud-drafts.js';
const enabled = document.querySelector('meta[name="hackmaster-cloud"][content="character-test"]');
if (enabled) installCloudCharacters();

function installCloudCharacters() {
  let user = null;
  let characters = [];
  let users = [];
  let selected = '';
  let assignments = [];
  let revisions = [];
  let busy = false;
  let status = 'Sign in to load your assigned characters.';
  const slots = [null, null];
  const current = [null, null];
  const baselines = [null, null];
  const drafts = [null, null];
  const creationDrafts = [null, null];
  let store = null;
  const loads = [];
  const pendingLoads = [null, null];
  const bar = document.createElement('div');
  Object.assign(bar.style, { position: 'fixed', right: '8px', bottom: '8px', zIndex: 100, padding: '4px', background: '#222', color: 'white' });
  const open = document.createElement('button');
  open.textContent = 'TEST · Cloud characters';
  bar.append(open);
  const dialog = document.createElement('dialog');
  Object.assign(dialog.style, { width: 'min(680px,90vw)', maxHeight: '85vh', overflow: 'auto', background: '#222', color: 'white' });
  document.body.append(bar, dialog);
  const encode = value => JSON.stringify(value);
  function stash(slot, doc) {
    const row = slots[slot];
    if (!store || !doc || encode(doc) === encode(row?.document || baselines[slot])) return;
    try { drafts[slot] = store.save(drafts[slot], row?.id || null, row?.version || 0, doc); }
    catch { status = 'Local draft storage is unavailable/full. Export your character before closing this page.'; }
  }
  function download(value, filename) {
    const url = URL.createObjectURL(new Blob([JSON.stringify(value, null, 2)], { type: 'application/json' }));
    const a = document.createElement('a'); a.href = url; a.download = filename; a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }
  async function api(path, method = 'GET', data) {
    const response = await fetch(`/api/${path}`, {
      method, credentials: 'same-origin', redirect: 'manual', cache: 'no-store',
      headers: { 'Content-Type': 'application/json' }, body: data === undefined ? undefined : encode(data),
      signal: AbortSignal.timeout(20000),
    });
    if (response.type === 'opaqueredirect' || response.status === 401 || (response.status >= 300 && response.status < 400)) {
      throw new Error('Your login expired. Your local draft is preserved. Sign in again, then recover the draft.');
    }
    if (!(response.headers.get('Content-Type') || '').includes('application/json')) throw new Error('The character service is unavailable. Your local draft is preserved.');
    const result = await response.json();
    if (!response.ok) throw new Error(result.error || `Request failed (${response.status}). Your draft is preserved.`);
    return result;
  }
  async function task(work) {
    if (busy) return;
    busy = true; render();
    try { await work(); }
    catch (error) { status = error.message || String(error); }
    finally { busy = false; render(); }
  }
  async function refresh() {
    const firstLogin = !user;
    const session = await api('session');
    if (user && user.id !== session.user.id) throw new Error('Account changed. Export your open drafts, then reload before using the new account.');
    user = session.user;
    store ||= new DraftStore(localStorage, user.id);
    characters = await api('characters');
    if (user.admin) users = await api('users');
    else users = [];
    if (!characters.some(c => c.id === selected)) selected = characters[0]?.id || '';
    await details();
    if (firstLogin) status = 'Signed in. Select a character to load, or create one from the simulator.';
  }
  async function details() {
    assignments = []; revisions = [];
    if (user?.admin && selected) {
      assignments = await api(`characters/${selected}/assignments`);
      revisions = await api(`characters/${selected}/revisions`);
    }
  }
  function queueLoad(slot, row) {
    stash(slot, current[slot]);
    // Rust acknowledges successful schema/catalog validation before changing the slot binding.
    pendingLoads[slot] = row;
    loads.push({ slot, document: row.document });
    dialog.close();
  }
  async function save(slot, create = false) {
    const doc = current[slot];
    if (!doc) throw new Error('The simulator is still loading.');
    const row = slots[slot];
    if (!create && !row) throw new Error('Load an assigned character into this slot first.');
    stash(slot, doc);
    // Creation has a separate operation identity even when copying a bound character.
    let savedDraft;
    try {
      savedDraft = create
        ? (creationDrafts[slot] = store.save(creationDrafts[slot], null, 0, doc))
        : (drafts[slot] = store.save(drafts[slot], row.id, row.version, doc));
    } catch { throw new Error('Cannot preserve a draft locally. Export your character before retrying the save.'); }
    const mutation_id = savedDraft.mutation_id;
    // Capture the sent snapshot; edits made while the request is pending remain dirty.
    const sent = structuredClone(doc);
    const result = await api(create ? 'characters' : `characters/${row.id}`, create ? 'POST' : 'PUT', {
      document: sent, version: row?.version, mutation_id,
    });
    slots[slot] = result;
    try {
      store.acknowledge(savedDraft);
      if (drafts[slot] && encode(drafts[slot].document) === encode(sent)) store.acknowledge(drafts[slot]);
    } catch { /* A leftover draft is safe and can be recovered. */ }
    drafts[slot] = null; creationDrafts[slot] = null;
    if (encode(current[slot]) !== encode(sent)) stash(slot, current[slot]);
    status = `${result.name}: saved online (revision ${result.version}).`;
    selected = result.id;
    await refresh();
  }
  function element(tag, text, parent = dialog) {
    const el = document.createElement(tag);
    if (text !== undefined) el.textContent = text;
    parent.append(el); return el;
  }
  function button(label, action, parent = dialog, disabled = false) {
    const el = element('button', label, parent);
    el.style.margin = '4px'; el.disabled = busy || disabled;
    el.onclick = () => task(action); return el;
  }
  function render() {
    const dirty = current.some((doc, i) => doc && encode(doc) !== encode(slots[i]?.document || baselines[i]));
    open.textContent = `TEST · Cloud characters${dirty ? ' • unsaved' : ''}`;
    open.title = status;
    dialog.replaceChildren();
    element('h2', 'Character saves — TEST environment');
    element('p', user ? `${user.email}${user.admin ? ' · Administrator' : ''}` : 'Not signed in');
    const message = element('p', status); message.setAttribute('role', 'status');
    button('Close', () => dialog.close());
    button('Refresh', async () => { await refresh(); status = 'Character list refreshed.'; });
    button('Sign in again', () => location.assign('/'));
    if (user) button('Sign out', () => location.assign('/cdn-cgi/access/logout'));
    if (!user) return;
    element('h3', 'Characters you can edit');
    if (!characters.length) element('p', 'No characters assigned yet. Ask the administrator to assign one after your first login.');
    const select = element('select');
    select.setAttribute('aria-label', 'Assigned character'); select.disabled = busy;
    for (const row of characters) {
      const option = element('option', row.name, select); option.value = row.id; option.selected = row.id === selected;
    }
    select.onchange = () => task(async () => { selected = select.value; await details(); });
    for (let slot = 0; slot < 2; slot++) {
      const section = element('div'); section.style.marginTop = '12px';
      element('strong', `Simulator character ${slot + 1}: ${slots[slot]?.name || 'local character'}`, section);
      button('Load selected', async () => {
        stash(slot, current[slot]);
        const row = await api(`characters/${selected}`);
        queueLoad(slot, row);
      }, section, !selected);
      const savedDrafts = element('select', undefined, section);
      savedDrafts.setAttribute('aria-label', `Local draft for slot ${slot + 1}`);
      let available = [];
      try { available = store.list(selected); } catch { status = 'Could not read local drafts. Export the current character before leaving.'; }
      for (const saved of available) {
        const option = element('option', `${saved.document.player?.name || 'Character'} · ${saved.updatedAt} · based on revision ${saved.version}`, savedDrafts);
        option.value = saved.id;
      }
      button('Recover local draft', async () => {
        const latest = await api(`characters/${selected}`); // Recheck current access before recovery.
        const saved = available.find(d => d.id === savedDrafts.value);
        if (!saved) throw new Error('No local draft for this character on this browser.');
        queueLoad(slot, { ...latest, version: saved.version, document: saved.document, recovered: true, recoveryDraft: saved });
        status = 'Recovered local draft. If the server changed, saving will report a conflict; export the draft before reconciling.';
      }, section, !selected || !available.length);
      button('Save online', () => save(slot), section, !slots[slot]);
      button('Export current', () => download(current[slot], `character-${slot + 1}.json`), section, !current[slot]);
      if (user.admin) {
        button('Create from current / imported preset', () => save(slot, true), section, !current[slot]);
        let creations = [];
        try { creations = store.list(null); } catch { /* Storage error is reported by stash/save. */ }
        const creationSelect = element('select', undefined, section);
        creationSelect.setAttribute('aria-label', `Creation draft for slot ${slot + 1}`);
        for (const saved of creations) {
          const option = element('option', `${saved.document.player?.name || 'Character'} · ${saved.updatedAt}`, creationSelect);
          option.value = saved.id;
        }
        button('Recover local creation draft', () => {
          const saved = creations.find(d => d.id === creationSelect.value);
          if (!saved) throw new Error('No local creation draft in this slot.');
          queueLoad(slot, { importOnly: true, document: saved.document, creationDraft: saved });
        }, section, !creations.length);
      }
    }
    if (!user.admin) return;
    element('p', 'To import an existing fighter preset, select it in the simulator editor, then create a cloud character from that slot. All current editable fields are included.');
    button('Import character JSON', () => {
      const input = document.createElement('input'); input.type = 'file'; input.accept = '.json,application/json';
      input.onchange = () => task(async () => {
        const file = input.files[0]; if (!file) return;
        if (file.size > 256 * 1024) throw new Error('Character file is too large.');
        const parsed = JSON.parse(await file.text());
        const doc = parsed.character?.document || parsed.document || parsed;
        // Validate in Rust before offering creation, using slot one as an import workspace.
        queueLoad(0, { importOnly: true, document: doc });
      });
      input.click();
    });
    if (!selected) return;
    element('h3', 'Editing access');
    for (const person of users) {
      const line = element('div');
      element('span', person.email, line);
      const assigned = assignments.some(a => a.id === person.id);
      button(assigned ? 'Revoke' : 'Assign', async () => {
        await api(`characters/${selected}/assignments`, assigned ? 'DELETE' : 'PUT', { user_id: person.id });
        await details(); status = `Editing access ${assigned ? 'revoked' : 'assigned'} for ${person.email}.`;
      }, line);
    }
    element('h3', 'Revision history');
    element('p', 'Restoring creates a new revision and preserves the existing history.');
    const revision = element('select'); revision.setAttribute('aria-label', 'Revision to restore');
    for (const row of revisions) {
      const option = element('option', `Revision ${row.version} · ${row.updated_at}`, revision); option.value = row.version;
    }
    if (revisions.length && revisions.at(-1).version > 1) button('Load older revisions', async () => {
      const older = await api(`characters/${selected}/revisions?before=${revisions.at(-1).version}`);
      revisions.push(...older);
    });
    button('Restore revision', async () => {
      const row = await api(`characters/${selected}`);
      const result = await api(`characters/${selected}/restore`, 'POST', { version: row.version, restore_version: Number(revision.value), mutation_id: crypto.randomUUID() });
      status = `Restored as revision ${result.version}. Load the character to use it in the simulator.`;
      await refresh();
    }, dialog, !revisions.length);
    button('Export selected saved character', async () => download(await api(`characters/${selected}/export`), 'saved-character.json'));
  }
  window.hackmasterCloud = {
    takeLoads: () => encode(loads.splice(0)),
    publish(slot, text) {
      current[slot] = JSON.parse(text);
      if (!baselines[slot]) baselines[slot] = structuredClone(current[slot]);
      if (!pendingLoads[slot]) stash(slot, current[slot]);
      // Avoid replacing controls while the user is choosing a character/revision.
      open.textContent = `TEST · Cloud characters${current.some((doc, i) => doc && encode(doc) !== encode(slots[i]?.document || baselines[i])) ? ' • unsaved' : ''}`;
      open.title = status;
    },
    loaded(slot, error) {
      const row = pendingLoads[slot]; pendingLoads[slot] = null;
      if (error) { status = error; render(); dialog.showModal(); return; }
      slots[slot] = row.importOnly ? null : row;
      drafts[slot] = row.recoveryDraft ? { ...row.recoveryDraft, id: crypto.randomUUID() } : null;
      creationDrafts[slot] = row.creationDraft || null;
      if (row.recovered) {
        // Recovered text is an unsaved draft, not a server acknowledgement.
        slots[slot] = { ...row, document: null };
      }
      status = row.importOnly ? 'Imported into simulator character 1. Review it, then create a cloud character.' : 'Character loaded. Edit it in the simulator, then Save online.';
      render();
    },
    error(message) { status = message; open.title = message; },
  };
  open.onclick = () => { render(); dialog.showModal(); if (!user) void task(refresh); };
  window.addEventListener('beforeunload', event => {
    if (current.some((doc, i) => doc && encode(doc) !== encode(slots[i]?.document || baselines[i]))) {
      event.preventDefault(); event.returnValue = '';
    }
  });
  render();
  void task(refresh);
}
