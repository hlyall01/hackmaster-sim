-- Retain history for administrator recovery while removing deleted characters from play.
CREATE TABLE character_deletions (
  character_id TEXT PRIMARY KEY REFERENCES characters(id),
  deleted_by TEXT NOT NULL REFERENCES users(id),
  deleted_version INTEGER NOT NULL,
  deleted_at TEXT NOT NULL
);
-- Only the administrator restore endpoint may update a deleted character.
CREATE TRIGGER character_recovered AFTER UPDATE ON characters BEGIN
  DELETE FROM character_deletions WHERE character_id=NEW.id;
END;
