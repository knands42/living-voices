-- One complete, playable adventure seeded into every fresh database so the full
-- flow (two acts, act closure, distillation, image prompts) can be exercised
-- without authoring anything first.
INSERT INTO adventure (id, title, headline, tone, status)
VALUES ('7a1c0f42-9d3e-4b6a-8f21-000000000001',
        'The King''s Errand',
        'A summons sealed in green wax, three days of salt road, and a reward the crown has already promised twice.',
        'warm, wry, low-stakes fantasy',
        'private');

INSERT INTO adventure_tag (adventure_id, tag)
VALUES ('7a1c0f42-9d3e-4b6a-8f21-000000000001', 'fantasy'),
       ('7a1c0f42-9d3e-4b6a-8f21-000000000001', 'whimsical'),
       ('7a1c0f42-9d3e-4b6a-8f21-000000000001', 'intimate'),
       ('7a1c0f42-9d3e-4b6a-8f21-000000000001', 'grounded');

INSERT INTO narrator (id, adventure_id, name, instructions)
VALUES ('7a1c0f42-9d3e-4b6a-8f21-000000000002',
        '7a1c0f42-9d3e-4b6a-8f21-000000000001',
        'Narrator',
        'Narrate dry and warm, with a courier''s eye for roads and small mercies. Keep sentences short and let the weather and the gatehouse do the talking.');

INSERT INTO lore (id, adventure_id, text)
VALUES ('7a1c0f42-9d3e-4b6a-8f21-000000000003',
        '7a1c0f42-9d3e-4b6a-8f21-000000000001',
        'The palace at Veyra keeps court above the salt road, and its gate opens for one thing only: a royal seal, carried by hand, unbroken. The crown pays what a promise is worth. A courier who arrives with the seal intact may ask for a reward of their choosing, but the asking must be plain, because the court has no patience for riddles and less for greed.');

INSERT OR IGNORE INTO tag (value)
VALUES ('protagonist'),
       ('ally'),
       ('archivist'),
       ('gate-warden');

INSERT INTO character (id, adventure_id, name, genre, headline, backstory, appearance,
                       appearance_current_body, appearance_current_outfit, appearance_current_pose,
                       relations, message_example, status, is_player)
VALUES ('7a1c0f42-9d3e-4b6a-8f21-000000000011',
        '7a1c0f42-9d3e-4b6a-8f21-000000000001',
        'Iven Ashgrove',
        'male',
        'A hedge-courier with one royal summons, no horse, and three days of salt in his boots.',
        'Iven carried parcels for the harbour guild until a summons with his own name in the wax arrived. The pay was generous enough that he stopped asking who had sent it.',
        'man in his early thirties, dark weathered skin, close-cropped black hair, a pale scar through one eyebrow, lean and road-worn, painterly fantasy illustration, soft morning light',
        NULL,
        'dust-grey travel cloak over a padded linen jacket, boots mended twice, the king''s summons tucked inside the coat against the ribs',
        NULL,
        '["Carries the summons of: the crown"]',
        NULL,
        'present',
        1),
       ('7a1c0f42-9d3e-4b6a-8f21-000000000012',
        '7a1c0f42-9d3e-4b6a-8f21-000000000001',
        'Sera Quill',
        'female',
        'The crown''s archivist, sent to meet the courier before the gate does.',
        'Sera has read every promise the crown ever made and remembers the ones it broke. She meets couriers on the road so the palace cannot pretend it forgot them.',
        'woman in her late forties, brown skin, silver-streaked hair pinned with a brass pen, narrow clever eyes, ink-stained fingers, painterly fantasy illustration, lantern light',
        NULL,
        'long bottle-green scholar''s coat, a chain of small brass keys at the belt, a satchel of rolled ledgers across the body',
        NULL,
        '["Sent to meet: Iven Ashgrove"]',
        'You: "You are late."

Sera: *does not look up from the ledger* "The crown measures lateness in promises, not hours. Yours is still whole, so we are both on time."

You: "And the reward?"

Sera: *turns a page* "Ask plainly when the king asks. Court rewards grow teeth when they are dressed up."',
        'present',
        0),
       ('7a1c0f42-9d3e-4b6a-8f21-000000000013',
        '7a1c0f42-9d3e-4b6a-8f21-000000000001',
        'Bram Hollis',
        'male',
        'Gate warden of Veyra, forty years of standing between the crown and the road.',
        'Bram has opened the gate for kings, mourners, and liars. He opens it for a courier only when the seal is whole and the story holds.',
        'man in his sixties, broad-shouldered, cropped grey beard, deep-lined face, one eye clouded, painterly fantasy illustration, late afternoon sun',
        NULL,
        'worn plate over a padded gambeson, the crown''s gate badge on a leather cord, gauntlets hooked to his belt',
        NULL,
        '["Guards the gate of: the palace"]',
        'You: "The seal is whole."

Bram: *turns the wax toward the light, once* "Whole is not the same as true. Stand still while I decide which one you brought."

You: "And if you decide wrong?"

Bram: *grunts, hands it back* "Then the road keeps the blame, not me."',
        'present',
        0);

