BEGIN;

update migration set migration_seq = 34 where migration_label = 'web-action-table';
update migration set migration_seq = 35 where migration_label = 'add-ant-comments-and-github-id-links';

alter sequence migration_migration_seq_seq restart with 36;

insert into migration (migration_label, created_at, updated_at)
values
  ('reset-migration-sequence-to-36', now(), now())
;

COMMIT;
