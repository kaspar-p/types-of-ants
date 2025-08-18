BEGIN;

alter table ant
alter column suggested_content type varchar(1024)
;

alter table ant_release
alter column ant_content type varchar(1024)
;

insert into migration (migration_label, created_at, updated_at)
values
  ('increase-ant-content-space', now(), now())
;

COMMIT;
