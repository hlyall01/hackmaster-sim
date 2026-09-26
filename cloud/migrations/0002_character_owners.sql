CREATE TABLE character_owners (
  character_id TEXT PRIMARY KEY REFERENCES characters(id),
  user_id TEXT NOT NULL REFERENCES users(id)
);
-- Recover the original creator, not the most recent editor.
INSERT INTO character_owners SELECT character_id,updated_by FROM revisions WHERE version=1;
CREATE INDEX character_owners_user ON character_owners(user_id,character_id);
CREATE TRIGGER character_owner_created AFTER INSERT ON characters BEGIN
  INSERT INTO character_owners VALUES(NEW.id,NEW.updated_by);
END;
