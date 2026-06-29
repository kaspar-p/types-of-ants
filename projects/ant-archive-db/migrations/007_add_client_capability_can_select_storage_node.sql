BEGIN;

alter table archive_client
add column capability_can_select_storage_node boolean not null default false;

insert into migration (migration_label) values ('add-client-capability-can-select-storage-node');

COMMIT;
