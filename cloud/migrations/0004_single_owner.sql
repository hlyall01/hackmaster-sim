-- Refuse ambiguous legacy assignments instead of choosing an owner silently.
CREATE TABLE ownership_migration_guard (valid INTEGER CHECK(valid=1));
INSERT INTO ownership_migration_guard SELECT CASE WHEN EXISTS(
  SELECT a.character_id FROM assignments a JOIN character_owners o ON o.character_id=a.character_id
  WHERE a.user_id<>o.user_id GROUP BY a.character_id HAVING COUNT(*)>1
) THEN 0 ELSE 1 END;
DROP TABLE ownership_migration_guard;
CREATE TABLE ownership_history (
  id INTEGER PRIMARY KEY,
  character_id TEXT NOT NULL REFERENCES characters(id),
  previous_owner TEXT NOT NULL REFERENCES users(id),
  new_owner TEXT NOT NULL REFERENCES users(id),
  changed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE TRIGGER character_owner_transferred AFTER UPDATE OF user_id ON character_owners
WHEN OLD.user_id<>NEW.user_id BEGIN
  INSERT INTO ownership_history(character_id,previous_owner,new_owner) VALUES(NEW.character_id,OLD.user_id,NEW.user_id);
END;
UPDATE character_owners SET user_id=(SELECT a.user_id FROM assignments a
  WHERE a.character_id=character_owners.character_id AND a.user_id<>character_owners.user_id)
WHERE EXISTS(SELECT 1 FROM assignments a WHERE a.character_id=character_owners.character_id AND a.user_id<>character_owners.user_id);
DROP TABLE assignments;
