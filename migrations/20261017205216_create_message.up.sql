-- Message is the immutable conversation log: rows are appended and never updated
-- or deleted, so there is no updated_at column and no touch trigger
CREATE TABLE message
(
    seq                   INTEGER PRIMARY KEY AUTOINCREMENT,
    adventure_id          TEXT    NOT NULL REFERENCES adventure (id) ON DELETE CASCADE,
    character_id          TEXT    NULL REFERENCES character (id) ON DELETE SET NULL,
    narrator_id           TEXT    NULL REFERENCES narrator (id) ON DELETE SET NULL,
    speaker_name          TEXT    NOT NULL,
    text                  TEXT    NOT NULL,
    targeted_characters   TEXT    NOT NULL DEFAULT "[]",   -- json: ["<character-uuid>", ...]
    ooc                   INTEGER NOT NULL DEFAULT 0,
    created_at            TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),

    CONSTRAINT message_text_not_blank CHECK (length(trim(text)) > 0),
    CONSTRAINT message_speaker_not_blank CHECK (length(trim(speaker_name)) > 0),
    CONSTRAINT message_ooc_is_bool CHECK (ooc IN (0, 1))
);

CREATE INDEX message_adventure_seq_idx ON message (adventure_id, seq);
