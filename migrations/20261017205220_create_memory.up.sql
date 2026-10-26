-- one row per act. Acts are the ONLY story unit:
-- An act lasts roughly 200-600 messages.
CREATE TABLE act
(
    id               TEXT    NOT NULL PRIMARY KEY,
    adventure_id     TEXT    NOT NULL REFERENCES adventure (id) ON DELETE CASCADE,
    ordinal          INTEGER NOT NULL,
    goal             TEXT    NOT NULL,           -- "recover the crown"
    trigger          TEXT,                       -- optional authored entry condition
    status           TEXT    NOT NULL,           -- unstarted|active|resolved|failed|bypassed
    index_line       TEXT,                       -- immutable: "ACT 1 Crown: tavern lead, cave trail, vault guardian — crown recovered"
    digest           TEXT,                       -- ~120 words, prose, written at act close
    from_seq         INTEGER          DEFAULT 0, -- first message seq covered
    to_seq           INTEGER          DEFAULT 0, -- last message seq covered

    running_digest   TEXT,                       -- live act progress: dont append, only override
    digest_to_seq    INTEGER,                    -- live act progress: last message seq folded into running_digest
    last_closure_score INTEGER        NOT NULL DEFAULT 0, -- live act progress: most recent act_closure score (0-5) the autonomous model judged

    created_at       TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at       TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),

    CONSTRAINT act_id_is_uuid CHECK (length(id) = 36),
    CONSTRAINT act_goal_not_blank CHECK (length(trim(goal)) > 0),
    CONSTRAINT act_status_known
        CHECK (status IN ('unstarted', 'active', 'resolved', 'failed', 'bypassed')),
    CONSTRAINT act_span_ordered CHECK (to_seq = 0 OR to_seq >= from_seq),
    CONSTRAINT act_last_closure_range
        CHECK (last_closure_score >= 0 AND last_closure_score <= 5)
);

CREATE INDEX act_adventure_ordinal_idx ON act (adventure_id, ordinal);

CREATE TRIGGER act_touch_updated_at
    AFTER UPDATE
    ON act
    FOR EACH ROW
    WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE act
    SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    WHERE id = OLD.id;
END;

-- facts are rows, not prose
CREATE TABLE fact
(
    id           TEXT NOT NULL PRIMARY KEY,
    adventure_id TEXT NOT NULL REFERENCES adventure (id) ON DELETE CASCADE,
    subject      TEXT NOT NULL, -- "elara", "the crown", "the order"
    fact         TEXT NOT NULL,
    act_id       TEXT NOT NULL REFERENCES act (id) ON DELETE SET NULL,
    permanence   TEXT NOT NULL, -- permanent | plot
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),

    CONSTRAINT fact_id_is_uuid CHECK (length(id) = 36),
    CONSTRAINT fact_subject_not_blank CHECK (length(trim(subject)) > 0),
    CONSTRAINT fact_fact_not_blank CHECK (length(trim(fact)) > 0),
    CONSTRAINT fact_permanence_known
        CHECK (permanence IN ('permanent', 'plot'))
);

CREATE INDEX fact_adventure_subject_idx ON fact (adventure_id, subject);
CREATE INDEX fact_act_idx ON fact (act_id);

-- exactly one live row per adventure
CREATE TABLE state
(
    id                 TEXT NOT NULL PRIMARY KEY,
    adventure_id       TEXT NOT NULL UNIQUE REFERENCES adventure (id) ON DELETE CASCADE,
    day                TEXT NOT NULL DEFAULT "unknown", -- "Day 2, dusk"
    place              TEXT NOT NULL DEFAULT "unknown", -- "cave mouth"
    characters_present TEXT NOT NULL DEFAULT "[]",      -- json: ["kaelen","elara"]
    open_threads       TEXT NOT NULL DEFAULT "[]",      -- json: ["crown trail","blood on the stones"]
    conditions         TEXT NOT NULL DEFAULT "[]",      -- json: ["kaelen: left arm, minor", "mood: tense","storm approaching"]
    updated_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),

    CONSTRAINT state_id_is_uuid CHECK (length(id) = 36)
);

CREATE TRIGGER state_touch_updated_at
    AFTER UPDATE
    ON state
    FOR EACH ROW
    WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE state
    SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    WHERE id = OLD.id;
END;
