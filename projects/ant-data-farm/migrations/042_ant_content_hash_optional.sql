BEGIN;

alter table ant_release
  -- Support unsigned u32s
  alter column ant_content_hash type bigint,
  -- Make optional so the client-side can order them
  alter column ant_content_hash drop not null
;

insert into migration (migration_label)
values
  ('make-ant-release-ant-content-hash-optional')
;

COMMIT;
