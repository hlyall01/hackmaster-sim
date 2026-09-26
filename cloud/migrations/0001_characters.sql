CREATE TABLE users (
  id TEXT PRIMARY KEY,
  email TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
-- Administrators are bootstrapped through a trusted database operation, never a client request.
CREATE TABLE administrators (user_id TEXT PRIMARY KEY REFERENCES users(id));
CREATE TABLE characters (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  document TEXT NOT NULL CHECK(json_valid(document)),
  version INTEGER NOT NULL CHECK(version > 0),
  updated_by TEXT NOT NULL REFERENCES users(id),
  updated_at TEXT NOT NULL,
  mutation_id TEXT NOT NULL UNIQUE
);
CREATE TABLE assignments (
  character_id TEXT NOT NULL REFERENCES characters(id),
  user_id TEXT NOT NULL REFERENCES users(id),
  assigned_by TEXT NOT NULL REFERENCES users(id),
  PRIMARY KEY(character_id,user_id)
);
CREATE INDEX assignments_user ON assignments(user_id,character_id);
CREATE TABLE revisions (
  character_id TEXT NOT NULL REFERENCES characters(id),
  version INTEGER NOT NULL,
  name TEXT NOT NULL,
  document TEXT NOT NULL CHECK(json_valid(document)),
  updated_by TEXT NOT NULL REFERENCES users(id),
  updated_at TEXT NOT NULL,
  mutation_id TEXT NOT NULL UNIQUE,
  PRIMARY KEY(character_id,version)
);
-- History and the character update share the same SQLite transaction, even on failure.
CREATE TRIGGER character_created AFTER INSERT ON characters BEGIN
  INSERT INTO revisions VALUES(NEW.id,NEW.version,NEW.name,NEW.document,NEW.updated_by,NEW.updated_at,NEW.mutation_id);
END;
CREATE TRIGGER character_saved AFTER UPDATE ON characters BEGIN
  INSERT INTO revisions VALUES(NEW.id,NEW.version,NEW.name,NEW.document,NEW.updated_by,NEW.updated_at,NEW.mutation_id);
END;
