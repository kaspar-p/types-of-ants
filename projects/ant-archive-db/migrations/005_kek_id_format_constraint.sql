BEGIN;

alter table archive_kek_version
  add constraint chk_kek_id_format
    check (kek_id ~ '^[a-z0-9_-]+$');

insert into migration (migration_label) values ('kek-id-format-constraint');

COMMIT;
