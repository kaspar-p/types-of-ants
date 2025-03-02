BEGIN;

insert into ant (suggested_content, ant_user_id, created_at)
  values
    ('ant slower than a snail', (select user_id from registered_user where user_name = 'nobody'), '2025-03-17 04:00:00'),
    ('ant bursting through the wall like the kool-aid man', (select user_id from registered_user where user_name = 'nobody'), '2025-03-17 04:00:00'),
    ('swing state voter ant', (select user_id from registered_user where user_name = 'nobody'), '2025-03-17 04:00:00'),
    ('ant tracked by a satellite', (select user_id from registered_user where user_name = 'nobody'), '2025-03-17 04:00:00'),
    ('ant who knows a thing or two about dns', (select user_id from registered_user where user_name = 'nobody'), '2025-03-17 04:00:00'),
    ('the type of ant to dig himself a hole', (select user_id from registered_user where user_name = 'nobody'), '2025-03-17 04:00:00'),
    ('die antwort (german)', (select user_id from registered_user where user_name = 'nobody'), '2025-03-17 04:00:00'),
    ('magnetic ant', (select user_id from registered_user where user_name = 'nobody'), '2025-03-17 04:00:00'),
    ('yummy ant', (select user_id from registered_user where user_name = 'nobody'), '2025-03-17 04:00:00'),
    ('antigone (sophocles)', (select user_id from registered_user where user_name = 'nobody'), '2025-03-17 04:00:00'),
    ('ant who has you under its thumb', (select user_id from registered_user where user_name = 'nobody'), '2025-03-17 04:00:00'),
    ('ant with leverage over you (blackmail ant)', (select user_id from registered_user where user_name = 'nobody'), '2025-03-17 04:00:00'),
    ('ant but it''s really hungry', (select user_id from registered_user where user_name = 'nobody'), '2025-03-17 04:00:00'),
    ('syndey sweeney ant', (select user_id from registered_user where user_name = 'nobody'), '2025-03-17 04:00:00'),
    ('ant hogging all the fully loaded nachos', (select user_id from registered_user where user_name = 'nobody'), '2025-03-17 04:00:00'),
    ('ant dressed like a hotdog (itysl ant)', (select user_id from registered_user where user_name = 'nobody'), '2025-03-17 04:00:00'),
    ('ant stuck in ssl hell', (select user_id from registered_user where user_name = 'nobody'), '2025-03-17 04:00:00'),
    ('ant who _knows_ it''s being followed by other ants', (select user_id from registered_user where user_name = 'nobody'), '2025-03-17 04:00:00'),
    ('ant with huge cake (6krill)', (select user_id from registered_user where user_name = 'nobody'), '2025-03-17 04:00:00'),
    ('constant', (select user_id from registered_user where user_name = 'nobody'), '2025-03-17 04:00:00'),
    ('ant convinced spiders aren''t real (sheltered)', (select user_id from registered_user where user_name = 'nobody'), '2025-03-17 04:00:00'),
    ('ant eating an ant', (select user_id from registered_user where user_name = 'nobody'), '2025-03-17 04:00:00'),
    ('ant bringing the whole colony down (negative attitude)', (select user_id from registered_user where user_name = 'nobody'), '2025-03-17 04:00:00'),
    ('ant refuting the allegations', (select user_id from registered_user where user_name = 'nobody'), '2025-03-17 04:00:00'),
    ('ant queen but recently birthed and no subjects', (select user_id from registered_user where user_name = 'nobody'), '2025-03-17 04:00:00'),
    ('anthony', (select user_id from registered_user where user_name = 'nobody'), '2025-03-17 04:00:00')
;

insert into release (release_number, release_label)
  values
    (35, 'v35')
;

