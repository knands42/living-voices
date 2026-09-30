-- Default tag vocabulary for adventures and characters.
-- Shared vocabulary: the same tag can label an adventure, a character, or both.
-- Insert or ignore so this migration is safe to re-run on an already-seeded DB.
INSERT OR IGNORE INTO tag (value) VALUES
    -- Genre / setting
    ('fantasy'),
    ('sci-fi'),
    ('historical'),
    ('horror'),
    ('contemporary'),
    -- Tone
    ('dark'),
    ('light-hearted'),
    ('gritty'),
    ('whimsical'),
    ('romantic'),
    ('comedic'),
    -- Scale / scope
    ('epic'),
    ('intimate'),
    ('political'),
    ('grounded'),
    -- Content notes
    ('supernatural'),
    ('mature'),
    ('family-friendly'),
    ('action-heavy');
