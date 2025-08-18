BEGIN;

alter table migration
alter column created_at set default now(),
alter column updated_at set default now()
;

insert into migration (migration_label)
values
  ('set-migration-created-deleted-defaults')
;

COMMIT;
