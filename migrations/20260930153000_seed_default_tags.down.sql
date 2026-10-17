DELETE FROM tag
WHERE value IN (
    -- Genre / setting
    'fantasy', 'sci-fi', 'historical', 'horror', 'contemporary',
    -- Tone
    'dark', 'light-hearted', 'gritty', 'whimsical', 'romantic', 'comedic',
    -- Scale / scope
    'epic', 'intimate', 'political', 'grounded',
    -- Content notes
    'supernatural', 'mature', 'family-friendly', 'action-heavy'
);
