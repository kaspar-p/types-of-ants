BEGIN;

alter table archive_object
    add column tek_derivation_key bytea;

insert into migration (migration_label) values ('add-tek-derivation-key');

COMMIT;