INSERT INTO character_trait (character_id, position, value)
VALUES ('7a1c0f42-9d3e-4b6a-8f21-000000000011', 0, 'Keeps the seal where he can feel it'),
       ('7a1c0f42-9d3e-4b6a-8f21-000000000011', 1, 'Asks what a thing is worth, not what it costs'),
       ('7a1c0f42-9d3e-4b6a-8f21-000000000012', 0, 'Quotes the ledger from memory'),
       ('7a1c0f42-9d3e-4b6a-8f21-000000000012', 1, 'Never repeats a promise without checking it'),
       ('7a1c0f42-9d3e-4b6a-8f21-000000000013', 0, 'Checks the seal before the face'),
       ('7a1c0f42-9d3e-4b6a-8f21-000000000013', 1, 'Speaks in short sentences');

INSERT INTO character_tag (character_id, tag)
VALUES ('7a1c0f42-9d3e-4b6a-8f21-000000000011', 'protagonist'),
       ('7a1c0f42-9d3e-4b6a-8f21-000000000011', 'grounded'),
       ('7a1c0f42-9d3e-4b6a-8f21-000000000012', 'ally'),
       ('7a1c0f42-9d3e-4b6a-8f21-000000000012', 'archivist'),
       ('7a1c0f42-9d3e-4b6a-8f21-000000000013', 'gate-warden'),
       ('7a1c0f42-9d3e-4b6a-8f21-000000000013', 'grounded');

INSERT INTO act (id, adventure_id, ordinal, goal, trigger, status, running_digest,
                 last_closure_score)
VALUES ('7a1c0f42-9d3e-4b6a-8f21-000000000021',
        '7a1c0f42-9d3e-4b6a-8f21-000000000001',
        0,
        'Reach the palace gate with the king''s seal whole and unbroken.',
        NULL,
        'active',
        'Iven Ashgrove carried the king''s summons three days up the salt road, arriving late but with the seal unbroken. Sera Quill, keeper of the crown''s promises, met him before the gate and vouched for the hand that carried it. At the gatehouse, warden Bram Hollis turned the seal over, accepted it, and opened the way. The reward has not yet been named.',
        4),
       ('7a1c0f42-9d3e-4b6a-8f21-000000000022',
        '7a1c0f42-9d3e-4b6a-8f21-000000000001',
        1,
        'Stand before the king and name the reward the errand is owed.',
        'The warden accepts the seal and the gate opens.',
        'unstarted',
        NULL,
        0);

INSERT INTO fact (id, adventure_id, facts)
VALUES ('7a1c0f42-9d3e-4b6a-8f21-000000000031',
        '7a1c0f42-9d3e-4b6a-8f21-000000000001',
        '[{"subject": "the king''s summons", "fact": "Must be carried by hand to the palace gate and delivered sealed."},
          {"subject": "the reward", "fact": "Is whatever the courier asks for plainly, once the seal is accepted."},
          {"subject": "Sera Quill", "fact": "Met the courier on the road to check the seal before the gate."},
          {"subject": "Bram Hollis", "fact": "Will not open the gate until he has turned the seal over himself."},
          {"subject": "Iven Ashgrove", "fact": "Arrived late with the seal unbroken."}]');

INSERT INTO state (id, adventure_id, day, place, characters_present, open_threads,
                   character_conditions)
VALUES ('7a1c0f42-9d3e-4b6a-8f21-000000000032',
        '7a1c0f42-9d3e-4b6a-8f21-000000000001',
        'Third morning of the king''s summons',
        'Below the gate of Veyra palace',
        '["Sera Quill", "Bram Hollis"]',
        '["The seal is whole but the warden has not yet sworn to it", "No one has named the reward out loud"]',
        '[{"character": "Sera Quill", "note": "ink on her gloves, ledger open"},
          {"character": "Bram Hollis", "note": "still in his gate harness, spear grounded"}]');

