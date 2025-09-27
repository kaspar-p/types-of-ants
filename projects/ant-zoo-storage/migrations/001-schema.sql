BEGIN;

create schema typesofants;
set search_path to typesofants;
alter database typesofants set search_path to typesofants;

create domain identifier as varchar(64);

create or replace function random_string(int) returns text as $$
  select substr(md5(random()::text), 0, $1+1);
$$ language sql;

create table migration (
  migration_id identifier primary key default ('m-' || random_string(8)), -- Unique migration ID
  migration_seq serial not null, -- The sequence number, in order.
  migration_label varchar(255) unique not null, -- Human readable label
  created_at timestamp with time zone not null default now(),
  updated_at timestamp with time zone not null default now()
);

create table project (
  project_id identifier primary key not null, -- Identifiable label, e.g. 'ant-data-farm'

  owned boolean not null, -- Whether we own this project. True for 'ant-data-farm', false for some WebDAV deployments owning backups

  created_at timestamp with time zone not null default now(),
  updated_at timestamp with time zone not null default now(),
  deleted_at timestamp with time zone
);

create table architecture (
  architecture_id identifier primary key,

  rust_target varchar(255), -- The target name for the Rust cross compiler to consume.
  docker_platform varchar(255) -- The docker platform for publishing images meant for this host.
);

create table host (
  host_id identifier primary key,
  
  architecture_id identifier not null, -- The architecture of the host

  constraint fk_architecture foreign key (architecture_id) references architecture(architecture_id)
);

create table project_instance (
  project_instance_id identifier primary key default ('proj-' || random_string(10)),

  project_id identifier not null, -- The project that's deployed.
  host_id identifier not null, -- The host it's deployed onto.

  environment varchar(16) not null, -- "prod", "beta", "dev", ...

  deployment identifier not null, -- The current version/deployment that this instance is running.

  created_at timestamp with time zone not null default now(),
  updated_at timestamp with time zone not null default now(),
  deleted_at timestamp with time zone,

  unique (project_id, host_id), -- Cannot deploy the same project onto the same host more than once.

  constraint fk_project foreign key (project_id) references project(project_id),
  constraint fk_host foreign key (host_id) references host(host_id)
);

create table backup (
  backup_id identifier primary key default ('b-' || random_string(16)), -- Unique ID for the backup.
  
  project_instance_id identifier not null, -- Project instance that this was a backup of.
  destination_storage_instance_id identifier not null, -- The instance of ant-fs or any file server that this host was backed up to.

  created_at timestamp with time zone not null default now(), -- When the backup was started.
  updated_at timestamp with time zone not null default now(), -- When the backup was updated.
  deleted_at timestamp with time zone, -- Null unless the backup was deleted.

  constraint fk_project_instance foreign key (project_instance_id) references project_instance(project_instance_id),
  constraint fk_fs foreign key (destination_storage_instance_id) references project_instance(project_instance_id)
);

insert into migration (migration_label) values
  ('bootstrap-schema')
;

COMMIT;
