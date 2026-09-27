-- Enforce the limit atomically, including concurrent requests and admin operations.
CREATE TRIGGER character_owner_limit_create BEFORE INSERT ON character_owners
WHEN (SELECT COUNT(*) FROM character_owners o WHERE o.user_id=NEW.user_id
  AND NOT EXISTS(SELECT 1 FROM character_deletions d WHERE d.character_id=o.character_id)) >= 10
BEGIN SELECT RAISE(ABORT,'CHARACTER_LIMIT_10'); END;

CREATE TRIGGER character_owner_limit_transfer BEFORE UPDATE OF user_id ON character_owners
WHEN NEW.user_id<>OLD.user_id
  AND NOT EXISTS(SELECT 1 FROM character_deletions WHERE character_id=NEW.character_id)
  AND (SELECT COUNT(*) FROM character_owners o WHERE o.user_id=NEW.user_id
    AND NOT EXISTS(SELECT 1 FROM character_deletions d WHERE d.character_id=o.character_id)) >= 10
BEGIN SELECT RAISE(ABORT,'CHARACTER_LIMIT_10'); END;

-- Restoring a revision removes the deletion marker in the same transaction.
CREATE TRIGGER character_owner_limit_restore BEFORE DELETE ON character_deletions
WHEN (SELECT COUNT(*) FROM character_owners o
  WHERE o.user_id=(SELECT user_id FROM character_owners WHERE character_id=OLD.character_id)
  AND NOT EXISTS(SELECT 1 FROM character_deletions d WHERE d.character_id=o.character_id)) >= 10
BEGIN SELECT RAISE(ABORT,'CHARACTER_LIMIT_10'); END;
