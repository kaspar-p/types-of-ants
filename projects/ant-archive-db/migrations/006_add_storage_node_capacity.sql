BEGIN;

alter table archive_storage_node
add column capacity_bytes bigint;

update archive_storage_node
  set capacity_bytes = 0
  where capacity_bytes is null
;

alter table archive_storage_node
alter column capacity_bytes set not null;

insert into migration (migration_label) values ('add-storage-node-capacity');

COMMIT;
