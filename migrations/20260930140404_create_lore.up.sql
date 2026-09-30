-- Adventure.lore is a value object, but it has its own identity (Lore.id)
CREATE TABLE lore
(
    id           TEXT NOT NULL PRIMARY KEY,
    adventure_id TEXT NOT NULL UNIQUE REFERENCES adventure (id) ON DELETE CASCADE,
    text         TEXT NOT NULL DEFAULT '',
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CONSTRAINT lore_id_is_uuid CHECK (length(id) = 36)
) WITHOUT ROWID;

CREATE TRIGGER lore_touch_updated_at
    AFTER UPDATE ON lore
    FOR EACH ROW
    WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE lore
    SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    WHERE id = OLD.id;
END;
