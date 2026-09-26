// Each editing session owns a separate record: reloads, slots and tabs never overwrite
// somebody else's draft. Records are deleted only after their exact snapshot is committed.
export class DraftStore {
  constructor(storage, userId) {
    this.storage = storage;
    this.prefix = `HackmasterSim/cloud-v2/${userId}/`;
  }
  list(characterId) {
    const records = [];
    for (let i = 0; i < this.storage.length; i++) {
      const key = this.storage.key(i);
      if (!key?.startsWith(this.prefix)) continue;
      const record = JSON.parse(this.storage.getItem(key));
      if (record.characterId === characterId) records.push(record);
    }
    return records.sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
  }
  save(previous, characterId, version, document) {
    const same = previous && previous.characterId === characterId && previous.version === version
      && JSON.stringify(previous.document) === JSON.stringify(document);
    const record = {
      id: previous?.id || crypto.randomUUID(), characterId, version,
      document: structuredClone(document),
      mutation_id: same ? previous.mutation_id : crypto.randomUUID(),
      updatedAt: new Date().toISOString(),
    };
    this.storage.setItem(this.prefix + record.id, JSON.stringify(record));
    return record;
  }
  acknowledge(record) {
    const key = this.prefix + record.id;
    const stored = this.storage.getItem(key);
    if (stored && JSON.parse(stored).mutation_id === record.mutation_id) this.storage.removeItem(key);
  }
}
