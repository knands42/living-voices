DELETE FROM message
WHERE adventure_id = '7a1c0f42-9d3e-4b6a-8f21-000000000001';

DELETE FROM character_tag
WHERE character_id IN (SELECT id FROM character
                       WHERE adventure_id = '7a1c0f42-9d3e-4b6a-8f21-000000000001');

DELETE FROM character_trait
WHERE character_id IN (SELECT id FROM character
                       WHERE adventure_id = '7a1c0f42-9d3e-4b6a-8f21-000000000001');

DELETE FROM character
WHERE adventure_id = '7a1c0f42-9d3e-4b6a-8f21-000000000001';

DELETE FROM act
WHERE adventure_id = '7a1c0f42-9d3e-4b6a-8f21-000000000001';

DELETE FROM fact
WHERE adventure_id = '7a1c0f42-9d3e-4b6a-8f21-000000000001';

DELETE FROM state
WHERE adventure_id = '7a1c0f42-9d3e-4b6a-8f21-000000000001';

DELETE FROM lore
WHERE adventure_id = '7a1c0f42-9d3e-4b6a-8f21-000000000001';

DELETE FROM narrator
WHERE adventure_id = '7a1c0f42-9d3e-4b6a-8f21-000000000001';

DELETE FROM adventure_tag
WHERE adventure_id = '7a1c0f42-9d3e-4b6a-8f21-000000000001';

DELETE FROM adventure
WHERE id = '7a1c0f42-9d3e-4b6a-8f21-000000000001';

DELETE FROM tag
WHERE value IN ('protagonist', 'ally', 'archivist', 'gate-warden')
  AND NOT EXISTS (SELECT 1 FROM adventure_tag WHERE adventure_tag.tag = tag.value)
  AND NOT EXISTS (SELECT 1 FROM character_tag WHERE character_tag.tag = tag.value);
