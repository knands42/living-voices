-- A character belongs to exactly one adventure
CREATE TABLE character
(
    id            TEXT    NOT NULL PRIMARY KEY,
    adventure_id   TEXT    NOT NULL REFERENCES adventure (id) ON DELETE CASCADE,
    name          TEXT    NOT NULL,
    description   TEXT    NOT NULL DEFAULT '',
    backstory     TEXT    NOT NULL DEFAULT '',
    status        TEXT    NOT NULL DEFAULT 'active',
    portrait_path TEXT,
    created_at    TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at    TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),

    CONSTRAINT character_id_is_uuid CHECK (length(id) = 36),
    CONSTRAINT character_name_not_blank CHECK (length(trim(name)) > 0),
    CONSTRAINT character_status_known CHECK (status IN ('active', 'inactive', 'archived'))
);

CREATE INDEX character_adventure_name_idx ON character (adventure_id, name);
CREATE INDEX character_name_idx ON character (name);

-- Character.traits: an ordered list, so repeats are allowed and position is
-- part of the key.
CREATE TABLE character_trait
(
    character_id TEXT    NOT NULL REFERENCES character (id) ON DELETE CASCADE,
    position     INTEGER NOT NULL,
    value        TEXT    NOT NULL,

    PRIMARY KEY (character_id, position),

    CONSTRAINT character_trait_position_not_negative CHECK (position >= 0),
    CONSTRAINT character_trait_value_not_blank CHECK (length(trim(value)) > 0)
) WITHOUT ROWID;

-- Character.tags: many-to-many against the shared tag vocabulary, so one row
-- of tag can be carried by many characters across many adventures.
CREATE TABLE character_tag
(
    character_id TEXT NOT NULL REFERENCES character (id) ON DELETE CASCADE,
    tag          TEXT NOT NULL REFERENCES tag (value) ON UPDATE CASCADE ON DELETE CASCADE,

    PRIMARY KEY (character_id, tag)
) WITHOUT ROWID;

-- Same no-op-on-duplicate behaviour as adventure_tag.
CREATE TRIGGER character_tag_ignore_duplicate_insert
    BEFORE INSERT ON character_tag
    FOR EACH ROW
    WHEN EXISTS (SELECT 1 FROM character_tag
                 WHERE character_id = NEW.character_id AND tag = NEW.tag)
BEGIN
    SELECT RAISE(IGNORE);
END;

-- Character.relations is a HashMap<Uuid, String>, so there is exactly one
-- relation per target character per direction.
CREATE TABLE character_relation
(
    character_id         TEXT    NOT NULL REFERENCES character (id) ON DELETE CASCADE,
    related_character_id TEXT    NOT NULL REFERENCES character (id) ON DELETE CASCADE,
    relation             TEXT    NOT NULL,
    created_at           TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),

    PRIMARY KEY (character_id, related_character_id),

    CONSTRAINT character_relation_not_self CHECK (character_id <> related_character_id),
    CONSTRAINT character_relation_not_blank CHECK (length(trim(relation)) > 0)
) WITHOUT ROWID;

CREATE TRIGGER character_touch_updated_at
    AFTER UPDATE ON character
    FOR EACH ROW
    WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE character
    SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    WHERE id = OLD.id;
END;
