-- Memory is the compressed view of the message log.
CREATE TABLE summary
(
    id           TEXT    NOT NULL PRIMARY KEY,
    adventure_id TEXT    NOT NULL REFERENCES adventure (id) ON DELETE CASCADE,
    text         TEXT    NOT NULL,
    from_seq     INTEGER NOT NULL,
    to_seq       INTEGER NOT NULL,
    created_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),

    CONSTRAINT summary_id_is_uuid CHECK (length(id) = 36),
    CONSTRAINT summary_range_ordered CHECK (from_seq <= to_seq),
    CONSTRAINT summary_range_positive CHECK (from_seq >= 1),
    CONSTRAINT summary_text_not_blank CHECK (length(trim(text)) > 0)
);

-- The cursor lookup is always "latest covered seq for this adventure".
CREATE INDEX summary_adventure_from_seq_to_seq_idx ON summary (adventure_id, from_seq, to_seq);
CREATE INDEX summary_adventure_to_seq_idx ON summary (adventure_id, to_seq);

-- Facts are the atomic, durable statements extracted alongside a summary
CREATE TABLE fact
(
    id                TEXT NOT NULL PRIMARY KEY,
    adventure_id      TEXT NOT NULL REFERENCES adventure (id) ON DELETE CASCADE,
    source_summary_id TEXT NULL REFERENCES summary (id) ON DELETE CASCADE,
    text              TEXT NOT NULL,
    created_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),

    CONSTRAINT fact_id_is_uuid CHECK (length(id) = 36),
    CONSTRAINT fact_text_not_blank CHECK (length(trim(text)) > 0)
);

CREATE INDEX fact_adventure_idx ON fact (adventure_id);
CREATE INDEX fact_source_summary_idx ON fact (source_summary_id);
