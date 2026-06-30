BEGIN;

alter table archive_kek_version
add column alias varchar(16) unique;

insert into migration (migration_label) values ('add-alias-to-kek');

COMMIT;
