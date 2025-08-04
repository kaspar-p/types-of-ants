BEGIN;

insert into migration (migration_label, migration_seq, created_at, updated_at)
values
  ('backfill-001-bootstrap-schema', 27, now(), now()),
  ('backfill-002-ant', 28, now(), now()),
  ('backfill-003-release', 29, now(), now()),
  ('backfill-004-ant-release', 30, now(), now()),
  ('backfill-005-ant-tweeted', 31, now(), now()),
  ('backfill-006-ant-declined', 32, now(), now())
;

insert into migration (migration_label, migration_seq, created_at, updated_at)
values
  ('backfill-migrations-001-through-006', 33, now(), now())
;

COMMIT;
