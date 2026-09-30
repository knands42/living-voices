-- Tags are a shared vocabulary, not a per-owner copy
CREATE TABLE tag
(
    value      TEXT NOT NULL PRIMARY KEY,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),

    CONSTRAINT tag_value_not_blank CHECK (length(trim(value)) > 0),
    CONSTRAINT tag_value_is_lowercase CHECK (value = lower(value))
) WITHOUT ROWID;

CREATE TRIGGER tag_ignore_duplicate_insert
    BEFORE INSERT ON tag
    FOR EACH ROW
    WHEN EXISTS (SELECT 1 FROM tag WHERE value = NEW.value)
BEGIN
    SELECT RAISE(IGNORE);
END;

CREATE TRIGGER tag_ignore_duplicate_update
    BEFORE UPDATE OF value ON tag
    FOR EACH ROW
    WHEN EXISTS (SELECT 1 FROM tag WHERE value = NEW.value AND value <> OLD.value)
BEGIN
    SELECT RAISE(IGNORE);
END;

CREATE TRIGGER tag_touch_updated_at
    AFTER UPDATE ON tag
    FOR EACH ROW
    WHEN NEW.updated_at = OLD.updated_at
BEGIN
    -- NEW.value, not OLD.value: the row has already been renamed by the time an
    -- AFTER trigger runs, so OLD.value no longer identifies it.
    UPDATE tag
    SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    WHERE value = NEW.value;
END;