INSERT INTO message (adventure_id, seq, character_id, narrator_id, text)
VALUES ('7a1c0f42-9d3e-4b6a-8f21-000000000001', 1, NULL,
        '7a1c0f42-9d3e-4b6a-8f21-000000000002',
        'The summons found you at the Bell and Barrow, sealed in green wax and heavy as a coin. Three days of salt road later the palace at Veyra stands above the last bend, banners snapping, and a woman in a bottle-green coat is waiting where no one waits.'),
       ('7a1c0f42-9d3e-4b6a-8f21-000000000001', 2,
        '7a1c0f42-9d3e-4b6a-8f21-000000000011', NULL,
        '*shifts the satchel strap without stopping* "If you are here to take it off my hands, you will need a better seal than mine."'),
       ('7a1c0f42-9d3e-4b6a-8f21-000000000001', 3,
        '7a1c0f42-9d3e-4b6a-8f21-000000000012', NULL,
        '*falls into step beside you, a brass pen behind one ear* "I am here to see the seal arrive unbroken, not to carry it. Sera Quill, keeper of the crown''s promises. You are late."'),
       ('7a1c0f42-9d3e-4b6a-8f21-000000000001', 4,
        '7a1c0f42-9d3e-4b6a-8f21-000000000011', NULL,
        '"The road was longer than the map."'),
       ('7a1c0f42-9d3e-4b6a-8f21-000000000001', 5,
        '7a1c0f42-9d3e-4b6a-8f21-000000000012', NULL,
        '*opens a small ledger without slowing* "The map is the crown''s, so that is also the crown''s problem. The warden will not care. Bram Hollis has turned away better stories than yours."'),
       ('7a1c0f42-9d3e-4b6a-8f21-000000000001', 6, NULL,
        '7a1c0f42-9d3e-4b6a-8f21-000000000002',
        'The gatehouse throws a long shadow across the road. A grey-bearded warden watches from the arch, spear grounded, the crown''s badge dull at his throat. He does not step aside.'),
       ('7a1c0f42-9d3e-4b6a-8f21-000000000001', 7,
        '7a1c0f42-9d3e-4b6a-8f21-000000000013', NULL,
        '"Stop there. Seal first, face second. That is how it is done."'),
       ('7a1c0f42-9d3e-4b6a-8f21-000000000001', 8,
        '7a1c0f42-9d3e-4b6a-8f21-000000000011', NULL,
        '*draws the summons from inside the coat and holds the wax toward the light* "Unbroken. Count the days on the road if you like. The seal will not change."'),
       ('7a1c0f42-9d3e-4b6a-8f21-000000000001', 9,
        '7a1c0f42-9d3e-4b6a-8f21-000000000013', NULL,
        '*takes it in both hands, turns it once, and grunts* "It is whole. And late." *his clouded eye finds the woman* "Quill. You vouch for the hand that carried it?"'),
       ('7a1c0f42-9d3e-4b6a-8f21-000000000001', 10,
        '7a1c0f42-9d3e-4b6a-8f21-000000000012', NULL,
        '"I do. I read his name in the ledger before he was born to it."'),
       ('7a1c0f42-9d3e-4b6a-8f21-000000000001', 11,
        '7a1c0f42-9d3e-4b6a-8f21-000000000013', NULL,
        '*hands the summons back, then sets his palm against the gate* "Then the road is finished. Whatever the king owes you, ask it plainly and do not dress it up. His patience is shorter than mine."'),
       ('7a1c0f42-9d3e-4b6a-8f21-000000000001', 12, NULL,
        '7a1c0f42-9d3e-4b6a-8f21-000000000002',
        'The gate begins to open. Beyond it the courtyard is bright and loud with the business of a court that has been waiting. The king is at the far end, and he is looking at you.');

UPDATE act
SET from_seq      = (SELECT MIN(seq) FROM message
                     WHERE adventure_id = '7a1c0f42-9d3e-4b6a-8f21-000000000001'),
    to_seq        = (SELECT MAX(seq) FROM message
                     WHERE adventure_id = '7a1c0f42-9d3e-4b6a-8f21-000000000001'),
    digest_to_seq = (SELECT MAX(seq) FROM message
                     WHERE adventure_id = '7a1c0f42-9d3e-4b6a-8f21-000000000001')
WHERE id = '7a1c0f42-9d3e-4b6a-8f21-000000000021';
