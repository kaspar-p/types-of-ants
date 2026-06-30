BEGIN;

alter table archive_storage_node
add column protocol varchar(16) not null default 'https';

insert into migration (migration_label) values ('add-protocol-to-storage-nodes');

COMMIT;
