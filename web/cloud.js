// Enabled only by the isolated character-test deployment's HTML marker.
import { DraftStore } from './cloud-drafts.js';
const enabled = document.querySelector('meta[name="hackmaster-cloud"][content="character-test"]');
if (enabled) installCloudCharacters();

function installCloudCharacters() {
  let user = null;
  let guest = new URLSearchParams(location.search).get('guest') === '1';
  let characters = [];
  let roster = [];
  let users = [];
  let selected = '';
  let adminSelected = '';
  let adminPlayer = '';
  let adminView = false;
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
  const adminOpen = document.createElement('button');
  adminOpen.textContent = 'Admin · Assign characters';
  adminOpen.hidden = true;
  bar.append(adminOpen);
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
    const endpoint = guest && method === 'GET' && /^roster(?:\/[^/]+)?$/.test(path)
      ? '/guest/characters' + path.slice('roster'.length) : `/api/${path}`;
    const response = await fetch(endpoint, {
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
    if (guest) {
      roster = await api('roster');
      characters = []; users = []; assignments = []; revisions = [];
      selected = roster.some(c => c.id === selected) ? selected : roster[0]?.id || '';
      status = 'Guest mode · All characters are available for simulation. Sign in to create or save.';
      return;
    }
    const firstLogin = !user;
    const session = await api('session');
    if (user && user.id !== session.user.id) throw new Error('Account changed. Export your open drafts, then reload before using the new account.');
    user = session.user;
    store ||= new DraftStore(localStorage, user.id);
    characters = await api('characters');
    roster = await api('roster');
    if (user.admin) users = await api('users');
    else users = [];
    if (!roster.some(c => c.id === selected)) selected = roster[0]?.id || '';
    if (!characters.some(c => c.id === adminSelected)) adminSelected = characters[0]?.id || '';
    if (!users.some(p => p.id === adminPlayer)) adminPlayer = users.find(p => !p.admin)?.id || users[0]?.id || '';
    await details();
    if (firstLogin) status = 'Signed in. Open Core → Online Characters to load or save a character.';
  }
  async function details() {
    assignments = []; revisions = [];
    if (user?.admin && adminSelected) {
      assignments = await api(`characters/${adminSelected}/assignments`);
      revisions = await api(`characters/${adminSelected}/revisions`);
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
    if (guest || !user) throw new Error('Sign in to create or save characters.');
    const doc = current[slot];
    if (!doc) throw new Error('The simulator is still loading.');
    const row = slots[slot];
    if (!create && (!row || row.can_edit === 0)) throw new Error('This party character is read-only. Create your own copy to save changes.');
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
    if (create) adminSelected = result.id;
    await refresh();
  }
  async function deleteCharacter(id, version) {
    const row = roster.find(c => c.id === id);
    if (guest || !user || !row?.is_owner) throw new Error('Only the creator can delete this character.');
    if (!window.confirm(`Delete "${row.name}" from My Characters and everyone's Party Members? An administrator can recover it from revision history. Your open simulation copy and local drafts will be kept.`)) return;
    await api(`characters/${id}`, 'DELETE', {version});
    // Do not discard simulation edits or drafts; detach all copies only after commit.
    for (let slot = 0; slot < slots.length; slot++) {
      if (slots[slot]?.id !== id) continue;
      stash(slot, current[slot]);
      slots[slot] = null; drafts[slot] = null;
    }
    roster = roster.filter(c => c.id !== id);
    status = `${row.name}: deleted online. Your open simulation copy is kept.`;
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
    const dirty = !guest && current.some((doc, i) => doc && encode(doc) !== encode(slots[i]?.document || baselines[i]));
    open.textContent = `TEST · Cloud characters${dirty ? ' • unsaved' : ''}`;
    open.title = status;
    dialog.replaceChildren();
    adminOpen.hidden = !user?.admin;
    element('h2', adminView && user?.admin ? 'Assign characters — TEST environment' : 'Character saves — TEST environment');
    element('p', user ? `${user.email}${user.admin ? ' · Administrator' : ''}` : guest ? 'Guest · Simulation only' : 'Choose how to continue');
    const message = element('p', status); message.setAttribute('role', 'status');
    button('Close', () => dialog.close());
    button('Refresh', async () => { await refresh(); status = 'Character list refreshed.'; });
    button(user ? 'Sign in again' : 'Sign in with Google', () => location.assign('/api/login'));
    if (!user) button('Continue without logging in', async () => {
      guest = true;
      const url = new URL(location.href); url.searchParams.set('guest', '1');
      history.replaceState(null, '', url);
      await refresh(); dialog.close();
    });
    if (user) button('Sign out', async () => {
      // Process Access's cookie clearing without navigating to its login-only page.
      const result = await fetch('/cdn-cgi/access/logout', {credentials:'same-origin', redirect:'manual', signal:AbortSignal.timeout(20000)});
      if (result.status >= 400) throw new Error('Sign out failed. Please try again.');
      location.assign('/?guest=1');
    });
    if (!user) return;
    if (adminView && user.admin) { renderAdmin(); return; }
    element('h3', 'Online characters and local drafts');
    if (!roster.length) element('p', 'No online characters yet. Create your first character from Core.');
    const select = element('select');
    select.setAttribute('aria-label', 'Assigned character'); select.disabled = busy;
    for (const row of roster) {
      const option = element('option', row.name + (row.can_edit ? '' : ' · simulation copy'), select); option.value = row.id; option.selected = row.id === selected;
    }
    select.onchange = () => task(async () => { selected = select.value; await details(); });
    const chosen = roster.find(c => c.id === selected);
    if (chosen?.is_owner) button('Delete my character', () => deleteCharacter(chosen.id, chosen.version));
    for (let slot = 0; slot < 2; slot++) {
      const section = element('div'); section.style.marginTop = '12px';
      element('strong', `Simulator character ${slot + 1}: ${slots[slot]?.name || 'local character'}`, section);
      button('Load selected', async () => {
        stash(slot, current[slot]);
        const row = await api(`roster/${selected}`);
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
        const latest = await api(`roster/${selected}`); // Recheck current edit rights; party copies remain read-only.
        const saved = available.find(d => d.id === savedDrafts.value);
        if (!saved) throw new Error('No local draft for this character on this browser.');
        queueLoad(slot, { ...latest, version: saved.version, document: saved.document, recovered: true, recoveryDraft: saved });
        status = 'Recovered local draft. If the server changed, saving will report a conflict; export the draft before reconciling.';
      }, section, !selected || !available.length);
      button('Save online', () => save(slot), section, !slots[slot] || slots[slot].can_edit === 0);
      button('Export current', () => download(current[slot], `character-${slot + 1}.json`), section, !current[slot]);
      button('Create my character', () => save(slot, true), section, !current[slot]);
      renderCreationDrafts(slot, section);
    }
  }
  function renderCreationDrafts(slot, section) {
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
  function renderAdmin() {
    const selectLabel = element('label', 'Character to manage');
    selectLabel.style.display = 'block';
    selectLabel.style.margin = '16px 0';
    const select = element('select', undefined, selectLabel);
    Object.assign(select.style, {display:'block',width:'100%',padding:'8px',marginTop:'6px'});
    select.setAttribute('aria-label', 'Character to manage'); select.disabled = busy;
    for (const row of characters) {
      const option = element('option', row.name + (row.deleted ? ' · Deleted' : ''), select); option.value = row.id; option.selected = row.id === adminSelected;
    }
    select.onchange = () => task(async () => { adminSelected = select.value; await details(); });
    if (!characters.length) element('p', 'No cloud characters yet. Add one below to assign it.');
    if (adminSelected) {
      const name = characters.find(c => c.id === adminSelected)?.name || 'Character';
      element('h3', `Assign ${name}`);
      const playerLabel = element('label', 'Player');
      const player = element('select', undefined, playerLabel);
      Object.assign(player.style, {display:'block',width:'100%',padding:'8px',margin:'6px 0'});
      player.setAttribute('aria-label', 'Player to assign'); player.disabled = busy;
      for (const person of users) {
        const option = element('option', person.email, player); option.value = person.id; option.selected = person.id === adminPlayer;
      }
      player.onchange = () => { adminPlayer = player.value; render(); };
      const person = users.find(p => p.id === adminPlayer);
      const assigned = assignments.some(a => a.id === adminPlayer);
      button(assigned ? 'Already assigned' : 'Assign character', async () => {
        const id = adminSelected;
        await api(`characters/${id}/assignments`, 'PUT', { user_id: person.id });
        await details(); status = `${name} assigned to ${person.email}.`;
      }, dialog, !person || assigned);
      element('p', 'Players appear here after their first sign-in. Administrators can edit every character.');
      element('h3', `Who can edit ${name}`);
      if (!assignments.length) element('p', 'No players assigned.');
      for (const person of assignments) {
        const line = element('div');
        element('span', person.email + (person.is_owner ? ' · Creator (always has access)' : ''), line);
        if (person.is_owner) continue;
        button('Revoke', async () => {
          await api(`characters/${adminSelected}/assignments`, 'DELETE', { user_id: person.id });
          await details(); status = `${name}: editing access revoked for ${person.email}.`;
        }, line);
      }
    }
    const manage = element('details');
    element('summary', 'Add a cloud character', manage);
    const panel = element('div', undefined, manage);
    panel.style.padding = '8px 0';
    for (let slot = 0; slot < 2; slot++) {
      const section = element('div', undefined, panel);
      element('p', `Simulator character ${slot + 1}: ${current[slot]?.player?.name || 'Loading…'}`, section);
      {
        button('Create from current / imported preset', () => save(slot, true), section, !current[slot]);
        renderCreationDrafts(slot, section);
      }
    }
    element('p', 'To import an existing fighter preset, select it in the simulator editor, then create a cloud character from that slot. All current editable fields are included.', panel);
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
    }, panel);
    if (!adminSelected) return;
    element('h3', 'Revision history');
    element('p', 'Restoring creates a new revision and preserves the existing history.');
    const revision = element('select'); revision.setAttribute('aria-label', 'Revision to restore');
    for (const row of revisions) {
      const option = element('option', `Revision ${row.version} · ${row.updated_at}`, revision); option.value = row.version;
    }
    if (revisions.length && revisions.at(-1).version > 1) button('Load older revisions', async () => {
      const older = await api(`characters/${adminSelected}/revisions?before=${revisions.at(-1).version}`);
      revisions.push(...older);
    });
    button('Restore revision', async () => {
      const row = await api(`characters/${adminSelected}`);
      const result = await api(`characters/${adminSelected}/restore`, 'POST', { version: row.version, restore_version: Number(revision.value), mutation_id: crypto.randomUUID() });
      status = `Restored as revision ${result.version}. Load the character to use it in the simulator.`;
      await refresh();
    }, dialog, !revisions.length);
    button('Export selected saved character', async () => download(await api(`characters/${adminSelected}/export`), 'saved-character.json'));
  }
  window.hackmasterCloud = {
    snapshot(slot) {
      const row = slots[slot];
      return encode({ signed_in: !!user, guest, busy, status, admin: !!user?.admin,
        characters: roster.map(c => ({id:c.id, name:c.name, is_owner:!!c.is_owner, can_edit:!!c.can_edit})),
        loaded_id: row?.id || '', loaded_name: row?.name || '',
        can_delete: !!user && !guest && !!row && !!roster.find(c => c.id === row.id)?.is_owner,
        can_save: !!user && !guest && !!row && row.can_edit !== 0,
        dirty: !guest && !!current[slot] && encode(current[slot]) !== encode(row?.document || baselines[slot]),
      });
    },
    action(slot, action, id) {
      if (action === 'signin') { location.assign('/api/login'); return; }
      if (action === 'manage') { adminView = false; render(); dialog.showModal(); return; }
      if (guest && !['load', 'refresh'].includes(action)) return;
      if (action === 'admin' && user?.admin) { adminView = true; render(); dialog.showModal(); void task(refresh); return; }
      void task(async () => {
        if (action === 'refresh') { await refresh(); status = 'Online characters refreshed.'; }
        else if (action === 'load') {
          const row = await api(`roster/${id}`);
          queueLoad(slot, row);
        } else if (action === 'save') await save(slot);
        else if (action === 'create') await save(slot, true);
        else if (action === 'delete') {
          const row = slots[slot];
          if (row) await deleteCharacter(row.id, row.version);
        }
      });
    },
    takeLoads: () => encode(loads.splice(0)),
    publish(slot, text) {
      current[slot] = JSON.parse(text);
      if (!baselines[slot]) baselines[slot] = structuredClone(current[slot]);
      if (!pendingLoads[slot]) stash(slot, current[slot]);
      // Avoid replacing controls while the user is choosing a character/revision.
      open.textContent = `TEST · Cloud characters${!guest && current.some((doc, i) => doc && encode(doc) !== encode(slots[i]?.document || baselines[i])) ? ' • unsaved' : ''}`;
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
      status = guest ? 'Party character loaded for simulation. Guest changes cannot be saved.' : row.importOnly ? 'Imported. Review the character, then create an online copy from Core.' : row.can_edit === 0 ? 'Party character loaded for simulation. Create your own copy to save changes.' : 'Character loaded. Edit it, then Save online.';
      render();
    },
    error(message) { status = message; open.title = message; },
  };
  adminOpen.onclick = () => { adminView = true; render(); dialog.showModal(); void task(refresh); };
  open.onclick = () => { adminView = false; render(); dialog.showModal(); if (!user) void task(refresh); };
  window.addEventListener('beforeunload', event => {
    if (!guest && current.some((doc, i) => doc && encode(doc) !== encode(slots[i]?.document || baselines[i]))) {
      event.preventDefault(); event.returnValue = '';
    }
  });
  render();
  void task(async () => {
    try { await refresh(); }
    catch (error) {
      if (guest) throw error;
      status = 'Sign in to create and save your characters, or continue as a guest to simulate with the full party roster.';
      dialog.showModal();
    }
  });
}
