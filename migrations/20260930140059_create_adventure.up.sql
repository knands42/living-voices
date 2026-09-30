-- Adventure is the root aggregate: it owns its lore (1:1) and its characters (1:N)
CREATE TABLE adventure
(
    id             TEXT NOT NULL PRIMARY KEY,
    title          TEXT NOT NULL,
    description    TEXT NOT NULL DEFAULT '',
    genre          TEXT,
    tone           TEXT,
    created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    last_opened_at TEXT,

    CONSTRAINT adventure_id_is_uuid CHECK (length(id) = 36),
    CONSTRAINT adventure_title_not_blank CHECK (length(trim(title)) > 0)
);

CREATE INDEX adventure_recent_idx ON adventure (updated_at DESC);
CREATE INDEX adventure_genre_idx ON adventure (genre);

-- AdventureMetadata.tags: many-to-many against the shared tag vocabulary, so
-- the same tag can be carried by any number of adventures.
CREATE TABLE adventure_tag
(
    adventure_id TEXT NOT NULL REFERENCES adventure (id) ON DELETE CASCADE,
    tag          TEXT NOT NULL REFERENCES tag (value) ON UPDATE CASCADE ON DELETE CASCADE,

    PRIMARY KEY (adventure_id, tag)
) WITHOUT ROWID;

-- Adding a tag the adventure already carries is a no-op rather than an error.
CREATE TRIGGER adventure_tag_ignore_duplicate_insert
    BEFORE INSERT ON adventure_tag
    FOR EACH ROW
    WHEN EXISTS (SELECT 1 FROM adventure_tag
                 WHERE adventure_id = NEW.adventure_id AND tag = NEW.tag)
BEGIN
    SELECT RAISE(IGNORE);
END;


CREATE TRIGGER adventure_touch_updated_at
    AFTER UPDATE ON adventure
    FOR EACH ROW
    WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE adventure
    SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    WHERE id = OLD.id;
END;
