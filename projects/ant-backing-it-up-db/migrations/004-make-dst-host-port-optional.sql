BEGIN;

alter table backup alter column destination_host drop not null;
alter table backup alter column destination_port drop not null;

insert into migration (migration_label) values ('make-destination-host-and-port-optional');

COMMIT;