insert into ant_release (ant_id, release_number, ant_content, ant_content_hash)
  values
    ((select ant_id from ant where suggested_content = 'ant slower than a snail' and created_at = '2025-03-17 04:00:00'), 35, 'ant slower than a snail', 355716548),
    ((select ant_id from ant where suggested_content = 'ant bursting through the wall like the kool-aid man' and created_at = '2025-03-17 04:00:00'), 35, 'ant bursting through the wall like the kool-aid man', 184927356),
    ((select ant_id from ant where suggested_content = 'swing state voter ant' and created_at = '2025-03-17 04:00:00'), 35, 'swing state voter ant', 870568607),
    ((select ant_id from ant where suggested_content = 'ant tracked by a satellite' and created_at = '2025-03-17 04:00:00'), 35, 'ant tracked by a satellite', 1746143235),
    ((select ant_id from ant where suggested_content = 'ant who knows a thing or two about dns' and created_at = '2025-03-17 04:00:00'), 35, 'ant who knows a thing or two about dns', 1596060841),
    ((select ant_id from ant where suggested_content = 'the type of ant to dig himself a hole' and created_at = '2025-03-17 04:00:00'), 35, 'the type of ant to dig himself a hole', 647176000),
    ((select ant_id from ant where suggested_content = 'die antwort (german)' and created_at = '2025-03-17 04:00:00'), 35, 'die antwort (german)', 1496262043),
    ((select ant_id from ant where suggested_content = 'magnetic ant' and created_at = '2025-03-17 04:00:00'), 35, 'magnetic ant', 1561910731),
    ((select ant_id from ant where suggested_content = 'yummy ant' and created_at = '2025-03-17 04:00:00'), 35, 'yummy ant', 1426231685),
    ((select ant_id from ant where suggested_content = 'antigone (sophocles)' and created_at = '2025-03-17 04:00:00'), 35, 'antigone (sophocles)', 500576000),
    ((select ant_id from ant where suggested_content = 'ant who has you under its thumb' and created_at = '2025-03-17 04:00:00'), 35, 'ant who has you under its thumb', 97803125),
    ((select ant_id from ant where suggested_content = 'ant with leverage over you (blackmail ant)' and created_at = '2025-03-17 04:00:00'), 35, 'ant with leverage over you (blackmail ant)', 666792369),
    ((select ant_id from ant where suggested_content = 'ant but it''s really hungry' and created_at = '2025-03-17 04:00:00'), 35, 'ant but it''s really hungry', 978380786),
    ((select ant_id from ant where suggested_content = 'syndey sweeney ant' and created_at = '2025-03-17 04:00:00'), 35, 'syndey sweeney ant', 479138838),
    ((select ant_id from ant where suggested_content = 'ant hogging all the fully loaded nachos' and created_at = '2025-03-17 04:00:00'), 35, 'ant hogging all the fully loaded nachos', 418591286),
    ((select ant_id from ant where suggested_content = 'ant dressed like a hotdog (itysl ant)' and created_at = '2025-03-17 04:00:00'), 35, 'ant dressed like a hotdog (itysl ant)', 793677304),
    ((select ant_id from ant where suggested_content = 'ant stuck in ssl hell' and created_at = '2025-03-17 04:00:00'), 35, 'ant stuck in ssl hell', 1310234351),
    ((select ant_id from ant where suggested_content = 'ant who _knows_ it''s being followed by other ants' and created_at = '2025-03-17 04:00:00'), 35, 'ant who _knows_ it''s being followed by other ants', 1705999227),
    ((select ant_id from ant where suggested_content = 'ant with huge cake (6krill)' and created_at = '2025-03-17 04:00:00'), 35, 'ant with huge cake (6krill)', 775431338),
    ((select ant_id from ant where suggested_content = 'constant' and created_at = '2025-03-17 04:00:00'), 35, 'constant', 1579672485),
    ((select ant_id from ant where suggested_content = 'ant convinced spiders aren''t real (sheltered)' and created_at = '2025-03-17 04:00:00'), 35, 'ant convinced spiders aren''t real (sheltered)', 164892162),
    ((select ant_id from ant where suggested_content = 'ant eating an ant' and created_at = '2025-03-17 04:00:00'), 35, 'ant eating an ant', 1734199378),
    ((select ant_id from ant where suggested_content = 'ant bringing the whole colony down (negative attitude)' and created_at = '2025-03-17 04:00:00'), 35, 'ant bringing the whole colony down (negative attitude)', 722314645),
    ((select ant_id from ant where suggested_content = 'ant refuting the allegations' and created_at = '2025-03-17 04:00:00'), 35, 'ant refuting the allegations', 66822875),
    ((select ant_id from ant where suggested_content = 'ant queen but recently birthed and no subjects' and created_at = '2025-03-17 04:00:00'), 35, 'ant queen but recently birthed and no subjects', 1769805522),
    ((select ant_id from ant where suggested_content = 'anthony' and created_at = '2025-03-17 04:00:00'), 35, 'anthony', 1300571002)
;

insert into migration (migration_label)
  values
    ('ant-release:2025.2.17')
;

COMMIT;