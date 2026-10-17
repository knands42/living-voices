-- A narrator belongs to exactly one adventure and gives it its voice.
CREATE TABLE narrator
(
    id           TEXT    NOT NULL PRIMARY KEY,
    adventure_id TEXT    NOT NULL UNIQUE REFERENCES adventure (id) ON DELETE CASCADE,
    name         TEXT    NOT NULL,
    created_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),

    CONSTRAINT narrator_id_is_uuid CHECK (length(id) = 36),
    CONSTRAINT narrator_name_not_blank CHECK (length(trim(name)) > 0)
) WITHOUT ROWID;

CREATE INDEX narrator_name_idx ON narrator (name);

CREATE TRIGGER narrator_touch_updated_at
    AFTER UPDATE ON narrator
    FOR EACH ROW
    WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE narrator
    SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    WHERE id = OLD.id;
END;
