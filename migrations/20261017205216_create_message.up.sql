-- Message is the immutable conversation log: rows are appended and never updated
-- or deleted, so there is no updated_at column and no touch trigger
CREATE TABLE message
(
    seq          INTEGER PRIMARY KEY AUTOINCREMENT,
    id           TEXT    NOT NULL UNIQUE,
    adventure_id TEXT    NOT NULL REFERENCES adventure (id) ON DELETE CASCADE,
    character_id TEXT    NULL REFERENCES character (id) ON DELETE SET NULL,
    speaker_name TEXT    NOT NULL,
    kind         TEXT    NOT NULL,
    text         TEXT    NOT NULL,
    target_name  TEXT    NULL,
    created_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),

    CONSTRAINT message_id_is_uuid CHECK (length(id) = 36),
    CONSTRAINT message_kind_known
        CHECK (kind IN ('narration', 'say', 'do', 'think', 'whisper')),
    CONSTRAINT message_text_not_blank CHECK (length(trim(text)) > 0),
    CONSTRAINT message_speaker_not_blank CHECK (length(trim(speaker_name)) > 0)
);

CREATE INDEX message_adventure_seq_idx ON message (adventure_id, seq);
