BEGIN;

drop domain if exists build_version cascade;
create domain build_version as varchar(64);

drop table if exists architecture;
create table architecture (
  architecture_id uuid unique primary key default gen_random_uuid(), -- The unique architecture ID.
  label varchar(256) not null, -- A human-readable label for the architecture
  os varchar(64) not null, -- The operating system for the architecture
  arch varchar(64) not null -- The architecture of the machine.
);

insert into architecture (label, os, arch)
  values
    ('Raspberry Pi', 'raspbian', 'arm'),
    ('Libre', 'ubuntu', 'arm')
  ;

alter table host
  add architecture_id uuid, -- The architecture of the current host.
  add constraint fk_architecture foreign key (architecture_id) references architecture(architecture_id)
;

update host
  set architecture_id = (select architecture_id from architecture where os = 'raspbian')
  where host_hostname = 'antworker000.hosts.typesofants.org'
;

update host
  set architecture_id = (select architecture_id from architecture where os = 'ubuntu')
  where host_hostname = 'antworker001.hosts.typesofants.org'
;

update host
  set architecture_id = (select architecture_id from architecture where os = 'ubuntu')
  where host_hostname = 'antworker002.hosts.typesofants.org'
;

alter table host
  alter column architecture_id set not null
;

insert into migration (migration_label, created_at, updated_at)
  values
    ('add-host-architecture-id-column', now(), now())
;

COMMIT;
