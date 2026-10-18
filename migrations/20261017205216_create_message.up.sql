-- Message is the immutable conversation log: rows are appended and never updated
-- or deleted, so there is no updated_at column and no touch trigger
CREATE TABLE message
(
    seq                    INTEGER PRIMARY KEY AUTOINCREMENT,
    adventure_id           TEXT    NOT NULL REFERENCES adventure (id) ON DELETE CASCADE,
    character_id           TEXT    NULL REFERENCES character (id) ON DELETE SET NULL,
    narrator_id            TEXT    NULL REFERENCES narrator (id) ON DELETE SET NULL,
    speaker_name           TEXT    NOT NULL,
    kind                   TEXT    NOT NULL,
    text                   TEXT    NOT NULL,
    target_name            TEXT    NULL,
    -- A message carries up to two actions (e.g. whispering while doing something),
    -- each part with its own text and optional target
    secondary_kind         TEXT    NULL,
    secondary_text         TEXT    NULL,
    secondary_target_name  TEXT    NULL,
    created_at             TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),

    CONSTRAINT message_kind_known
        CHECK (kind IN ('narration', 'say', 'do', 'think', 'ooc')),
    CONSTRAINT message_secondary_kind_known
        CHECK (secondary_kind IS NULL OR secondary_kind IN ('narration', 'say', 'do', 'think', 'ooc')),
    CONSTRAINT message_target_kind_known
        CHECK (target_name IS NULL OR kind IN ('say', 'whisper', 'do')),
    CONSTRAINT message_secondary_target_kind_known
        CHECK (secondary_target_name IS NULL OR secondary_kind IN ('say', 'whisper', 'do')),
    CONSTRAINT message_text_not_blank CHECK (length(trim(text)) > 0),
    CONSTRAINT message_secondary_part_complete
        CHECK (
            (secondary_kind IS NULL AND secondary_text IS NULL AND secondary_target_name IS NULL)
            OR (secondary_kind IS NOT NULL AND length(trim(secondary_text)) > 0)
        ),
    CONSTRAINT message_speaker_not_blank CHECK (length(trim(speaker_name)) > 0)
);

CREATE INDEX message_adventure_seq_idx ON message (adventure_id, seq);
