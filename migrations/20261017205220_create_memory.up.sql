-- ACT          (story movement — "Find the crown")
--  └── EPISODE     (one scene — the retrieval unit)
--       └── MESSAGES (verbatim turns — the raw log)

-- one row per act. Acts are the ONLY story unit:
-- An act lasts roughly 200-600 messages.
CREATE TABLE acts (
  id          TEXT    NOT NULL PRIMARY KEY,
  adventure_id TEXT   NOT NULL REFERENCES adventure (id) ON DELETE CASCADE,
  ordinal     INTEGER NOT NULL,
  goal        TEXT    NOT NULL,     -- "recover the crown"
  trigger     TEXT,                 -- optional authored entry condition
  status      TEXT    NOT NULL,     -- unstarted|active|resolved|failed|bypassed
  index_line  TEXT,                 -- "ACT 1 Crown: tavern lead, cave trail, vault guardian — crown recovered"
  digest      TEXT,                 -- ~120 words, prose, written at act close
  from_seq    INTEGER,              -- first message seq covered
  to_seq      INTEGER,              -- last message seq covered
  created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),

  CONSTRAINT acts_id_is_uuid CHECK (length(id) = 36),
  CONSTRAINT acts_goal_not_blank CHECK (length(trim(goal)) > 0),
  CONSTRAINT acts_status_known
      CHECK (status IN ('unstarted', 'active', 'resolved', 'failed', 'bypassed')),
  CONSTRAINT acts_span_ordered CHECK (from_seq IS NULL OR to_seq IS NULL OR to_seq >= from_seq)
);

CREATE INDEX acts_adventure_ordinal_idx ON acts (adventure_id, ordinal);

CREATE TRIGGER acts_touch_updated_at
    AFTER UPDATE ON acts
    FOR EACH ROW
    WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE acts
    SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    WHERE id = OLD.id;
END;

-- one row per closed episode (a scene): the retrieval unit.
-- from_seq/to_seq tell the tail what it may drop.
CREATE TABLE episodes (
  id            TEXT    NOT NULL PRIMARY KEY,
  adventure_id  TEXT    NOT NULL REFERENCES adventure (id) ON DELETE CASCADE,
  act_id        TEXT    NULL REFERENCES acts (id) ON DELETE SET NULL,
  scene         TEXT    NOT NULL,   -- "Millbrook tavern", "cave mouth"
  summary       TEXT    NOT NULL,   -- 2-3 sentences
  importance    INTEGER NOT NULL,   -- 1-10, scored at write time
  entities      TEXT    NOT NULL,   -- json: ["elara","vault","rune-key"]
  embedding     BLOB,               -- optional small vector
  from_seq      INTEGER NOT NULL,
  to_seq        INTEGER NOT NULL,
  created_at    TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),

  CONSTRAINT episodes_id_is_uuid CHECK (length(id) = 36),
  CONSTRAINT episodes_scene_not_blank CHECK (length(trim(scene)) > 0),
  CONSTRAINT episodes_summary_not_blank CHECK (length(trim(summary)) > 0),
  CONSTRAINT episodes_importance_range CHECK (importance BETWEEN 1 AND 10),
  CONSTRAINT episodes_span_ordered CHECK (to_seq >= from_seq)
);

CREATE INDEX episodes_adventure_seq_idx ON episodes (adventure_id, from_seq);
CREATE INDEX episodes_act_idx ON episodes (act_id);

-- facts are rows, not prose. The fact outlives the episode that produced it:
-- episode_id is provenance (a citation), not a retrieval key.
CREATE TABLE facts (
  id             TEXT NOT NULL PRIMARY KEY,
  adventure_id   TEXT NOT NULL REFERENCES adventure (id) ON DELETE CASCADE,
  subject        TEXT NOT NULL,     -- "elara", "the crown", "the order"
  fact           TEXT NOT NULL,
  episode_id     TEXT NULL REFERENCES episodes (id) ON DELETE SET NULL,
  permanence     TEXT NOT NULL,     -- permanent | plot | scene
  superseded_by  TEXT NULL REFERENCES facts (id) ON DELETE SET NULL,
  created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),

  CONSTRAINT facts_id_is_uuid CHECK (length(id) = 36),
  CONSTRAINT facts_subject_not_blank CHECK (length(trim(subject)) > 0),
  CONSTRAINT facts_fact_not_blank CHECK (length(trim(fact)) > 0),
  CONSTRAINT facts_permanence_known
      CHECK (permanence IN ('permanent', 'plot', 'scene')),
  CONSTRAINT facts_not_self_superseded CHECK (superseded_by IS NULL OR superseded_by <> id)
);

CREATE INDEX facts_adventure_subject_idx ON facts (adventure_id, subject);
CREATE INDEX facts_episode_idx ON facts (episode_id);

-- exactly one live row per adventure
CREATE TABLE state (
  id            TEXT NOT NULL PRIMARY KEY,
  adventure_id  TEXT NOT NULL UNIQUE REFERENCES adventure (id) ON DELETE CASCADE,
  day           TEXT,               -- "Day 2, dusk"
  place         TEXT,               -- "cave mouth"
  present       TEXT,               -- json: ["kaelen","elara"]
  open_threads  TEXT,               -- json: ["crown trail","blood on the stones"]
  wounds        TEXT,               -- json: ["kaelen: left arm, minor"]
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
