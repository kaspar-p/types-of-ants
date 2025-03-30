BEGIN;

insert into ant_tweeted (ant_id, tweeted_at)
  values
    ((select ant_id from ant where suggested_content = 'ant so long it looks weird' and created_at = '2022-06-30 19:16:53'), '2023-06-22 02:41:26'),
    ((select ant_id from ant where suggested_content = 'ant that creates contemporary art' and created_at = '2022-05-22 20:11:50'), '2023-06-24 12:18:00'),
    ((select ant_id from ant where suggested_content = 'ant who''s grinding' and created_at = '2022-05-15 05:57:01'), '2023-06-23 23:58:26'),
    ((select ant_id from ant where suggested_content = 'ant who wants to be your friend' and created_at = '2022-05-15 05:50:43'), '2023-06-25 15:27:00'),
    ((select ant_id from ant where suggested_content = 'red ant' and created_at = '2022-04-20 00:11:57'), '2023-06-21 02:29:26')
;

COMMIT;
