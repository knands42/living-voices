-- one row per act. Acts are the ONLY story unit:
-- An act lasts roughly 200-600 messages.
CREATE TABLE act (
  id          TEXT    NOT NULL PRIMARY KEY,
  adventure_id TEXT   NOT NULL REFERENCES adventure (id) ON DELETE CASCADE,
  ordinal     INTEGER NOT NULL,
  goal        TEXT    NOT NULL,     -- "recover the crown"
  trigger     TEXT,                 -- optional authored entry condition
  status      TEXT    NOT NULL,     -- unstarted|active|resolved|failed|bypassed
  index_line  TEXT,                 -- immutable: "ACT 1 Crown: tavern lead, cave trail, vault guardian — crown recovered"
  digest      TEXT,                 -- ~120 words, prose, written at act close
  running_digest TEXT,              -- dont append, only override
  digest_to_seq  INTEGER,           -- mutable watermark: last message seq folded into running_digest
  from_seq    INTEGER,              -- first message seq covered
  to_seq      INTEGER,              -- last message seq covered
  created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),

  CONSTRAINT act_id_is_uuid CHECK (length(id) = 36),
  CONSTRAINT act_goal_not_blank CHECK (length(trim(goal)) > 0),
  CONSTRAINT act_status_known
      CHECK (status IN ('unstarted', 'active', 'resolved', 'failed', 'bypassed')),
  CONSTRAINT act_span_ordered CHECK (from_seq IS NULL OR to_seq IS NULL OR to_seq >= from_seq)
);

CREATE INDEX act_adventure_ordinal_idx ON act (adventure_id, ordinal);

CREATE TRIGGER act_touch_updated_at
    AFTER UPDATE ON act
    FOR EACH ROW
    WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE act
    SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    WHERE id = OLD.id;
END;

-- facts are rows, not prose
CREATE TABLE fact (
  id             TEXT NOT NULL PRIMARY KEY,
  adventure_id   TEXT NOT NULL REFERENCES adventure (id) ON DELETE CASCADE,
  subject        TEXT NOT NULL,     -- "elara", "the crown", "the order"
  fact           TEXT NOT NULL,
  act_id         TEXT NOT NULL REFERENCES act (id) ON DELETE SET NULL,
  permanence     TEXT NOT NULL,     -- permanent | plot
  created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),

  CONSTRAINT fact_id_is_uuid CHECK (length(id) = 36),
  CONSTRAINT fact_subject_not_blank CHECK (length(trim(subject)) > 0),
  CONSTRAINT fact_fact_not_blank CHECK (length(trim(fact)) > 0),
  CONSTRAINT fact_permanence_known
      CHECK (permanence IN ('permanent', 'plot', 'scene')),
  CONSTRAINT fact_not_self_superseded CHECK (superseded_by IS NULL OR superseded_by <> id)
);

CREATE INDEX fact_adventure_subject_idx ON fact (adventure_id, subject);
CREATE INDEX fact_act_idx ON fact (act_id);

-- exactly one live row per adventure
CREATE TABLE state (
  id            TEXT NOT NULL PRIMARY KEY,
  adventure_id  TEXT NOT NULL UNIQUE REFERENCES adventure (id) ON DELETE CASCADE,
  day           TEXT,               -- "Day 2, dusk"
  place         TEXT,               -- "cave mouth"
  characters_present       TEXT,               -- json: ["kaelen","elara"]
  open_threads  TEXT,               -- json: ["crown trail","blood on the stones"]
  conditions        TEXT,               -- json: ["kaelen: left arm, minor", "mood: tense","storm approaching"]
  updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),

  CONSTRAINT state_id_is_uuid CHECK (length(id) = 36)
);

CREATE TRIGGER state_touch_updated_at
    AFTER UPDATE ON state
    FOR EACH ROW
    WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE state
    SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    WHERE id = OLD.id;
END;
